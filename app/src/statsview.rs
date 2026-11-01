//! Widok statystyk: czas muzyki i filmów, ostatnie dni, najczęściej odtwarzane i historia.

use std::collections::HashMap;
use std::path::Path;

use openmp_core::library::{self, now};
use openmp_core::meta::{self, TrackInfo};
use slint::{ModelRc, VecModel};

use crate::app::*;
use crate::*;

/// Czas w godzinach i minutach („12 h 05 min”, „45 min”).
fn hm(secs: f64) -> String {
    let m = (secs / 60.0).round() as u64;
    if m >= 60 {
        tr!("{} h {} min", m / 60, format!("{:02}", m % 60))
    } else {
        tr!("{} min", m)
    }
}

/// Dzień miesiąca i miesiąc z numeru dnia od 1970 (kalendarz gregoriański).
fn date(day: u32) -> (u32, u32) {
    let z = day as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (d, m)
}

fn ago(t: u64) -> String {
    let d = now().saturating_sub(t);
    match d {
        0..=59 => tr!("just now").into(),
        60..=3599 => tr!("{} min ago", d / 60),
        3600..=86_399 => tr!("{} h ago", d / 3600),
        _ => trn!("{n} day ago", "{n} days ago", d / 86_400),
    }
}

fn top(v: Vec<(String, String, f64, String)>, n: usize) -> Vec<StatTop> {
    let mut v = v;
    v.sort_by(|a, b| b.2.total_cmp(&a.2));
    v.truncate(n);
    let max = v.first().map(|x| x.2).unwrap_or(1.0).max(1e-9);
    v.into_iter().map(|(title, sub, val, label)| StatTop { title: ss(title), sub: ss(sub), value: ss(label), frac: (val / max) as f32 }).collect()
}

pub fn show(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    let st = &app.stats;
    let lib: HashMap<String, &TrackInfo> = app.lib.tracks.iter().chain(app.lib.videos.iter()).map(|t| (t.path.to_lowercase(), t)).collect();
    let name = |k: &str| -> (String, String) {
        match lib.get(k) {
            Some(t) => (t.display_title(), t.display_artist()),
            None => (Path::new(k).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), String::new()),
        }
    };
    let is_video = |k: &str| meta::is_video(Path::new(k));
    // Czas pliku: zmierzony, a dla starszych danych (sprzed pomiaru) – odtworzenia × długość.
    let spent = |k: &str, s: &library::Stat| -> f64 {
        if s.listened > 0.0 {
            s.listened
        } else {
            lib.get(k).map(|t| t.duration.max(0.0) * s.plays as f64).unwrap_or(0.0)
        }
    };

    // Podsumowanie.
    let today = library::day(now());
    // Łącznie: wg plików (z szacunkiem starszych danych), nie mniej niż wg dni.
    let music: f64 = st.map.iter().filter(|(k, _)| !is_video(k)).map(|(k, s)| spent(k, s)).sum::<f64>().max(st.days.values().map(|d| d.music).sum());
    let video: f64 = st.map.iter().filter(|(k, _)| is_video(k)).map(|(k, s)| spent(k, s)).sum::<f64>().max(st.days.values().map(|d| d.video).sum());
    let week: f64 = st.days.range(today.saturating_sub(6)..=today).map(|(_, d)| d.music + d.video).sum();
    let t = st.days.get(&today).copied().unwrap_or_default();
    let plays_m: u64 = st.map.iter().filter(|(k, _)| !is_video(k)).map(|(_, s)| s.plays as u64).sum();
    let plays_v: u64 = st.map.iter().filter(|(k, _)| is_video(k)).map(|(_, s)| s.plays as u64).sum();
    let watched = st.map.iter().filter(|(k, s)| is_video(k) && s.watched).count();
    let cards = vec![
        StatCard { label: ss(tr!("Music")), value: ss(hm(music)), sub: ss(trn!("{n} play", "{n} plays", plays_m)) },
        StatCard { label: ss(tr!("Movies")), value: ss(hm(video)), sub: ss(format!("{} · {}", trn!("{n} play", "{n} plays", plays_v), trn!("{n} watched", "{n} watched", watched))) },
        StatCard { label: ss(tr!("Today")), value: ss(hm(t.music + t.video)), sub: ss(tr!("music {} · movies {}", hm(t.music), hm(t.video))) },
        StatCard { label: ss(tr!("Last 7 days")), value: ss(hm(week)), sub: ss(tr!("{} a day on average", hm(week / 7.0))) },
    ];

    // Ostatnie 14 dni.
    let span: Vec<u32> = (0..14).map(|i| today - 13 + i).collect();
    let max = span.iter().map(|d| st.days.get(d).map(|x| x.music + x.video).unwrap_or(0.0)).fold(60.0f64, f64::max);
    let days: Vec<StatDay> = span
        .iter()
        .map(|d| {
            let x = st.days.get(d).copied().unwrap_or_default();
            let (dd, mm) = date(*d);
            StatDay {
                label: ss(format!("{dd}.{mm:02}")),
                music: (x.music / max) as f32,
                video: (x.video / max) as f32,
                tip: ss(format!("{dd}.{mm:02}: {}", tr!("music {}, movies {}", hm(x.music), hm(x.video)))),
            }
        })
        .collect();

    // Najczęściej odtwarzane.
    let tracks = top(
        st.map
            .iter()
            .filter(|(k, s)| !is_video(k) && s.plays > 0)
            .map(|(k, s)| {
                let (a, b) = name(k);
                (a, b, s.plays as f64, format!("{}×", s.plays))
            })
            .collect(),
        12,
    );
    let mut by_artist: HashMap<String, (u64, f64)> = HashMap::new();
    for (k, s) in st.map.iter().filter(|(k, s)| !is_video(k) && s.plays > 0) {
        let a = name(k).1;
        if a.is_empty() {
            continue;
        }
        let e = by_artist.entry(a).or_default();
        e.0 += s.plays as u64;
        e.1 += spent(k, s);
    }
    let artists = top(by_artist.into_iter().map(|(a, (p, l))| (a, hm(l), p as f64, format!("{p}×"))).collect(), 12);
    let videos = top(
        st.map
            .iter()
            .filter(|(k, s)| is_video(k) && (s.plays > 0 || s.listened > 0.0))
            .map(|(k, s)| {
                let (a, _) = name(k);
                let sub = format!("{}{}", if s.watched { tr!("watched · ") } else { "" }, if s.last > 0 { ago(s.last) } else { String::new() });
                let t = spent(k, s);
                (a, sub, t.max(s.plays as f64), hm(t))
            })
            .collect(),
        12,
    );

    // Historia.
    let history: Vec<StatTop> = st
        .log
        .iter()
        .rev()
        .take(30)
        .map(|(t, k)| {
            let (a, b) = name(k);
            StatTop { title: ss(a), sub: ss(b), value: ss(ago(*t)), frac: if is_video(k) { 1.0 } else { 0.0 } }
        })
        .collect();

    let l = ui.global::<Lib>();
    l.set_stat_cards(ModelRc::new(VecModel::from(cards)));
    l.set_stat_days(ModelRc::new(VecModel::from(days)));
    l.set_stat_tracks(ModelRc::new(VecModel::from(tracks)));
    l.set_stat_artists(ModelRc::new(VecModel::from(artists)));
    l.set_stat_videos(ModelRc::new(VecModel::from(videos)));
    l.set_stat_history(ModelRc::new(VecModel::from(history)));
}
