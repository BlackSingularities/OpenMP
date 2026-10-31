//! Pomiar głośności EBU R128 dla plików bez tagów ReplayGain – wynik w tej samej skali
//! (odniesienie −18 LUFS, jak ReplayGain 2.0), zapamiętywany w loudness.json.

use std::collections::HashMap;
use std::path::Path;

use ebur128::{EbuR128, Mode};
use serde::{Deserialize, Serialize};

use super::decoder::Source;

/// Głośność odniesienia ReplayGain 2.0.
const REFERENCE_LUFS: f64 = -18.0;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Loudness {
    /// Wzmocnienie w dB do poziomu odniesienia.
    pub gain: f32,
    /// Szczyt próbki (liniowo, 1.0 = pełna skala).
    pub peak: f32,
}

/// Mierzy cały plik (blokujące – kilka sekund na album, wywoływać w tle).
pub fn measure(path: &Path) -> Option<Loudness> {
    let mut src = Source::open(path).ok()?;
    let rate = src.info.rate.max(1);
    let mut meter = EbuR128::new(2, rate, Mode::I | Mode::SAMPLE_PEAK).ok()?;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if !src.read(&mut buf) {
            break;
        }
        if src.info.rate != rate {
            // Zmiana częstotliwości w środku pliku – pomiar byłby błędny.
            return None;
        }
        meter.add_frames_f32(&buf).ok()?;
    }
    let lufs = meter.loudness_global().ok()?;
    if !lufs.is_finite() {
        return None;
    }
    let peak = meter.sample_peak(0).ok()?.max(meter.sample_peak(1).ok()?);
    Some(Loudness { gain: (REFERENCE_LUFS - lufs) as f32, peak: peak as f32 })
}

/// Zapamiętane pomiary: klucz jak w statystykach biblioteki (ścieżka małymi literami).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub map: HashMap<String, Loudness>,
}

impl Store {
    pub fn load() -> Store {
        crate::paths::load_json("loudness.json")
    }
    pub fn save(&self) {
        crate::paths::save_json("loudness.json", self);
    }
    pub fn get(&self, path: &str) -> Option<Loudness> {
        self.map.get(&crate::library::key(path)).copied()
    }
    pub fn insert(&mut self, path: &str, l: Loudness) {
        self.map.insert(crate::library::key(path), l);
    }
}
