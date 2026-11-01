//! Skróty klawiszowe, polecenia menu głównego, widoki (kompaktowy, mini), zamykanie programu.

use slint::ComponentHandle;

use crate::app::*;
use crate::*;

const UP: char = '\u{F700}';
const DOWN: char = '\u{F701}';
const LEFT: char = '\u{F702}';
const RIGHT: char = '\u{F703}';
const F1: char = '\u{F704}';
const F2: char = '\u{F705}';
const F11: char = '\u{F70E}';
const HOME: char = '\u{F729}';
const END: char = '\u{F72B}';
const PGUP: char = '\u{F72C}';
const PGDN: char = '\u{F72D}';
const MEDIA_PLAY: char = '\u{F730}';
const BACKSPACE: char = '\u{8}';
const ENTER: char = '\n';
const ESC: char = '\u{1b}';
const DEL: char = '\u{7f}';

pub fn help() -> Vec<KeyHelp> {
    let k = |a: &str, b: &str| KeyHelp { keys: ss(a), action: ss(b) };
    vec![
        k(tr!("Space"), trc!("shortcut", "Play / pause")),
        k("Ctrl + ← / →", tr!("Previous / next track")),
        k("← / →", tr!("Seek by one step (settings), with Shift – 1 s, with Alt – 60 s")),
        k("↑ / ↓", tr!("Volume (in video); in a list – select a track")),
        k("Ctrl + ↑ / ↓", tr!("Volume")),
        k("Enter", tr!("Play selected")),
        k("M", tr!("Mute")),
        k("S", tr!("Shuffle; in video – screenshot")),
        k("R", tr!("Repeat: off / list / track")),
        k("E", tr!("Equalizer and effects")),
        k("L", tr!("Song lyrics; in video – A-B loop")),
        k("Q", tr!("Add selected to the queue")),
        k("Del", tr!("Remove selected from the list")),
        k("Alt + ↑ / ↓", tr!("Move selected in the list")),
        k("Ctrl + A", tr!("Select all")),
        k(tr!("Ctrl + Alt + Space / ← / → / ↑ / ↓"), tr!("Global: play/pause, track, volume")),
        k("Ctrl + D", tr!("Favorites")),
        k("Ctrl + 1…5 / Ctrl + 0", tr!("Rate selected (5 = favorite) / remove rating")),
        k("F2", tr!("Edit tags")),
        k("Ctrl + F", tr!("Search the list")),
        k("Ctrl + O / Ctrl + Shift + O", tr!("Open files / folder")),
        k("Ctrl + U", tr!("Open URL (stream)")),
        k("Ctrl + N / Ctrl + W", tr!("New list / close tab")),
        k("Ctrl + Tab", tr!("Next list tab")),
        k("Ctrl + S", tr!("Save list as M3U8")),
        k("F11 / Ctrl + M", tr!("Full screen / mini player")),
        k("Ctrl + B", tr!("Navigation panel")),
        k("Ctrl + T", tr!("Always on top")),
        k("Ctrl + J", tr!("Show the current track in the list")),
        k("V / Shift + V", tr!("Video: switch subtitles / audio track")),
        k(tr!("F, double-click, click on picture"), tr!("Video: movie full screen (Esc – back to the window)")),
        k("Esc", tr!("Video: exit full screen; in the window – to the library (the movie keeps playing)")),
        k("P", tr!("Video: picture in picture (mini player)")),
        k("+ / − / 0", tr!("Video: zoom in / zoom out / picture as in file")),
        k(tr!("Ctrl + Shift + arrows"), tr!("Video: pan the zoomed picture")),
        k("Ctrl + R", tr!("Video: rotate by 90°")),
        k("C", tr!("Video: aspect ratio")),
        k("B / G", tr!("Video: debanding / ambient glow")),
        k("I", tr!("Video: playback details")),
        k("Shift + S", tr!("Video: screenshot without subtitles")),
        k("[ / ] / Backspace", tr!("Video: slower / faster / normal")),
        k(", / .", tr!("Video: frame back / forward")),
        k("PgUp / PgDn", tr!("Video: previous / next chapter")),
        k("Z / X", tr!("Video: subtitle delay −/+ 100 ms")),
        k("T", tr!("Video: always on top")),
        k("Ctrl + ,", tr!("Settings")),
        k("Ctrl + Q", tr!("Quit")),
    ]
}

pub fn open_settings(app: &mut App, page: i32) {
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_settings_page(page);
        ui.global::<Ui>().set_settings(true);
    }
}

/// Pełny ekran okna – ten sam układ co w oknie (okładka albo obraz filmu z opisem obok,
/// lista, panel nawigacji). Z pełnego ekranu filmu – powrót do okna.
pub fn toggle_full(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    match u.get_mode() {
        1 => return crate::video::leave_video(app),
        2 => return,
        _ => {}
    }
    let fs = !u.get_fullscreen();
    if fs && !ui.window().is_maximized() {
        app.music_geom = Some(crate::win::geometry(&ui));
    } else if !fs {
        app.music_geom = None;
    }
    crate::win::fullscreen_frame(&ui, fs);
    ui.window().set_fullscreen(fs);
    u.set_fullscreen(fs);
}

pub fn set_mode(app: &mut App, mode: i32) {
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    let cur = u.get_mode();
    if cur == mode {
        return;
    }
    if mode == 2 {
        if cur == 1 {
            crate::video::leave_video(app);
        }
        if u.get_fullscreen() {
            // Z pełnego ekranu: okno wraca do poprzedniego rozmiaru (zapamiętanego przy wejściu).
            crate::win::fullscreen_frame(&ui, false);
            ui.window().set_fullscreen(false);
            u.set_fullscreen(false);
        } else if !ui.window().is_maximized() {
            app.music_geom = Some(crate::win::geometry(&ui));
        } else {
            ui.window().set_maximized(false);
        }
        u.set_mode(2);
        crate::win::set_ontop(app, true);
        fit_mini(app);
    } else if mode == 0 {
        u.set_mode(0);
        crate::win::set_ontop(app, app.s.always_on_top);
        if let Some(g) = app.music_geom.take() {
            crate::win::set_geometry(&ui, g);
        }
    }
}

/// Rozmiar miniodtwarzacza: pasek muzyki albo obraz w obrazie w proporcjach filmu (w rogu ekranu).
pub fn fit_mini(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    if ui.global::<Ui>().get_mode() != 2 {
        return;
    }
    let sf = ui.window().scale_factor();
    let (x, y, _, _) = crate::win::geometry(&ui);
    if ui.global::<Video>().get_active() {
        let ratio = ui.global::<Video>().get_ratio().clamp(0.5, 3.0);
        let w = 480.0 * sf;
        let h = (w / ratio).clamp(160.0 * sf, 640.0 * sf);
        let (w, h) = (w as u32, h as u32);
        match crate::win::monitor(&ui) {
            Some((mx, my, mw, mh)) => {
                let m = (24.0 * sf) as i32;
                crate::win::set_geometry(&ui, (mx + mw as i32 - w as i32 - m, my + mh as i32 - h as i32 - m * 3, w, h));
            }
            None => crate::win::set_geometry(&ui, (x, y, w, h)),
        }
    } else {
        crate::win::set_geometry(&ui, (x, y, (540.0 * sf) as u32, (150.0 * sf) as u32));
    }
}

pub fn quit(app: &mut App) {
    openmp_core::paths::log("zamknięcie programu");
    crate::win::tray_remove();
    openmp_core::instance::release();
    crate::player::save_resume(app);
    save_all(app);
    app.engine.stop();
    if let Some(m) = &app.mpv {
        let _ = m.command(&["stop"]);
    }
    crate::win::keep_awake(app, false);
    if let Some(ui) = app.ui() {
        let _ = ui.hide();
    }
    let _ = slint::quit_event_loop();
}

pub fn save_all(app: &mut App) {
    if let Some(ui) = app.ui() {
        let mode = ui.global::<Ui>().get_mode();
        let max = ui.window().is_maximized();
        app.session.maximized = max;
        if mode == 0 && !max && !ui.window().is_fullscreen() {
            app.session.window = Some(crate::win::geometry(&ui));
        } else if let Some(g) = app.music_geom {
            app.session.window = Some(g);
        }
    }
    if let Some(c) = &app.cur {
        app.session.list_id = c.list.unwrap_or(0);
        app.session.idx = c.idx;
        app.session.path = if c.station.is_some() { String::new() } else { c.info.path.clone() };
        app.session.pos = if c.item != 0 { crate::player::position(app).0 } else { 0.0 };
    }
    app.session.save();
    app.store.save();
    app.stats.save();
    app.s.save();
    app.dirty = false;
}

/// Polecenia menu głównego (i wspólne skróty).
pub fn action(app: &mut App, id: &str) {
    match id {
        "open-files" => crate::dialogs::open_files(app),
        "add-files" => crate::dialogs::add_files(app),
        "open-folder" => crate::dialogs::open_folder(app),
        "open-url" => {
            if let Some(ui) = app.ui() {
                ui.global::<Ui>().set_url_text(ss(""));
                ui.global::<Ui>().set_url_dialog(true);
            }
        }
        "new-list" => {
            let n = app.store.lists.len() + 1;
            crate::lists::create_list(app, &tr!("List {}", n));
            if let Some(ui) = app.ui() {
                ui.global::<Lib>().set_view(0);
            }
        }
        "import-list" => crate::dialogs::import_list(app, true),
        "export-list" => crate::dialogs::export_list(app),
        "equalizer" => {
            if let Some(ui) = app.ui() {
                let u = ui.global::<Ui>();
                u.set_equalizer(!u.get_equalizer());
            }
        }
        "sleep" => {
            let (x, y) = app.ui().map(|u| (u.window().size().width as f32 / u.window().scale_factor() - 300.0, 360.0)).unwrap_or((200.0, 300.0));
            crate::menu::sleep_menu(app, x, y);
        }
        "full" => toggle_full(app),
        "mini" => set_mode(app, 2),
        "ontop" => {
            app.s.always_on_top = !app.s.always_on_top;
            if let Some(ui) = app.ui() {
                ui.global::<Prefs>().set_ontop(app.s.always_on_top);
            }
            crate::win::set_ontop(app, app.s.always_on_top);
            app.s.save();
        }
        "settings" => open_settings(app, 0),
        "update" => crate::dialogs::check_update(app, true),
        "about" => {
            if let Some(ui) = app.ui() {
                ui.global::<Ui>().set_about(true);
            }
        }
        "coffee" => {
            let _ = open::that(openmp_core::SUPPORT_URL);
        }
        "quit" => quit(app),
        _ => {}
    }
}

/// Klawisz w oknie (gdy żadne pole tekstowe nie ma fokusu).
pub fn key(app: &mut App, text: &str, ctrl: bool, shift: bool, alt: bool) {
    if text == "stop-after" {
        if let Some(ui) = app.ui() {
            let p = ui.global::<Player>();
            p.set_stop_after(!p.get_stop_after());
            let on = p.get_stop_after();
            app.toast(if on { tr!("Stop after the current track") } else { tr!("Continuous playback") });
        }
        crate::player::refresh_next(app);
        return;
    }
    if text == "video-menu" {
        // Z klawiatury – pośrodku u góry okna.
        let x = app.ui().map(|u| u.window().size().width as f32 / u.window().scale_factor() / 2.0 - 140.0).unwrap_or(40.0);
        crate::menu::video_menu(app, x, 80.0, false);
        return;
    }
    let Some(c) = text.chars().next() else { return };
    let lc = c.to_ascii_lowercase();
    let video = app.is_video_mode();
    if video {
        app.last_mouse = std::time::Instant::now();
    }
    let step = app.s.seek_step;
    if ctrl && lc == 'v' && !shift {
        paste(app);
        return;
    }
    if ctrl {
        match lc {
            'o' if shift => crate::dialogs::open_folder(app),
            'o' => crate::dialogs::open_files(app),
            'u' => action(app, "open-url"),
            'n' => action(app, "new-list"),
            'w' => {
                let i = app.store.active;
                crate::lists::close_tab(app, i);
            }
            's' => crate::dialogs::export_list(app),
            'm' => set_mode(app, 2),
            't' => action(app, "ontop"),
            'q' => quit(app),
            ',' => open_settings(app, 0),
            'a' => crate::lists::command(app, "select-all"),
            'd' => crate::lists::command(app, "fav"),
            '0'..='5' => crate::lists::command(app, &format!("rate-{lc}")),
            'i' => crate::lists::command(app, "info"),
            // Panel nawigacji – w oknie i na pełnym ekranie (poza miniodtwarzaczem).
            'b' if !video => {
                if let Some(ui) = app.ui().filter(|u| u.global::<Ui>().get_mode() == 0) {
                    app.s.show_sidebar = !app.s.show_sidebar;
                    ui.global::<Prefs>().set_show_sidebar(app.s.show_sidebar);
                    app.dirty = true;
                }
            }
            'j' => {
                crate::lists::scroll_to_current(app);
            }
            'f' => {
                if let Some(ui) = app.ui() {
                    ui.global::<Lib>().set_view(0);
                }
            }
            '\t' => {
                let n = app.store.lists.len();
                let i = if shift { (app.store.active + n - 1) % n } else { (app.store.active + 1) % n };
                crate::lists::switch_tab(app, i);
            }
            'r' if video => crate::video::rotate(app),
            _ => match c {
                // Przesuwanie powiększonego obrazu.
                LEFT if video && shift => crate::video::pan(app, 0.05, 0.0),
                RIGHT if video && shift => crate::video::pan(app, -0.05, 0.0),
                UP if video && shift => crate::video::pan(app, 0.0, 0.05),
                DOWN if video && shift => crate::video::pan(app, 0.0, -0.05),
                LEFT => crate::player::prev(app),
                RIGHT => crate::player::next(app),
                UP => crate::player::set_volume(app, app.s.volume + 0.05),
                DOWN => crate::player::set_volume(app, app.s.volume - 0.05),
                _ => {}
            },
        }
        return;
    }
    if alt {
        match c {
            UP => crate::lists::move_selected(app, true),
            DOWN => crate::lists::move_selected(app, false),
            LEFT => crate::player::seek_rel(app, -60.0),
            RIGHT => crate::player::seek_rel(app, 60.0),
            _ => {}
        }
        return;
    }
    match c {
        ' ' | MEDIA_PLAY => crate::player::play_pause(app),
        LEFT => crate::player::seek_rel(app, if shift { -1.0 } else { -step }),
        RIGHT => crate::player::seek_rel(app, if shift { 1.0 } else { step }),
        UP if video => crate::player::set_volume(app, app.s.volume + 0.05),
        DOWN if video => crate::player::set_volume(app, app.s.volume - 0.05),
        UP | DOWN => move_cursor(app, if c == UP { -1 } else { 1 }, shift),
        HOME => crate::player::seek_abs(app, 0.0),
        ENTER if video => crate::video::toggle_fullscreen(app),
        ENTER if app.ui().is_some_and(|u| u.global::<Lib>().get_view() == 1) => crate::libview::command(app, "l-play"),
        ENTER => {
            if let Some(&i) = crate::lists::selected_sorted(app).first() {
                let id = app.active_id();
                crate::player::play(app, id, i, None);
            }
        }
        DEL => crate::lists::remove_selected(app),
        F1 => action(app, "about"),
        F2 => crate::lists::command(app, "tags"),
        F11 if video => crate::video::toggle_fullscreen(app),
        F11 => toggle_full(app),
        ESC => {
            if video {
                if app.ui().map(|u| u.global::<Video>().get_panel()).unwrap_or(false) {
                    if let Some(ui) = app.ui() {
                        ui.global::<Video>().set_panel(false);
                    }
                } else {
                    // Z pełnego ekranu filmu do okna – film gra dalej obok opisu.
                    crate::video::leave_video(app);
                }
            } else if app.ui().map(|u| u.global::<Ui>().get_mode() == 2).unwrap_or(false) {
                set_mode(app, 0);
            } else if app.ui().map(|u| u.global::<Ui>().get_fullscreen()).unwrap_or(false) {
                toggle_full(app);
            }
        }
        BACKSPACE if video => crate::video::set(app, "speed", 1.0),
        BACKSPACE => crate::libview::back(app),
        PGUP if video => crate::video::chapter(app, -1),
        PGDN if video => crate::video::chapter(app, 1),
        END => {}
        _ => match lc {
            'm' => crate::player::toggle_mute(app),
            'r' => cycle_repeat(app),
            'e' => action(app, "equalizer"),
            'q' => crate::lists::command(app, "enqueue"),
            'f' if video => crate::video::toggle_fullscreen(app),
            // Film gra w oknie obok opisu – F otwiera jego pełny ekran.
            'f' if app.cur.as_ref().is_some_and(|c| c.video) => crate::video::toggle_fullscreen(app),
            't' if video => crate::video::toggle_ontop(app),
            's' if video => crate::video::screenshot(app, !shift),
            'p' if video => set_mode(app, 2),
            'c' if video => crate::video::cycle_aspect(app),
            'b' if video => crate::video::toggle_deband(app),
            'g' if video => crate::video::command(app, "v-ambient"),
            '=' | '+' if video => crate::video::zoom_by(app, 0.25),
            '-' if video => crate::video::zoom_by(app, -0.25),
            '0' if video => crate::video::reset_view(app),
            's' => toggle_shuffle(app),
            'l' if video => crate::video::ab_loop(app),
            'l' => {
                if let Some(ui) = app.ui() {
                    let p = ui.global::<Prefs>();
                    p.set_side_panel(if p.get_side_panel() == 1 { 0 } else { 1 });
                }
            }
            'v' if video => crate::video::cycle_track(app, if shift { "aid" } else { "sid" }),
            'a' if video => crate::video::cycle_track(app, "aid"),
            '[' if video => {
                let s = app.ui().map(|u| u.global::<Video>().get_speed()).unwrap_or(1.0);
                crate::video::set(app, "speed", (s - 0.1).max(0.25));
            }
            ']' if video => {
                let s = app.ui().map(|u| u.global::<Video>().get_speed()).unwrap_or(1.0);
                crate::video::set(app, "speed", (s + 0.1).min(4.0));
            }
            ',' if video => crate::video::frame_step(app, true),
            '.' if video => crate::video::frame_step(app, false),
            'z' if video => {
                let d = app.ui().map(|u| u.global::<Video>().get_sub_delay()).unwrap_or(0.0);
                crate::video::set(app, "sub-delay", d - 0.1);
            }
            'x' if video => {
                let d = app.ui().map(|u| u.global::<Video>().get_sub_delay()).unwrap_or(0.0);
                crate::video::set(app, "sub-delay", d + 0.1);
            }
            'i' if video => crate::video::open_panel(app, 4),
            _ => {}
        },
    }
}

fn move_cursor(app: &mut App, d: i32, shift: bool) {
    let (lib, cols) = app.ui().map(|u| (u.global::<Lib>().get_view() == 1, u.global::<Ui>().get_grid_cols().max(1))).unwrap_or((false, 1));
    // Siatka okładek: w górę/dół o cały wiersz.
    let d = if app.s.track_grid { d * cols } else { d };
    if lib {
        let n = app.libv.tracks.len() as i32;
        if n == 0 {
            return;
        }
        let cur = app.libv.anchor.or_else(|| app.libv.sel.iter().max().copied()).map(|c| c as i32).unwrap_or(-1);
        let i = (cur + d).clamp(0, n - 1) as usize;
        crate::libview::row_clicked(app, i, false, shift);
        app.libv.anchor = Some(i);
        return;
    }
    let mut visible: Vec<usize> = app.row_of.keys().copied().collect();
    visible.sort();
    if visible.is_empty() {
        return;
    }
    let cur = app.anchor.or_else(|| app.sel.iter().max().copied());
    let pos = cur.and_then(|c| visible.iter().position(|&v| v == c)).map(|p| p as i32).unwrap_or(-1);
    let np = (pos + d).clamp(0, visible.len() as i32 - 1) as usize;
    crate::lists::select_click(app, visible[np], false, shift);
    app.anchor = Some(visible[np]);
}

pub fn toggle_shuffle(app: &mut App) {
    app.s.shuffle = !app.s.shuffle;
    app.shuffle_len = 0;
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_shuffle(app.s.shuffle);
    }
    app.pending = None;
    crate::player::refresh_next(app);
    app.toast(if app.s.shuffle { tr!("Shuffle: on") } else { tr!("Shuffle: off") });
    app.s.save();
}

pub fn cycle_repeat(app: &mut App) {
    app.s.repeat = (app.s.repeat + 1) % 3;
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_repeat(app.s.repeat);
    }
    app.pending = None;
    crate::player::refresh_next(app);
    app.toast([tr!("Repeat off"), tr!("Repeat list"), tr!("Repeat track")][app.s.repeat as usize]);
    app.s.save();
}

/// Ctrl+V: link (YouTube, strumień, plik w sieci) albo ścieżki plików ze schowka – od razu gra.
fn paste(app: &mut App) {
    let Some(text) = clipboard_text() else { return };
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let lower = text.to_ascii_lowercase();
    if ["http://", "https://", "rtsp://", "rtmp://", "mms://"].iter().any(|p| lower.starts_with(p)) {
        crate::player::open_url(app, &text);
        return;
    }
    let paths: Vec<String> = text.lines().map(|l| l.trim().trim_matches('"').to_string()).filter(|l| std::path::Path::new(l).exists()).collect();
    if paths.is_empty() {
        app.toast(tr!("The clipboard has no link or file path"));
    } else {
        crate::player::open_paths(app, paths, true, None);
    }
}

#[cfg(windows)]
fn clipboard_text() -> Option<String> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(h: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn GetClipboardData(f: u32) -> *mut c_void;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalLock(h: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(h: *mut c_void) -> i32;
    }
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let mut out = None;
        let h = GetClipboardData(CF_UNICODETEXT);
        if !h.is_null() {
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let mut n = 0;
                while *p.add(n) != 0 && n < 1 << 20 {
                    n += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        out
    }
}

#[cfg(not(windows))]
fn clipboard_text() -> Option<String> {
    None
}
