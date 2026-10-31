//! Próba silnika bez interfejsu: dwa wygenerowane pliki WAV, przejście bez przerwy, przewijanie,
//! przenikanie i analizator (głośność 0 – nic nie słychać).

use std::time::Duration;

use openmp_core::audio::engine::{Engine, Item};
use openmp_core::audio::viz::Analyzer;

fn wav(path: &std::path::Path, secs: f32, freq: f32, rate: u32) {
    let n = (secs * rate as f32) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n * 4).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(n * 4).to_le_bytes());
    for i in 0..n {
        let v = ((i as f32 / rate as f32 * freq * std::f32::consts::TAU).sin() * 12000.0) as i16;
        b.extend_from_slice(&v.to_le_bytes());
        b.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

fn main() {
    let dir = std::env::temp_dir().join("openmp-test");
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.wav"), dir.join("b.wav"));
    wav(&a, 3.0, 440.0, 44100);
    wav(&b, 3.0, 1000.0, 48000);
    let e = Engine::new(None);
    e.set_volume(0.0);
    e.load(Item { id: 1, path: a.clone(), gain: 1.0 }, 0.0);
    e.set_next(Some(Item { id: 2, path: b.clone(), gain: 1.0 }));
    e.set_paused(false);
    let mut an = Analyzer::new();
    for i in 0..40 {
        std::thread::sleep(Duration::from_millis(200));
        let s = e.status();
        let total = e.scope(Analyzer::frames(), an.input());
        an.update(32, e.rate(), total, 0.2, 0.0);
        let top = an.bars.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1)).map(|x| x.0).unwrap_or(0);
        println!("{i:2} id={:?} pos={:.2} dur={:.2} ended={} rate={} vu={:.2} topbar={top}", s.id, s.pos, s.info.duration, s.ended, s.info.rate, an.vu[0]);
        if i == 3 {
            e.seek(2.0);
        }
        if i == 20 {
            e.set_crossfade(1.5);
            e.load(Item { id: 3, path: a.clone(), gain: 1.0 }, 0.0);
            e.set_next(Some(Item { id: 4, path: b.clone(), gain: 1.0 }));
        }
        while let Ok(ev) = e.events.try_recv() {
            println!("event {ev:?}");
        }
    }
}
