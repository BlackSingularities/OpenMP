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

/// Okno w proporcjach filmu, wyśrodkowane, najwyżej 80% ekranu.
pub fn fit_to(ui: &AppWindow, w: f64, h: f64) {
    let Some((mx, my, mw, mh)) = monitor(ui) else { return };
    let (maxw, maxh) = (mw as f64 * 0.8, mh as f64 * 0.8);
    let scale = (maxw / w).min(maxh / h).min(ui.window().scale_factor() as f64);
    let (nw, nh) = ((w * scale).round().max(480.0), (h * scale).round().max(270.0));
    let x = mx + ((mw as f64 - nw) / 2.0) as i32;
    let y = my + ((mh as f64 - nh) / 2.0) as i32;
    set_geometry(ui, (x, y, nw as u32, nh as u32));
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
