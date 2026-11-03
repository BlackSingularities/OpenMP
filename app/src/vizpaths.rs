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
pub const WAVE: i32 = 11;
pub const BLOB: i32 = 12;
pub const PULSE: i32 = 13;
pub const CAPSULES: i32 = 14;

/// Kolejność przełączania: klasyczne, nowe, na końcu wyłączona.
pub const ORDER: [i32; 15] = [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 4];

/// O jeden tryb dalej (`dir` > 0) albo wstecz, w kolejności `ORDER`.
pub fn step(mode: i32, dir: i32) -> i32 {
    let n = ORDER.len() as i32;
    let i = ORDER.iter().position(|m| *m == mode).unwrap_or(0) as i32;
    ORDER[(i + dir.signum()).rem_euclid(n) as usize]
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

/// Wygładzone widmo: krzywa Catmulla-Roma przez wierzchołki słupków.
fn smooth(pts: &[(f32, f32)], s: &mut String) {
    for i in 0..pts.len().saturating_sub(1) {
        let p0 = pts[i.saturating_sub(1)];
        let (p1, p2) = (pts[i], pts[i + 1]);
        let p3 = pts[(i + 2).min(pts.len() - 1)];
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        let _ = write!(s, " C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}", c1.0, c1.1, c2.0, c2.1, p2.0, p2.1);
    }
}

/// Fala: gładki kształt symetryczny względem środka (układ 1000×100).
pub fn wave(bars: &[f32]) -> String {
    let n = bars.len().max(2);
    let x = |i: usize| i as f32 * 1000.0 / (n - 1) as f32;
    let top: Vec<(f32, f32)> = bars.iter().enumerate().map(|(i, b)| (x(i), 50.0 - b.clamp(0.0, 1.0) * 47.0 - 0.8)).collect();
    let bottom: Vec<(f32, f32)> = top.iter().rev().map(|&(x, y)| (x, 100.0 - y)).collect();
    let mut s = String::with_capacity(n * 90);
    let _ = write!(s, "M {:.1} {:.1}", top[0].0, top[0].1);
    smooth(&top, &mut s);
    let _ = write!(s, " L {:.1} {:.1}", bottom[0].0, bottom[0].1);
    smooth(&bottom, &mut s);
    s.push_str(" Z");
    s
}

/// Plama: zamknięty, płynny kształt wokół środka – promień z widma (układ 1000×1000).
pub fn blob(bars: &[f32]) -> String {
    let n = bars.len().max(3);
    let k = n * 2;
    let bass = bars.iter().take(4).sum::<f32>() / 4.0;
    let r0 = 250.0 + bass.clamp(0.0, 1.0) * 40.0;
    let pts: Vec<(f32, f32)> = (0..k)
        .map(|j| {
            let b = bars[if j < n { j } else { 2 * n - 1 - j }].clamp(0.0, 1.0);
            let a = j as f32 / k as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let r = r0 + b * 200.0;
            (500.0 + a.cos() * r, 500.0 + a.sin() * r)
        })
        .collect();
    let mut s = String::with_capacity(k * 60);
    let _ = write!(s, "M {:.1} {:.1}", pts[0].0, pts[0].1);
    // Zamknięta krzywa: sąsiedzi brani w kółko.
    for i in 0..k {
        let p0 = pts[(i + k - 1) % k];
        let (p1, p2) = (pts[i], pts[(i + 1) % k]);
        let p3 = pts[(i + 2) % k];
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        let _ = write!(s, " C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}", c1.0, c1.1, c2.0, c2.1, p2.0, p2.1);
    }
    s.push_str(" Z");
    s
}

/// Okrąg o promieniu `r` i grubości `w` jako pierścień (układ 1000×1000).
fn ring(s: &mut String, r: f32, w: f32) {
    let (a, b) = (r + w / 2.0, (r - w / 2.0).max(1.0));
    let _ = write!(s, "M {:.1} 500 A {a:.1} {a:.1} 0 1 0 {:.1} 500 A {a:.1} {a:.1} 0 1 0 {:.1} 500 Z ", 500.0 - a, 500.0 + a, 500.0 - a);
    let _ = write!(s, "M {:.1} 500 A {b:.1} {b:.1} 0 1 1 {:.1} 500 A {b:.1} {b:.1} 0 1 1 {:.1} 500 Z ", 500.0 - b, 500.0 + b, 500.0 - b);
}

/// Puls: pierścienie basu, środka i góry pasma rozszerzające się z głośnością.
pub fn pulse(bars: &[f32]) -> String {
    let n = bars.len().max(3);
    let avg = |a: usize, b: usize| bars[a..b.max(a + 1).min(n)].iter().sum::<f32>() / (b - a).max(1) as f32;
    let (lo, mid, hi) = (avg(0, n / 6), avg(n / 6, n / 2), avg(n / 2, n));
    let mut s = String::with_capacity(600);
    ring(&mut s, 120.0 + lo.clamp(0.0, 1.0) * 120.0, 18.0 + lo * 30.0);
    ring(&mut s, 270.0 + mid.clamp(0.0, 1.0) * 110.0, 10.0 + mid * 18.0);
    ring(&mut s, 400.0 + hi.clamp(0.0, 1.0) * 80.0, 5.0 + hi * 10.0);
    s
}

/// Kapsułki: zaokrąglone słupki rosnące od środka (układ 1000×100).
pub fn capsules(bars: &[f32]) -> String {
    let n = bars.len().max(1);
    let step = 1000.0 / n as f32;
    let bw = step * 0.56;
    let r = bw / 2.0;
    let mut s = String::with_capacity(n * 90);
    for (i, b) in bars.iter().enumerate() {
        let h = b.clamp(0.0, 1.0) * 46.0;
        let x = i as f32 * step + (step - bw) / 2.0;
        let (y0, y1) = (50.0 - h - r * 0.1, 50.0 + h + r * 0.1);
        // Prostokąt z półokręgami na końcach (w układzie rozciąganym – łuk eliptyczny).
        let ry = r * 0.1;
        let _ = write!(s, "M {x:.1} {y0:.1} A {r:.1} {ry:.2} 0 0 1 {:.1} {y0:.1} L {:.1} {y1:.1} A {r:.1} {ry:.2} 0 0 1 {x:.1} {y1:.1} Z ", x + bw, x + bw);
    }
    s
}
