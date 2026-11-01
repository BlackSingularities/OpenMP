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
    let _ = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
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
