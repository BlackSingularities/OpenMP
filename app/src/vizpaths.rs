//! Dodatkowe wizualizacje jako ścieżki: lustro, okrąg, widmo 3D (wodospad), diody,
//! oscyloskop stereo (Lissajous) i kropki. Układ 1000×100, okrąg i Lissajous 1000×1000.

use std::collections::VecDeque;
use std::fmt::Write;

/// Numery trybów (4 – wyłączona, zostaje dla zgodności z ustawieniami).
pub const MIRROR: i32 = 5;
pub const RADIAL: i32 = 6;
pub const WATERFALL: i32 = 7;
pub const LEDS: i32 = 8;
pub const LISSAJOUS: i32 = 9;
pub const DOTS: i32 = 10;

/// Kolejność przełączania: klasyczne, nowe, na końcu wyłączona.
pub const ORDER: [i32; 11] = [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 4];

pub fn next(mode: i32) -> i32 {
    let i = ORDER.iter().position(|m| *m == mode).unwrap_or(0);
    ORDER[(i + 1) % ORDER.len()]
}

/// Słupki rosnące od środka w górę i w dół.
pub fn mirror(bars: &[f32]) -> String {
    let n = bars.len().max(1);
    let step = 1000.0 / n as f32;
    let bw = step * 0.7;
    let mut s = String::with_capacity(n * 40);
    for (i, b) in bars.iter().enumerate() {
        let h = b.clamp(0.0, 1.0) * 48.0 + 0.6;
        let x = i as f32 * step + (step - bw) / 2.0;
        let _ = write!(s, "M {x:.1} {:.1} h {bw:.1} v {:.1} h {:.1} Z ", 50.0 - h, h * 2.0, -bw);
    }
    s
}

/// Promienie wokół okręgu (układ 1000×1000).
pub fn radial(bars: &[f32]) -> String {
    let n = bars.len().max(1);
    let mut s = String::with_capacity(n * 80);
    let (cx, cy, r0) = (500.0f32, 500.0f32, 230.0f32);
    let half = std::f32::consts::PI / n as f32 * 0.7;
    // Połowa widma po każdej stronie – obraz symetryczny.
    for k in 0..n * 2 {
        let b = bars[if k < n { k } else { 2 * n - 1 - k }].clamp(0.0, 1.0);
        let a = k as f32 / (n * 2) as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let r1 = r0 + 8.0 + b * 250.0;
        let p = |ang: f32, r: f32| (cx + ang.cos() * r, cy + ang.sin() * r);
        let (x1, y1) = p(a - half / 2.0, r0);
        let (x2, y2) = p(a + half / 2.0, r0);
        let (x3, y3) = p(a + half / 2.0, r1);
        let (x4, y4) = p(a - half / 2.0, r1);
        let _ = write!(s, "M {x1:.1} {y1:.1} L {x2:.1} {y2:.1} L {x3:.1} {y3:.1} L {x4:.1} {y4:.1} Z ");
    }
    // Pierścień wewnętrzny.
    let steps = 72;
    for k in 0..=steps {
        let a = k as f32 / steps as f32 * std::f32::consts::TAU;
        let _ = write!(s, "{} {:.1} {:.1}", if k == 0 { "M" } else { " L" }, cx + a.cos() * (r0 - 6.0), cy + a.sin() * (r0 - 6.0));
    }
    for k in (0..=steps).rev() {
        let a = k as f32 / steps as f32 * std::f32::consts::TAU;
        let _ = write!(s, " L {:.1} {:.1}", cx + a.cos() * (r0 - 12.0), cy + a.sin() * (r0 - 12.0));
    }
    s.push_str(" Z");
    s
}

/// Widmo 3D: ostatnie widma ułożone w głąb, jak góry (linie do obrysu).
pub fn waterfall(hist: &VecDeque<Vec<f32>>) -> String {
    let mut s = String::with_capacity(hist.len() * 64 * 16);
    let depth = hist.len().max(1);
    let (dx, dy) = (14.0f32, 4.6f32);
    let w = 1000.0 - dx * (depth as f32 - 1.0);
    // Od najstarszego (z tyłu) do najnowszego (z przodu).
    for (k, bars) in hist.iter().enumerate().rev() {
        let n = bars.len().max(2);
        let ox = k as f32 * dx;
        let base = 100.0 - k as f32 * dy;
        for (i, b) in bars.iter().enumerate() {
            let x = ox + i as f32 * w / (n - 1) as f32;
            let y = base - b.clamp(0.0, 1.0) * 62.0;
            let _ = write!(s, "{}{x:.1} {y:.1}", if i == 0 { "M " } else { " L " });
        }
        s.push(' ');
    }
    s
}

/// Kolumny diod (14 poziomów).
pub fn leds(bars: &[f32]) -> String {
    let n = bars.len().max(1);
    let step = 1000.0 / n as f32;
    let bw = step * 0.72;
    let levels = 14;
    let seg = 100.0 / levels as f32;
    let mut s = String::with_capacity(n * levels * 30);
    for (i, b) in bars.iter().enumerate() {
        let on = (b.clamp(0.0, 1.0) * levels as f32).round() as usize;
        let x = i as f32 * step + (step - bw) / 2.0;
        for l in 0..on.max(1) {
            let y = 100.0 - (l as f32 + 1.0) * seg;
            let _ = write!(s, "M {x:.1} {y:.1} h {bw:.1} v {:.1} h {:.1} Z ", seg * 0.68, -bw);
        }
    }
    s
}

/// Oscyloskop stereo: środek (L+R) w pionie, różnica (L−R) w poziomie (układ 1000×1000).
pub fn lissajous(samples: &[f32]) -> String {
    let frames = samples.len() / 2;
    let take = frames.min(1400);
    let start = frames - take;
    let mut s = String::with_capacity(take * 14);
    let mut first = true;
    for f in (start..frames).step_by(2) {
        let (l, r) = (samples[f * 2], samples[f * 2 + 1]);
        let x = 500.0 + (l - r) * 0.7071 * 420.0;
        let y = 500.0 - (l + r) * 0.7071 * 420.0;
        let _ = write!(s, "{}{:.1} {:.1}", if first { "M " } else { " L " }, x.clamp(0.0, 1000.0), y.clamp(0.0, 1000.0));
        first = false;
    }
    s
}

/// Siatka kropek – wysokość słupka to liczba zapalonych kropek.
pub fn dots(bars: &[f32]) -> String {
    let n = bars.len().max(1);
    let step = 1000.0 / n as f32;
    let rows = 16;
    let gap = 100.0 / rows as f32;
    let r = (step * 0.28).min(gap * 0.3 * 10.0);
    let mut s = String::with_capacity(n * rows * 30);
    for (i, b) in bars.iter().enumerate() {
        let on = (b.clamp(0.0, 1.0) * rows as f32).round() as usize;
        let cx = i as f32 * step + step / 2.0;
        for l in 0..on.max(1) {
            let cy = 100.0 - (l as f32 + 0.5) * gap;
            let _ = write!(s, "M {:.1} {:.1} h {:.1} v {:.1} h {:.1} Z ", cx - r, cy - gap * 0.25, r * 2.0, gap * 0.5, -r * 2.0);
        }
    }
    s
}
