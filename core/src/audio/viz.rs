//! Analizator dla wyświetlacza: widmo (FFT, pasma logarytmiczne z opadającymi szczytami),
//! oscyloskop i wskaźniki wysterowania – z próbek, które właśnie słychać.

use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

const N: usize = 4096;

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
    pub bars: Vec<f32>,
    pub peaks: Vec<f32>,
    peak_vel: Vec<f32>,
    pub vu: [f32; 2],
    pub vu_peak: [f32; 2],
    samples: Vec<f32>,
    last_total: u64,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Analyzer {
        let fft = FftPlanner::new().plan_fft_forward(N);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let window = (0..N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos()).collect();
        Analyzer {
            fft,
            window,
            buf: vec![Complex32::default(); N],
            scratch,
            bars: Vec::new(),
            peaks: Vec::new(),
            peak_vel: Vec::new(),
            vu: [0.0; 2],
            vu_peak: [0.0; 2],
            samples: Vec::new(),
            last_total: 0,
        }
    }

    /// Bufor dla silnika (ostatnie ramki stereo).
    pub fn input(&mut self) -> &mut Vec<f32> {
        &mut self.samples
    }

    pub fn frames() -> usize {
        N
    }

    /// Przelicza wszystko z `self.samples`. `total` – licznik ramek źródła (gdy stoi, wszystko opada).
    /// `dt` – czas od poprzedniej klatki w sekundach.
    pub fn update(&mut self, bands: usize, rate: u32, total: u64, dt: f32, sensitivity: f32) {
        let fresh = total != self.last_total;
        self.last_total = total;
        if self.bars.len() != bands {
            self.bars = vec![0.0; bands];
            self.peaks = vec![0.0; bands];
            self.peak_vel = vec![0.0; bands];
        }
        let have = self.samples.len() / 2;
        let mut target = vec![0.0f32; bands];
        let mut rms = [0.0f32; 2];
        if fresh && have >= N {
            let s = &self.samples[(have - N) * 2..];
            for i in 0..N {
                let m = (s[i * 2] + s[i * 2 + 1]) * 0.5;
                self.buf[i] = Complex32::new(m * self.window[i], 0.0);
            }
            self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
            let nyq = rate as f32 / 2.0;
            let (lo, hi) = (30.0f32, 16000.0f32.min(nyq));
            for (b, t) in target.iter_mut().enumerate() {
                let f0 = lo * (hi / lo).powf(b as f32 / bands as f32);
                let f1 = lo * (hi / lo).powf((b + 1) as f32 / bands as f32);
                let i0 = ((f0 / nyq) * (N / 2) as f32) as usize;
                let i1 = (((f1 / nyq) * (N / 2) as f32) as usize).max(i0 + 1).min(N / 2);
                let mut mx = 0.0f32;
                for k in i0..i1 {
                    mx = mx.max(self.buf[k].norm());
                }
                // Normalizacja okna Hanna i lekkie podbicie wysokich (naturalny wygląd widma).
                let mag = mx / (N as f32 / 4.0) * (1.0 + b as f32 / bands as f32 * 2.0);
                let db = 20.0 * mag.max(1e-9).log10();
                *t = ((db + 72.0 - sensitivity) / 66.0).clamp(0.0, 1.0);
            }
            // Wysterowanie z ostatnich ~50 ms.
            let w = (rate as usize / 20).min(N);
            for f in s[(N - w) * 2..].chunks_exact(2) {
                rms[0] += f[0] * f[0];
                rms[1] += f[1] * f[1];
            }
            for r in &mut rms {
                *r = (*r / w as f32).sqrt();
            }
        }
        let fall = (dt * 2.8).min(1.0);
        for i in 0..bands {
            let t = target[i];
            self.bars[i] = if t > self.bars[i] { t * 0.7 + self.bars[i] * 0.3 } else { (self.bars[i] - fall * 0.9).max(t) };
            if self.bars[i] >= self.peaks[i] {
                self.peaks[i] = self.bars[i];
                self.peak_vel[i] = -0.35;
            } else {
                // Szczyt chwilę wisi, potem spada coraz szybciej.
                self.peak_vel[i] += dt * 2.2;
                if self.peak_vel[i] > 0.0 {
                    self.peaks[i] = (self.peaks[i] - self.peak_vel[i] * dt).max(self.bars[i]);
                }
            }
        }
        for c in 0..2 {
            let db = 20.0 * rms[c].max(1e-6).log10();
            let v = ((db + 48.0) / 48.0).clamp(0.0, 1.0);
            self.vu[c] = if v > self.vu[c] { v * 0.6 + self.vu[c] * 0.4 } else { (self.vu[c] - dt * 1.4).max(v) };
            self.vu_peak[c] = if self.vu[c] >= self.vu_peak[c] { self.vu[c] } else { (self.vu_peak[c] - dt * 0.35).max(self.vu[c]) };
        }
    }

    /// Ścieżka SVG oscyloskopu w układzie 1000×100 (kanał środkowy).
    pub fn scope_path(&self, points: usize) -> String {
        let have = self.samples.len() / 2;
        if have < 2 || self.last_total == 0 {
            return "M 0 50 L 1000 50".into();
        }
        let span = have.min(2048);
        let s = &self.samples[(have - span) * 2..];
        // Wyzwalanie na przejściu przez zero – fala stoi w miejscu zamiast pływać.
        let mut start = 0;
        for i in 1..span / 2 {
            let a = s[(i - 1) * 2] + s[(i - 1) * 2 + 1];
            let b = s[i * 2] + s[i * 2 + 1];
            if a <= 0.0 && b > 0.0 {
                start = i;
                break;
            }
        }
        let len = span - start;
        let mut out = String::with_capacity(points * 14);
        for p in 0..points {
            let i = start + p * (len / 2) / points;
            let v = (s[i * 2] + s[i * 2 + 1]) * 0.5;
            let x = p as f32 * 1000.0 / (points - 1) as f32;
            let y = 50.0 - v.clamp(-1.0, 1.0) * 46.0;
            out.push_str(if p == 0 { "M " } else { " L " });
            out.push_str(&format!("{x:.1} {y:.1}"));
        }
        out
    }

    /// Wypełniony kontur widma w układzie 1000×100.
    pub fn spectrum_path(&self) -> String {
        let n = self.bars.len();
        if n < 2 {
            return String::new();
        }
        let mut out = String::from("M 0 100");
        for (i, b) in self.bars.iter().enumerate() {
            let x = i as f32 * 1000.0 / (n - 1) as f32;
            out.push_str(&format!(" L {x:.1} {:.1}", 100.0 - b * 98.0));
        }
        out.push_str(" L 1000 100 Z");
        out
    }
}

/// Fala całego utworu do paska przewijania: `n` wartości szczytowych 0..1.
pub fn waveform(path: &std::path::Path, n: usize, cancel: &dyn Fn() -> bool) -> Option<Vec<f32>> {
    let mut src = super::decoder::Source::open(path).ok()?;
    let dur = src.info.duration;
    if dur <= 0.0 {
        return None;
    }
    let total = (dur * src.info.rate as f64) as usize;
    let per = (total / n).max(1);
    let mut out = vec![0.0f32; n];
    let mut tmp = Vec::new();
    let mut idx = 0usize;
    let mut cnt = 0usize;
    let mut peak = 0.0f32;
    let mut packets = 0u32;
    while src.read(&mut tmp) {
        packets += 1;
        if packets % 64 == 0 && cancel() {
            return None;
        }
        for f in tmp.chunks_exact(2) {
            peak = peak.max(f[0].abs().max(f[1].abs()));
            cnt += 1;
            if cnt >= per {
                if idx < n {
                    out[idx] = peak;
                }
                idx += 1;
                cnt = 0;
                peak = 0.0;
            }
        }
        tmp.clear();
    }
    let mx = out.iter().cloned().fold(0.0f32, f32::max).max(1e-3);
    Some(out.into_iter().map(|v| (v / mx).sqrt()).collect())
}
