//! Odtwarzanie na urządzeniach DLNA: szukanie odbiorników, wysłanie bieżącego pliku
//! (lokalny plik przez wbudowany serwer HTTP, strumień – wprost) i zatrzymanie.

use std::time::Duration;

use openmp_core::dlna;
use openmp_core::meta;

use crate::app::*;
use crate::*;

/// Szuka odbiorników w tle i pokazuje je w menu.
pub fn search(app: &mut App) {
    app.toast(tr!("Searching for devices in the network…"));
    std::thread::spawn(|| {
        let found = dlna::discover(Duration::from_secs(3));
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                app.dlna_found = found;
                show_menu(app);
            });
        });
    });
}

fn show_menu(app: &mut App) {
    if app.dlna_found.is_empty() && app.dlna.is_none() {
        app.toast(tr!("No DLNA devices found – the device must be on and in the same network"));
        return;
    }
    let mut items: Vec<MenuEntry> = app
        .dlna_found
        .iter()
        .enumerate()
        .map(|(i, r)| crate::menu::check(&format!("cast-{i}"), &r.name, app.dlna.as_ref().is_some_and(|d| d.control == r.control), ""))
        .collect();
    if app.dlna.is_some() {
        items.push(crate::menu::sep());
        items.push(crate::menu::item("cast-stop", tr!("Stop playing on the device"), "close", ""));
    }
    let x = app.ui().map(|u| u.window().size().width as f32 / u.window().scale_factor() / 2.0 - 140.0).unwrap_or(40.0);
    crate::menu::open(app, MenuCtx::Main, items, x, 80.0);
}

/// Wysyła bieżący plik na odbiornik `i` z ostatniego szukania; na komputerze odtwarzanie staje.
pub fn play_on(app: &mut App, i: usize) {
    let Some(r) = app.dlna_found.get(i).cloned() else { return };
    let Some(c) = app.cur.clone() else {
        app.toast(tr!("Start playing something first"));
        return;
    };
    let path = c.info.path.clone();
    let title = c.info.display_title();
    let remote = meta::is_url(&path);
    if remote && path.starts_with("cdda://") {
        app.toast(tr!("Audio CDs cannot be sent to a device"));
        return;
    }
    if crate::player::is_playing(app) {
        crate::player::play_pause(app);
    }
    app.toast(tr!("Sending to {}…", r.name));
    std::thread::spawn(move || {
        let url = if remote { Some(path.clone()) } else { dlna::serve(&path, &r) };
        let res = match url {
            Some(u) => dlna::play(&r, &u, &title, if remote { "audio/mpeg" } else { dlna::mime(&path) }),
            None => Err(tr!("no network connection to the device").to_string()),
        };
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(()) => {
                    app.toast(tr!("Playing on {}", r.name));
                    app.dlna = Some(r);
                }
                Err(e) => app.toast(tr!("The device did not start playback: {}", e)),
            });
        });
    });
}

pub fn stop(app: &mut App) {
    let Some(r) = app.dlna.take() else { return };
    std::thread::spawn(move || {
        let _ = dlna::stop(&r);
    });
    app.toast(tr!("Stopped playing on the device"));
}
