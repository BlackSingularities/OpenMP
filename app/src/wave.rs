//! Taśma utworu na pasku przewijania: kształt w wybranym stylu (układ 1000×100) i żywa część
//! przy znaczniku odtwarzania, poruszająca się z muzyką.
//!
//! Każdy styl daje warstwę główną i opcjonalnie drugą: rodzaj 0 – jaśniejsza (ściany i grzbiety
//! 3D), rodzaj 1 – przygaszona (odbicie).

use std::fmt::Write;

/// Nazwy stylów w kolejności numerów (ustawienia, menu).
pub const NAMES: [&str; 11] = ["Fala", "Słupki", "Od dołu", "Kontur", "Gęste słupki", "Lustro", "Diody", "Słupki 3D", "Fala 3D", "Linia", "Kropki"];

/// Warstwy taśmy: (główna, druga, rodzaj drugiej).
pub fn paths(w: &[f32], style: i32) -> (String, String, i32) {
    let n = w.len();
    if n < 2 {
        return (String::new(), String::new(), 0);
    }
    let mut a = String::with_capacity(n * 24);
    let mut b = String::new();
    let mut kind = 0;
    let x_of = |i: usize| i as f32 * 1000.0 / (n - 1) as f32;
    // Najwyższa wartość w k-tym z `count` przedziałów.
    let col = |k: usize, count: usize| {
        let s = k * n / count;
        let e = ((k + 1) * n / count).max(s + 1).min(n);
        w[s..e].iter().cloned().fold(0.0f32, f32::max).max(0.04)
    };
    match style {
        1 | 4 => {
            let count = if style == 4 { 750 } else { 420 };
            let step = 1000.0 / count as f32;
            for k in 0..count {
                let v = col(k, count);
                let bw = step * if style == 4 { 0.55 } else { 0.64 };
                let x = k as f32 * step + (step - bw) / 2.0;
                let h = v * 46.0;
                let _ = write!(a, "M {x:.2} {:.1} h {bw:.2} v {:.1} h {:.1} Z ", 50.0 - h, h * 2.0, -bw);
            }
        }
        2 => {
            a.push_str("M 0 100");
            for (i, v) in w.iter().enumerate() {
                let _ = write!(a, " L {:.1} {:.1}", x_of(i), 100.0 - v.max(0.06) * 94.0);
            }
            a.push_str(" L 1000 100 Z");
        }
        3 => {
            for (i, v) in w.iter().enumerate() {
                let _ = write!(a, "{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, x_of(i), 50.0 - v.max(0.06) * 46.0);
            }
            for (i, v) in w.iter().enumerate().rev() {
                let _ = write!(a, " L {:.1} {:.1}", x_of(i), 50.0 - v.max(0.06) * 46.0 + 5.0);
            }
            a.push_str(" Z M 0 49 L 1000 49 L 1000 51 L 0 51 Z");
        }
        // Lustro: słupki nad linią, odbicie pod nią.
        5 => {
            let count = 420;
            let step = 1000.0 / count as f32;
            let bw = step * 0.62;
            for k in 0..count {
                let v = col(k, count);
                let x = k as f32 * step + (step - bw) / 2.0;
                let h = v * 62.0;
                let _ = write!(a, "M {x:.2} {:.1} h {bw:.2} v {h:.1} h {:.1} Z ", 66.0 - h, -bw);
                let _ = write!(b, "M {x:.1} 68 h {bw:.2} v {:.1} h {:.1} Z ", h * 0.48, -bw);
            }
            kind = 1;
        }
        // Diody: kolumny z kwadratowych segmentów.
        6 => {
            let count = 300;
            let step = 1000.0 / count as f32;
            let bw = step * 0.7;
            for k in 0..count {
                let v = col(k, count);
                let x = k as f32 * step + (step - bw) / 2.0;
                let segs = (v * 12.0).ceil().max(1.0) as i32;
                for s in 0..segs {
                    let y = 100.0 - (s as f32 + 1.0) * 8.2;
                    let _ = write!(a, "M {x:.2} {y:.1} h {bw:.2} v 6 h {:.2} Z ", -bw);
                }
            }
        }
        // Słupki 3D: przód, bok i jaśniejszy wierzch.
        7 => {
            let count = 240;
            let step = 1000.0 / count as f32;
            let bw = step * 0.55;
            let d = step * 0.28;
            let dy = 7.0;
            for k in 0..count {
                let v = col(k, count);
                let x = k as f32 * step + step * 0.08;
                let top = 100.0 - v * 88.0;
                let _ = write!(a, "M {x:.2} {top:.1} h {bw:.2} v {:.1} h {:.1} Z ", 100.0 - top, -bw);
                // Bok (cień po prawej).
                let _ = write!(a, "M {:.1} {top:.1} l {d:.1} {:.1} V {:.1} l {:.1} {dy:.1} Z ", x + bw, -dy, 100.0 - dy, -d);
                // Wierzch.
                let _ = write!(b, "M {x:.1} {top:.1} l {d:.1} {:.1} h {bw:.2} l {:.1} {dy:.1} Z ", -dy, -d);
            }
        }
        // Fala 3D: fala od dołu i jasny grzbiet przesunięty w głąb.
        8 => {
            a.push_str("M 0 100");
            for (i, v) in w.iter().enumerate() {
                let _ = write!(a, " L {:.1} {:.1}", x_of(i), 100.0 - v.max(0.06) * 86.0);
            }
            a.push_str(" L 1000 100 Z");
            for (i, v) in w.iter().enumerate() {
                let _ = write!(b, "{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, (x_of(i) + 6.0).min(1000.0), 100.0 - v.max(0.06) * 86.0 - 9.0);
            }
            for (i, v) in w.iter().enumerate().rev() {
                let _ = write!(b, " L {:.1} {:.1}", x_of(i), 100.0 - v.max(0.06) * 86.0);
            }
            b.push_str(" Z");
        }
        // Linia: dwie cienkie wstęgi wzdłuż obwiedni.
        9 => {
            for sign in [-1.0f32, 1.0] {
                for (i, v) in w.iter().enumerate() {
                    let _ = write!(a, "{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, x_of(i), 50.0 + sign * v.max(0.06) * 46.0);
                }
                for (i, v) in w.iter().enumerate().rev() {
                    let _ = write!(a, " L {:.1} {:.1}", x_of(i), 50.0 + sign * (v.max(0.06) * 46.0 - 3.0));
                }
                a.push_str(" Z ");
            }
        }
        // Kropki: para kropek na obwiedni i drobna kropka w środku.
        10 => {
            let count = 420;
            let step = 1000.0 / count as f32;
            let r = step * 0.3;
            for k in 0..count {
                let v = col(k, count);
                let cx = k as f32 * step + step / 2.0;
                for cy in [50.0 - v * 44.0, 50.0 + v * 44.0] {
                    let _ = write!(a, "M {:.1} {cy:.1} h {:.1} v 4 h {:.1} Z ", cx - r, r * 2.0, -r * 2.0);
                }
                let _ = write!(b, "M {:.1} 49 h {r:.1} v 2 h {:.1} Z ", cx - r / 2.0, -r);
            }
            kind = 1;
        }
        _ => {
            for (i, v) in w.iter().enumerate() {
                let _ = write!(a, "{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, x_of(i), 50.0 - v.max(0.06) * 46.0);
            }
            for (i, v) in w.iter().enumerate().rev() {
                let _ = write!(a, " L {:.1} {:.1}", x_of(i), 50.0 + v.max(0.06) * 46.0);
            }
            a.push_str(" Z");
        }
    }
    (a, b, kind)
}

/// Rodzaje żywej części taśmy przy znaczniku (ustawienia).
pub const LIVE_NAMES: [&str; 7] = ["Wyłączona", "Puls taśmy", "Słupki widma", "Fala widma", "Kropki szczytów", "Obwiednia", "Fala uderzeniowa"];

/// Liczba kolumn stylu taśmy – puls porusza tymi samymi elementami co taśma.
fn columns(style: i32) -> usize {
    match style {
        4 => 750,
        6 => 300,
        7 => 240,
        _ => 420,
    }
}

/// Żywa część taśmy przy znaczniku, poruszająca się z muzyką.
/// `pos` 0..1, `bars` 0..1 z analizatora, `kind` – numer z `LIVE_NAMES`, `span` 0..2 – zasięg,
/// `t` – czas w sekundach (fala uderzeniowa).
pub fn live(w: &[f32], pos: f32, bars: &[f32], kind: i32, style: i32, span: i32, t: f32) -> String {
    if w.len() < 2 || bars.is_empty() || kind <= 0 {
        return String::new();
    }
    let n = w.len();
    let mut s = String::with_capacity(2400);
    let cols = columns(style);
    let step = 1000.0 / cols as f32;
    let half = [12i32, 22, 40][span.clamp(0, 2) as usize];
    let center = pos.clamp(0.0, 1.0) * 1000.0;
    let ck = (center / step) as i32;
    let bass = bars[..(bars.len() / 4).max(1)].iter().sum::<f32>() / (bars.len() / 4).max(1) as f32;
    // Obwiednia utworu w kolumnie `k` (jak w taśmie: najwyższa wartość przedziału).
    let col = |k: i32| {
        let k = k.clamp(0, cols as i32 - 1) as usize;
        let a = k * n / cols;
        let e = ((k + 1) * n / cols).max(a + 1).min(n);
        w[a..e].iter().cloned().fold(0.0f32, f32::max).max(0.04)
    };
    // Pasmo widma dla odległości od znacznika: środek – niskie tony, dalej – wyższe.
    let band = |j: i32| bars[(j.unsigned_abs() as usize * bars.len() / (half as usize + 1)).min(bars.len() - 1)];
    let fall = |j: i32| 1.0 - (j.abs() as f32 / (half as f32 + 1.0)).powf(1.4);
    let inside = |x: f32| (0.0..=1000.0 - step).contains(&x);
    match kind {
        // Puls taśmy: kolumny taśmy wokół znacznika rosną z muzyką.
        1 => {
            let bw = step * 0.6;
            for j in -half..=half {
                let k = ck + j;
                let x = k as f32 * step + (step - bw) / 2.0;
                if !inside(x) {
                    continue;
                }
                let v = (col(k) * (1.0 + (band(j) * 0.7 + bass * 0.5) * fall(j))).min(1.08);
                let h = v * 46.0;
                let _ = write!(s, "M {x:.2} {:.1} h {bw:.2} v {:.1} h {:.2} Z ", 50.0 - h, h * 2.0, -bw);
            }
        }
        // Słupki widma wyrastające z taśmy.
        2 => {
            for j in -half..=half {
                let x = center + j as f32 * step;
                if !inside(x) {
                    continue;
                }
                let base = col((x / step) as i32).max(0.08);
                let v = (base * (0.45 + band(j) * 1.1)).min(1.0) * fall(j);
                let h = v * 40.0;
                let _ = write!(s, "M {:.2} {:.1} h 1.4 v {:.1} h -1.4 Z ", x - 0.7, 50.0 - h, h * 2.0);
            }
        }
        // Fala widma: gładki, lustrzany kształt.
        3 => {
            let pts: Vec<(f32, f32)> = (-half..=half)
                .map(|j| (center + j as f32 * step, (band(j) * 0.9 + 0.1) * fall(j) * 44.0))
                .filter(|(x, _)| (0.0..=1000.0).contains(x))
                .collect();
            for (i, (x, h)) in pts.iter().enumerate() {
                let _ = write!(s, "{}{x:.2} {:.1}", if i == 0 { "M " } else { " L " }, 50.0 - h);
            }
            for (x, h) in pts.iter().rev() {
                let _ = write!(s, " L {x:.2} {:.1}", 50.0 + h);
            }
            if !pts.is_empty() {
                s.push_str(" Z");
            }
        }
        // Kropki szczytów nad i pod taśmą.
        4 => {
            let d = step * 0.7;
            for j in -half..=half {
                let k = ck + j;
                let x = k as f32 * step + (step - d) / 2.0;
                if !inside(x) {
                    continue;
                }
                let h = (col(k) * 46.0 + 3.0 + band(j) * 14.0 * fall(j)).min(49.0);
                for y in [50.0 - h - d, 50.0 + h] {
                    let _ = write!(s, "M {x:.2} {y:.1} h {d:.2} v {:.2} h {:.2} Z ", d * 2.0, -d);
                }
            }
        }
        // Obwiednia: cienka linia nad kształtem taśmy, unoszona basem.
        5 => {
            for sign in [-1.0f32, 1.0] {
                let mut first = true;
                let mut back = Vec::new();
                for j in -half..=half {
                    let k = ck + j;
                    let x = k as f32 * step + step / 2.0;
                    if !(0.0..=1000.0).contains(&x) {
                        continue;
                    }
                    let h = (col(k) * (1.0 + bass * 0.35 * fall(j)) * 46.0 + 2.0).min(49.0);
                    let _ = write!(s, "{}{x:.2} {:.1}", if first { "M " } else { " L " }, 50.0 + sign * h);
                    first = false;
                    back.push((x, h));
                }
                for (x, h) in back.iter().rev() {
                    let _ = write!(s, " L {x:.2} {:.1}", 50.0 + sign * (h - 1.6));
                }
                if !first {
                    s.push_str(" Z ");
                }
            }
        }
        // Fala uderzeniowa: jasny pierścień rozchodzący się od znacznika w rytm basu.
        _ => {
            let ring = (t * 1.6).fract();
            let reach = half as f32 * ring;
            let bw = step * 0.6;
            for j in -half..=half {
                let d = (j.abs() as f32 - reach).abs();
                if d > 3.0 {
                    continue;
                }
                let k = ck + j;
                let x = k as f32 * step + (step - bw) / 2.0;
                if !inside(x) {
                    continue;
                }
                let a = (1.0 - d / 3.0) * (1.0 - ring) * (0.5 + bass);
                let h = (col(k) * (1.0 + a * 0.6) * 46.0).min(49.0);
                let _ = write!(s, "M {x:.2} {:.1} h {bw:.2} v {:.1} h {:.2} Z ", 50.0 - h, h * 2.0, -bw);
            }
        }
    }
    s
}
