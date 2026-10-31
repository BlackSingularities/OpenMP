//! Wyświetlacz: opis utworu, okładka i tło, barwa z okładki, fala utworu, tekst piosenki,
//! informacje szczegółowe i analizator (klatka po klatce).

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use openmp_core::audio::decoder::StreamInfo;
use openmp_core::audio::viz::Analyzer;
use openmp_core::meta::{self, TrackInfo};
use slint::{Color, Image, Model, ModelRc, VecModel};

use crate::app::*;
use crate::*;

static GEN: AtomicU64 = AtomicU64::new(0);

fn khz(r: u32) -> String {
    if r == 0 {
        return String::new();
    }
    let k = r as f64 / 1000.0;
    if (k - k.round()).abs() < 0.01 {
        format!("{} kHz", k.round())
    } else {
        format!("{k:.1} kHz").replace('.', ",")
    }
}

fn badges(t: &TrackInfo, info: Option<&StreamInfo>) -> Vec<String> {
    let mut v = Vec::new();
    let fmt = info.map(|i| i.codec.clone()).filter(|c| !c.is_empty() && c != "Audio").unwrap_or_else(|| t.format());
    if !fmt.is_empty() {
        v.push(fmt);
    }
    let rate = info.map(|i| i.rate).filter(|r| *r > 0).unwrap_or(t.rate);
    if rate > 0 {
        v.push(khz(rate));
    }
    let bits = info.map(|i| i.bits).filter(|b| *b > 0).unwrap_or(t.bits as u32);
    if bits > 0 {
        v.push(format!("{bits} bit"));
    }
    let br = if t.bitrate > 0 { t.bitrate } else { info.map(|i| i.bitrate).unwrap_or(0) };
    if br > 0 {
        v.push(format!("{br} kb/s"));
    }
    let ch = if t.channels > 0 { t.channels as usize } else { info.map(|i| i.channels).unwrap_or(0) };
    match ch {
        1 => v.push("Mono".into()),
        2 => v.push("Stereo".into()),
        0 => {}
        n => v.push(format!("{n} kan.")),
    }
    v
}

/// Nowy utwór na wyświetlaczu.
pub fn show_track(app: &mut App) {
    let Some(c) = app.cur.clone() else { return };
    let Some(ui) = app.ui() else { return };
    let t = &c.info;
    let p = ui.global::<Player>();
    let title = match &c.station {
        Some(st) => st.name.clone(),
        None => t.display_title(),
    };
    p.set_title(ss(&title));
    p.set_artist(ss(match &c.station {
        Some(st) => st.info(),
        None => t.display_artist(),
    }));
    let mut album = t.album.clone();
    if t.year > 0 {
        album = if album.is_empty() { t.year.to_string() } else { format!("{album}  ·  {}", t.year) };
    }
    if c.video && album.is_empty() {
        album = Path::new(&t.path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    }
    p.set_album(ss(album));
    p.set_badges(crate::app::strings(&badges(t, None)));
    p.set_idle(false);
    p.set_fav(app.stats.get(&t.path).rating >= 5);
    p.set_position_text(ss(match c.list.and_then(|l| app.list_index(l)) {
        Some(li) => format!("{} / {}", c.idx + 1, app.store.lists[li].items.len()),
        None => String::new(),
    }));
    p.set_state_text(ss(if c.station.is_some() { "na żywo" } else { "" }));
    p.set_wave(ss(""));
    p.set_viz_note(ss(""));
    ui.global::<Video>().set_title(ss(&title));
    // Okładka, tło i barwa – w tle.
    let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let path = t.path.clone();
    let station_icon = c.station.as_ref().map(|s| s.favicon.clone()).filter(|f| !f.is_empty());
    let from_cover = app.s.accent_from_cover;
    let weak = app.ui.clone();
    let artist = if t.album_artist.trim().is_empty() { t.artist.clone() } else { t.album_artist.clone() };
    let album = t.album.clone();
    std::thread::spawn(move || {
        let bytes = match station_icon {
            Some(url) => ureq::get(&url).timeout(std::time::Duration::from_secs(6)).call().ok().and_then(|r| {
                let mut b = Vec::new();
                std::io::Read::read_to_end(&mut r.into_reader(), &mut b).ok()?;
                Some(b)
            }),
            None if meta::is_url(&path) => None,
            None if meta::is_video(Path::new(&path)) => meta::folder_cover(Path::new(&path)).and_then(|p| std::fs::read(p).ok()),
            None => meta::cover(Path::new(&path)).or_else(|| openmp_core::coverart::fetch(&artist, &album)),
        };
        let decoded = bytes.as_ref().and_then(|b| image::load_from_memory(b).ok());
        let (big, back, tint) = match &decoded {
            Some(img) => {
                let big = img.thumbnail(600, 600).to_rgba8();
                let small = img.thumbnail(48, 48).blur(2.5).to_rgba8();
                let tint = dominant(&img.thumbnail(32, 32).to_rgba8());
                (
                    Some(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(big.as_raw(), big.width(), big.height())),
                    Some(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(small.as_raw(), small.width(), small.height())),
                    Some(tint),
                )
            }
            None => (None, None, None),
        };
        if GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let _ = slint::invoke_from_event_loop(move || {
            if GEN.load(Ordering::SeqCst) != gen {
                return;
            }
            let Some(ui) = weak.upgrade() else { return };
            let p = ui.global::<Player>();
            match (big, back) {
                (Some(b), Some(k)) => {
                    p.set_cover(Image::from_rgba8(b));
                    p.set_backdrop(Image::from_rgba8(k));
                    p.set_has_cover(true);
                }
                _ => {
                    p.set_cover(Image::default());
                    p.set_backdrop(Image::default());
                    p.set_has_cover(false);
                }
            }
            let accent = ui.global::<Theme>().get_accent();
            let t = match tint {
                Some((r, g, b)) if from_cover => Color::from_rgb_u8(r, g, b),
                _ => accent,
            };
            ui.global::<Theme>().set_tint(t);
            later(move |app| {
                app.cover_bytes = bytes;
                crate::win::smtc_meta(app);
            });
        });
    });
    // Fala utworu
    app.wave_raw.clear();
    if app.s.show_waveform && !meta::is_url(&t.path) && !c.video {
        let path = t.path.clone();
        let style = app.s.wave_style;
        let weak = app.ui.clone();
        std::thread::spawn(move || {
            let cancel = || GEN.load(Ordering::SeqCst) != gen;
            let Some(w) = openmp_core::audio::viz::waveform(Path::new(&path), 500, &cancel) else { return };
            let s = wave_path(&w, style);
            let _ = slint::invoke_from_event_loop(move || {
                if GEN.load(Ordering::SeqCst) == gen {
                    if let Some(ui) = weak.upgrade() {
                        ui.global::<Player>().set_wave(ss(s));
                    }
                    later(move |app| app.wave_raw = w);
                }
            });
        });
    }
    load_lyrics(app);
    crate::prefs::auto_eq(app, true);
    details(app);
    crate::win::smtc_meta(app);
}

/// Barwa przewodnia okładki: najczęstszy nasycony odcień, rozjaśniony do czytelności.
fn dominant(img: &image::RgbaImage) -> (u8, u8, u8) {
    let mut buckets = [(0f32, 0f32, 0f32, 0f32); 24];
    for p in img.pixels() {
        let (r, g, b) = (p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        let sat = if mx > 0.0 { (mx - mn) / mx } else { 0.0 };
        if mx < 0.15 || sat < 0.2 {
            continue;
        }
        let h = hue(r, g, b);
        let i = ((h / 360.0) * 24.0) as usize % 24;
        let w = sat * mx;
        buckets[i].0 += r * w;
        buckets[i].1 += g * w;
        buckets[i].2 += b * w;
        buckets[i].3 += w;
    }
    let best = buckets.iter().max_by(|a, b| a.3.total_cmp(&b.3)).copied().unwrap_or_default();
    if best.3 <= 0.5 {
        return (0x4f, 0x8c, 0xff);
    }
    let (mut r, mut g, mut b) = (best.0 / best.3, best.1 / best.3, best.2 / best.3);
    // Rozjaśnienie do jasności ok. 0.85 i lekkie podbicie nasycenia.
    let mx = r.max(g).max(b).max(1e-3);
    let k = 0.88 / mx;
    r = (r * k).min(1.0);
    g = (g * k).min(1.0);
    b = (b * k).min(1.0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn hue(r: f32, g: f32, b: f32) -> f32 {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    if d <= 0.0 {
        return 0.0;
    }
    let h = if mx == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    if h < 0.0 {
        h + 360.0
    } else {
        h
    }
}

/// Parametry z dekodera (dokładniejsze niż z tagów) – raz po starcie utworu.
pub fn update_badges_from(app: &mut App, info: &StreamInfo) {
    if info.rate == 0 {
        return;
    }
    let Some(c) = &app.cur else { return };
    let mut b = badges(&c.info, Some(info));
    // Droga do karty: tryb wyłączny albo przepróbkowanie przez mikser Windows.
    let (out, _, excl) = app.engine.output();
    if excl {
        b.push(if out == info.rate { "Wyłączny".into() } else { format!("Wyłączny → {}", khz(out)) });
    } else if out > 0 && out != info.rate {
        b.push(format!("→ {}", khz(out)));
    }
    if let Some(ui) = app.ui() {
        let cur: Vec<String> = ui.global::<Player>().get_badges().iter().map(|s| s.to_string()).collect();
        if cur != b {
            ui.global::<Player>().set_badges(crate::app::strings(&b));
        }
    }
}

fn load_lyrics(app: &mut App) {
    app.lyrics.clear();
    app.lyrics_text.clear();
    let Some(c) = &app.cur else { return };
    let mut text = None;
    let mut source = "";
    if !meta::is_url(&c.info.path) && !c.video {
        text = meta::lyrics(Path::new(&c.info.path));
        source = "plik / tagi";
    }
    let online = text.is_none() && app.s.online_lyrics && !c.video && c.station.is_none();
    let (artist, title, album, dur) = (c.info.artist.clone(), c.info.title.clone(), c.info.album.clone(), c.info.duration);
    if text.is_none() && online {
        // Z pamięci podręcznej od razu, z sieci – w tle.
        match openmp_core::lrclib::cached(&artist, &title) {
            Some(t) if !t.trim().is_empty() => {
                text = Some(t);
                source = "LRCLIB";
            }
            Some(_) => {}
            None => {
                let gen = GEN.load(Ordering::SeqCst);
                std::thread::spawn(move || {
                    let Some(t) = openmp_core::lrclib::fetch(&artist, &title, &album, dur) else { return };
                    let _ = slint::invoke_from_event_loop(move || {
                        if GEN.load(Ordering::SeqCst) == gen {
                            later(move |app| set_lyrics(app, Some(t), "LRCLIB"));
                        }
                    });
                });
            }
        }
    }
    set_lyrics(app, text, source);
}

fn set_lyrics(app: &mut App, text: Option<String>, source: &str) {
    app.lyrics.clear();
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut synced = false;
    if let Some(text) = &text {
        let lrc = meta::parse_lrc(text);
        if !lrc.is_empty() {
            synced = true;
            lines = lrc.iter().map(|(_, l)| LyricLine { text: ss(l), active: false }).collect();
            app.lyrics = lrc;
        } else {
            lines = text.lines().map(|l| LyricLine { text: ss(l.trim()), active: false }).collect();
        }
    }
    app.lyrics_text = text.unwrap_or_default();
    app.m.lyrics.set_vec(lines);
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_lyrics_synced(synced);
        p.set_lyrics_active(-1);
        p.set_lyrics_source(ss(if app.lyrics_text.is_empty() { "" } else { source }));
    }
}

/// Zapis pobranego tekstu jako .lrc obok utworu – kolejne odtworzenia nie potrzebują sieci.
pub fn save_lyrics(app: &mut App) {
    let Some(c) = &app.cur else { return };
    if app.lyrics_text.is_empty() || meta::is_url(&c.info.path) {
        return;
    }
    let ok = openmp_core::lrclib::save_next_to(Path::new(&c.info.path), &app.lyrics_text);
    app.toast(if ok { "Zapisano tekst obok utworu (.lrc)" } else { "Nie udało się zapisać pliku .lrc" });
    if ok {
        if let Some(ui) = app.ui() {
            ui.global::<Player>().set_lyrics_source(ss("plik / tagi"));
        }
    }
}

/// Ścieżka taśmy utworu w układzie 1000×100 w wybranym stylu.
pub fn wave_path(w: &[f32], style: i32) -> String {
    let n = w.len();
    if n < 2 {
        return String::new();
    }
    let x_of = |i: usize| i as f32 * 1000.0 / (n - 1) as f32;
    let mut s = String::with_capacity(n * 24);
    match style {
        // Słupki z przerwami (segmenty).
        1 => {
            let count = 120usize;
            let step = 1000.0 / count as f32;
            for k in 0..count {
                let a = k * n / count;
                let b = ((k + 1) * n / count).max(a + 1).min(n);
                let v = w[a..b].iter().cloned().fold(0.0f32, f32::max).max(0.04);
                let x = k as f32 * step + step * 0.18;
                let bw = step * 0.64;
                let h = v * 46.0;
                s.push_str(&format!("M {x:.1} {:.1} h {bw:.1} v {:.1} h {:.1} Z ", 50.0 - h, h * 2.0, -bw));
            }
        }
        // Fala jednostronna, od dołu.
        2 => {
            s.push_str("M 0 100");
            for (i, v) in w.iter().enumerate() {
                s.push_str(&format!(" L {:.1} {:.1}", x_of(i), 100.0 - v.max(0.06) * 94.0));
            }
            s.push_str(" L 1000 100 Z");
        }
        // Kontur: cienka wstęga po górnej krawędzi fali i linia środka.
        3 => {
            for (i, v) in w.iter().enumerate() {
                s.push_str(&format!("{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, x_of(i), 50.0 - v.max(0.06) * 46.0));
            }
            for (i, v) in w.iter().enumerate().rev() {
                s.push_str(&format!(" L {:.1} {:.1}", x_of(i), 50.0 - v.max(0.06) * 46.0 + 5.0));
            }
            s.push_str(" Z M 0 49 L 1000 49 L 1000 51 L 0 51 Z");
        }
        // Fala symetryczna względem środka.
        _ => {
            for (i, v) in w.iter().enumerate() {
                s.push_str(&format!("{}{:.1} {:.1}", if i == 0 { "M " } else { " L " }, x_of(i), 50.0 - v.max(0.06) * 46.0));
            }
            for (i, v) in w.iter().enumerate().rev() {
                s.push_str(&format!(" L {:.1} {:.1}", x_of(i), 50.0 + v.max(0.06) * 46.0));
            }
            s.push_str(" Z");
        }
    }
    s
}

/// Nowy styl taśmy bez ponownego dekodowania utworu.
pub fn restyle_wave(app: &mut App) {
    if app.wave_raw.is_empty() {
        return;
    }
    let s = wave_path(&app.wave_raw, app.s.wave_style);
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_wave(ss(s));
    }
}

fn details(app: &mut App) {
    let Some(c) = &app.cur else { return };
    let t = &c.info;
    let st = app.stats.get(&t.path);
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |l: &str, s: String| {
        if !s.trim().is_empty() && s != "0" {
            v.push((l.into(), s));
        }
    };
    add("Tytuł", t.display_title());
    add("Wykonawca", t.artist.clone());
    add("Album", t.album.clone());
    add("Wykonawca albumu", t.album_artist.clone());
    add("Rok", if t.year > 0 { t.year.to_string() } else { String::new() });
    add("Gatunek", t.genre.clone());
    add("Utwór", if t.track > 0 { if t.disc > 0 { format!("{} (płyta {})", t.track, t.disc) } else { t.track.to_string() } } else { String::new() });
    add("Kompozytor", t.composer.clone());
    add("Długość", if t.duration > 0.0 { meta::fmt_time(t.duration) } else { String::new() });
    add("Format", badges(t, None).join(" · "));
    add("ReplayGain", match (t.rg_track, t.rg_album) {
        (Some(a), Some(b)) => format!("utwór {a:+.2} dB, album {b:+.2} dB"),
        (Some(a), None) => format!("utwór {a:+.2} dB"),
        (None, Some(b)) => format!("album {b:+.2} dB"),
        _ => String::new(),
    });
    add("Rozmiar", if t.size > 0 { format!("{:.1} MB", t.size as f64 / 1_048_576.0) } else { String::new() });
    add("Odtworzenia", st.plays.to_string());
    add("Komentarz", t.comment.clone());
    add("Plik", t.path.clone());
    let rows: Vec<Opt> = v.into_iter().map(|(l, s)| Opt { id: ss(""), label: ss(l), sub: ss(s), selected: false }).collect();
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_details(ModelRc::new(VecModel::from(rows)));
    }
}

/// Klatka analizatora i tekstu piosenki.
pub fn frame(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let now = Instant::now();
    let dt = now.duration_since(app.last_frame).as_secs_f32().min(0.1);
    app.last_frame = now;
    let mode = ui.global::<Ui>().get_mode();
    if mode == 1 || ui.window().is_minimized() {
        return;
    }
    let p = ui.global::<Player>();
    let playing = p.get_playing();
    let total = app.engine.scope(Analyzer::frames(), app.analyzer.input());
    let total = if playing { total } else { 0 };
    let bands = app.s.viz_bands.clamp(8, 128) as usize;
    app.analyzer.update(bands, app.engine.rate(), total, dt, app.s.viz_sensitivity);
    let viz = app.s.viz;
    if viz == 0 || viz == 1 {
        if app.m.bars.row_count() != bands {
            app.m.bars.set_vec(app.analyzer.bars.clone());
            app.m.peaks.set_vec(app.analyzer.peaks.clone());
        } else {
            for i in 0..bands {
                app.m.bars.set_row_data(i, app.analyzer.bars[i]);
                app.m.peaks.set_row_data(i, app.analyzer.peaks[i]);
            }
        }
        if viz == 1 {
            p.set_spectrum_path(ss(app.analyzer.spectrum_path()));
        }
    } else if viz == 2 {
        p.set_scope_path(ss(app.analyzer.scope_path(220)));
    } else if viz == 3 {
        p.set_vu_l(app.analyzer.vu[0]);
        p.set_vu_r(app.analyzer.vu[1]);
        p.set_vu_pl(app.analyzer.vu_peak[0]);
        p.set_vu_pr(app.analyzer.vu_peak[1]);
    }
    p.set_viz_active(true);
    // Tekst synchronizowany
    if !app.lyrics.is_empty() {
        let pos = p.get_pos() as f64 + 0.15;
        let idx = app.lyrics.iter().rposition(|(t, _)| *t <= pos).map(|i| i as i32).unwrap_or(-1);
        if idx != p.get_lyrics_active() {
            let old = p.get_lyrics_active();
            if old >= 0 && (old as usize) < app.m.lyrics.row_count() {
                if let Some(mut r) = app.m.lyrics.row_data(old as usize) {
                    r.active = false;
                    app.m.lyrics.set_row_data(old as usize, r);
                }
            }
            if idx >= 0 && (idx as usize) < app.m.lyrics.row_count() {
                if let Some(mut r) = app.m.lyrics.row_data(idx as usize) {
                    r.active = true;
                    app.m.lyrics.set_row_data(idx as usize, r);
                }
            }
            p.set_lyrics_active(idx);
        }
    }
}

pub fn cycle_viz(app: &mut App) {
    app.s.viz = (app.s.viz + 1) % 5;
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_viz_mode(app.s.viz);
    }
    let names = ["Wizualizacja: słupki", "Wizualizacja: widmo", "Wizualizacja: oscyloskop", "Wizualizacja: wskaźniki VU", "Wizualizacja wyłączona"];
    let n = names[app.s.viz as usize % 5];
    app.toast(n);
    app.dirty = true;
}
