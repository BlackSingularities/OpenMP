//! Radio internetowe: lista stacji (radio-browser.info albo zapisane) i odtwarzanie przez mpv
//! z bieżącym tytułem utworu ze strumienia (ICY).

use openmp_core::meta::TrackInfo;
use openmp_core::radio::{self, Station};

use crate::app::*;
use crate::*;

fn to_row(app: &App, s: &Station) -> crate::Station {
    crate::Station { name: ss(&s.name), url: ss(&s.url), info: ss(s.info()), saved: app.saved_stations.iter().any(|x| x.url == s.url) }
}

fn publish(app: &mut App) {
    let rows: Vec<crate::Station> = app.stations.iter().map(|s| to_row(app, s)).collect();
    app.m.stations.set_vec(rows);
}

pub fn open(app: &mut App) {
    if app.stations.is_empty() {
        let src = app.ui().map(|u| u.global::<Lib>().get_radio_source()).unwrap_or(0);
        load(app, src, String::new());
    }
}

pub fn load(app: &mut App, source: i32, query: String) {
    if source == 2 && query.trim().is_empty() {
        app.stations = app.saved_stations.clone();
        publish(app);
        return;
    }
    if let Some(ui) = app.ui() {
        ui.global::<Lib>().set_radio_busy(true);
    }
    std::thread::spawn(move || {
        let res = radio::fetch(if source == 2 { 1 } else { source }, &query);
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                if let Some(ui) = app.ui() {
                    ui.global::<Lib>().set_radio_busy(false);
                }
                match res {
                    Ok(v) => {
                        app.stations = v;
                        publish(app);
                    }
                    Err(e) => {
                        app.stations.clear();
                        publish(app);
                        app.toast(tr!("Station directory unavailable: {}", e));
                    }
                }
            });
        });
    });
}

pub fn play_index(app: &mut App, i: usize) {
    if let Some(s) = app.stations.get(i).cloned() {
        play_station(app, s);
    }
}

pub fn play_station(app: &mut App, s: Station) {
    let t = TrackInfo { path: s.url.clone(), title: s.name.clone(), ..Default::default() };
    crate::player::start_track(app, None, 0, t, Some(s), None);
}

/// Następna / poprzednia stacja z bieżącego katalogu.
pub fn step(app: &mut App, d: i32) {
    let Some(cur) = app.cur.as_ref().and_then(|c| c.station.clone()) else { return };
    if app.stations.is_empty() {
        return;
    }
    let pos = app.stations.iter().position(|s| s.url == cur.url).map(|p| p as i32).unwrap_or(-1);
    let n = app.stations.len() as i32;
    let next = ((pos + d) % n + n) % n;
    play_index(app, next as usize);
}

pub fn toggle_save(app: &mut App, i: usize) {
    let Some(s) = app.stations.get(i).cloned() else { return };
    if let Some(p) = app.saved_stations.iter().position(|x| x.url == s.url) {
        app.saved_stations.remove(p);
    } else {
        app.saved_stations.push(s);
    }
    radio::save_saved(&app.saved_stations);
    let src = app.ui().map(|u| u.global::<Lib>().get_radio_source()).unwrap_or(0);
    if src == 2 {
        app.stations = app.saved_stations.clone();
    }
    publish(app);
}

/// Własna stacja z adresu.
pub fn add_url(app: &mut App, url: &str) {
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    let name = url.trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or(url).to_string();
    let s = Station { name, url: url.to_string(), ..Default::default() };
    if !app.saved_stations.iter().any(|x| x.url == s.url) {
        app.saved_stations.push(s.clone());
        radio::save_saved(&app.saved_stations);
    }
    play_station(app, s);
    if app.ui().map(|u| u.global::<Lib>().get_radio_source()).unwrap_or(0) == 2 {
        app.stations = app.saved_stations.clone();
        publish(app);
    }
}

/// Tytuł utworu ze strumienia – raz na sekundę.
pub fn icy_tick(app: &mut App) {
    let Some(c) = &app.cur else { return };
    if c.station.is_none() && !openmp_core::meta::is_url(&c.info.path) {
        return;
    }
    let Some(m) = &app.mpv else { return };
    let title = m.get_str("metadata/by-key/icy-title").or_else(|| m.get_str("media-title")).unwrap_or_default();
    if title != app.icy && !title.is_empty() && title != c.info.path {
        app.icy = title.clone();
        let name = c.station.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        if let Some(ui) = app.ui() {
            let p = ui.global::<Player>();
            if name.is_empty() {
                p.set_title(ss(&title));
            } else {
                // „Wykonawca - Tytuł” ze strumienia rozdzielamy na dwa wiersze.
                match title.split_once(" - ") {
                    Some((a, t)) => {
                        p.set_title(ss(t.trim()));
                        p.set_artist(ss(a.trim()));
                    }
                    None => {
                        p.set_title(ss(&title));
                        p.set_artist(ss(""));
                    }
                }
                p.set_album(ss(&name));
            }
        }
        crate::win::smtc_meta(app);
    }
}
