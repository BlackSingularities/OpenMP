//! Dekodowanie plików audio (Symphonia) do stereo f32 i przepróbkowanie do częstotliwości urządzenia.
//!
//! Gapless: Symphonia przycina opóźnienie i dopełnienie kodera (LAME, iTunSMPB), więc kolejne
//! utwory albumu łączą się bez przerwy.

use std::fs::File;
use std::path::Path;

use rubato::{Resampler as _, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Parametry techniczne źródła (wyświetlacz, informacje o pliku).
#[derive(Clone, Debug, Default)]
pub struct StreamInfo {
    pub codec: String,
    pub rate: u32,
    pub channels: usize,
    pub bits: u32,
    pub bitrate: u32,
    pub duration: f64,
}

/// Otwarty plik: pakiety → stereo f32 w natywnej częstotliwości.
pub struct Source {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    pub info: StreamInfo,
    buf: Option<SampleBuffer<f32>>,
    spec_channels: usize,
    /// Ostatni znany czas (sekundy) – do pozycji po przewinięciu.
    pub time: f64,
}

fn codec_name(c: symphonia::core::codecs::CodecType) -> &'static str {
    use symphonia::core::codecs::*;
    match c {
        CODEC_TYPE_MP3 => "MP3",
        CODEC_TYPE_MP2 => "MP2",
        CODEC_TYPE_MP1 => "MP1",
        CODEC_TYPE_FLAC => "FLAC",
        CODEC_TYPE_AAC => "AAC",
        CODEC_TYPE_ALAC => "ALAC",
        CODEC_TYPE_VORBIS => "Vorbis",
        CODEC_TYPE_OPUS => "Opus",
        CODEC_TYPE_ADPCM_IMA_WAV | CODEC_TYPE_ADPCM_MS => "ADPCM",
        _ => symphonia::default::get_codecs().get_codec(c).map(|d| if d.short_name.starts_with("pcm") { "PCM" } else { "Audio" }).unwrap_or("Audio"),
    }
}

impl Source {
    pub fn open(path: &Path) -> Result<Source, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions { enable_gapless: true, ..Default::default() }, &MetadataOptions::default())
            .map_err(|e| e.to_string())?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("Brak ścieżki dźwiękowej")?;
        let params = track.codec_params.clone();
        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| e.to_string())?;
        let rate = params.sample_rate.unwrap_or(44100);
        let channels = params.channels.map(|c| c.count()).unwrap_or(2);
        let duration = match (params.n_frames, params.time_base) {
            (Some(n), Some(tb)) => {
                let t = tb.calc_time(n);
                t.seconds as f64 + t.frac
            }
            (Some(n), None) => n as f64 / rate as f64,
            _ => 0.0,
        };
        let bitrate = if duration > 0.0 { (size as f64 * 8.0 / duration / 1000.0) as u32 } else { 0 };
        Ok(Source {
            track_id: track.id,
            format,
            decoder,
            info: StreamInfo {
                codec: codec_name(params.codec).into(),
                rate,
                channels,
                bits: params.bits_per_sample.or(params.bits_per_coded_sample).unwrap_or(0),
                bitrate,
                duration,
            },
            buf: None,
            spec_channels: channels,
            time: 0.0,
        })
    }

    /// Dokłada do `out` stereo kolejnego pakietu. `false` – koniec pliku (albo nieodwracalny błąd).
    pub fn read(&mut self, out: &mut Vec<f32>) -> bool {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(_) => return false,
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec: SignalSpec = *decoded.spec();
                    if self.info.rate != spec.rate {
                        self.info.rate = spec.rate;
                    }
                    let frames = decoded.capacity() as u64;
                    let need_new = self.buf.as_ref().is_none_or(|b| b.capacity() < frames as usize * spec.channels.count());
                    if need_new {
                        self.buf = Some(SampleBuffer::new(frames, spec));
                    }
                    let buf = self.buf.as_mut().unwrap();
                    buf.copy_interleaved_ref(decoded);
                    let ch = spec.channels.count().max(1);
                    self.spec_channels = ch;
                    let s = buf.samples();
                    to_stereo(s, ch, out);
                    let tb = self.format.tracks().iter().find(|t| t.id == self.track_id).and_then(|t| t.codec_params.time_base);
                    if let Some(tb) = tb {
                        let t = tb.calc_time(packet.ts() + packet.dur());
                        self.time = t.seconds as f64 + t.frac;
                    }
                    return true;
                }
                // Uszkodzony pakiet – pomijamy, jak każdy porządny odtwarzacz.
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(symphonia::core::errors::Error::IoError(_)) => return false,
                Err(_) => return false,
            }
        }
    }

    /// Przewija do `secs`; zwraca faktyczny czas.
    pub fn seek(&mut self, secs: f64) -> f64 {
        let t = Time { seconds: secs.max(0.0) as u64, frac: secs.max(0.0).fract() };
        match self.format.seek(SeekMode::Accurate, SeekTo::Time { time: t, track_id: Some(self.track_id) }) {
            Ok(s) => {
                self.decoder.reset();
                let tb = self.format.tracks().iter().find(|t| t.id == self.track_id).and_then(|t| t.codec_params.time_base);
                self.time = tb.map(|tb| {
                    let t = tb.calc_time(s.actual_ts);
                    t.seconds as f64 + t.frac
                })
                .unwrap_or(secs);
                self.time
            }
            Err(_) => self.time,
        }
    }
}

/// Dowolny układ kanałów → stereo (mono podwojone, wielokanałowy zmiksowany w dół).
fn to_stereo(s: &[f32], ch: usize, out: &mut Vec<f32>) {
    match ch {
        1 => {
            out.reserve(s.len() * 2);
            for &v in s {
                out.push(v);
                out.push(v);
            }
        }
        2 => out.extend_from_slice(s),
        n => {
            // FL FR FC LFE SL SR … – środek i tył z wagą, potem normalizacja.
            let norm = 1.0 / (1.0 + 0.707 + if n >= 6 { 0.5 } else { 0.0 });
            out.reserve(s.len() / n * 2);
            for f in s.chunks_exact(n) {
                let c = if n >= 3 { f[2] * 0.707 } else { 0.0 };
                let (sl, sr) = if n >= 6 { (f[4] * 0.5, f[5] * 0.5) } else { (0.0, 0.0) };
                out.push((f[0] + c + sl) * norm);
                out.push((f[1] + c + sr) * norm);
            }
        }
    }
}

/// Przepróbkowanie stereo do częstotliwości wyjścia, z prędkością odtwarzania (jak gramofon – z tonacją).
pub struct Resample {
    inner: Option<SincFixedIn<f32>>,
    from: u32,
    to: u32,
    speed: f64,
    planar: [Vec<f32>; 2],
}

const CHUNK: usize = 1024;

static QUALITY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// Jakość przepróbkowania dla kolejnych utworów: 0 – szybka, 1 – dobra, 2 – najlepsza.
pub fn set_resample_quality(q: u32) {
    QUALITY.store(q.min(2), std::sync::atomic::Ordering::Relaxed);
}

impl Resample {
    pub fn new(from: u32, to: u32, speed: f64) -> Resample {
        let mut r = Resample { inner: None, from, to, speed, planar: [Vec::new(), Vec::new()] };
        r.rebuild();
        r
    }

    fn rebuild(&mut self) {
        self.inner = if self.from == self.to && (self.speed - 1.0).abs() < 1e-6 {
            None
        } else {
            let params = match QUALITY.load(std::sync::atomic::Ordering::Relaxed) {
                0 => SincInterpolationParameters {
                    sinc_len: 48,
                    f_cutoff: 0.91,
                    interpolation: SincInterpolationType::Linear,
                    oversampling_factor: 64,
                    window: WindowFunction::BlackmanHarris2,
                },
                1 => SincInterpolationParameters {
                    sinc_len: 128,
                    f_cutoff: 0.95,
                    interpolation: SincInterpolationType::Cubic,
                    oversampling_factor: 128,
                    window: WindowFunction::BlackmanHarris2,
                },
                _ => SincInterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.97,
                    interpolation: SincInterpolationType::Cubic,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                },
            };
            SincFixedIn::new(self.to as f64 / self.from as f64, 4.0, params, CHUNK, 2).ok().map(|mut r| {
                let _ = r.set_resample_ratio_relative(1.0 / self.speed, false);
                r
            })
        };
    }

    pub fn set_speed(&mut self, speed: f64) {
        let speed = speed.clamp(0.25, 4.0);
        if (speed - self.speed).abs() < 1e-6 {
            return;
        }
        self.speed = speed;
        // Działający resampler zostaje także przy powrocie do 1.0 – przełączenie w trakcie dałoby trzask.
        match &mut self.inner {
            Some(r) => {
                let _ = r.set_resample_ratio_relative(1.0 / speed, true);
            }
            None => self.rebuild(),
        }
    }

    pub fn reset(&mut self) {
        self.planar[0].clear();
        self.planar[1].clear();
        if let Some(r) = &mut self.inner {
            r.reset();
        }
    }

    /// Przetwarza stereo `input` i dopisuje wynik do `out`. Reszta poniżej porcji czeka na kolejne dane.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let Some(r) = &mut self.inner else {
            out.extend_from_slice(input);
            return;
        };
        for f in input.chunks_exact(2) {
            self.planar[0].push(f[0]);
            self.planar[1].push(f[1]);
        }
        loop {
            let need = r.input_frames_next();
            if self.planar[0].len() < need {
                break;
            }
            let chunk = [&self.planar[0][..need], &self.planar[1][..need]];
            if let Ok(res) = r.process(&chunk, None) {
                out.reserve(res[0].len() * 2);
                for i in 0..res[0].len() {
                    out.push(res[0][i]);
                    out.push(res[1][i]);
                }
            }
            self.planar[0].drain(..need);
            self.planar[1].drain(..need);
        }
    }

    /// Koniec pliku – dopełnia ciszą ostatnią porcję, żeby nie zgubić końcówki.
    pub fn flush(&mut self, out: &mut Vec<f32>) {
        let Some(r) = &mut self.inner else { return };
        if self.planar[0].is_empty() {
            return;
        }
        let have = self.planar[0].len();
        let need = r.input_frames_next();
        let ratio = r.output_frames_next() as f64 / need as f64;
        self.planar[0].resize(need, 0.0);
        self.planar[1].resize(need, 0.0);
        let chunk = [&self.planar[0][..], &self.planar[1][..]];
        if let Ok(res) = r.process(&chunk, None) {
            let keep = ((have as f64 * ratio) as usize).min(res[0].len());
            for i in 0..keep {
                out.push(res[0][i]);
                out.push(res[1][i]);
            }
        }
        self.planar[0].clear();
        self.planar[1].clear();
    }
}

/// Szybkie sprawdzenie, czy Symphonia otworzy plik (inaczej gra mpv).
pub fn supported(path: &Path) -> bool {
    Source::open(path).is_ok()
}
