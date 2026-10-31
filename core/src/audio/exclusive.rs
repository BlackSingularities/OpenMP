//! Wyjście WASAPI w trybie wyłącznym: karta przełącza się na częstotliwość pliku, a mikser
//! Windows nie dotyka dźwięku (bit-perfect przy wyłączonym korektorze i głośności 100%).
//!
//! Klient audio żyje w osobnym wątku (COM), który na każde zdarzenie karty dopisuje jeden okres.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use wasapi::{Direction, SampleType, StreamMode, WaveFormat};

use super::engine::{render, Dither, Shared};

/// Format próbki przyjęty przez kartę.
#[derive(Clone, Copy, Debug)]
enum Fmt {
    F32,
    /// Liczba całkowita: bajty na próbkę i bity znaczące.
    Int { store: usize, valid: u32 },
}

pub(super) struct Exclusive {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub rate: u32,
    pub bits: u32,
}

impl Drop for Exclusive {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn find_device(name: Option<&str>) -> Result<wasapi::Device, wasapi::WasapiError> {
    if let Some(n) = name {
        if let Ok(d) = wasapi::DeviceCollection::new(&Direction::Render).and_then(|c| c.get_device_with_name(n)) {
            return Ok(d);
        }
    }
    wasapi::get_default_device(&Direction::Render)
}

/// Pierwszy format i częstotliwość, które karta przyjmie w trybie wyłącznym (najpierw `want`).
fn negotiate(client: &wasapi::AudioClient, want: u32) -> Option<(WaveFormat, Fmt, u32)> {
    let mut rates = vec![want];
    for r in [48000, 44100, 96000, 88200, 192000] {
        if !rates.contains(&r) {
            rates.push(r);
        }
    }
    let fmts = [
        (32, 32, SampleType::Float, Fmt::F32),
        (32, 24, SampleType::Int, Fmt::Int { store: 4, valid: 24 }),
        (24, 24, SampleType::Int, Fmt::Int { store: 3, valid: 24 }),
        (32, 32, SampleType::Int, Fmt::Int { store: 4, valid: 32 }),
        (16, 16, SampleType::Int, Fmt::Int { store: 2, valid: 16 }),
    ];
    for rate in rates {
        for (store, valid, st, f) in &fmts {
            let wf = WaveFormat::new(*store, *valid, st, rate as usize, 2, None);
            if let Ok(ok) = client.is_supported_exclusive_with_quirks(&wf) {
                return Some((ok, *f, rate));
            }
        }
    }
    None
}

impl Exclusive {
    /// Otwiera kartę na wyłączność, możliwie w częstotliwości `want`.
    pub fn open(device: Option<String>, want: u32, sh: Arc<Shared>) -> Result<Exclusive, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        let (tx, rx) = crossbeam_channel::bounded::<Result<(u32, u32), String>>(1);
        let thread = std::thread::Builder::new()
            .name("openmp-wasapi".into())
            .spawn(move || {
                let _ = wasapi::initialize_mta();
                let init = || -> Result<_, String> {
                    let dev = find_device(device.as_deref()).map_err(|e| e.to_string())?;
                    let mut client = dev.get_iaudioclient().map_err(|e| e.to_string())?;
                    let (wf, fmt, rate) = negotiate(&client, want).ok_or("karta nie przyjmuje żadnego formatu stereo")?;
                    let period = client.calculate_aligned_period_near(100_000, Some(128), &wf).map_err(|e| e.to_string())?;
                    client.initialize_client(&wf, &Direction::Render, &StreamMode::EventsExclusive { period_hns: period }).map_err(|e| e.to_string())?;
                    let ev = client.set_get_eventhandle().map_err(|e| e.to_string())?;
                    let frames = client.get_buffer_size().map_err(|e| e.to_string())? as usize;
                    let rc = client.get_audiorenderclient().map_err(|e| e.to_string())?;
                    let id = dev.get_id().unwrap_or_default();
                    Ok((client, ev, rc, fmt, rate, frames, wf.get_blockalign() as usize, id))
                };
                let (client, ev, rc, fmt, rate, frames, block, id) = match init() {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                let bits = match fmt {
                    Fmt::F32 => 32,
                    Fmt::Int { valid, .. } => valid,
                };
                let mut tmp = Vec::new();
                let mut bytes = vec![0u8; frames * block];
                let mut fade = 0.0f32;
                let mut dither = Dither::default();
                // Mikser Windows nie działa w trybie wyłącznym – głośność systemu stosujemy sami.
                let sysvol = endpoint_volume(&id);
                // Pierwszy okres trzeba wpisać przed startem strumienia.
                let mut fill = |tmp: &mut Vec<f32>, bytes: &mut Vec<u8>, fade: &mut f32| {
                    render(&sh, tmp, frames, rate, fade);
                    if let Some(v) = &sysvol {
                        let g = system_gain(v);
                        if g < 0.9999 {
                            for s in tmp.iter_mut() {
                                *s *= g;
                            }
                        }
                    }
                    encode(tmp, fmt, &mut dither, bytes);
                };
                fill(&mut tmp, &mut bytes, &mut fade);
                if rc.write_to_device(frames, &bytes, None).is_err() || client.start_stream().is_err() {
                    let _ = tx.send(Err("nie udało się uruchomić strumienia".into()));
                    return;
                }
                let _ = tx.send(Ok((rate, bits)));
                while !st.load(Ordering::Relaxed) {
                    if ev.wait_for_event(500).is_err() {
                        // Karta przestała wołać (odłączona, zabrana przez inny program).
                        sh.failed.store(true, Ordering::Relaxed);
                        break;
                    }
                    fill(&mut tmp, &mut bytes, &mut fade);
                    if rc.write_to_device(frames, &bytes, None).is_err() {
                        sh.failed.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                let _ = client.stop_stream();
            })
            .map_err(|e| e.to_string())?;
        match rx.recv() {
            Ok(Ok((rate, bits))) => Ok(Exclusive { stop, thread: Some(thread), rate, bits }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => Err("wątek wyjścia zakończył się".into()),
        }
    }
}

/// Stereo f32 → bajty w formacie karty (z ditherem TPDF przy 16 i 24 bitach).
fn encode(src: &[f32], fmt: Fmt, d: &mut Dither, out: &mut [u8]) {
    match fmt {
        Fmt::F32 => {
            for (s, o) in src.iter().zip(out.chunks_exact_mut(4)) {
                o.copy_from_slice(&s.clamp(-1.0, 1.0).to_le_bytes());
            }
        }
        Fmt::Int { store, valid } => {
            for (s, o) in src.iter().zip(out.chunks_exact_mut(store)) {
                let v = d.quantize(*s, valid);
                let v = if store == 4 && valid < 32 { v << (32 - valid) } else { v };
                o.copy_from_slice(&v.to_le_bytes()[..store]);
            }
        }
    }
}

/// Regulacja głośności Windows dla urządzenia – gdy karta nie robi tego sprzętowo.
fn endpoint_volume(id: &str) -> Option<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume> {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    // ENDPOINT_HARDWARE_SUPPORT_VOLUME (endpointvolume.h) – w crate windows tylko za dodatkową funkcją.
    const ENDPOINT_HARDWARE_SUPPORT_VOLUME: u32 = 0x1;
    use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    if id.is_empty() {
        return None;
    }
    unsafe {
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let w: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        let dev = en.GetDevice(PCWSTR(w.as_ptr())).ok()?;
        let v: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None).ok()?;
        // Karta ścisza sama (sprzętowo) – nie mnożymy drugi raz.
        if v.QueryHardwareSupport().ok()? & ENDPOINT_HARDWARE_SUPPORT_VOLUME != 0 {
            return None;
        }
        Some(v)
    }
}

/// Bieżąca głośność systemu jako mnożnik (wyciszenie – zero).
fn system_gain(v: &windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume) -> f32 {
    unsafe {
        if v.GetMute().map(|m| m.as_bool()).unwrap_or(false) {
            return 0.0;
        }
        match v.GetMasterVolumeLevel() {
            Ok(db) => 10f32.powf(db / 20.0).min(1.0),
            Err(_) => 1.0,
        }
    }
}
