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
/// Czytanie napisów z kontenera – nowa ścieżka albo film przerywa poprzednie.
static SUB_GEN: AtomicU64 = AtomicU64::new(0);

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

/// Ikona parametru przy tytule (nazwa z Icons.get).
fn badge_icon(b: &str) -> &'static str {
    let l = b.to_lowercase();
    if l.ends_with('p') && l[..l.len() - 1].chars().all(|c| c.is_ascii_digit()) || l == "4k" || l == "8k" {
        "aspect"
    } else if l.contains("kl./s") {
        "movie"
    } else if l.contains("khz") {
        "equalizer"
    } else if l.contains("bit") && !l.contains("kb/s") {
        "tune"
    } else if l.contains("kb/s") {
        "trending"
    } else if l.contains("hdr") || l == "hlg" {
        "eye"
    } else if l == "mono" || l == "stereo" || l.contains("kan.") {
        "audiotrack"
    } else if l.contains("wyłączny") || l.starts_with('→') {
        "volume"
    } else {
        "file"
    }
}

/// Parametry przy tytule z ikonami – muzyka i film tak samo.
pub fn set_badges(p: &Player, b: &[String]) {
    p.set_badges(crate::app::strings(b));
    let icons: Vec<String> = b.iter().map(|s| badge_icon(s).to_string()).collect();
    p.set_badge_icons(crate::app::strings(&icons));
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
    p.set_overview(ss(""));
    p.set_credits(ss(""));
    // Film: polski tytuł, rok, gatunki, ocena i opis z TMDB (w tle, gdy jest klucz).
    if c.video && !meta::is_url(&t.path) && !app.s.tmdb_key.is_empty() {
        let path = t.path.clone();
        std::thread::spawn(move || {
            let Some(info) = openmp_core::tmdb::lookup(Path::new(&path)) else { return };
            let _ = slint::invoke_from_event_loop(move || {
                later(move |app| {
                    if !app.cur.as_ref().is_some_and(|c| c.info.path == path) {
                        return;
                    }
                    let Some(ui) = app.ui() else { return };
                    let p = ui.global::<Player>();
                    if !info.title.is_empty() {
                        p.set_title(ss(&info.title));
                    }
                    p.set_album(ss(info.line()));
                    p.set_overview(ss(&info.overview));
                    p.set_credits(ss(info.credits()));
                });
            });
        });
    }
    set_badges(&p, &badges(t, None));
    p.set_idle(false);
    p.set_fav(app.stats.get(&t.path).rating >= 5);
    p.set_position_text(ss(match c.list.and_then(|l| app.list_index(l)) {
        Some(li) => format!("{} / {}", c.idx + 1, app.store.lists[li].items.len()),
        None => String::new(),
    }));
    p.set_state_text(ss(if c.station.is_some() { "na żywo" } else { "" }));
    p.set_wave(ss(""));
    p.set_wave2(ss(""));
    p.set_wave_live(ss(""));
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
        let (big, back, tint, pal) = match &decoded {
            Some(img) => {
                let big = img.thumbnail(600, 600).to_rgba8();
                // Tło i szkło: większa miniatura i mocne rozmycie gaussowskie – po rozciągnięciu
                // na okno gładkie przejścia barw zamiast kwadratów.
                let small = img.thumbnail(160, 160).blur(12.0).to_rgba8();
                let thumb = img.thumbnail(32, 32).to_rgba8();
                let tint = dominant(&thumb);
                let pal = palette(&thumb, 3);
                (
                    Some(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(big.as_raw(), big.width(), big.height())),
                    Some(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(small.as_raw(), small.width(), small.height())),
                    Some(tint),
                    pal,
                )
            }
            None => (None, None, None, Vec::new()),
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
                    let k = Image::from_rgba8(k);
                    p.set_backdrop(k.clone());
                    p.set_has_cover(true);
                    let th = ui.global::<Theme>();
                    th.set_glass_image(k);
                    th.set_glass_has(true);
                }
                _ => {
                    p.set_cover(Image::default());
                    p.set_backdrop(Image::default());
                    p.set_has_cover(false);
                    ui.global::<Theme>().set_glass_has(false);
                }
            }
            let accent = ui.global::<Theme>().get_accent();
            let t = match tint {
                Some((r, g, b)) if from_cover => Color::from_rgb_u8(r, g, b),
                _ => accent,
            };
            let th = ui.global::<Theme>();
            th.set_tint(t);
            // Gradient tła: trzy barwy okładki (brakujące – odcienie akcentu).
            let c = |i: usize, f: f32| pal.get(i).map(|&(r, g, b)| Color::from_rgb_u8(r, g, b)).unwrap_or_else(|| if f >= 0.0 { t.brighter(f) } else { t.darker(-f) });
            th.set_grad_a(c(0, 0.0));
            th.set_grad_b(c(1, -0.35));
            th.set_grad_c(c(2, 0.3));
            later(move |app| {
                app.tint_target = None;
                app.cover_bytes = bytes;
                crate::win::smtc_meta(app);
            });
        });
    });
    // Fala utworu
    app.wave_raw.clear();
    if app.s.show_waveform && !meta::is_url(&t.path) && !c.video {
        let path = t.path.clone();
        std::thread::spawn(move || {
            let cancel = || GEN.load(Ordering::SeqCst) != gen;
            let Some(w) = openmp_core::audio::viz::waveform(Path::new(&path), 1500, &cancel) else { return };
            let _ = slint::invoke_from_event_loop(move || {
                if GEN.load(Ordering::SeqCst) == gen {
                    later(move |app| {
                        app.wave_raw = w;
                        restyle_wave(app);
                    });
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
/// Okładka czarno-biała (bez wyraźnych barw) – biel zamiast niebieskiego.
fn dominant(img: &image::RgbaImage) -> (u8, u8, u8) {
    palette(img, 1).first().copied().unwrap_or((0xe8, 0xe8, 0xe8))
}

/// Do `n` najmocniejszych odcieni obrazu (od najczęstszego), rozjaśnionych do czytelności.
/// Pusta, gdy obraz nie ma wyraźnych barw.
pub fn palette(img: &image::RgbaImage, n: usize) -> Vec<(u8, u8, u8)> {
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
    let mut order: Vec<usize> = (0..24).collect();
    order.sort_by(|&a, &b| buckets[b].3.total_cmp(&buckets[a].3));
    let total: f32 = buckets.iter().map(|b| b.3).sum();
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let bk = buckets[i];
        // Pierwsza barwa musi być wyraźna, kolejne – nie śladowe (inaczej powtarzamy pierwszą).
        if bk.3 <= 0.5 || (k > 0 && bk.3 < total * 0.06) || out.len() >= n {
            break;
        }
        let (mut r, mut g, mut b) = (bk.0 / bk.3, bk.1 / bk.3, bk.2 / bk.3);
        // Rozjaśnienie do jasności ok. 0.85 i lekkie podbicie nasycenia.
        let mx = r.max(g).max(b).max(1e-3);
        let f = 0.88 / mx;
        r = (r * f).min(1.0);
        g = (g * f).min(1.0);
        b = (b * f).min(1.0);
        out.push(((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8));
    }
    out
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
            set_badges(&ui.global::<Player>(), &b);
        }
    }
}

fn load_lyrics(app: &mut App) {
    app.lyrics.clear();
    app.lyrics_text.clear();
    let Some(c) = &app.cur else { return };
    let video = c.video;
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_subs_mode(video);
        // Wyszukiwanie dotyczy tylko bieżącego utworu – nowy zaczyna bez niego.
        p.set_lyrics_query(ss(""));
    }
    app.sub_live = false;
    SUB_GEN.fetch_add(1, Ordering::SeqCst);
    // Napisy filmu przychodzą po wczytaniu pliku przez mpv (sub_panel_setup).
    if video {
        app.sub_panel.clear();
        app.sub_panel_id = 0;
        app.sub_live = false;
        publish_sub_langs(app);
        set_lyrics(app, None, "");
        return;
    }
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
            lines = lrc.iter().map(|(_, l)| LyricLine { text: ss(l), active: false, hit: false }).collect();
            app.lyrics = lrc;
        } else {
            lines = text.lines().map(|l| LyricLine { text: ss(l.trim()), active: false, hit: false }).collect();
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
    apply_search(app, true);
}

/// Ścieżki napisów filmu do panelu tekstu – po wczytaniu pliku. Domyślnie ta na ekranie,
/// potem pierwsza w preferowanym języku.
pub fn sub_panel_setup(app: &mut App, tracks: &[openmp_core::mpv::TrackEntry]) {
    // Napisy obrazkowe (PGS, DVD, DVB) nie mają tekstu.
    let text = |t: &&openmp_core::mpv::TrackEntry| t.kind == "sub" && !matches!(t.codec.as_str(), "hdmv_pgs_subtitle" | "dvd_subtitle" | "dvb_subtitle" | "pgs" | "vobsub");
    app.sub_panel = tracks.iter().filter(text).cloned().collect();
    let keep = app.sub_panel.iter().any(|t| t.id == app.sub_panel_id);
    if !keep {
        let langs: Vec<String> = app.s.sub_lang.split(',').map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect();
        app.sub_panel_id = app
            .sub_panel
            .iter()
            .find(|t| t.selected)
            .or_else(|| langs.iter().find_map(|l| app.sub_panel.iter().find(|t| t.lang.to_lowercase() == *l)))
            .or(app.sub_panel.first())
            .map(|t| t.id)
            .unwrap_or(0);
    }
    publish_sub_langs(app);
    load_sub_panel(app);
}

pub fn pick_sub_panel(app: &mut App, i: i32) {
    let Some(t) = app.sub_panel.get(i.max(0) as usize) else { return };
    app.sub_panel_id = t.id;
    publish_sub_langs(app);
    load_sub_panel(app);
}

fn publish_sub_langs(app: &App) {
    let names: Vec<String> = app
        .sub_panel
        .iter()
        .map(|t| {
            let mut n = if t.lang.is_empty() { format!("Ścieżka {}", t.id) } else { t.lang.clone() };
            if !t.title.is_empty() {
                n = format!("{n} · {}", t.title);
            }
            n
        })
        .collect();
    let idx = app.sub_panel.iter().position(|t| t.id == app.sub_panel_id).map(|i| i as i32).unwrap_or(-1);
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_sub_langs(crate::app::strings(&names));
        p.set_sub_lang(idx);
    }
}

/// Plik zewnętrzny – cały od razu; ścieżka z kontenera – zbierana na bieżąco jako
/// niewidoczne napisy dodatkowe mpv.
fn load_sub_panel(app: &mut App) {
    app.sub_live = false;
    SUB_GEN.fetch_add(1, Ordering::SeqCst);
    if let Some(m) = &app.mpv {
        m.set("secondary-sid", "no");
    }
    let Some(t) = app.sub_panel.iter().find(|t| t.id == app.sub_panel_id).cloned() else {
        set_lyrics(app, None, "");
        return;
    };
    if t.external && !t.external_filename.is_empty() {
        if let Ok(b) = std::fs::read(&t.external_filename) {
            let lines = meta::parse_subs(&decode_text(&b));
            if !lines.is_empty() {
                set_sub_lines(app, lines);
                return;
            }
        }
    }
    app.sub_live = true;
    set_sub_lines(app, Vec::new());
    if let Some(m) = &app.mpv {
        m.set("secondary-sub-visibility", "no");
        m.set("secondary-sid", &t.id.to_string());
    }
    // Napisy z pliku Matroska – całe w tle; do tego czasu kwestie dopisują się na bieżąco.
    let gen = SUB_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(path) = app.cur.as_ref().map(|c| c.info.path.clone()).filter(|p| !meta::is_url(p)) else { return };
    if t.src_id < 0 {
        return;
    }
    let (id, src) = (t.id, t.src_id as u64);
    std::thread::spawn(move || {
        let stop = || SUB_GEN.load(Ordering::SeqCst) != gen;
        let Some(lines) = openmp_core::mkvsubs::subtitles(Path::new(&path), src, &stop) else { return };
        if lines.is_empty() {
            return;
        }
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                let same = app.cur.as_ref().is_some_and(|c| c.info.path == path);
                if SUB_GEN.load(Ordering::SeqCst) == gen && same && app.sub_panel_id == id {
                    app.sub_live = false;
                    set_sub_lines(app, lines);
                }
            })
        });
    });
}

fn set_sub_lines(app: &mut App, lines: Vec<(f64, String)>) {
    app.m.lyrics.set_vec(lines.iter().map(|(_, l)| LyricLine { text: ss(l), active: false, hit: false }).collect::<Vec<_>>());
    app.lyrics = lines;
    app.lyrics_text.clear();
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_lyrics_synced(true);
        p.set_lyrics_active(-1);
        p.set_lyrics_source(ss("napisy"));
    }
    apply_search(app, true);
}

/// UTF-8 albo (częste w polskich SRT) Windows-1250.
fn decode_text(b: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(b) {
        return s.to_string();
    }
    b.iter()
        .map(|&c| match c {
            0x8C => 'Ś',
            0x8F => 'Ź',
            0x9C => 'ś',
            0x9F => 'ź',
            0xA3 => 'Ł',
            0xA5 => 'Ą',
            0xAF => 'Ż',
            0xB3 => 'ł',
            0xB9 => 'ą',
            0xBF => 'ż',
            0xC6 => 'Ć',
            0xCA => 'Ę',
            0xD1 => 'Ń',
            0xD3 => 'Ó',
            0xE6 => 'ć',
            0xEA => 'ę',
            0xF1 => 'ń',
            0xF3 => 'ó',
            c => char::from(c),
        })
        .collect()
}

/// Kolejna kwestia z napisów z kontenera – dopisana w kolejności czasu.
fn poll_live_subs(app: &mut App) {
    if !app.sub_live {
        return;
    }
    let Some(m) = &app.mpv else { return };
    // Ta sama ścieżka na ekranie – mpv nie da jej drugi raz jako dodatkowej.
    let same = m.get_str("sid").is_some_and(|s| s == app.sub_panel_id.to_string());
    let (tp, sp) = if same { ("sub-text", "sub-start") } else { ("secondary-sub-text", "secondary-sub-start") };
    let Some(text) = m.get_str(tp) else { return };
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(start) = m.get_f64(sp) else { return };
    if text.is_empty() || app.lyrics.iter().any(|(t, _)| (t - start).abs() < 0.05) {
        return;
    }
    let i = app.lyrics.partition_point(|(t, _)| *t < start);
    app.lyrics.insert(i, (start, text.clone()));
    app.m.lyrics.insert(i, LyricLine { text: ss(text), active: false, hit: false });
    if let Some(ui) = app.ui() {
        // Wstawienie przesuwa wiersze – podświetlenie od nowa.
        let p = ui.global::<Player>();
        let mut old = p.get_lyrics_active();
        if old >= i as i32 {
            old += 1;
        }
        if old >= 0 && (old as usize) < app.m.lyrics.row_count() {
            if let Some(mut r) = app.m.lyrics.row_data(old as usize) {
                r.active = false;
                app.m.lyrics.set_row_data(old as usize, r);
            }
        }
        p.set_lyrics_active(-1);
    }
    apply_search(app, false);
}

/// Wyszukiwanie w tekście albo napisach bieżącego utworu (tylko w nim – bez indeksu).
pub fn lyrics_search(app: &mut App) {
    apply_search(app, true);
}

/// Zaznacza pasujące wiersze; `jump` – przewija do pierwszego trafienia.
fn apply_search(app: &mut App, jump: bool) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Player>();
    let q = p.get_lyrics_query().trim().to_lowercase();
    let mut hits = Vec::new();
    for i in 0..app.m.lyrics.row_count() {
        let Some(mut r) = app.m.lyrics.row_data(i) else { continue };
        let hit = !q.is_empty() && r.text.to_lowercase().contains(&q);
        if hit {
            hits.push(i);
        }
        if r.hit != hit {
            r.hit = hit;
            app.m.lyrics.set_row_data(i, r);
        }
    }
    p.set_lyrics_hits(hits.len() as i32);
    if jump || p.get_lyrics_hit() as usize >= hits.len() {
        p.set_lyrics_hit(0);
        p.set_lyrics_focus(hits.first().map(|&i| i as i32).unwrap_or(-1));
    }
    app.lyrics_hits = hits;
}

/// Następne / poprzednie trafienie (Enter, strzałki przy polu).
pub fn lyrics_next(app: &mut App, d: i32) {
    let n = app.lyrics_hits.len() as i32;
    let Some(ui) = app.ui() else { return };
    if n == 0 {
        return;
    }
    let p = ui.global::<Player>();
    let h = (p.get_lyrics_hit() + d).rem_euclid(n);
    p.set_lyrics_hit(h);
    p.set_lyrics_focus(app.lyrics_hits[h as usize] as i32);
}

/// Klik w wiersz: utwór albo film przewija się do tej linii (tekst z czasami / napisy).
pub fn lyrics_seek(app: &mut App, i: i32) {
    let Some(&(t, _)) = app.lyrics.get(i.max(0) as usize) else { return };
    if app.cur.is_none() {
        // Zaznaczenie: zaznaczony utwór zaczyna grać od tej linii.
        let in_lib = app.ui().is_some_and(|u| u.global::<Lib>().get_view() == 1);
        if in_lib {
            let sel = crate::libview::selected(app);
            if sel.len() == 1 {
                crate::libview::play_tracks_at(app, sel, 0, false, Some(t));
            }
        } else if let Some(&idx) = crate::lists::selected_sorted(app).first() {
            let id = app.active_id();
            crate::player::play(app, id, idx, Some(t));
        }
        return;
    }
    let video = app.cur.as_ref().is_some_and(|c| c.video);
    let delay = match app.ui() {
        Some(ui) if video => ui.global::<Video>().get_sub_delay() as f64,
        _ => 0.0,
    };
    crate::player::seek_abs(app, (t + delay).max(0.0));
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

/// Nowy styl taśmy bez ponownego dekodowania utworu.
pub fn restyle_wave(app: &mut App) {
    let (a, b, kind) = crate::wave::paths(&app.wave_raw, app.s.wave_style);
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_wave(ss(a));
        p.set_wave2(ss(b));
        p.set_wave2_kind(kind);
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
    poll_live_subs(app);
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
    // Żywa część taśmy przy znaczniku odtwarzania.
    if app.s.show_waveform && app.s.live_style > 0 && playing && !app.wave_raw.is_empty() && p.get_duration() > 0.0 {
        let pos = p.get_pos() / p.get_duration();
        let t = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0) % 1000.0) as f32;
        p.set_wave_live(ss(crate::wave::live(&app.wave_raw, pos, &app.analyzer.bars, app.s.live_style, app.s.wave_style, app.s.live_span, t)));
    } else if !p.get_wave_live().is_empty() {
        p.set_wave_live(ss(""));
    }
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
    } else if viz >= 5 {
        use crate::vizpaths as vp;
        let path = match viz {
            vp::MIRROR => vp::mirror(&app.analyzer.bars),
            vp::RADIAL => vp::radial(&app.analyzer.bars),
            vp::WATERFALL => {
                app.viz_hist.push_front(app.analyzer.bars.clone());
                app.viz_hist.truncate(18);
                vp::waterfall(&app.viz_hist)
            }
            vp::LEDS => vp::leds(&app.analyzer.bars),
            vp::LISSAJOUS => {
                let s = app.analyzer.input().clone();
                vp::lissajous(&s)
            }
            _ => vp::dots(&app.analyzer.bars),
        };
        p.set_viz_path(ss(path));
    } else if viz == 3 {
        p.set_vu_l(app.analyzer.vu[0]);
        p.set_vu_r(app.analyzer.vu[1]);
        p.set_vu_pl(app.analyzer.vu_peak[0]);
        p.set_vu_pr(app.analyzer.vu_peak[1]);
    }
    p.set_viz_active(true);
    // Tekst synchronizowany
    if !app.lyrics.is_empty() {
        // Napisy filmu przesunięte razem z obrazem.
        let delay = if app.cur.as_ref().is_some_and(|c| c.video) { ui.global::<Video>().get_sub_delay() as f64 } else { 0.0 };
        let pos = p.get_pos() as f64 + 0.15 - delay;
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
    app.s.viz = crate::vizpaths::next(app.s.viz);
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_viz_mode(app.s.viz);
    }
    let names = ["słupki", "widmo", "oscyloskop", "wskaźniki VU", "", "lustro", "okrąg", "widmo 3D", "diody", "stereo (Lissajous)", "kropki"];
    let n = names.get(app.s.viz as usize).copied().unwrap_or("");
    app.toast(if n.is_empty() { "Wizualizacja wyłączona".to_string() } else { format!("Wizualizacja: {n}") });
    app.dirty = true;
}

/// Tryb zaznaczenia: gdy nic nie gra, w miejscu „Teraz odtwarzane” są szczegóły zaznaczonego
/// pliku, kilku plików albo otwartego folderu / grupy w bibliotece.
pub fn sel_info(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Player>();
    if app.cur.is_some() {
        p.set_sel_mode(false);
        app.sel_text.clear();
        app.sel_desc = None;
        return;
    }
    let lib = ui.global::<Lib>();
    let in_lib = lib.get_view() == 1;
    let sel: Vec<TrackInfo> = if in_lib { crate::libview::selected(app) } else { crate::lists::selected_tracks(app) };
    let single = (sel.len() == 1).then(|| sel[0].clone());
    let mut rows: Vec<(&str, &str, String)> = Vec::new();
    let (title, sub, cover_path);
    let fmt_size = |b: u64| if b >= 1 << 30 { format!("{:.2} GB", b as f64 / (1u64 << 30) as f64).replace('.', ",") } else { format!("{:.1} MB", b as f64 / 1_048_576.0).replace('.', ",") };
    if sel.len() == 1 {
        let t = &sel[0];
        let video = meta::is_video(Path::new(&t.path));
        title = t.display_title();
        sub = if video { Path::new(&t.path).parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default() } else { t.display_artist() };
        if !t.album.is_empty() {
            rows.push(("album", "Album", t.album.clone()));
        }
        if t.year > 0 {
            rows.push(("calendar", "Rok", t.year.to_string()));
        }
        if !t.genre.is_empty() {
            rows.push(("tag", "Gatunek", t.genre.clone()));
        }
        if t.duration > 0.0 {
            rows.push(("clock", "Długość", meta::fmt_time(t.duration)));
        }
        let b = badges(t, None);
        if !b.is_empty() {
            rows.push(("file", "Format", b.join(" · ")));
        }
        if t.size > 0 {
            rows.push(("download", "Rozmiar", fmt_size(t.size)));
        }
        let st = app.stats.get(&t.path);
        if st.plays > 0 {
            rows.push(("history", if video { "Obejrzane" } else { "Odtworzenia" }, st.plays.to_string()));
        }
        rows.push(("folder-open", "Plik", t.path.clone()));
        cover_path = t.path.clone();
    } else {
        // Kilka plików albo cała grupa (otwarty folder, album, wykonawca) w bibliotece.
        let group: Vec<TrackInfo> = if sel.len() > 1 { sel } else if in_lib && !app.libv.stack.is_empty() { app.libv.tracks.clone() } else { Vec::new() };
        if group.is_empty() {
            p.set_sel_mode(false);
            sel_extras(app, None);
            return;
        }
        let n = group.len();
        title = if sel_is_group(app, in_lib, n) { lib.get_title().to_string() } else { format!("{n} {}", crate::lists::plural(n, "zaznaczony plik", "zaznaczone pliki", "zaznaczonych plików")) };
        sub = format!("{n} {}", crate::lists::plural(n, "pozycja", "pozycje", "pozycji"));
        let dur: f64 = group.iter().map(|t| t.duration.max(0.0)).sum();
        let size: u64 = group.iter().map(|t| t.size).sum();
        if dur > 0.0 {
            rows.push(("clock", "Łącznie", meta::fmt_time(dur)));
        }
        if size > 0 {
            rows.push(("download", "Rozmiar", fmt_size(size)));
        }
        let mut artists: Vec<String> = group.iter().map(|t| t.display_artist()).filter(|a| !a.is_empty()).collect();
        artists.sort();
        artists.dedup();
        if !artists.is_empty() {
            let more = if artists.len() > 3 { format!(" i {} innych", artists.len() - 3) } else { String::new() };
            rows.push(("artist", "Wykonawcy", format!("{}{more}", artists.iter().take(3).cloned().collect::<Vec<_>>().join(", "))));
        }
        let mut fmts: Vec<String> = group.iter().map(|t| t.format()).filter(|f| !f.is_empty()).collect();
        fmts.sort();
        fmts.dedup();
        if !fmts.is_empty() {
            rows.push(("file", "Formaty", fmts.join(", ")));
        }
        let mut years: Vec<u32> = group.iter().map(|t| t.year).filter(|y| *y > 0).collect();
        years.sort();
        if let (Some(a), Some(b)) = (years.first(), years.last()) {
            rows.push(("calendar", "Lata", if a == b { a.to_string() } else { format!("{a}–{b}") }));
        }
        if let Some(d) = Path::new(&group[0].path).parent() {
            rows.push(("folder-open", "Folder", d.to_string_lossy().into_owned()));
        }
        cover_path = group[0].path.clone();
    }
    let opts: Vec<Opt> = rows.into_iter().map(|(i, l, v)| Opt { id: ss(i), label: ss(l), sub: ss(v), selected: false }).collect();
    p.set_sel_title(ss(title));
    p.set_sel_sub(ss(sub));
    p.set_sel_rows(ModelRc::new(VecModel::from(opts)));
    match crate::covers::get(&cover_path) {
        Some(img) => {
            p.set_sel_cover(img);
            p.set_sel_has_cover(true);
        }
        None => p.set_sel_has_cover(false),
    }
    sel_extras(app, single);
    p.set_sel_mode(true);
}

/// Zaznaczony jeden plik: opis (film – z TMDB, muzyka – komentarz z tagów) i tekst piosenki
/// obok, jak przy odtwarzaniu. Przy zmianie zaznaczenia ładowane od nowa.
fn sel_extras(app: &mut App, t: Option<TrackInfo>) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Player>();
    let path = t.as_ref().map(|t| t.path.clone()).unwrap_or_default();
    let fresh = app.sel_text != path;
    if fresh {
        app.sel_text = path.clone();
        app.sel_desc = None;
        set_lyrics(app, None, "");
    }
    p.set_sel_overview(ss(""));
    p.set_sel_credits(ss(""));
    let Some(t) = t else { return };
    if meta::is_url(&t.path) {
        return;
    }
    if t.video || meta::is_video(Path::new(&t.path)) {
        if let Some(info) = &app.sel_desc {
            apply_sel_desc(&p, info);
        } else if fresh && !app.s.tmdb_key.is_empty() {
            std::thread::spawn(move || {
                let Some(info) = openmp_core::tmdb::lookup(Path::new(&path)) else { return };
                let _ = slint::invoke_from_event_loop(move || {
                    later(move |app| {
                        if app.cur.is_some() || app.sel_text != path {
                            return;
                        }
                        if let Some(ui) = app.ui() {
                            apply_sel_desc(&ui.global::<Player>(), &info);
                        }
                        app.sel_desc = Some(info);
                    });
                });
            });
        }
        return;
    }
    let comment = t.comment.trim();
    if comment.chars().count() >= 20 {
        p.set_sel_overview(ss(comment));
    }
    if !fresh {
        return;
    }
    if let Some(text) = meta::lyrics(Path::new(&t.path)) {
        return set_lyrics(app, Some(text), "plik / tagi");
    }
    if !app.s.online_lyrics {
        return;
    }
    match openmp_core::lrclib::cached(&t.artist, &t.title) {
        Some(text) if !text.trim().is_empty() => set_lyrics(app, Some(text), "LRCLIB"),
        Some(_) => {}
        None => {
            std::thread::spawn(move || {
                let Some(text) = openmp_core::lrclib::fetch(&t.artist, &t.title, &t.album, t.duration) else { return };
                let _ = slint::invoke_from_event_loop(move || {
                    later(move |app| {
                        if app.cur.is_none() && app.sel_text == path {
                            set_lyrics(app, Some(text), "LRCLIB");
                        }
                    });
                });
            });
        }
    }
}

/// Opis filmu z TMDB w zaznaczeniu: polski tytuł, rok i gatunki, opis, reżyseria i obsada.
fn apply_sel_desc(p: &Player, info: &openmp_core::tmdb::Info) {
    if !info.title.is_empty() {
        p.set_sel_title(ss(&info.title));
    }
    let line = info.line();
    if !line.is_empty() {
        p.set_sel_sub(ss(line));
    }
    p.set_sel_overview(ss(&info.overview));
    p.set_sel_credits(ss(info.credits()));
}

/// Bez zaznaczenia w otwartym folderze / grupie – opis dotyczy całej grupy.
fn sel_is_group(app: &App, in_lib: bool, n: usize) -> bool {
    in_lib && app.libv.sel.is_empty() && n == app.libv.tracks.len()
}

/// Zamknięcie bieżącego pliku (krzyżyk przy „Teraz odtwarzane”): zatrzymanie i powrót
/// do trybu zaznaczenia.
pub fn close_current(app: &mut App) {
    crate::player::stop(app);
    app.cur = None;
    app.lyrics.clear();
    app.lyrics_text.clear();
    app.m.lyrics.set_vec(Vec::new());
    app.sub_live = false;
    SUB_GEN.fetch_add(1, Ordering::SeqCst);
    GEN.fetch_add(1, Ordering::SeqCst);
    crate::video::clear(app);
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_idle(true);
        p.set_playing(false);
        p.set_title(ss(""));
        p.set_artist(ss(""));
        p.set_album(ss(""));
        p.set_credits(ss(""));
        p.set_overview(ss(""));
        p.set_position_text(ss(""));
        set_badges(&p, &[]);
        p.set_cover(Image::default());
        p.set_backdrop(Image::default());
        p.set_has_cover(false);
        p.set_subs_mode(false);
        p.set_lyrics_source(ss(""));
        ui.global::<Theme>().set_glass_has(false);
        ui.global::<Theme>().set_tint(ui.global::<Theme>().get_accent());
    }
    crate::win::smtc_state(app);
    sel_info(app);
}
