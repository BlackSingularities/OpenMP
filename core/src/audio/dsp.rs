//! Obróbka dźwięku w czasie rzeczywistym: 18-pasmowy korektor, przedwzmacniacz, balans,
//! szerokość stereo, mono, zamiana kanałów, przesłuch słuchawkowy i ogranicznik (bez przesterowań
//! przy podbitym korektorze).

use serde::{Deserialize, Serialize};

/// Częstotliwości środkowe pasm (jak w dużych odtwarzaczach – co pół oktawy).
pub const BANDS: [f32; 18] = [
    31.0, 63.0, 87.0, 125.0, 175.0, 250.0, 350.0, 500.0, 700.0, 1000.0, 1400.0, 2000.0, 2800.0, 4000.0, 5600.0, 8000.0, 11200.0,
    16000.0,
];

/// Zakres suwaka pasma w dB.
pub const MAX_DB: f32 = 15.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DspSettings {
    pub eq_on: bool,
    pub gains: Vec<f32>,
    pub preamp: f32,
    pub balance: f32,
    pub width: f32,
    pub mono: bool,
    pub swap: bool,
    pub crossfeed: bool,
    pub limiter: bool,
    /// Korektor parametryczny (np. profil słuchawek z AutoEQ): filtry, przedwzmocnienie, nazwa.
    pub peq_on: bool,
    pub peq: Vec<Peq>,
    pub peq_pre: f32,
    pub peq_name: String,
}

/// Filtr korektora parametrycznego: 0 – szczytowy, 1 – półkowy niski, 2 – półkowy wysoki.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Peq {
    pub kind: u8,
    pub f: f32,
    pub q: f32,
    pub db: f32,
}

/// Profil AutoEQ („ParametricEQ.txt”): „Preamp: -6.2 dB” i „Filter 1: ON PK Fc 105 Hz Gain -3.2 dB Q 0.70”.
pub fn parse_autoeq(text: &str) -> Option<(f32, Vec<Peq>)> {
    let mut pre = 0.0;
    let mut v = Vec::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.first().is_some_and(|x| x.eq_ignore_ascii_case("preamp:")) {
            pre = w.get(1).and_then(|x| x.parse().ok()).unwrap_or(0.0);
            continue;
        }
        if !w.first().is_some_and(|x| x.eq_ignore_ascii_case("filter")) || !w.iter().any(|x| x.eq_ignore_ascii_case("on")) {
            continue;
        }
        let after = |k: &str| w.iter().position(|x| x.eq_ignore_ascii_case(k)).and_then(|i| w.get(i + 1)).and_then(|x| x.parse::<f32>().ok());
        let kind = if w.iter().any(|x| x.eq_ignore_ascii_case("LSC") || x.eq_ignore_ascii_case("LS")) {
            1
        } else if w.iter().any(|x| x.eq_ignore_ascii_case("HSC") || x.eq_ignore_ascii_case("HS")) {
            2
        } else if w.iter().any(|x| x.eq_ignore_ascii_case("PK") || x.eq_ignore_ascii_case("PEQ")) {
            0
        } else {
            continue;
        };
        let (Some(f), Some(db)) = (after("Fc"), after("Gain")) else { continue };
        v.push(Peq { kind, f, q: after("Q").unwrap_or(0.707).max(0.05), db });
    }
    (!v.is_empty()).then_some((pre, v))
}

impl Default for DspSettings {
    fn default() -> Self {
        DspSettings {
            eq_on: false,
            gains: vec![0.0; BANDS.len()],
            preamp: 0.0,
            balance: 0.0,
            width: 1.0,
            mono: false,
            swap: false,
            crossfeed: false,
            limiter: true,
            peq_on: false,
            peq: Vec::new(),
            peq_pre: 0.0,
            peq_name: String::new(),
        }
    }
}

/// Gotowe ustawienia korektora: nazwa i 18 wartości w dB.
pub fn presets() -> Vec<(&'static str, [f32; 18])> {
    vec![
        ("Płasko", [0.0; 18]),
        ("Mocny bas", [7.0, 6.5, 6.0, 5.0, 4.0, 2.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("Bas i soprany", [6.0, 5.5, 5.0, 3.5, 2.0, 0.5, -0.5, -1.5, -1.5, -1.0, 0.0, 1.0, 2.5, 3.5, 4.5, 5.0, 5.5, 5.5]),
        ("Soprany", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 6.5, 7.0, 7.0]),
        ("Rock", [5.0, 4.5, 4.0, 3.0, 1.5, -0.5, -1.5, -2.0, -1.5, -0.5, 1.0, 2.0, 3.0, 3.5, 4.0, 4.5, 4.5, 4.5]),
        ("Pop", [-1.0, -0.5, 0.0, 1.0, 2.5, 3.5, 4.0, 4.0, 3.5, 2.5, 1.0, 0.0, -0.5, -1.0, -1.0, -0.5, 0.0, 0.0]),
        ("Jazz", [3.0, 2.5, 2.0, 1.5, 1.0, 0.0, -1.0, -1.0, -0.5, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.0, 3.0, 3.0]),
        ("Klasyka", [4.0, 3.5, 3.0, 2.5, 1.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5, -1.0, -1.5, -2.0, -2.5, -3.0]),
        ("Elektronika", [6.0, 5.5, 5.0, 3.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 0.5, 0.0, 1.0, 2.5, 4.0, 5.0, 5.5, 5.5]),
        ("Hip-hop", [7.0, 6.5, 6.0, 4.5, 3.0, 1.0, 0.0, -0.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.0, 2.0, 2.0]),
        ("Metal", [4.5, 4.0, 3.5, 2.0, 0.0, -2.0, -3.0, -2.5, -1.0, 0.5, 2.0, 3.5, 4.5, 4.5, 4.0, 3.5, 3.0, 3.0]),
        ("Akustyczne", [3.0, 2.5, 2.0, 1.5, 1.0, 0.5, 0.5, 1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 2.5, 3.0, 2.5, 2.0, 1.5]),
        ("Wokal", [-2.0, -2.0, -1.5, -1.0, 0.0, 1.5, 3.0, 4.0, 4.5, 4.5, 4.0, 3.0, 2.0, 1.0, 0.0, -0.5, -1.0, -1.5]),
        ("Podcast i mowa", [-6.0, -5.0, -4.0, -2.0, 0.0, 1.5, 3.0, 4.0, 4.0, 3.5, 3.0, 2.5, 1.5, 0.5, -1.0, -2.0, -3.0, -4.0]),
        ("Taneczna", [6.0, 6.0, 5.0, 3.0, 1.0, 0.0, -1.0, -1.0, 0.0, 0.5, 1.0, 2.0, 3.0, 3.0, 2.5, 2.0, 2.0, 2.0]),
        ("Głośniki laptopa", [-6.0, -4.0, -2.0, 0.0, 2.0, 3.0, 3.5, 3.0, 2.0, 1.0, 1.0, 1.5, 2.0, 2.5, 3.0, 2.0, 1.0, 0.0]),
        ("Słuchawki", [3.0, 3.0, 2.5, 1.5, 0.5, 0.0, -0.5, -0.5, 0.0, 0.0, 0.5, 1.0, 0.5, -0.5, 0.5, 2.0, 3.0, 3.0]),
        ("Głośność nocna", [-4.0, -3.5, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 2.5, 2.5, 2.0, 1.5, 1.0, 0.0, -1.0, -2.0, -3.0, -4.0]),
    ]
}

/// Preset dla gatunku z tagu – dopasowanie po słowach kluczowych (jak w OpenAMP).
pub fn preset_for_genre(genre: &str) -> Option<&'static str> {
    let g = genre.to_lowercase();
    if g.trim().is_empty() {
        return None;
    }
    let map: [(&[&str], &str); 13] = [
        (&["podcast", "speech", "audiobook", "spoken", "mowa", "audiobook", "słuchowisko"], "Podcast i mowa"),
        (&["metal", "hardcore", "grindcore"], "Metal"),
        (&["hip", "rap", "trap", "r&b", "rnb", "grime"], "Hip-hop"),
        (&["rock", "punk", "grunge", "alternative", "indie"], "Rock"),
        (&["dance", "disco", "eurodance", "disco polo"], "Taneczna"),
        (&["electr", "edm", "house", "techno", "trance", "dubstep", "drum", "synth"], "Elektronika"),
        (&["jazz", "blues", "swing", "soul", "funk"], "Jazz"),
        (&["classic", "klasycz", "orchestr", "baroque", "barok", "opera", "symphon", "symfon"], "Klasyka"),
        (&["acoustic", "akust", "folk", "country", "singer"], "Akustyczne"),
        (&["vocal", "wokal", "a cappella", "chanson", "poezja"], "Wokal"),
        (&["reggae", "dub", "ska"], "Mocny bas"),
        (&["ambient", "chill", "lo-fi", "lofi", "downtempo", "new age"], "Głośność nocna"),
        (&["pop"], "Pop"),
    ];
    map.iter().find(|(keys, _)| keys.iter().any(|k| g.contains(k))).map(|(_, p)| *p)
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: [f32; 2],
    z2: [f32; 2],
}

#[derive(Clone, Copy)]
enum Kind {
    Peak,
    LowShelf,
    HighShelf,
}

impl Biquad {
    /// Współczynniki RBJ (Audio EQ Cookbook); stan filtra zostaje – zmiana suwaka nie trzaska.
    fn set(&mut self, kind: Kind, f: f32, q: f32, db: f32, rate: f32) {
        let a = 10f32.powf(db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * (f / rate).min(0.49);
        let (sn, cs) = w0.sin_cos();
        let alpha = sn / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            Kind::Peak => (1.0 + alpha * a, -2.0 * cs, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cs, 1.0 - alpha / a),
            Kind::LowShelf => {
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cs + s),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cs),
                    a * ((a + 1.0) - (a - 1.0) * cs - s),
                    (a + 1.0) + (a - 1.0) * cs + s,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cs),
                    (a + 1.0) + (a - 1.0) * cs - s,
                )
            }
            Kind::HighShelf => {
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cs + s),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cs),
                    a * ((a + 1.0) + (a - 1.0) * cs - s),
                    (a + 1.0) - (a - 1.0) * cs + s,
                    2.0 * ((a - 1.0) - (a + 1.0) * cs),
                    (a + 1.0) - (a - 1.0) * cs - s,
                )
            }
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    #[inline]
    fn run(&mut self, c: usize, x: f32) -> f32 {
        // Transposed Direct Form II – stabilna przy f32.
        let y = self.b0 * x + self.z1[c];
        self.z1[c] = self.b1 * x - self.a1 * y + self.z2[c];
        self.z2[c] = self.b2 * x - self.a2 * y;
        y
    }
}

/// Stan obróbki dla danej częstotliwości wyjścia.
pub struct Dsp {
    pub settings: DspSettings,
    rate: f32,
    bands: Vec<(Biquad, bool)>,
    pbands: Vec<Biquad>,
    ppre: f32,
    pre: f32,
    lim_gain: f32,
    xf: [f32; 2],
}

impl Dsp {
    pub fn new(rate: u32) -> Dsp {
        let mut d = Dsp { settings: DspSettings::default(), rate: rate as f32, bands: Vec::new(), pbands: Vec::new(), ppre: 1.0, pre: 1.0, lim_gain: 1.0, xf: [0.0; 2] };
        d.apply(DspSettings::default());
        d
    }

    pub fn set_rate(&mut self, rate: u32) {
        if rate as f32 != self.rate {
            self.rate = rate as f32;
            let s = self.settings.clone();
            self.apply(s);
        }
    }

    pub fn apply(&mut self, s: DspSettings) {
        if self.bands.len() != BANDS.len() {
            self.bands = vec![(Biquad::default(), false); BANDS.len()];
        }
        for (i, f) in BANDS.iter().enumerate() {
            let db = s.gains.get(i).copied().unwrap_or(0.0).clamp(-MAX_DB, MAX_DB);
            let kind = match i {
                0 => Kind::LowShelf,
                i if i == BANDS.len() - 1 => Kind::HighShelf,
                _ => Kind::Peak,
            };
            let (bq, active) = &mut self.bands[i];
            bq.set(kind, *f, if matches!(kind, Kind::Peak) { 2.0 } else { 0.8 }, db, self.rate);
            *active = db.abs() > 0.01;
        }
        self.pre = 10f32.powf(s.preamp.clamp(-20.0, 20.0) / 20.0);
        // Filtry parametryczne – stan zostaje, gdy liczba filtrów się nie zmienia.
        if self.pbands.len() != s.peq.len() {
            self.pbands = vec![Biquad::default(); s.peq.len()];
        }
        for (bq, p) in self.pbands.iter_mut().zip(s.peq.iter()) {
            let kind = match p.kind {
                1 => Kind::LowShelf,
                2 => Kind::HighShelf,
                _ => Kind::Peak,
            };
            bq.set(kind, p.f.clamp(10.0, 22_000.0), p.q.clamp(0.05, 20.0), p.db.clamp(-30.0, 30.0), self.rate);
        }
        self.ppre = 10f32.powf(s.peq_pre.clamp(-30.0, 10.0) / 20.0);
        self.settings = s;
    }

    /// Obróbka stereo w miejscu.
    pub fn process(&mut self, buf: &mut [f32]) {
        let s = &self.settings;
        let eq = s.eq_on;
        let pre = if eq { self.pre } else { 1.0 };
        let bal = s.balance.clamp(-1.0, 1.0);
        let (gl, gr) = (if bal > 0.0 { 1.0 - bal } else { 1.0 }, if bal < 0.0 { 1.0 + bal } else { 1.0 });
        let width = s.width.clamp(0.0, 2.0);
        // Przesłuch: dolnoprzepustowy przeciek ok. 700 Hz do drugiego kanału (Bauer, uproszczony).
        let xa = (-2.0 * std::f32::consts::PI * 700.0 / self.rate).exp();
        let lim_release = (-1.0 / (0.15 * self.rate)).exp();
        for f in buf.chunks_exact_mut(2) {
            let (mut l, mut r) = if s.swap { (f[1], f[0]) } else { (f[0], f[1]) };
            if s.peq_on && !self.pbands.is_empty() {
                l *= self.ppre;
                r *= self.ppre;
                for bq in self.pbands.iter_mut() {
                    l = bq.run(0, l);
                    r = bq.run(1, r);
                }
            }
            if eq {
                l *= pre;
                r *= pre;
                for (bq, active) in self.bands.iter_mut() {
                    if *active {
                        l = bq.run(0, l);
                        r = bq.run(1, r);
                    }
                }
            }
            if s.mono {
                let m = (l + r) * 0.5;
                l = m;
                r = m;
            } else if (width - 1.0).abs() > 0.01 {
                let m = (l + r) * 0.5;
                let sd = (l - r) * 0.5 * width;
                l = m + sd;
                r = m - sd;
            }
            if s.crossfeed {
                self.xf[0] = xa * self.xf[0] + (1.0 - xa) * l;
                self.xf[1] = xa * self.xf[1] + (1.0 - xa) * r;
                let (nl, nr) = (l * 0.8 + self.xf[1] * 0.3, r * 0.8 + self.xf[0] * 0.3);
                l = nl;
                r = nr;
            }
            l *= gl;
            r *= gr;
            if s.limiter {
                let peak = l.abs().max(r.abs());
                // Rusza dopiero przy realnym przesterowaniu – pełna skala gra bez ściszania.
                let want = if peak * self.lim_gain > 0.999 { 0.999 / peak } else { 1.0 };
                // Atak natychmiastowy, powolne odpuszczanie.
                self.lim_gain = if want < self.lim_gain { want } else { want + (self.lim_gain - want) * lim_release };
                l *= self.lim_gain;
                r *= self.lim_gain;
            }
            f[0] = l;
            f[1] = r;
        }
    }
}
