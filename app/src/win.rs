//! Integracja z Windows: okno bez ramki (przesuwanie, zmiana rozmiaru, na wierzchu),
//! panel multimediów systemu (klawisze, nakładka głośności), blokada usypiania, Eksplorator.

use std::time::Duration;

use slint::winit_030::winit::window::{ResizeDirection, WindowLevel};
use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};

use crate::app::*;
use crate::*;

pub fn hwnd(ui: &AppWindow) -> isize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    ui.window()
        .with_winit_window(|w| match w.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
            _ => 0,
        })
        .unwrap_or(0)
}

/// Uchwyt okna Win32 dowolnego okna Slint.
pub fn hwnd_of(w: &slint::Window) -> isize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    w.with_winit_window(|w| match w.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    })
    .unwrap_or(0)
}

/// Pełny ekran bez ramki Windows 11: obramowanie DWM (1 px) i zaokrąglone rogi wyłączone,
/// po wyjściu – z powrotem domyślne.
pub fn fullscreen_frame(ui: &AppWindow, fs: bool) {
    let h = hwnd(ui);
    if h == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_DONOTROUND};
        let hw = HWND(h as *mut _);
        // 0xFFFFFFFE – bez obramowania, 0xFFFFFFFF – kolor domyślny systemu.
        let color: u32 = if fs { 0xFFFF_FFFE } else { 0xFFFF_FFFF };
        let _ = DwmSetWindowAttribute(hw, DWMWA_BORDER_COLOR, &color as *const _ as *const _, 4);
        let pref = if fs { DWMWCP_DONOTROUND } else { DWMWCP_DEFAULT };
        let _ = DwmSetWindowAttribute(hw, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, std::mem::size_of_val(&pref) as u32);
    }
}

pub fn drag(ui: &AppWindow) {
    ui.window().with_winit_window(|w| {
        let _ = w.drag_window();
    });
}

pub fn resize(ui: &AppWindow, d: i32) {
    let dir = match d {
        1 => ResizeDirection::North,
        2 => ResizeDirection::South,
        3 => ResizeDirection::West,
        4 => ResizeDirection::East,
        5 => ResizeDirection::NorthWest,
        6 => ResizeDirection::NorthEast,
        7 => ResizeDirection::SouthWest,
        _ => ResizeDirection::SouthEast,
    };
    ui.window().with_winit_window(|w| {
        let _ = w.drag_resize_window(dir);
    });
}

pub fn set_ontop(app: &App, on: bool) {
    if let Some(ui) = app.ui() {
        ui.window().with_winit_window(|w| w.set_window_level(if on { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal }));
    }
}

pub fn geometry(ui: &AppWindow) -> (i32, i32, u32, u32) {
    let p = ui.window().position();
    let s = ui.window().size();
    (p.x, p.y, s.width, s.height)
}

pub fn set_geometry(ui: &AppWindow, g: (i32, i32, u32, u32)) {
    ui.window().set_size(PhysicalSize::new(g.2.max(300), g.3.max(96)));
    ui.window().set_position(PhysicalPosition::new(g.0, g.1));
}

/// Obszar roboczy monitora, na którym jest okno: (x, y, szer., wys.).
pub fn monitor(ui: &AppWindow) -> Option<(i32, i32, u32, u32)> {
    ui.window()
        .with_winit_window(|w| {
            w.current_monitor().map(|m| {
                let p = m.position();
                let s = m.size();
                (p.x, p.y, s.width, s.height)
            })
        })
        .flatten()
}

/// Prawdziwe szkło okna: akryl Windows 11 (DWM) rozmywa to, co jest pod oknem. Okno ma
/// przezroczyste tło, a barwę szkła rysuje interfejs (Theme.glass). Ciemny akryl przy ciemnym
/// motywie. Na starszym Windowsie atrybut jest pomijany – zostaje zwykła barwa okna.
pub fn acrylic(h: isize, dark: bool) {
    if h == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE};
        use windows::Win32::UI::Controls::MARGINS;
        let hw = HWND(h as *mut _);
        let d: i32 = dark as i32;
        let _ = DwmSetWindowAttribute(hw, DWMWA_USE_IMMERSIVE_DARK_MODE, &d as *const _ as *const _, std::mem::size_of_val(&d) as u32);
        // Okno bez ramki: tło systemowe sięga całej powierzchni okna.
        let m = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
        let _ = DwmExtendFrameIntoClientArea(hw, &m);
        let kind = DWMSBT_TRANSIENTWINDOW;
        let _ = DwmSetWindowAttribute(hw, DWMWA_SYSTEMBACKDROP_TYPE, &kind as *const _ as *const _, std::mem::size_of_val(&kind) as u32);
    }
}

/// Akryl wszystkich okien programu (po zmianie motywu – ciemny albo jasny).
pub fn acrylic_all(app: &App) {
    let dark = app.s.theme != "light";
    if let Some(ui) = app.ui() {
        acrylic(hwnd(&ui), dark);
    }
    if let Some(d) = &app.dialogs {
        acrylic(hwnd_of(d.win.window()), dark);
    }
    if let Some(m) = &app.menu_win {
        acrylic(hwnd_of(m.window()), dark);
    }
}

/// Zaokrąglone rogi i cień okna bez ramki (Windows 11).
pub fn round_corners(ui: &AppWindow) {
    let h = hwnd(ui);
    if h == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(HWND(h as *mut _), DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, std::mem::size_of_val(&pref) as u32);
    }
}

/// Nie usypiaj komputera i ekranu, dopóki gra film.
pub fn keep_awake(app: &mut App, on: bool) {
    let on = on && app.s.prevent_sleep;
    if on == app.awake {
        return;
    }
    app.awake = on;
    unsafe {
        use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
        if on {
            SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

pub fn reveal(path: &str) {
    use std::os::windows::process::CommandExt;
    // Explorer nie rozumie "/select,ścieżka" w całości wziętego w cudzysłów (tak robi .arg()
    // przy spacjach) i otwiera wtedy Dokumenty — stąd raw_arg i cudzysłów tylko wokół ścieżki.
    let p = path.replace('/', "\\");
    let _ = std::process::Command::new("explorer").raw_arg(format!("/select,\"{p}\"")).spawn();
}

/// Panel multimediów Windows (SMTC): klawisze multimedialne, słuchawki, nakładka głośności.
pub fn smtc_init(app: &mut App) {
    if !app.s.media_keys || app.smtc.is_some() {
        return;
    }
    let Some(ui) = app.ui() else { return };
    let h = hwnd(&ui);
    if h == 0 {
        return;
    }
    let cfg = PlatformConfig { display_name: "OpenMP", dbus_name: "openmp", hwnd: Some(h as *mut std::ffi::c_void) };
    let Ok(mut c) = MediaControls::new(cfg) else { return };
    let ok = c.attach(|e: MediaControlEvent| {
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match e {
                MediaControlEvent::Play => crate::player::set_paused(app, false),
                MediaControlEvent::Pause => crate::player::set_paused(app, true),
                MediaControlEvent::Toggle => crate::player::play_pause(app),
                MediaControlEvent::Next => crate::player::next(app),
                MediaControlEvent::Previous => crate::player::prev(app),
                MediaControlEvent::Stop => crate::player::stop(app),
                MediaControlEvent::SetPosition(MediaPosition(d)) => crate::player::seek_abs(app, d.as_secs_f64()),
                MediaControlEvent::SeekBy(dir, d) => {
                    let s = d.as_secs_f64();
                    crate::player::seek_rel(app, if matches!(dir, souvlaki::SeekDirection::Forward) { s } else { -s })
                }
                MediaControlEvent::Seek(dir) => crate::player::seek_rel(app, if matches!(dir, souvlaki::SeekDirection::Forward) { 10.0 } else { -10.0 }),
                MediaControlEvent::SetVolume(v) => crate::player::set_volume(app, v as f32),
                MediaControlEvent::Raise => raise(app),
                MediaControlEvent::Quit => crate::keys::quit(app),
                _ => {}
            });
        });
    });
    if ok.is_ok() {
        app.smtc = Some(c);
        smtc_meta(app);
        smtc_state(app);
    }
}

pub fn smtc_meta(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Player>();
    let (title, artist, album) = (p.get_title().to_string(), p.get_artist().to_string(), p.get_album().to_string());
    let dur = app.cur.as_ref().map(|c| c.info.duration).filter(|d| *d > 0.0);
    // Okładka dla panelu Windows – plik w pamięci podręcznej.
    let cover = app.cover_bytes.as_ref().and_then(|b| {
        let path = openmp_core::paths::cache().join("smtc-cover.img");
        std::fs::write(&path, b).ok()?;
        Some(format!("file:///{}", path.to_string_lossy().replace('\\', "/")))
    });
    tray_tip(&if artist.is_empty() { title.clone() } else { format!("{title} – {artist}") });
    let Some(c) = app.smtc.as_mut() else { return };
    let _ = c.set_metadata(MediaMetadata {
        title: Some(&title),
        artist: Some(&artist),
        album: Some(&album),
        cover_url: cover.as_deref(),
        duration: dur.map(Duration::from_secs_f64),
    });
}

pub fn smtc_state(app: &mut App) {
    let playing = crate::player::is_playing(app);
    let has = app.cur.as_ref().is_some_and(|c| c.item != 0);
    let pos = crate::player::position(app).0;
    let Some(c) = app.smtc.as_mut() else { return };
    let progress = Some(MediaPosition(Duration::from_secs_f64(pos.max(0.0))));
    let _ = c.set_playback(if !has {
        MediaPlayback::Stopped
    } else if playing {
        MediaPlayback::Playing { progress }
    } else {
        MediaPlayback::Paused { progress }
    });
}

/// Wywołanie okna na pierwszy plan (druga instancja, panel Windows).
pub fn raise(app: &mut App) {
    if TRAY.get() != 0 {
        show_main(TRAY.get());
    }
    if let Some(ui) = app.ui() {
        ui.window().set_minimized(false);
        ui.window().with_winit_window(|w| {
            w.focus_window();
        });
    }
}

/// Okno menu: bez pozycji na pasku zadań i z zaokrąglonymi rogami (Windows 11).
pub fn popup_style(w: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let h = w
        .with_winit_window(|w| match w.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
            _ => 0,
        })
        .unwrap_or(0);
    if h == 0 {
        return;
    }
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL};
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW};
        let hw = HWND(h as *mut _);
        let ex = GetWindowLongPtrW(hw, GWL_EXSTYLE);
        SetWindowLongPtrW(hw, GWL_EXSTYLE, (ex | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize));
        let pref = DWMWCP_ROUNDSMALL;
        let _ = DwmSetWindowAttribute(hw, DWMWA_WINDOW_CORNER_PREFERENCE, &pref as *const _ as *const _, std::mem::size_of_val(&pref) as u32);
    }
}

// Przyciski miniatury na pasku zadań: poprzedni, odtwórz/pauza, następny.

const THUMB_PREV: u32 = 1;
const THUMB_PLAY: u32 = 2;
const THUMB_NEXT: u32 = 3;
const THUMB_SIZE: i32 = 16;

thread_local! {
    static THUMB: std::cell::RefCell<Option<Thumb>> = const { std::cell::RefCell::new(None) };
}

struct Thumb {
    hwnd: isize,
    list: windows::Win32::UI::Shell::ITaskbarList3,
    icons: [windows::Win32::UI::WindowsAndMessaging::HICON; 4],
    added: bool,
    playing: bool,
    created_msg: u32,
}

/// Ikona 16×16 z prostych kształtów: 0 – poprzedni, 1 – odtwórz, 2 – pauza, 3 – następny.
fn thumb_icon(kind: u8) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
    use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};
    let n = THUMB_SIZE;
    let mut px = vec![0u32; (n * n) as usize];
    let tri = |x: f32, y: f32, x0: f32, x1: f32, right: bool| {
        // Trójkąt od x0 do x1, wierzchołek w prawo albo w lewo, wysokość 3..13.
        if x < x0 || x > x1 {
            return false;
        }
        let t = if right { (x - x0) / (x1 - x0) } else { (x1 - x) / (x1 - x0) };
        let half = 5.0 * (1.0 - t);
        (y - 8.0).abs() <= half
    };
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let on = match kind {
                0 => (fx >= 3.0 && fx <= 5.0 && fy >= 3.0 && fy <= 13.0) || tri(fx, fy, 5.0, 13.0, false),
                1 => tri(fx, fy, 4.0, 13.0, true),
                2 => fy >= 3.0 && fy <= 13.0 && ((fx >= 4.0 && fx <= 7.0) || (fx >= 9.0 && fx <= 12.0)),
                _ => (fx >= 11.0 && fx <= 13.0 && fy >= 3.0 && fy <= 13.0) || tri(fx, fy, 3.0, 11.0, true),
            };
            if on {
                px[(y * n + x) as usize] = 0xffff_ffff;
            }
        }
    }
    let mask = vec![0u8; (n * n / 8) as usize];
    unsafe {
        let color = CreateBitmap(n, n, 1, 32, Some(px.as_ptr() as *const _));
        let hmask = CreateBitmap(n, n, 1, 1, Some(mask.as_ptr() as *const _));
        let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: hmask, hbmColor: color };
        let icon = CreateIconIndirect(&info).unwrap_or(HICON::default());
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(hmask.into());
        icon
    }
}

fn thumb_button(id: u32, icon: windows::Win32::UI::WindowsAndMessaging::HICON, tip: &str) -> windows::Win32::UI::Shell::THUMBBUTTON {
    use windows::Win32::UI::Shell::{THBF_ENABLED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON};
    let mut sz = [0u16; 260];
    for (i, c) in tip.encode_utf16().take(259).enumerate() {
        sz[i] = c;
    }
    THUMBBUTTON { dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS, iId: id, iBitmap: 0, hIcon: icon, szTip: sz, dwFlags: THBF_ENABLED }
}

fn thumb_add(t: &mut Thumb) {
    use windows::Win32::Foundation::HWND;
    if t.added {
        return;
    }
    let play = if t.playing { (t.icons[2], tr!("Pause")) } else { (t.icons[1], tr!("Play")) };
    let buttons = [thumb_button(THUMB_PREV, t.icons[0], tr!("Previous")), thumb_button(THUMB_PLAY, play.0, play.1), thumb_button(THUMB_NEXT, t.icons[3], tr!("Next"))];
    unsafe {
        if t.list.ThumbBarAddButtons(HWND(t.hwnd as *mut _), &buttons).is_ok() {
            t.added = true;
        }
    }
}

unsafe extern "system" fn thumb_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::{DefSubclassProc, THBN_CLICKED};
    use windows::Win32::UI::WindowsAndMessaging::WM_COMMAND;
    if msg == WM_COMMAND && ((wparam.0 >> 16) & 0xffff) as u32 == THBN_CLICKED {
        match (wparam.0 & 0xffff) as u32 {
            THUMB_PREV => later(crate::player::prev),
            THUMB_PLAY => later(crate::player::play_pause),
            THUMB_NEXT => later(crate::player::next),
            _ => {}
        }
        return windows::Win32::Foundation::LRESULT(0);
    }
    // Explorer utworzył (albo odtworzył po restarcie) przycisk na pasku zadań.
    // try_borrow: wywołania WinAPI w thumb_state mogą wrócić tu synchronicznie.
    let created = THUMB.with(|t| t.try_borrow().ok().and_then(|t| t.as_ref().map(|t| t.created_msg)).unwrap_or(0));
    if created != 0 && msg == created {
        THUMB.with(|t| {
            if let Ok(mut b) = t.try_borrow_mut() {
                if let Some(t) = b.as_mut() {
                    t.added = false;
                    thumb_add(t);
                }
            }
        });
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Przyciski miniatury głównego okna – wołane raz po pokazaniu okna.
pub fn thumb_init(ui: &AppWindow) {
    use windows::core::w;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{ITaskbarList3, SetWindowSubclass, TaskbarList};
    use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
    let h = hwnd(ui);
    if h == 0 || THUMB.with(|t| t.borrow().is_some()) {
        return;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(list) = CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) else { return };
        if list.HrInit().is_err() {
            return;
        }
        let created_msg = RegisterWindowMessageW(w!("TaskbarButtonCreated"));
        let mut t = Thumb { hwnd: h, list, icons: [thumb_icon(0), thumb_icon(1), thumb_icon(2), thumb_icon(3)], added: false, playing: false, created_msg };
        thumb_add(&mut t);
        THUMB.with(|c| *c.borrow_mut() = Some(t));
        let _ = SetWindowSubclass(HWND(h as *mut _), Some(thumb_proc), 0x4f4d50, 0);
    }
}

/// Ikona odtwórz/pauza zgodna ze stanem odtwarzania.
pub fn thumb_state(app: &App) {
    use windows::Win32::Foundation::HWND;
    let playing = crate::player::is_playing(app);
    THUMB.with(|t| {
        let Ok(mut b) = t.try_borrow_mut() else { return };
        let Some(t) = b.as_mut() else { return };
        if !t.added {
            thumb_add(t);
        }
        if t.playing == playing || !t.added {
            t.playing = playing;
            return;
        }
        t.playing = playing;
        let btn = if playing { thumb_button(THUMB_PLAY, t.icons[2], tr!("Pause")) } else { thumb_button(THUMB_PLAY, t.icons[1], tr!("Play")) };
        unsafe {
            let _ = t.list.ThumbBarUpdateButtons(HWND(t.hwnd as *mut _), &[btn]);
        }
    });
}

// Globalne skróty: działają także, gdy aktywne jest inne okno.

const HOTKEYS: [(i32, u32, &str); 5] = [
    (0x4f01, 0x20, "play-pause"), // Ctrl+Alt+Spacja
    (0x4f02, 0x27, "next"),       // Ctrl+Alt+→
    (0x4f03, 0x25, "prev"),       // Ctrl+Alt+←
    (0x4f04, 0x26, "vol-up"),     // Ctrl+Alt+↑
    (0x4f05, 0x28, "vol-down"),   // Ctrl+Alt+↓
];

unsafe extern "system" fn hotkey_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::WM_HOTKEY;
    if msg == WM_HOTKEY {
        if let Some(&(_, _, action)) = HOTKEYS.iter().find(|h| h.0 as usize == wparam.0) {
            match action {
                "play-pause" => later(crate::player::play_pause),
                "next" => later(crate::player::next),
                "prev" => later(crate::player::prev),
                "vol-up" => later(|app| crate::player::set_volume(app, app.s.volume + 0.05)),
                _ => later(|app| crate::player::set_volume(app, app.s.volume - 0.05)),
            }
            return windows::Win32::Foundation::LRESULT(0);
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Rejestruje albo zwalnia globalne skróty Ctrl+Alt+… dla głównego okna.
/// Skrót zajęty przez inny program jest pomijany; zwraca liczbę zarejestrowanych.
pub fn hotkeys(app: &App, on: bool) -> usize {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
    use windows::Win32::UI::Shell::{RemoveWindowSubclass, SetWindowSubclass};
    let Some(ui) = app.ui() else { return 0 };
    let h = hwnd(&ui);
    if h == 0 {
        return 0;
    }
    let hw = HWND(h as *mut _);
    let mut n = 0;
    unsafe {
        for (id, _, _) in HOTKEYS {
            let _ = UnregisterHotKey(Some(hw), id);
        }
        let _ = RemoveWindowSubclass(hw, Some(hotkey_proc), 0x4f4d51);
        if !on {
            return 0;
        }
        let _ = SetWindowSubclass(hw, Some(hotkey_proc), 0x4f4d51, 0);
        for (id, vk, _) in HOTKEYS {
            if RegisterHotKey(Some(hw), id, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, vk).is_ok() {
                n += 1;
            }
        }
    }
    n
}

/// Język ustawień regionalnych Windows, np. „pl-PL” (pusty, gdy nie da się odczytać).
pub fn system_language() -> String {
    let mut buf = [0u16; 85];
    let n = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buf) };
    if n <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize - 1])
}

// Ikona w zasobniku systemowym: kliknięcie pokazuje okno, prawy przycisk – menu odtwarzania.
// Zamknięcie okna chowa je wtedy do zasobnika, a muzyka gra dalej.

const TRAY_MSG: u32 = 0x8000 + 0x4f; // WM_APP + …
const TRAY_SHOW: usize = 1;
const TRAY_PLAY: usize = 2;
const TRAY_NEXT: usize = 3;
const TRAY_PREV: usize = 4;
const TRAY_QUIT: usize = 5;

thread_local! {
    /// Okno z ikoną w zasobniku (0 – brak ikony) i komunikat „TaskbarCreated” (restart Eksploratora).
    static TRAY: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
    static TRAY_CREATED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn tray_data(h: isize) -> windows::Win32::UI::Shell::NOTIFYICONDATAW {
    windows::Win32::UI::Shell::NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<windows::Win32::UI::Shell::NOTIFYICONDATAW>() as u32,
        hWnd: windows::Win32::Foundation::HWND(h as *mut _),
        uID: 1,
        ..Default::default()
    }
}

fn set_tip(d: &mut windows::Win32::UI::Shell::NOTIFYICONDATAW, text: &str) {
    let w: Vec<u16> = text.encode_utf16().take(d.szTip.len() - 1).collect();
    d.szTip = [0; 128];
    d.szTip[..w.len()].copy_from_slice(&w);
}

/// Mała ikona programu z pliku .exe.
fn app_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
    use windows::core::PCWSTR;
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let w = wide(&exe);
    let mut small = windows::Win32::UI::WindowsAndMessaging::HICON::default();
    unsafe {
        windows::Win32::UI::Shell::ExtractIconExW(PCWSTR(w.as_ptr()), 0, None, Some(&mut small), 1);
    }
    small
}

/// Pokazuje (albo chowa) ikonę w zasobniku.
pub fn tray(app: &App, on: bool) {
    use windows::core::w;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD};
    use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
    let Some(ui) = app.ui() else { return };
    let h = hwnd(&ui);
    if h == 0 {
        return;
    }
    let hw = HWND(h as *mut _);
    unsafe {
        if on {
            if TRAY.get() != 0 {
                return;
            }
            let mut d = tray_data(h);
            d.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            d.uCallbackMessage = TRAY_MSG;
            d.hIcon = app_icon();
            set_tip(&mut d, "OpenMP");
            let _ = Shell_NotifyIconW(NIM_ADD, &d);
            TRAY.set(h);
            TRAY_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));
            let _ = SetWindowSubclass(hw, Some(tray_proc), 0x4f4d52, 0);
        } else if TRAY.get() != 0 {
            tray_remove();
            let _ = RemoveWindowSubclass(hw, Some(tray_proc), 0x4f4d52);
            show_main(h);
        }
    }
}

/// Usuwa ikonę z zasobnika (wyłączenie opcji, zamknięcie programu).
pub fn tray_remove() {
    let h = TRAY.get();
    if h != 0 {
        unsafe {
            let _ = windows::Win32::UI::Shell::Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_DELETE, &tray_data(h));
        }
        TRAY.set(0);
    }
}

/// Podpowiedź ikony: bieżący utwór.
pub fn tray_tip(text: &str) {
    let h = TRAY.get();
    if h == 0 {
        return;
    }
    let mut d = tray_data(h);
    d.uFlags = windows::Win32::UI::Shell::NIF_TIP;
    set_tip(&mut d, if text.is_empty() { "OpenMP" } else { text });
    unsafe {
        let _ = windows::Win32::UI::Shell::Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_MODIFY, &d);
    }
}

/// Zamknięcie okna przy ikonie w zasobniku: okno się chowa, program działa dalej.
pub fn hide_to_tray() -> bool {
    let h = TRAY.get();
    if h == 0 {
        return false;
    }
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(windows::Win32::Foundation::HWND(h as *mut _), windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
    }
    true
}

fn show_main(h: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW};
    let hw = HWND(h as *mut _);
    unsafe {
        let _ = ShowWindow(hw, if IsIconic(hw).as_bool() { SW_RESTORE } else { SW_SHOW });
        let _ = SetForegroundWindow(hw);
    }
}

fn tray_menu(hwnd: windows::Win32::Foundation::HWND) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu, MF_SEPARATOR, MF_STRING, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    };
    let items = [
        (TRAY_SHOW, tr!("Show OpenMP")),
        (0, ""),
        (TRAY_PLAY, tr!("Play / pause")),
        (TRAY_NEXT, tr!("Next")),
        (TRAY_PREV, tr!("Previous")),
        (0, ""),
        (TRAY_QUIT, tr!("Quit")),
    ];
    let labels: Vec<Vec<u16>> = items.iter().map(|(_, l)| wide(l)).collect();
    unsafe {
        let Ok(m) = CreatePopupMenu() else { return };
        for ((id, _), l) in items.iter().zip(&labels) {
            if *id == 0 {
                let _ = AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null());
            } else {
                let _ = AppendMenuW(m, MF_STRING, *id, PCWSTR(l.as_ptr()));
            }
        }
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        // Bez tego menu nie znika po kliknięciu obok.
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, None, hwnd, None).0 as usize;
        let _ = DestroyMenu(m);
        match cmd {
            TRAY_SHOW => show_main(hwnd.0 as isize),
            TRAY_PLAY => later(crate::player::play_pause),
            TRAY_NEXT => later(crate::player::next),
            TRAY_PREV => later(crate::player::prev),
            TRAY_QUIT => later(crate::keys::quit),
            _ => {}
        }
    }
}

unsafe extern "system" fn tray_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP};
    if msg == TRAY_MSG {
        let ev = (lparam.0 & 0xffff) as u32;
        if ev == WM_LBUTTONUP || ev == WM_LBUTTONDBLCLK {
            show_main(hwnd.0 as isize);
        } else if ev == WM_RBUTTONUP || ev == WM_CONTEXTMENU {
            tray_menu(hwnd);
        }
        return windows::Win32::Foundation::LRESULT(0);
    }
    // Eksplorator uruchomiony od nowa – ikonę trzeba dodać jeszcze raz.
    let created = TRAY_CREATED.get();
    if created != 0 && msg == created && TRAY.get() != 0 {
        TRAY.set(0);
        later(|app| {
            if app.s.tray {
                tray(app, true);
                if let Some(t) = app.ui().map(|u| u.global::<Player>().get_title().to_string()) {
                    tray_tip(&t);
                }
            }
        });
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Litery napędów CD/DVD, np. „D:”.
pub fn cd_drives() -> Vec<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_CDROM: u32 = 5;
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:", (b'A' + i) as char))
        .filter(|d| {
            let root: Vec<u16> = format!("{d}\\").encode_utf16().chain(std::iter::once(0)).collect();
            unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_CDROM }
        })
        .collect()
}
