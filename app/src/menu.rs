//! Menu (główne, kontekstowe) – jeden wygląd dla całego programu; pozycje buduje Rust.

use slint::{ModelRc, VecModel};

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
    app.menu = ctx;
    let Some(ui) = app.ui() else { return };
    let u = ui.global::<Ui>();
    u.set_menu(ModelRc::new(VecModel::from(items)));
    u.set_menu_x(x);
    u.set_menu_y(y);
    ui.invoke_open_menu();
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
    let playing = crate::player::is_playing(app);
    let items = vec![
        item("v-pause", if playing { "Pauza" } else { "Odtwórz" }, if playing { "pause" } else { "play" }, "Spacja"),
        check("v-full", "Pełny ekran", fs, "F"),
        check("v-ontop", "Zawsze na wierzchu", ontop, "T"),
        sep(),
        item("v-audio", "Ścieżki dźwiękowe i prędkość…", "audiotrack", "A"),
        item("v-subs", "Napisy…", "subtitles", "V"),
        item("v-image", "Obraz i proporcje…", "aspect", ""),
        item("v-chapters", "Rozdziały…", "bookmark", ""),
        sep(),
        item("v-shot", "Zrzut ekranu", "camera", "S"),
        item("v-ab", "Pętla A-B", "repeat", "L"),
        sep(),
        item("open-files", "Otwórz plik…", "file", "Ctrl+O"),
        item("v-back", "Wróć do muzyki", "back", "Esc"),
    ];
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
    open(app, MenuCtx::Sleep, items, x, (y - 290.0).max(10.0));
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
        MenuCtx::Main | MenuCtx::None => crate::keys::action(app, id),
    }
}
