//! Wyłącznik czasowy: o czasie, po utworze, po liście – z łagodnym wyciszeniem.

use std::time::{Duration, Instant};

use crate::app::*;
use crate::*;

const FADE: f32 = 20.0;

pub fn command(app: &mut App, id: &str) {
    app.sleep = match id {
        "sleep-track" => Some(SleepMode::EndOfTrack),
        "sleep-list" => Some(SleepMode::EndOfList),
        "sleep-off" => None,
        s => s.strip_prefix("sleep-").and_then(|m| m.parse::<u64>().ok()).map(|m| SleepMode::At(Instant::now() + Duration::from_secs(m * 60))),
    };
    if app.sleep.is_none() {
        app.sleep_fade = None;
        crate::player::apply_volume(app);
    }
    crate::player::refresh_next(app);
    let msg = match &app.sleep {
        Some(SleepMode::At(_)) => tr!("Sleep timer: in {} min", id.trim_start_matches("sleep-")),
        Some(SleepMode::EndOfTrack) => tr!("Sleep timer: after the current track").into(),
        Some(SleepMode::EndOfList) => tr!("Sleep timer: when the list ends").into(),
        None => tr!("Sleep timer off").into(),
    };
    app.toast(msg);
    tick(app);
}

/// Zatrzymanie z powodu wyłącznika.
pub fn fire(app: &mut App) {
    app.sleep = None;
    crate::player::stop(app);
    app.sleep_fade = None;
    crate::player::apply_volume(app);
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_sleep_text(ss(""));
    }
    app.toast(tr!("The sleep timer stopped playback"));
}

pub fn tick(app: &mut App) {
    let text = match &app.sleep {
        Some(SleepMode::At(t)) => {
            let left = t.saturating_duration_since(Instant::now()).as_secs_f32();
            if left <= 0.0 {
                fire(app);
                return;
            }
            if left < FADE {
                app.sleep_fade = Some(left / FADE);
                crate::player::apply_volume(app);
            }
            let s = left.ceil() as u64;
            format!("{}:{:02}", s / 60, s % 60)
        }
        Some(SleepMode::EndOfTrack) => trc!("sleep timer", "track").into(),
        Some(SleepMode::EndOfList) => trc!("sleep timer", "list").into(),
        None => String::new(),
    };
    if let Some(ui) = app.ui() {
        if ui.global::<Player>().get_sleep_text().as_str() != text {
            ui.global::<Player>().set_sleep_text(ss(text));
        }
    }
}
