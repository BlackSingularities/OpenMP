//! Menu (główne, kontekstowe) – jeden wygląd dla całego programu; pozycje buduje Rust.

use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::app::*;
use crate::*;

pub fn item(id: &str, label: &str, icon: &str, shortcut: &str) -> MenuEntry {
    item_en(id, label, icon, shortcut, true)
}

pub fn item_en(id: &str, label: &str, icon: &str, shortcut: &str, enabled: bool) -> MenuEntry {
    MenuEntry { id: ss(id), label: ss(label), icon: ss(icon), shortcut: ss(shortcut), checked: false, enabled, sep: false }
}

pub fn check(id: &str, label: &str, on: bool, shortcut: &str) -> MenuEntry {
    MenuEntry { id: ss(id), label: ss(label), icon: ss(""), shortcut: ss(shortcut), checked: on, enabled: true, sep: false }
}

pub fn sep() -> MenuEntry {
    MenuEntry { sep: true, enabled: true, ..Default::default() }
}

pub fn open(app: &mut App, ctx: MenuCtx, items: Vec<MenuEntry>, x: f32, y: f32) {
    open_at(app, ctx, items, x, y, false);
}

/// Menu nad punktem (x, y) – dolną krawędzią tuż nad przyciskiem, wg prawdziwej wysokości.
pub fn open_above(app: &mut App, ctx: MenuCtx, items: Vec<MenuEntry>, x: f32, y: f32) {
    open_at(app, ctx, items, x, y, true);
}

fn open_at(app: &mut App, ctx: MenuCtx, items: Vec<MenuEntry>, x: f32, y: f32, above: bool) {
    app.menu = ctx;
    let Some(ui) = app.ui() else { return };
    if app.menu_win.is_none() {
        let Ok(mw) = MenuWindow::new() else { return };
        mw.on_activated(|id| {
            later(move |app| {
                hide(app);
                activated(app, &id);
            })
        });
        mw.on_dismiss(|| later(hide));
        // Klik poza menu (utrata fokusu) zamyka je, jak menu systemowe.
        mw.window().on_winit_window_event(|_, ev| {
            if let slint::winit_030::winit::event::WindowEvent::Focused(false) = ev {
                later(hide);
            }
            slint::winit_030::EventResult::Propagate
        });
        app.menu_win = Some(mw);
    }
    let Some(mw) = &app.menu_win else { return };
    // Wygląd jak w oknie głównym: motyw i akcent z okładki.
    let t = ui.global::<Theme>();
    let mt = mw.global::<Theme>();
    mt.set_dark(t.get_dark());
    mt.set_accent(t.get_accent());
    mt.set_tint(t.get_tint());
    mt.set_scope(t.get_scope());
    mt.set_corners(t.get_corners());
    mt.set_text_scale(t.get_text_scale());
    mt.set_dense(t.get_dense());
    mt.set_glass(t.get_glass());
    mt.set_glass_tint(t.get_glass_tint());
    mt.set_glass_image(t.get_glass_image());
    mt.set_glass_has(t.get_glass_has());
    mw.set_items(ModelRc::new(VecModel::from(items)));
    let sf = ui.window().scale_factor();
    let w = 280.0 * sf;
    let h = mw.get_content_height() * sf;
    // Początek obszaru okna (bez niewidocznych ramek Windows) – x, y są liczone od niego.
    let (wx, wy) = ui
        .window()
        .with_winit_window(|w| w.inner_position().ok().map(|p| (p.x, p.y)))
        .flatten()
        .unwrap_or_else(|| {
            let (gx, gy, _, _) = crate::win::geometry(&ui);
            (gx, gy)
        });
    // Pozycja kliknięcia na ekranie; menu nie wychodzi poza monitor.
    let mut px = wx as f32 + x * sf;
    let mut py = wy as f32 + y * sf;
    if above {
        py -= h + 6.0 * sf;
    }
    if let Some((mx, my, mw_, mh)) = crate::win::monitor(&ui) {
        px = px.min((mx + mw_ as i32) as f32 - w - 4.0).max(mx as f32);
        // Za nisko – menu przesuwa się w górę tylko o tyle, ile trzeba (zostaje przy kursorze).
        py = py.min((my + mh as i32) as f32 - h - 4.0).max(my as f32);
    }
    let pos = slint::PhysicalPosition::new(px as i32, py as i32);
    mw.window().set_size(slint::PhysicalSize::new(w as u32, h.max(10.0) as u32));
    mw.window().set_position(pos);
    let _ = mw.show();
    crate::win::popup_style(mw.window());
    crate::win::acrylic(crate::win::hwnd_of(mw.window()), app.s.theme != "light");
    // Pierwsze pokazanie potrafi zignorować pozycję – ustawiamy ją jeszcze raz. Menu zawsze
    // nad oknem programu, także gdy to jest „zawsze na wierzchu” (bez przeskakiwania pod nie).
    mw.window().set_position(pos);
    mw.window().with_winit_window(|w| {
        w.set_window_level(slint::winit_030::winit::window::WindowLevel::AlwaysOnTop);
        w.focus_window();
    });
}

/// Zamyka okno menu.
pub fn hide(app: &mut App) {
    if let Some(mw) = &app.menu_win {
        let _ = mw.hide();
    }
}

/// Wyjście dźwięku – lista urządzeń.
pub fn device_menu(app: &mut App, x: f32, y: f32) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Prefs>();
    let cur = p.get_device();
    let items: Vec<MenuEntry> = p.get_devices().iter().enumerate().map(|(i, d)| check(&format!("dev-{i}"), &d, i as i32 == cur, "")).collect();
    open_above(app, MenuCtx::Device, items, x - 120.0, y);
}

pub fn main_menu(app: &mut App, x: f32, y: f32) {
    let full = app.ui().map(|u| u.global::<Ui>().get_fullscreen()).unwrap_or(false);
    let items = vec![
        item("open-files", tr!("Open files…"), "file", "Ctrl+O"),
        item("open-folder", tr!("Open folder…"), "folder-open", "Ctrl+Shift+O"),
        item("open-url", tr!("Open URL (stream, radio)…"), "link", "Ctrl+U"),
        item("open-cd", tr!("Play audio CD"), "album", ""),
        item("cast", if app.dlna.is_some() { tr!("Playing on a device (DLNA)…") } else { tr!("Play on a device (DLNA)…") }, "radio", ""),
        sep(),
        item("new-list", tr!("New list"), "playlist", "Ctrl+N"),
        item("import-list", tr!("Import playlist (M3U, PLS, XSPF, CUE)…"), "download", ""),
        item("export-list", tr!("Save list as M3U8…"), "download", "Ctrl+S"),
        sep(),
        item("equalizer", tr!("Equalizer and effects"), "equalizer", "E"),
        item("sleep", tr!("Sleep timer…"), "timer", ""),
        check("full", tr!("Full screen"), full, "F11"),
        item("mini", tr!("Mini player"), "pip", "Ctrl+M"),
        check("ontop", tr!("Always on top"), app.s.always_on_top, "Ctrl+T"),
        sep(),
        item("settings", tr!("Settings"), "settings", "Ctrl+,"),
        item("update", tr!("Check for updates"), "refresh", ""),
        item("about", tr!("About"), "info", "F1"),
        item("coffee", tr!("Buy the author a coffee"), "coffee", ""),
        sep(),
        item("quit", tr!("Quit"), "close", "Ctrl+Q"),
    ];
    open(app, MenuCtx::Main, items, x, y);
}

pub fn video_menu(app: &mut App, x: f32, y: f32, above: bool) {
    let fs = app.ui().map(|u| u.global::<Video>().get_fullscreen()).unwrap_or(false);
    let ontop = app.ui().map(|u| u.global::<Video>().get_ontop()).unwrap_or(false);
    let aspect = app.ui().map(|u| u.global::<Video>().get_aspect()).unwrap_or(0);
    let playing = crate::player::is_playing(app);
    let mut items = vec![
        item("v-pause", if playing { tr!("Pause") } else { tr!("Play") }, if playing { "pause" } else { "play" }, tr!("Space")),
        check("v-full", tr!("Full screen"), fs, "F"),
        check("v-ontop", tr!("Always on top"), ontop, "T"),
        item("v-mini", tr!("Picture in picture"), "pip", "P"),
        sep(),
        item("v-audio", tr!("Audio tracks and speed…"), "audiotrack", "A"),
        item("v-subs", tr!("Subtitles…"), "subtitles", "V"),
        item("v-image", tr!("Picture: colors, zoom…"), "tune", ""),
        item("v-chapters", tr!("Chapters…"), "bookmark", ""),
        item("v-info", tr!("Playback details"), "info", "I"),
        sep(),
    ];
    let an = crate::video::aspect_names();
    items.push(item("v-aspect-next", &tr!("Aspect ratio: {}", an[aspect.clamp(0, 5) as usize]), "aspect", "C"));
    items.extend([
        item("v-rotate", tr!("Rotate by 90°"), "refresh", "Ctrl+R"),
        item("v-reset", tr!("Picture as in file"), "aspect", "0"),
        check("v-deband", tr!("Debanding"), app.s.deband, "B"),
        check("v-ambient", tr!("Ambient glow"), app.s.ambient > 0, "G"),
        sep(),
        item("v-shot", tr!("Screenshot"), "camera", "S"),
        item("v-shot-clean", tr!("Screenshot without subtitles"), "camera", "Shift+S"),
        item("v-ab", tr!("A-B loop"), "repeat", "L"),
        sep(),
        item("open-files", tr!("Open file…"), "file", "Ctrl+O"),
        item("v-audio-only", tr!("Audio only (picture off in the background)"), "audiotrack", ""),
        item("v-back", tr!("Library (the movie keeps playing)"), "back", "Esc"),
    ]);
    // Przy kursorze (prawy klik) albo nad przyciskiem „Więcej”.
    open_at(app, MenuCtx::Video, items, x, y, above);
}

pub fn sleep_menu(app: &mut App, x: f32, y: f32) {
    let mut items = vec![];
    for m in [15, 30, 45, 60, 90, 120] {
        items.push(item(&format!("sleep-{m}"), &tr!("In {} min", m), "timer", ""));
    }
    items.push(sep());
    items.push(item("sleep-track", tr!("After the current track"), "note", ""));
    items.push(item("sleep-list", tr!("When the list ends"), "playlist", ""));
    if app.sleep.is_some() {
        items.push(sep());
        items.push(item("sleep-off", tr!("Turn off the sleep timer"), "close", ""));
    }
    open_above(app, MenuCtx::Sleep, items, x, y);
}

/// Wybór pozycji menu.
pub fn activated(app: &mut App, id: &str) {
    let ctx = std::mem::take(&mut app.menu);
    match ctx {
        MenuCtx::List => crate::lists::command(app, id),
        MenuCtx::Lib => crate::libview::command(app, id),
        MenuCtx::Tab(i) => crate::lists::tab_command(app, i, id),
        MenuCtx::Tile(k) => crate::libview::tile_command(app, &k, id),
        MenuCtx::Nav(n) => crate::libview::nav_command(app, &n, id),
        MenuCtx::Sleep => crate::sleep::command(app, id),
        MenuCtx::Columns(p) => crate::columns::command(app, p, id),
        MenuCtx::Sort => crate::sorting::command(app, id),
        MenuCtx::Video => crate::video::command(app, id),
        MenuCtx::Device => {
            if let (Some(ui), Some(i)) = (app.ui(), id.strip_prefix("dev-").and_then(|s| s.parse::<i32>().ok())) {
                ui.global::<Prefs>().set_device(i);
                crate::prefs::changed(app, "device");
            }
        }
        MenuCtx::Main | MenuCtx::None => crate::keys::action(app, id),
    }
}
