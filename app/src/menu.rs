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
    mw.set_items(ModelRc::new(VecModel::from(items)));
    let sf = ui.window().scale_factor();
    let w = 280.0 * sf;
    let h = mw.get_content_height() * sf;
    let (wx, wy, _, _) = crate::win::geometry(&ui);
    // Pozycja kliknięcia na ekranie; menu nie wychodzi poza monitor.
    let mut px = wx as f32 + x * sf;
    let mut py = wy as f32 + y * sf;
    if above {
        py -= h + 6.0 * sf;
    }
    if let Some((mx, my, mw_, mh)) = crate::win::monitor(&ui) {
        py = py.max(my as f32);
        px = px.min((mx + mw_ as i32) as f32 - w - 4.0).max(mx as f32);
        if py + h > (my + mh as i32) as f32 {
            py = (py - h).max(my as f32);
        }
    }
    mw.window().set_size(slint::PhysicalSize::new(w as u32, h.max(10.0) as u32));
    mw.window().set_position(slint::PhysicalPosition::new(px as i32, py as i32));
    let _ = mw.show();
    crate::win::popup_style(mw.window());
    mw.window().with_winit_window(|w| w.focus_window());
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
    let compact = app.s.compact;
    let items = vec![
        item("open-files", "Otwórz pliki…", "file", "Ctrl+O"),
        item("open-folder", "Otwórz folder…", "folder-open", "Ctrl+Shift+O"),
        item("open-url", "Otwórz adres (strumień, radio)…", "link", "Ctrl+U"),
        sep(),
        item("new-list", "Nowa lista", "playlist", "Ctrl+N"),
        item("import-list", "Importuj listę (M3U, PLS, XSPF, CUE)…", "download", ""),
        item("export-list", "Zapisz listę jako M3U8…", "download", "Ctrl+S"),
        sep(),
        item("equalizer", "Korektor i efekty", "equalizer", "E"),
        item("sleep", "Wyłącznik czasowy…", "timer", ""),
        check("compact", "Widok kompaktowy", compact, "Ctrl+K"),
        item("mini", "Miniodtwarzacz", "pip", "Ctrl+M"),
        check("ontop", "Zawsze na wierzchu", app.s.always_on_top, "Ctrl+T"),
        sep(),
        item("settings", "Ustawienia", "settings", "Ctrl+,"),
        item("update", "Sprawdź aktualizacje", "refresh", ""),
        item("about", "O programie", "info", "F1"),
        item("coffee", "Postaw kawę autorowi", "coffee", ""),
        sep(),
        item("quit", "Zakończ", "close", "Ctrl+Q"),
    ];
    open(app, MenuCtx::Main, items, x, y);
}

pub fn video_menu(app: &mut App) {
    let fs = app.ui().map(|u| u.global::<Video>().get_fullscreen()).unwrap_or(false);
    let ontop = app.ui().map(|u| u.global::<Video>().get_ontop()).unwrap_or(false);
    let aspect = app.ui().map(|u| u.global::<Video>().get_aspect()).unwrap_or(0);
    let playing = crate::player::is_playing(app);
    let wide = app.s.video_ui == 1;
    let mut items = vec![
        item("v-pause", if playing { "Pauza" } else { "Odtwórz" }, if playing { "pause" } else { "play" }, "Spacja"),
        check("v-full", "Pełny ekran", fs, "F"),
        check("v-ontop", "Zawsze na wierzchu", ontop, "T"),
        item("v-mini", "Obraz w obrazie", "pip", "P"),
        sep(),
        check("v-layout-0", "Widok kinowy", !wide, "W"),
        check("v-layout-1", "Widok szeroki (z panelami)", wide, "W"),
        sep(),
        item("v-audio", "Ścieżki dźwiękowe i prędkość…", "audiotrack", "A"),
        item("v-subs", "Napisy…", "subtitles", "V"),
        item("v-image", "Obraz: barwy, powiększenie…", "tune", ""),
        item("v-chapters", "Rozdziały…", "bookmark", ""),
        item("v-info", "Szczegóły odtwarzania", "info", "I"),
        sep(),
    ];
    let an = ["automatyczne", "16:9", "4:3", "21:9", "rozciągnięte", "wypełnij ekran"];
    items.push(item("v-aspect-next", &format!("Proporcje: {}", an[aspect.clamp(0, 5) as usize]), "aspect", "C"));
    items.extend([
        item("v-rotate", "Obróć o 90°", "refresh", "Ctrl+R"),
        item("v-reset", "Obraz jak w pliku", "aspect", "0"),
        check("v-deband", "Usuwanie pasm", app.s.deband, "B"),
        check("v-ambient", "Poświata z obrazu", app.s.ambient > 0, "G"),
        sep(),
        item("v-shot", "Zrzut ekranu", "camera", "S"),
        item("v-shot-clean", "Zrzut ekranu bez napisów", "camera", "Shift+S"),
        item("v-ab", "Pętla A-B", "repeat", "L"),
        sep(),
        item("open-files", "Otwórz plik…", "file", "Ctrl+O"),
        item("v-audio-only", "Tylko dźwięk (obraz w tle wyłączony)", "audiotrack", ""),
        item("v-back", "Biblioteka (film gra dalej)", "back", "Esc"),
    ]);
    let (x, y) = app.ui().map(|u| (u.window().size().width as f32 / u.window().scale_factor() / 2.0 - 130.0, 80.0)).unwrap_or((40.0, 80.0));
    open(app, MenuCtx::Video, items, x, y);
}

pub fn sleep_menu(app: &mut App, x: f32, y: f32) {
    let mut items = vec![];
    for m in [15, 30, 45, 60, 90, 120] {
        items.push(item(&format!("sleep-{m}"), &format!("Za {m} min"), "timer", ""));
    }
    items.push(sep());
    items.push(item("sleep-track", "Po bieżącym utworze", "note", ""));
    items.push(item("sleep-list", "Po zakończeniu listy", "playlist", ""));
    if app.sleep.is_some() {
        items.push(sep());
        items.push(item("sleep-off", "Wyłącz wyłącznik", "close", ""));
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
