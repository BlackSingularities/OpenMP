//! Odtwarzanie: wybór silnika (muzyka / mpv), następny i poprzedni utwór, losowanie, kolejka,
//! powtarzanie, ReplayGain, wznawianie i zegar stanu.

use std::path::{Path, PathBuf};

use openmp_core::audio::engine::{self, Item};
use openmp_core::meta::{self, TrackInfo};
use openmp_core::mpv::Event as MpvEvent;
use rand::seq::SliceRandom;
use slint::ComponentHandle;

use crate::app::*;
use crate::*;

/// Wzmocnienie ReplayGain (liniowe) dla utworu; bez tagów – z własnego pomiaru R128.
pub fn gain(app: &App, t: &TrackInfo) -> f32 {
    if app.s.replaygain == 0 {
        return 1.0;
    }
    let tags = if app.s.replaygain == 2 { t.rg_album.or(t.rg_track) } else { t.rg_track.or(t.rg_album) };
    let (db, peak) = match tags {
        Some(db) => (db, t.rg_peak),
        None => match app.loudness.get(&t.path) {
            Some(l) => (l.gain, Some(l.peak)),
            None => return 1.0,
        },
    };
    let mut g = 10f32.powf((db + app.s.rg_preamp) / 20.0);
    if let Some(p) = peak.filter(|p| *p > 0.0) {
        g = g.min(1.0 / p);
    }
    g
}

/// Plik bez tagów ReplayGain: pomiar głośności w tle, po nim nowe wzmocnienie dla `item`.
fn ensure_loudness(app: &mut App, t: &TrackInfo, item: u64) {
    if app.s.replaygain == 0 || t.rg_track.is_some() || t.rg_album.is_some() || app.loudness.get(&t.path).is_some() {
        return;
    }
    if !app.measuring.insert(openmp_core::library::key(&t.path)) {
        return;
    }
    let t = t.clone();
    std::thread::spawn(move || {
        let l = openmp_core::audio::loudness::measure(Path::new(&t.path));
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                app.measuring.remove(&openmp_core::library::key(&t.path));
                let Some(l) = l else { return };
                app.loudness.insert(&t.path, l);
                app.loudness.save();
                let g = gain(app, &t);
                app.engine.set_gain(item, g);
            });
        });
    });
}

fn engine_ok(t: &TrackInfo) -> bool {
    let p = Path::new(&t.path);
    !meta::is_url(&t.path) && !meta::is_video(p) && engine::can_play(p)
}

pub fn track_at(app: &App, list: u64, idx: usize) -> Option<TrackInfo> {
    let li = app.list_index(list)?;
    app.store.lists[li].items.get(idx).cloned()
}

/// Uruchamia utwór z listy.
pub fn play(app: &mut App, list: u64, idx: usize, start: Option<f64>) {
    let Some(t) = track_at(app, list, idx) else { return };
    if let Some(c) = &app.cur {
        if let Some(l) = c.list {
            if app.history.last() != Some(&(l, c.idx)) {
                app.history.push((l, c.idx));
                if app.history.len() > 200 {
                    app.history.remove(0);
                }
            }
        }
    }
    if let Some(li) = app.list_index(list) {
        app.store.lists[li].current = Some(idx);
    }
    start_track(app, Some(list), idx, t, None, start);
}

/// Wspólny start: plik z listy, plik spoza listy albo stacja radiowa.
pub fn start_track(app: &mut App, list: Option<u64>, idx: usize, t: TrackInfo, station: Option<openmp_core::radio::Station>, start: Option<f64>) {
    save_resume(app);
    let item = app.next_seq();
    app.pending = None;
    app.ab = (None, None);
    app.icy.clear();
    let path = PathBuf::from(&t.path);
    let video = meta::is_video(&path) || t.video;
    let use_engine = station.is_none() && engine_ok(&t);
    let resume = start.or_else(|| {
        let st = app.stats.get(&t.path);
        let long = video || t.duration > 20.0 * 60.0;
        (long && app.s.video_resume && st.resume > 10.0).then_some(st.resume)
    });
    if use_engine {
        if let Some(m) = &app.mpv {
            m.command_async(&["stop"]);
        }
        app.engine.set_loopback(false);
        crate::video::leave_video(app);
        app.engine.load(Item { id: item, path: path.clone(), gain: gain(app, &t) }, resume.unwrap_or(0.0));
        ensure_loudness(app, &t, item);
        app.engine.set_paused(app.pause_after_load);
        app.cur = Some(Playing { list, idx, item, info: t.clone(), backend: Backend::Engine, video: false, station: None });
        crate::video::clear(app);
        refresh_next(app);
    } else {
        let Some(m) = app.mpv.clone() else {
            app.toast(format!("Brak silnika wideo (libmpv) – nie można odtworzyć: {}", t.display_title()));
            return;
        };
        app.engine.stop();
        m.set("vid", if video { "auto" } else { "no" });
        m.set("start", &resume.filter(|r| *r > 0.0).map(|r| format!("{r:.2}")).unwrap_or_else(|| "none".into()));
        m.set_flag("pause", app.pause_after_load);
        if video {
            // Każdy film zaczyna się bez powiększenia, przesunięcia i obrotu poprzedniego.
            m.set_f64("video-zoom", 0.0);
            m.set_f64("video-pan-x", 0.0);
            m.set_f64("video-pan-y", 0.0);
            m.set("video-rotate", "0");
        }
        crate::video::apply_audio_filters(app);
        m.command_async(&["loadfile", &t.path, "replace"]);
        app.cur = Some(Playing { list, idx, item, info: t.clone(), backend: Backend::Mpv, video, station: station.clone() });
        app.video_size = (0, 0);
        crate::video::clear(app);
        if let Some(ui) = app.ui() {
            let v = ui.global::<Video>();
            v.set_zoom(0.0);
            v.set_rotate(0);
        }
        if video {
            // Film otwiera widok filmu albo gra w miejscu okładki – okno muzyki zostaje.
            if app.s.video_open_view || app.is_video_mode() {
                crate::video::enter_video(app);
            }
        } else {
            crate::video::leave_video(app);
        }
        // Wizualizacja dźwięku z mpv – przez pętlę zwrotną WASAPI.
        app.engine.set_loopback(!video);
        if let Some(ui) = app.ui() {
            ui.global::<Video>().set_loading(true);
            ui.global::<Video>().set_error(ss(""));
            ui.global::<Video>().set_audio_only(false);
        }
        crate::video::refresh_active(app);
    }
    app.pause_after_load = false;
    app.stats.played(&t.path);
    if !meta::is_url(&t.path) {
        app.stats.touch_recent(&t.path);
    }
    app.dirty = true;
    crate::display::show_track(app);
    crate::lists::refresh_playing(app);
    if app.s.follow_playing {
        crate::lists::scroll_to_current(app);
    }
}

/// Następny do zagrania (`auto` – koniec utworu; ręczne „dalej” pomija powtarzanie utworu).
/// `take` – zdejmuje pozycję z kolejki.
pub fn next_of(app: &mut App, auto: bool, take: bool) -> Option<(u64, usize, bool)> {
    if let Some(&(l, i)) = app.queue.first() {
        if take {
            app.queue.remove(0);
        }
        return Some((l, i, true));
    }
    let c = app.cur.as_ref()?;
    let list = c.list?;
    let idx = c.idx;
    let li = app.list_index(list)?;
    let len = app.store.lists[li].items.len();
    if len == 0 {
        return None;
    }
    if auto && app.s.repeat == 2 {
        return Some((list, idx, false));
    }
    if app.s.shuffle {
        ensure_shuffle(app, list, len);
        let pos = app.shuffle.iter().position(|&i| i == idx).unwrap_or(0);
        if pos + 1 < app.shuffle.len() {
            return Some((list, app.shuffle[pos + 1], false));
        }
        if app.s.repeat == 1 || !auto {
            if take {
                // Nowe przetasowanie na kolejną rundę.
                app.shuffle_len = 0;
                ensure_shuffle(app, list, len);
            }
            return Some((list, app.shuffle[0], false));
        }
        return None;
    }
    if idx + 1 < len {
        Some((list, idx + 1, false))
    } else if app.s.repeat == 1 || !auto {
        Some((list, 0, false))
    } else {
        None
    }
}

fn ensure_shuffle(app: &mut App, list: u64, len: usize) {
    if app.shuffle_list == list && app.shuffle_len == len && app.shuffle.len() == len {
        return;
    }
    let mut v: Vec<usize> = (0..len).collect();
    v.shuffle(&mut rand::rng());
    // Bieżący utwór na początek – runda zaczyna się od niego.
    if let Some(c) = &app.cur {
        if c.list == Some(list) {
            if let Some(p) = v.iter().position(|&i| i == c.idx) {
                v.swap(0, p);
            }
        }
    }
    app.shuffle = v;
    app.shuffle_list = list;
    app.shuffle_len = len;
}

/// Zapowiada silnikowi następny utwór (przejście bez przerwy / przenikanie).
pub fn refresh_next(app: &mut App) {
    let Some(c) = app.cur.clone() else { return };
    if c.backend != Backend::Engine || app.sleep_blocks_next() {
        app.engine.set_next(None);
        app.pending = None;
        return;
    }
    match next_of(app, true, false) {
        Some((l, i, q)) => match track_at(app, l, i) {
            Some(t) if engine_ok(&t) => {
                let same = app.pending.as_ref().is_some_and(|p| p.list == l && p.idx == i);
                if !same {
                    let item = app.next_seq();
                    let g = gain(app, &t);
                    app.engine.set_next(Some(Item { id: item, path: PathBuf::from(&t.path), gain: g }));
                    ensure_loudness(app, &t, item);
                    app.pending = Some(Pending { item, list: l, idx: i, from_queue: q });
                }
            }
            _ => {
                app.engine.set_next(None);
                app.pending = None;
            }
        },
        None => {
            app.engine.set_next(None);
            app.pending = None;
        }
    }
}

impl App {
    /// Wyłącznik „po bieżącym” i „zatrzymaj po bieżącym” – bez zapowiadania następnego.
    pub fn sleep_blocks_next(&self) -> bool {
        matches!(self.sleep, Some(SleepMode::EndOfTrack)) || self.ui().map(|u| u.global::<Player>().get_stop_after()).unwrap_or(false)
    }
}

pub fn next(app: &mut App) {
    if app.cur.as_ref().is_some_and(|c| c.station.is_some()) {
        crate::radio::step(app, 1);
        return;
    }
    if let Some((l, i, _)) = next_of(app, false, true) {
        play(app, l, i, None);
    }
}

pub fn prev(app: &mut App) {
    if app.cur.as_ref().is_some_and(|c| c.station.is_some()) {
        crate::radio::step(app, -1);
        return;
    }
    if position(app).0 > 4.0 {
        seek_abs(app, 0.0);
        return;
    }
    let Some(c) = app.cur.clone() else { return };
    let Some(list) = c.list else { return };
    if app.s.shuffle {
        if let Some((l, i)) = app.history.pop() {
            start_track_from_history(app, l, i);
            return;
        }
    }
    let len = app.list_index(list).map(|li| app.store.lists[li].items.len()).unwrap_or(0);
    if len == 0 {
        return;
    }
    let i = if c.idx == 0 { if app.s.repeat == 1 { len - 1 } else { 0 } } else { c.idx - 1 };
    play(app, list, i, None);
}

fn start_track_from_history(app: &mut App, l: u64, i: usize) {
    if let Some(t) = track_at(app, l, i) {
        start_track(app, Some(l), i, t, None, None);
    }
}

/// Koniec utworu albo strumienia.
pub fn on_end(app: &mut App) {
    let stop_after = app.ui().map(|u| u.global::<Player>().get_stop_after()).unwrap_or(false);
    if stop_after {
        if let Some(ui) = app.ui() {
            ui.global::<Player>().set_stop_after(false);
        }
        stop(app);
        return;
    }
    if matches!(app.sleep, Some(SleepMode::EndOfTrack)) {
        crate::sleep::fire(app);
        return;
    }
    if app.cur.as_ref().is_some_and(|c| c.station.is_some()) {
        // Strumień się urwał – próbujemy ponownie.
        if let Some(c) = app.cur.clone() {
            if let Some(st) = c.station {
                crate::radio::play_station(app, st);
            }
        }
        return;
    }
    if app.cur.as_ref().is_some_and(|c| c.video) && app.s.repeat != 2 {
        if let Some(c) = &app.cur {
            app.stats.entry(&c.info.path).resume = 0.0;
        }
        if !app.s.video_autonext {
            stop(app);
            return;
        }
    }
    match next_of(app, true, true) {
        Some((l, i, _)) => play(app, l, i, None),
        None => {
            if matches!(app.sleep, Some(SleepMode::EndOfList)) {
                crate::sleep::fire(app);
                return;
            }
            stop(app);
        }
    }
}

pub fn stop(app: &mut App) {
    save_resume(app);
    app.engine.stop();
    app.engine.set_loopback(false);
    if let Some(m) = &app.mpv {
        m.command_async(&["stop"]);
    }
    app.pending = None;
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        p.set_playing(false);
        p.set_pos(0.0);
    }
    if let Some(c) = &mut app.cur {
        // Zatrzymany utwór zostaje na wyświetlaczu – Play zaczyna go od początku.
        c.item = 0;
    }
    crate::video::leave_video(app);
    crate::video::refresh_active(app);
    crate::win::smtc_state(app);
    crate::win::keep_awake(app, false);
}

pub fn play_pause(app: &mut App) {
    let Some(c) = app.cur.clone() else {
        // Nic nie gra – zaczynamy od zaznaczonego albo pierwszego utworu aktywnej listy.
        let id = app.active_id();
        let idx = app.sel.iter().min().copied().unwrap_or(0);
        play(app, id, idx, None);
        return;
    };
    if c.item == 0 {
        match c.list {
            Some(l) => play(app, l, c.idx, None),
            None => {
                if let Some(st) = c.station {
                    crate::radio::play_station(app, st);
                } else {
                    start_track(app, None, 0, c.info, None, None);
                }
            }
        }
        return;
    }
    let paused = !is_playing(app);
    set_paused(app, !paused);
}

pub fn set_paused(app: &mut App, p: bool) {
    match app.cur.as_ref().map(|c| c.backend) {
        Some(Backend::Engine) => app.engine.set_paused(p),
        Some(Backend::Mpv) => {
            if let Some(m) = &app.mpv {
                m.set_flag("pause", p);
            }
        }
        None => {}
    }
    if p {
        save_resume(app);
    }
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_playing(!p);
        ui.global::<Video>().set_playing(!p);
    }
    crate::win::smtc_state(app);
}

pub fn is_playing(app: &App) -> bool {
    match app.cur.as_ref() {
        Some(c) if c.item != 0 => match c.backend {
            Backend::Engine => !app.engine.paused(),
            Backend::Mpv => app.mpv.as_ref().and_then(|m| m.get_flag("pause")).map(|p| !p).unwrap_or(false),
        },
        _ => false,
    }
}

/// (pozycja, długość) w sekundach.
pub fn position(app: &App) -> (f64, f64) {
    match app.cur.as_ref().map(|c| c.backend) {
        Some(Backend::Engine) => {
            let s = app.engine.status();
            let dur = if s.info.duration > 0.0 { s.info.duration } else { app.cur.as_ref().map(|c| c.info.duration).unwrap_or(0.0) };
            (s.pos, dur)
        }
        Some(Backend::Mpv) => match &app.mpv {
            Some(m) => (m.get_f64("time-pos").unwrap_or(0.0), m.get_f64("duration").unwrap_or(0.0)),
            None => (0.0, 0.0),
        },
        None => (0.0, 0.0),
    }
}

pub fn seek_abs(app: &mut App, t: f64) {
    match app.cur.as_ref().map(|c| c.backend) {
        Some(Backend::Engine) => {
            app.engine.seek(t);
            // Po przewinięciu dalej niż przenikanie – następny trzeba zapowiedzieć od nowa.
            app.pending = None;
            refresh_next(app);
        }
        Some(Backend::Mpv) => {
            if let Some(m) = &app.mpv {
                m.command_async(&["seek", &format!("{t:.3}"), "absolute+exact"]);
            }
        }
        None => {}
    }
}

pub fn seek_rel(app: &mut App, d: f64) {
    let (pos, dur) = position(app);
    let t = if dur > 0.0 { (pos + d).clamp(0.0, (dur - 0.5).max(0.0)) } else { (pos + d).max(0.0) };
    if app.cur.as_ref().is_some_and(|c| c.backend == Backend::Mpv) {
        if let Some(m) = &app.mpv {
            m.command_async(&["seek", &format!("{d:.3}"), "relative+exact"]);
        }
    } else {
        seek_abs(app, t);
    }
    if app.is_video_mode() {
        let sign = if d >= 0.0 { "+" } else { "−" };
        app.osd(format!("{sign}{} s  ·  {}", d.abs().round(), meta::fmt_time(t)));
    }
}

pub fn set_volume(app: &mut App, v: f32) {
    app.s.volume = v.clamp(0.0, 1.0);
    if app.s.muted && v > 0.0 {
        app.s.muted = false;
    }
    apply_volume(app);
    if app.is_video_mode() {
        app.osd(format!("Głośność {}%", (app.s.volume * 100.0).round()));
    }
}

pub fn apply_volume(app: &mut App) {
    let v = if app.s.muted { 0.0 } else { app.s.volume * app.sleep_fade.unwrap_or(1.0) };
    app.engine.set_volume(v);
    if let Some(m) = &app.mpv {
        // mpv liczy amplitudę jako (głośność/100)³, silnik muzyki jako v^1,5 – ta sama słyszalna głośność.
        // Filmy: pełna głośność może sięgać wzmocnienia z ustawień (do 200%).
        let top = if app.cur.as_ref().is_some_and(|c| c.video) { app.s.volume_boost.clamp(100.0, 200.0) as f64 } else { 100.0 };
        m.set_f64("volume", ((v as f64).sqrt() * top).clamp(0.0, top));
    }
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_volume(app.s.volume);
        ui.global::<Player>().set_muted(app.s.muted);
    }
    app.dirty = true;
}

pub fn toggle_mute(app: &mut App) {
    app.s.muted = !app.s.muted;
    apply_volume(app);
    if app.is_video_mode() {
        app.osd(if app.s.muted { "Wyciszono" } else { "Dźwięk włączony" });
    }
}

/// Zapamiętuje miejsce w filmie / długim nagraniu.
pub fn save_resume(app: &mut App) {
    let Some(c) = app.cur.clone() else { return };
    if c.item == 0 || c.station.is_some() {
        return;
    }
    let (pos, dur) = position(app);
    let long = c.video || dur > 20.0 * 60.0;
    if !long {
        return;
    }
    let st = app.stats.entry(&c.info.path);
    st.resume = if pos > 10.0 && (dur <= 0.0 || pos < dur - 15.0) { pos } else { 0.0 };
    app.dirty = true;
}

/// Zegar stanu (co klatkę): pozycja, zmiany utworu, końce, zdarzenia mpv.
pub fn tick(app: &mut App) {
    // Silnik muzyki
    while let Ok(ev) = app.engine.events.try_recv() {
        match ev {
            engine::Event::Error { id, path, msg } => {
                let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if app.cur.as_ref().is_some_and(|c| c.item == id) {
                    // Silnik nie umie – próbujemy przez mpv.
                    if let Some(c) = app.cur.clone() {
                        if app.mpv.is_some() {
                            let t = c.info.clone();
                            if let Some(m) = &app.mpv {
                                app.engine.stop();
                                m.set("vid", "no");
                                m.set("start", "none");
                                m.command_async(&["loadfile", &t.path, "replace"]);
                                m.set_flag("pause", false);
                                app.engine.set_loopback(true);
                            }
                            if let Some(cc) = &mut app.cur {
                                cc.backend = Backend::Mpv;
                            }
                            continue;
                        }
                    }
                    app.toast(format!("Nie można odtworzyć: {name} ({msg})"));
                    on_end(app);
                } else if app.pending.as_ref().is_some_and(|p| p.item == id) {
                    app.pending = None;
                }
            }
            engine::Event::Device(msg) => app.toast(msg),
        }
    }
    if let Some(c) = app.cur.clone() {
        if c.backend == Backend::Engine && c.item != 0 {
            let s = app.engine.status();
            if let Some(id) = s.id {
                if id != c.item {
                    if let Some(p) = app.pending.clone().filter(|p| p.item == id) {
                        // Silnik sam przeszedł do następnego (bez przerwy / przenikanie).
                        if p.from_queue && !app.queue.is_empty() {
                            app.queue.remove(0);
                        }
                        app.history.push((c.list.unwrap_or(0), c.idx));
                        if let Some(t) = track_at(app, p.list, p.idx) {
                            if let Some(li) = app.list_index(p.list) {
                                app.store.lists[li].current = Some(p.idx);
                            }
                            app.cur = Some(Playing { list: Some(p.list), idx: p.idx, item: id, info: t.clone(), backend: Backend::Engine, video: false, station: None });
                            app.pending = None;
                            app.stats.played(&t.path);
                            app.stats.touch_recent(&t.path);
                            app.dirty = true;
                            crate::display::show_track(app);
                            crate::lists::refresh_playing(app);
                            if app.s.follow_playing {
                                crate::lists::scroll_to_current(app);
                            }
                            refresh_next(app);
                        }
                    }
                }
            }
            if s.ended {
                on_end(app);
            }
            crate::display::update_badges_from(app, &s.info);
        }
    }
    // mpv
    if let Some(m) = app.mpv.clone() {
        for ev in m.poll() {
            match ev {
                MpvEvent::FileLoaded => {
                    m.set("start", "none");
                    crate::video::on_loaded(app);
                }
                MpvEvent::EndFile { eof, error } => {
                    if app.cur.as_ref().is_some_and(|c| c.backend == Backend::Mpv && c.item != 0) {
                        if error {
                            let name = app.cur.as_ref().map(|c| c.info.display_title()).unwrap_or_default();
                            if app.is_video_mode() {
                                if let Some(ui) = app.ui() {
                                    ui.global::<Video>().set_loading(false);
                                    ui.global::<Video>().set_error(ss(format!("Nie można odtworzyć: {name}")));
                                }
                            } else {
                                app.toast(format!("Nie można odtworzyć: {name}"));
                            }
                        }
                        if eof || error {
                            on_end(app);
                        }
                    }
                }
                MpvEvent::VideoReconfig => crate::video::on_reconfig(app),
                MpvEvent::PlaybackRestart => {
                    if let Some(ui) = app.ui() {
                        ui.global::<Video>().set_loading(false);
                    }
                }
                _ => {}
            }
        }
    }
    // Stan na ekranie
    let (pos, dur) = position(app);
    let playing = is_playing(app);
    if let Some(ui) = app.ui() {
        let p = ui.global::<Player>();
        if (pos - app.last_pos).abs() > 0.01 || (dur - app.last_dur).abs() > 0.01 {
            p.set_pos(pos as f32);
            p.set_duration(dur as f32);
            let v = ui.global::<Video>();
            v.set_pos(pos as f32);
            v.set_duration(dur as f32);
            app.last_pos = pos;
            app.last_dur = dur;
        }
        if p.get_playing() != playing {
            p.set_playing(playing);
            ui.global::<Video>().set_playing(playing);
            crate::win::smtc_state(app);
            let video_playing = playing && app.cur.as_ref().is_some_and(|c| c.video);
            crate::win::keep_awake(app, video_playing);
        }
    }
    // Pętla A-B
    if let (Some(a), Some(b)) = app.ab {
        if pos >= b && b > a {
            seek_abs(app, a);
        }
    }
}

/// Dodaje pliki (w tle czyta tagi), opcjonalnie gra pierwszy dodany.
pub fn open_paths(app: &mut App, paths: Vec<String>, play_first: bool, new_list: Option<String>) {
    let list_id = match new_list {
        Some(name) => crate::lists::create_list(app, &name),
        None => app.active_id(),
    };
    let weak = app.ui.clone();
    let count = paths.len();
    if count > 3 {
        app.toast(format!("Dodawanie: {count} pozycji…"));
    }
    std::thread::spawn(move || {
        let mut out: Vec<TrackInfo> = Vec::new();
        let add = |p: &Path, out: &mut Vec<TrackInfo>| {
            let ext = meta::ext(p);
            if meta::PLAYLISTS.contains(&ext.as_str()) {
                for (e, title, dur) in openmp_core::playlist::import(p) {
                    let mut t = if meta::is_url(&e) { TrackInfo { path: e.clone(), ..Default::default() } } else { meta::read(Path::new(&e)) };
                    if t.title.is_empty() {
                        if let Some(tt) = title {
                            t.title = tt;
                        }
                    }
                    if t.duration <= 0.0 {
                        t.duration = dur;
                    }
                    out.push(t);
                }
            } else if meta::is_media(p) {
                out.push(meta::read(p));
            }
        };
        for p in &paths {
            if meta::is_url(p) {
                out.push(TrackInfo { path: p.clone(), ..Default::default() });
                continue;
            }
            let pb = PathBuf::from(p);
            if pb.is_dir() {
                let mut files = openmp_core::library::collect(&[p.clone()], false);
                files.sort_by_key(|f| f.to_string_lossy().to_lowercase());
                let mut tracks: Vec<TrackInfo> = files.iter().map(|f| meta::read(f)).collect();
                tracks.sort_by(openmp_core::library::album_order);
                out.extend(tracks);
            } else {
                add(&pb, &mut out);
            }
        }
        let _ = slint::invoke_from_event_loop(move || {
            let _ = weak;
            later(move |app| {
                if out.is_empty() {
                    app.toast("Brak plików multimedialnych do dodania");
                    return;
                }
                let Some(li) = app.list_index(list_id) else { return };
                let start = app.store.lists[li].items.len();
                let n = out.len();
                app.store.lists[li].items.extend(out);
                app.dirty = true;
                if app.store.active != li {
                    app.store.active = li;
                }
                crate::lists::rebuild(app);
                crate::libview::rebuild_nav(app);
                refresh_next(app);
                if play_first {
                    play(app, list_id, start, None);
                } else if n > 3 {
                    app.toast(format!("Dodano {n} pozycji"));
                }
            });
        });
    });
}

/// Pojedynczy adres z okna „Otwórz adres”.
pub fn open_url(app: &mut App, url: &str) {
    let url = url.trim().to_string();
    if url.is_empty() {
        return;
    }
    open_paths(app, vec![url], true, None);
}

/// ListenBrainz: wysłuchany utwór (połowa albo 4 minuty) – raz na każde odtworzenie.
pub fn scrobble_tick(app: &mut App) {
    if app.s.listenbrainz_token.trim().is_empty() {
        return;
    }
    let Some(c) = &app.cur else { return };
    if c.video || c.station.is_some() || c.item == 0 || c.item == app.scrobbled || !matches!(c.backend, Backend::Engine) {
        return;
    }
    let s = app.engine.status();
    if s.id != Some(c.item) || s.paused || !openmp_core::listenbrainz::due(s.pos, c.info.duration.max(s.info.duration)) {
        return;
    }
    let at = openmp_core::library::now().saturating_sub(s.pos as u64);
    openmp_core::listenbrainz::submit(&app.s.listenbrainz_token, &c.info, at);
    app.scrobbled = c.item;
}
