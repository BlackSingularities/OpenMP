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
        k("Spacja", "Odtwarzanie / pauza"),
        k("Ctrl + ← / →", "Poprzedni / następny utwór"),
        k("← / →", "Przewiń o krok (ustawienia), z Shift – 1 s, z Alt – 60 s"),
        k("↑ / ↓", "Głośność (w wideo); na liście – wybór utworu"),
        k("Ctrl + ↑ / ↓", "Głośność"),
        k("Enter", "Odtwórz zaznaczony"),
        k("M", "Wycisz"),
        k("S", "Losowo; w wideo – zrzut ekranu"),
        k("R", "Powtarzanie: wył. / lista / utwór"),
        k("E", "Korektor i efekty"),
        k("L", "Tekst piosenki; w wideo – pętla A-B"),
        k("Q", "Dodaj zaznaczone do kolejki"),
        k("Del", "Usuń zaznaczone z listy"),
        k("Alt + ↑ / ↓", "Przesuń zaznaczone na liście"),
        k("Ctrl + A", "Zaznacz wszystko"),
        k("Ctrl + D", "Ulubione"),
        k("F2", "Edytuj tagi"),
        k("Ctrl + F", "Szukaj na liście"),
        k("Ctrl + O / Ctrl + Shift + O", "Otwórz pliki / folder"),
        k("Ctrl + U", "Otwórz adres (strumień)"),
        k("Ctrl + N / Ctrl + W", "Nowa lista / zamknij kartę"),
        k("Ctrl + Tab", "Następna karta listy"),
        k("Ctrl + S", "Zapisz listę jako M3U8"),
        k("Ctrl + K / Ctrl + M", "Widok kompaktowy / miniodtwarzacz"),
        k("Ctrl + T", "Zawsze na wierzchu"),
        k("Ctrl + J", "Pokaż bieżący utwór na liście"),
        k("V / Shift + V", "Wideo: przełącz napisy / ścieżkę dźwiękową"),
        k("F lub F11, dwuklik", "Wideo: pełny ekran"),
        k("Esc", "Wideo: wyjście z pełnego ekranu"),
        k("[ / ] / Backspace", "Wideo: wolniej / szybciej / normalnie"),
        k(", / .", "Wideo: klatka wstecz / naprzód"),
        k("PgUp / PgDn", "Wideo: poprzedni / następny rozdział"),
        k("Z / X", "Wideo: opóźnienie napisów −/+ 100 ms"),
        k("T", "Wideo: zawsze na wierzchu"),
        k("Ctrl + ,", "Ustawienia"),
        k("Ctrl + Q", "Zakończ"),
    ]
}

pub fn open_settings(app: &mut App, page: i32) {
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_settings_page(page);
        ui.global::<Ui>().set_settings(true);
    }
}

pub fn toggle_compact(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    app.s.compact = !app.s.compact;
    ui.global::<Prefs>().set_compact(app.s.compact);
    if !ui.window().is_maximized() {
        let (x, y, w, h) = crate::win::geometry(&ui);
        let sf = ui.window().scale_factor();
        if app.s.compact {
            app.session.window = Some((x, y, w, h));
            crate::win::set_geometry(&ui, (x, y, w, (292.0 * sf) as u32));
        } else {
            let h2 = app.session.window.map(|g| g.3).filter(|&hh| hh as f32 > 420.0 * sf).unwrap_or((760.0 * sf) as u32);
            crate::win::set_geometry(&ui, (x, y, w, h2));
        }
    }
    app.s.save();
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
        if !ui.window().is_maximized() {
            app.music_geom = Some(crate::win::geometry(&ui));
        } else {
            ui.window().set_maximized(false);
        }
        u.set_mode(2);
        let sf = ui.window().scale_factor();
        let (x, y, _, _) = crate::win::geometry(&ui);
        crate::win::set_geometry(&ui, (x, y, (520.0 * sf) as u32, (132.0 * sf) as u32));
        crate::win::set_ontop(app, true);
    } else if mode == 0 {
        u.set_mode(0);
        crate::win::set_ontop(app, app.s.always_on_top);
        if let Some(g) = app.music_geom.take() {
            crate::win::set_geometry(&ui, g);
        }
    }
}

pub fn quit(app: &mut App) {
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
            crate::lists::create_list(app, &format!("Lista {n}"));
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
        "compact" => toggle_compact(app),
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
            app.toast(if on { "Zatrzymanie po bieżącym utworze" } else { "Odtwarzanie ciągłe" });
        }
        crate::player::refresh_next(app);
        return;
    }
    if text == "video-menu" {
        crate::menu::video_menu(app);
        return;
    }
    let Some(c) = text.chars().next() else { return };
    let lc = c.to_ascii_lowercase();
    let video = app.is_video_mode();
    if video {
        app.last_mouse = std::time::Instant::now();
    }
    let step = app.s.seek_step;
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
            'k' => toggle_compact(app),
            'm' => set_mode(app, 2),
            't' => action(app, "ontop"),
            'q' => quit(app),
            ',' => open_settings(app, 0),
            'a' => crate::lists::command(app, "select-all"),
            'd' => crate::lists::command(app, "fav"),
            'i' => crate::lists::command(app, "info"),
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
            _ => match c {
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
        ENTER => {
            if let Some(&i) = crate::lists::selected_sorted(app).first() {
                let id = app.active_id();
                crate::player::play(app, id, i, None);
            }
        }
        DEL => crate::lists::remove_selected(app),
        F1 => action(app, "about"),
        F2 => crate::lists::command(app, "tags"),
        F11 => {
            if video {
                crate::video::toggle_fullscreen(app);
            }
        }
        ESC => {
            if video {
                let fs = app.ui().map(|u| u.global::<Video>().get_fullscreen()).unwrap_or(false);
                if fs {
                    crate::video::toggle_fullscreen(app);
                } else if app.ui().map(|u| u.global::<Video>().get_panel()).unwrap_or(false) {
                    if let Some(ui) = app.ui() {
                        ui.global::<Video>().set_panel(false);
                    }
                }
            } else if app.ui().map(|u| u.global::<Ui>().get_mode() == 2).unwrap_or(false) {
                set_mode(app, 0);
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
            't' if video => crate::video::toggle_ontop(app),
            's' if video => crate::video::screenshot(app),
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
            'i' if video => {
                let info = app.ui().map(|u| u.global::<Video>().get_info().to_string()).unwrap_or_default();
                app.osd(info);
            }
            _ => {}
        },
    }
}

fn move_cursor(app: &mut App, d: i32, shift: bool) {
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
    app.toast(if app.s.shuffle { "Losowo: włączone" } else { "Losowo: wyłączone" });
    app.s.save();
}

pub fn cycle_repeat(app: &mut App) {
    app.s.repeat = (app.s.repeat + 1) % 3;
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_repeat(app.s.repeat);
    }
    app.pending = None;
    crate::player::refresh_next(app);
    app.toast(["Powtarzanie wyłączone", "Powtarzaj listę", "Powtarzaj utwór"][app.s.repeat as usize]);
    app.s.save();
}
