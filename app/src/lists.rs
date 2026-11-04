//! Listy odtwarzania na ekranie: karty, wiersze z nagłówkami albumów, zaznaczanie, menu
//! kontekstowe, sortowanie, usuwanie, kolejka.

use std::collections::HashMap;
use std::path::Path;

use openmp_core::meta::{self, TrackInfo};
use openmp_core::playlist::Playlist;
use rand::seq::SliceRandom;
use slint::{Image, Model};

use crate::app::*;
use crate::*;

pub const ROW_H: f32 = 30.0;
pub const HEAD_H: f32 = 54.0;

fn album_key(t: &TrackInfo) -> String {
    if t.album.trim().is_empty() || t.video {
        return String::new();
    }
    let who = if t.album_artist.is_empty() { &t.artist } else { &t.album_artist };
    format!("{}\u{1}{}", t.album.to_lowercase(), who.to_lowercase())
}

/// Gwiazdki oceny, np. „★★★☆☆”.
pub fn stars(n: u8) -> String {
    let n = n.min(5) as usize;
    format!("{}{}", "★".repeat(n), "☆".repeat(5 - n))
}

/// Ocena przed formatem w kolumnie informacji (5 gwiazdek pokazuje już serce).
pub fn with_rating(r: &mut TrackRow, rating: u8) {
    if (1..5).contains(&rating) {
        r.info = ss(format!("{}  {}", stars(rating), r.info));
    }
}

/// Odcinek serialu z nazwy pliku: „S01E03”, „s1e3”, „1x03”.
pub fn episode(name: &str) -> Option<(u32, u32)> {
    let b = name.to_ascii_lowercase().into_bytes();
    let num = |i: usize| -> (u32, usize) {
        let mut j = i;
        let mut v = 0u32;
        while j < b.len() && b[j].is_ascii_digit() && j - i < 3 {
            v = v * 10 + (b[j] - b'0') as u32;
            j += 1;
        }
        (v, j)
    };
    for i in 0..b.len() {
        let start = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        if start && b[i] == b's' {
            let (s, j) = num(i + 1);
            if j > i + 1 && j < b.len() && b[j] == b'e' {
                let (e, k) = num(j + 1);
                if k > j + 1 {
                    return Some((s, e));
                }
            }
        }
        if start && b[i].is_ascii_digit() {
            let (s, j) = num(i);
            if j < b.len() && b[j] == b'x' && j - i <= 2 {
                let (e, k) = num(j + 1);
                if k >= j + 3 {
                    return Some((s, e));
                }
            }
        }
    }
    None
}

/// Postęp filmu na kafelku: obejrzana część i znacznik „obejrzany”.
pub fn with_progress(r: &mut TrackRow, t: &TrackInfo, st: &openmp_core::library::Stat) {
    if !r.video {
        return;
    }
    // Serial: sezon i odcinek pod tytułem.
    if let Some((s, e)) = Path::new(&t.path).file_stem().and_then(|n| episode(&n.to_string_lossy())) {
        r.artist = ss(tr!("Season {} · Episode {}", s, e));
    }
    r.watched = st.watched;
    if st.resume > 0.0 && t.duration > 0.0 {
        r.progress = (st.resume / t.duration).clamp(0.0, 1.0) as f32;
    }
}

pub fn track_row(t: &TrackInfo, idx: usize, num: String, playing: bool, selected: bool, fav: bool, queued: String) -> TrackRow {
    let mut info = t.format();
    if t.bitrate > 0 {
        info = format!("{info} · {}", t.bitrate);
    }
    TrackRow {
        header: false,
        idx: idx as i32,
        num: ss(num),
        title: ss(t.display_title()),
        artist: ss(t.display_artist()),
        album: ss(&t.album),
        dur: ss(if t.duration > 0.0 { meta::fmt_time(t.duration) } else { String::new() }),
        info: ss(info),
        playing,
        selected,
        fav,
        queued: ss(queued),
        in_queue: false,
        missing: false,
        progress: -1.0,
        watched: false,
        plays: 0,
        video: t.video || meta::is_video(Path::new(&t.path)),
        h_title: ss(""),
        h_sub: ss(""),
        h_cover: Image::default(),
        h_has_cover: false,
        cols: Default::default(),
    }
}

/// Jak `queue_paths`, ale na samej karcie „Kolejka” pusto – tam znacznik nic nie mówi.
fn active_queue_paths(app: &App) -> std::collections::HashSet<String> {
    if app.store.lists.get(app.store.active).is_some_and(|l| l.name == QUEUE_NAME) {
        return Default::default();
    }
    queue_paths(app)
}

/// Ścieżki (małymi literami) z karty „Kolejka” – do znacznika na innych listach i w bibliotece.
pub fn queue_paths(app: &App) -> std::collections::HashSet<String> {
    app.store.lists.iter().find(|l| l.name == QUEUE_NAME).map(|l| l.items.iter().map(|t| t.path.to_lowercase()).collect()).unwrap_or_default()
}

fn queued_label(app: &App, list: u64, idx: usize) -> String {
    app.queue.iter().position(|&(l, i)| l == list && i == idx).map(|p| (p + 1).to_string()).unwrap_or_default()
}

fn is_playing_item(app: &App, list: u64, idx: usize) -> bool {
    app.cur.as_ref().is_some_and(|c| c.list == Some(list) && c.idx == idx && c.item != 0)
}

/// Pełne przebudowanie wierszy aktywnej listy.
pub fn rebuild(app: &mut App) {
    let Some(ui) = app.ui() else { return };
    rebuild_tabs(app);
    let li = app.store.active.min(app.store.lists.len().saturating_sub(1));
    let list = app.store.lists[li].clone();
    let words = openmp_core::library::words(&app.filter);
    let grid = app.s.track_grid;
    let grouped = app.s.group_albums && words.is_empty() && !grid;
    let mut rows: Vec<TrackRow> = Vec::with_capacity(list.items.len() + 16);
    app.row_of.clear();
    let mut last_key: Option<String> = None;
    let qpaths = active_queue_paths(app);
    let mut i = 0;
    while i < list.items.len() {
        let t = &list.items[i];
        if !openmp_core::library::matches(t, &words) {
            i += 1;
            continue;
        }
        if grouped {
            let k = album_key(t);
            if !k.is_empty() && last_key.as_deref() != Some(&k) {
                // Grupa: kolejne utwory tego samego albumu.
                let mut n = 0;
                let mut dur = 0.0;
                let mut j = i;
                while j < list.items.len() && album_key(&list.items[j]) == k {
                    n += 1;
                    dur += list.items[j].duration;
                    j += 1;
                }
                if n >= 2 {
                    let who = if t.album_artist.is_empty() { t.display_artist() } else { t.album_artist.clone() };
                    let mut sub = vec![who];
                    if t.year > 0 {
                        sub.push(t.year.to_string());
                    }
                    if !t.genre.is_empty() {
                        sub.push(t.genre.clone());
                    }
                    sub.push(trn!("{n} track", "{n} tracks", n));
                    sub.push(meta::fmt_total(dur));
                    let cover = crate::covers::get(&t.path);
                    rows.push(TrackRow {
                        header: true,
                        idx: i as i32,
                        h_title: ss(&t.album),
                        h_sub: ss(sub.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("  ·  ")),
                        h_has_cover: cover.is_some(),
                        h_cover: cover.unwrap_or_default(),
                        ..Default::default()
                    });
                }
                last_key = Some(k);
            } else if k.is_empty() {
                last_key = None;
            }
        }
        let num = if grouped && t.track > 0 && last_key.is_some() { format!("{:02}", t.track) } else { (i + 1).to_string() };
        let st = app.stats.get(&t.path);
        let rating = st.rating;
        app.row_of.insert(i, rows.len());
        let mut r = track_row(t, i, num, is_playing_item(app, list.id, i), app.sel.contains(&i), rating >= 5, queued_label(app, list.id, i));
        r.in_queue = qpaths.contains(&t.path.to_lowercase());
        r.cols = crate::columns::values(t, &st, &r.artist, &r.info);
        r.plays = st.plays as i32;
        with_rating(&mut r, rating);
        with_progress(&mut r, t, &st);
        if grid {
            if let Some(img) = crate::covers::get(&t.path) {
                r.h_cover = img;
                r.h_has_cover = true;
            }
        }
        rows.push(r);
        i += 1;
    }
    app.m.rows.set_vec(rows);
    let total = list.total();
    let mut summary = trn!("{n} item", "{n} items", list.items.len());
    if total > 0.0 {
        summary.push_str(&format!("  ·  {}", meta::fmt_total(total)));
    }
    if !app.sel.is_empty() {
        let sd: f64 = app.sel.iter().filter_map(|&i| list.items.get(i)).map(|t| t.duration).sum();
        summary.push_str(&format!("  ·  {}", tr!("selected {} ({})", app.sel.len(), meta::fmt_total(sd))));
    }
    if !words.is_empty() {
        summary.push_str(&format!("  ·  {}", tr!("matching {}", app.row_of.len())));
    }
    ui.global::<Lists>().set_summary(ss(summary));
    ui.global::<Lists>().set_grouped(grouped);
    update_queue_panel(app);
}

pub fn rebuild_tabs(app: &mut App) {
    let tabs: Vec<TabItem> = app
        .store
        .lists
        .iter()
        .map(|l| TabItem { label: ss(display_name(&l.name)), count: ss(if l.items.is_empty() { String::new() } else { l.items.len().to_string() }) })
        .collect();
    app.m.tabs.set_vec(tabs);
    if let Some(ui) = app.ui() {
        ui.global::<Lists>().set_active(app.store.active as i32);
    }
}

/// Odświeża wiersze, które zmieniły stan „gra” (bez przebudowy całej listy).
pub fn refresh_playing(app: &mut App) {
    let id = app.active_id();
    let qpaths = active_queue_paths(app);
    let li = app.store.active;
    for r in 0..app.m.rows.row_count() {
        let Some(mut row) = app.m.rows.row_data(r) else { continue };
        if row.header {
            continue;
        }
        let p = is_playing_item(app, id, row.idx as usize);
        let q = queued_label(app, id, row.idx as usize);
        let iq = app.store.lists.get(li).and_then(|l| l.items.get(row.idx as usize)).is_some_and(|t| qpaths.contains(&t.path.to_lowercase()));
        if row.playing != p || row.queued.as_str() != q || row.in_queue != iq {
            row.playing = p;
            row.queued = ss(q);
            row.in_queue = iq;
            app.m.rows.set_row_data(r, row);
        }
    }
    crate::libview::refresh_playing(app);
}

fn refresh_selection(app: &mut App) {
    for r in 0..app.m.rows.row_count() {
        let Some(mut row) = app.m.rows.row_data(r) else { continue };
        if row.header {
            continue;
        }
        let s = app.sel.contains(&(row.idx as usize));
        if row.selected != s {
            row.selected = s;
            app.m.rows.set_row_data(r, row);
        }
    }
}

pub fn refresh_covers(app: &mut App, keys: &std::collections::HashSet<String>) {
    for r in 0..app.m.rows.row_count() {
        let Some(mut row) = app.m.rows.row_data(r) else { continue };
        if row.h_has_cover || (!row.header && !app.s.track_grid) {
            continue;
        }
        let li = app.store.active;
        let Some(t) = app.store.lists.get(li).and_then(|l| l.items.get(row.idx as usize)) else { continue };
        if keys.contains(&crate::covers::key(&t.path)) {
            if let Some(img) = crate::covers::get(&t.path) {
                row.h_cover = img;
                row.h_has_cover = true;
                app.m.rows.set_row_data(r, row);
            }
        }
    }
}

/// Przewija listę do bieżącego utworu (gdy jest na aktywnej karcie).
pub fn scroll_to_current(app: &mut App) {
    let Some(c) = app.cur.clone() else { return };
    if c.list != Some(app.active_id()) {
        return;
    }
    let Some(&row) = app.row_of.get(&c.idx) else { return };
    let mut y = 0.0;
    for r in 0..row {
        y += if app.m.rows.row_data(r).is_some_and(|x| x.header) { HEAD_H } else { ROW_H };
    }
    if let Some(ui) = app.ui() {
        let l = ui.global::<Lists>();
        l.set_scroll_y((y - 120.0).max(0.0));
        l.set_scroll_gen(l.get_scroll_gen() + 1);
    }
}

pub fn select_click(app: &mut App, idx: usize, ctrl: bool, shift: bool) {
    if shift {
        let a = app.anchor.unwrap_or(idx);
        let (lo, hi) = (a.min(idx), a.max(idx));
        if !ctrl {
            app.sel.clear();
        }
        // Zakres w kolejności wyświetlania (z filtrem – tylko widoczne).
        for (&i, _) in app.row_of.iter() {
            if i >= lo && i <= hi {
                app.sel.insert(i);
            }
        }
    } else if ctrl {
        if !app.sel.remove(&idx) {
            app.sel.insert(idx);
        }
        app.anchor = Some(idx);
    } else {
        app.sel.clear();
        app.sel.insert(idx);
        app.anchor = Some(idx);
    }
    refresh_selection(app);
    update_summary_only(app);
}

/// Pozycje siatki okładek w prostokącie: wiersze r0..r1 i kolumny c0..c1 przy `cols` kolumnach.
pub fn band_items(rows: &slint::VecModel<TrackRow>, (r0, r1, c0, c1, cols): (i32, i32, i32, i32, i32)) -> Vec<usize> {
    use slint::Model;
    let cols = cols.max(1) as usize;
    (0..rows.row_count())
        .filter(|&i| {
            let (r, c) = ((i / cols) as i32, (i % cols) as i32);
            r >= r0 && r <= r1 && c >= c0 && c <= c1
        })
        .filter_map(|i| rows.row_data(i).filter(|r| !r.header).map(|r| r.idx as usize))
        .collect()
}

/// Zaznaczenie prostokątem w siatce (z Ctrl – dołożone do kotwicy zamiast od nowa).
pub fn select_band(app: &mut App, band: (i32, i32, i32, i32, i32), ctrl: bool) {
    let items = band_items(&app.m.rows, band);
    app.sel.clear();
    if ctrl {
        if let Some(a) = app.anchor {
            app.sel.insert(a);
        }
    }
    app.sel.extend(items);
    refresh_selection(app);
    update_summary_only(app);
}

fn update_summary_only(app: &mut App) {
    // Podsumowanie zależy od zaznaczenia – przebudowa jest tania w porównaniu z resztą.
    let Some(ui) = app.ui() else { return };
    let li = app.store.active;
    let Some(list) = app.store.lists.get(li) else { return };
    let mut s = trn!("{n} item", "{n} items", list.items.len());
    let total = list.total();
    if total > 0.0 {
        s.push_str(&format!("  ·  {}", meta::fmt_total(total)));
    }
    if !app.sel.is_empty() {
        let sd: f64 = app.sel.iter().filter_map(|&i| list.items.get(i)).map(|t| t.duration).sum();
        s.push_str(&format!("  ·  {}", tr!("selected {} ({})", app.sel.len(), meta::fmt_total(sd))));
    }
    ui.global::<Lists>().set_summary(ss(s));
}

pub fn selected_sorted(app: &App) -> Vec<usize> {
    let mut v: Vec<usize> = app.sel.iter().copied().collect();
    v.sort();
    v
}

pub fn selected_tracks(app: &App) -> Vec<TrackInfo> {
    let li = app.store.active;
    let Some(list) = app.store.lists.get(li) else { return Vec::new() };
    selected_sorted(app).into_iter().filter_map(|i| list.items.get(i).cloned()).collect()
}

/// Nazwa karty bieżącego odtwarzania w zapisanych listach (identyfikator – na ekranie tłumaczona).
pub const QUEUE_NAME: &str = "Kolejka";

/// Nazwa listy do wyświetlenia.
pub fn display_name(name: &str) -> &str {
    if name == QUEUE_NAME {
        tr!("Queue")
    } else {
        name
    }
}

/// Karta „Kolejka” – bieżące odtwarzanie: tu trafia to, co gramy z biblioteki, otwarte pliki
/// i pozycje dodane do kolejki. Dawne karty „Biblioteka”/„Wideo”/„Domyślna” stają się kolejką.
pub fn queue_list(app: &mut App) -> u64 {
    if let Some(l) = app.store.lists.iter().find(|l| l.name == QUEUE_NAME) {
        return l.id;
    }
    if let Some(l) = app.store.lists.iter_mut().find(|l| l.name == "Biblioteka" || l.name == "Domyślna" || l.name == "Wideo") {
        l.name = QUEUE_NAME.into();
        app.dirty = true;
        return l.id;
    }
    let id = app.store.next_id();
    app.store.lists.insert(0, Playlist { id, name: QUEUE_NAME.into(), ..Default::default() });
    if app.store.lists.len() > 1 {
        app.store.active += 1;
    }
    app.dirty = true;
    id
}

/// Kopie utworów na koniec kolejki; zwraca indeksy dodanych pozycji.
fn append_to_queue(app: &mut App, tracks: Vec<TrackInfo>) -> (u64, Vec<usize>) {
    let qid = queue_list(app);
    let li = app.list_index(qid).unwrap();
    let start = app.store.lists[li].items.len();
    let n = tracks.len();
    app.store.lists[li].items.extend(tracks);
    app.dirty = true;
    (qid, (start..start + n).collect())
}

pub fn create_list(app: &mut App, name: &str) -> u64 {
    let id = app.store.next_id();
    app.store.lists.push(Playlist { id, name: name.to_string(), ..Default::default() });
    app.store.active = app.store.lists.len() - 1;
    app.sel.clear();
    app.dirty = true;
    rebuild(app);
    crate::libview::rebuild_nav(app);
    id
}

pub fn switch_tab(app: &mut App, i: usize) {
    if i >= app.store.lists.len() {
        return;
    }
    app.store.active = i;
    app.sel.clear();
    app.anchor = None;
    app.filter.clear();
    if let Some(ui) = app.ui() {
        ui.global::<Lists>().set_filter(ss(""));
        ui.global::<Lib>().set_view(0);
        ui.global::<Lib>().set_nav_selected(ss("playlist"));
    }
    app.dirty = true;
    rebuild(app);
    scroll_to_current(app);
}

pub fn close_tab(app: &mut App, i: usize) {
    if i >= app.store.lists.len() {
        return;
    }
    let id = app.store.lists[i].id;
    if app.store.lists[i].name == QUEUE_NAME {
        // Kolejka nie znika – zaczyna się od nowa, z grającym utworem (jeśli coś gra).
        let playing = app.cur.as_ref().filter(|c| c.station.is_none() && c.item != 0).map(|c| c.info.clone());
        app.store.lists[i].items = playing.iter().cloned().collect();
        if let Some(c) = &mut app.cur {
            if playing.is_some() {
                c.list = Some(id);
                c.idx = 0;
            } else if c.list == Some(id) {
                c.list = None;
            }
        }
        app.queue.retain(|q| q.0 != id);
        app.shuffle_len = 0;
        app.pending = None;
        if app.store.active == i {
            app.sel.clear();
        }
        app.dirty = true;
        rebuild(app);
        crate::libview::rebuild_nav(app);
        crate::player::refresh_next(app);
        return;
    }
    if app.store.lists.len() == 1 {
        app.store.lists[0].items.clear();
    } else {
        app.store.lists.remove(i);
        if app.store.active >= app.store.lists.len() {
            app.store.active = app.store.lists.len() - 1;
        } else if app.store.active > i {
            app.store.active -= 1;
        }
    }
    if let Some(c) = &mut app.cur {
        if c.list == Some(id) {
            c.list = None;
        }
    }
    app.queue.retain(|q| q.0 != id);
    app.sel.clear();
    app.dirty = true;
    rebuild(app);
    crate::libview::rebuild_nav(app);
    crate::player::refresh_next(app);
}

/// Po zmianie kolejności/usunięciu: odnajduje bieżący utwór po ścieżce.
pub fn fix_current(app: &mut App, list: u64) {
    let Some(li) = app.list_index(list) else { return };
    if let Some(c) = &mut app.cur {
        if c.list == Some(list) {
            let items = &app.store.lists[li].items;
            if items.get(c.idx).map(|t| &t.path) != Some(&c.info.path) {
                match items.iter().enumerate().min_by_key(|(i, t)| if t.path == c.info.path { (*i as i64 - c.idx as i64).abs() } else { i64::MAX }) {
                    Some((i, t)) if t.path == c.info.path => c.idx = i,
                    _ => c.list = None,
                }
            }
        }
    }
    app.queue.retain(|&(l, i)| l != list || i < app.store.lists[li].items.len());
    app.shuffle_len = 0;
    app.pending = None;
    crate::player::refresh_next(app);
}

pub fn remove_selected(app: &mut App) {
    let li = app.store.active;
    let id = app.active_id();
    let sel = app.sel.clone();
    if sel.is_empty() {
        return;
    }
    let mut i = 0;
    app.store.lists[li].items.retain(|_| {
        let keep = !sel.contains(&i);
        i += 1;
        keep
    });
    app.sel.clear();
    app.dirty = true;
    fix_current(app, id);
    rebuild(app);
    crate::libview::rebuild_nav(app);
}

fn modify_active(app: &mut App, f: impl FnOnce(&mut Vec<TrackInfo>)) {
    let li = app.store.active;
    let id = app.active_id();
    f(&mut app.store.lists[li].items);
    app.sel.clear();
    app.dirty = true;
    fix_current(app, id);
    rebuild(app);
    crate::libview::rebuild_nav(app);
}

pub fn move_selected(app: &mut App, up: bool) {
    let li = app.store.active;
    let id = app.active_id();
    let mut sel = selected_sorted(app);
    if sel.is_empty() {
        return;
    }
    let len = app.store.lists[li].items.len();
    if (up && sel[0] == 0) || (!up && *sel.last().unwrap() + 1 >= len) {
        return;
    }
    if !up {
        sel.reverse();
    }
    for &i in &sel {
        let j = if up { i - 1 } else { i + 1 };
        app.store.lists[li].items.swap(i, j);
    }
    app.sel = sel.iter().map(|&i| if up { i - 1 } else { i + 1 }).collect();
    app.dirty = true;
    fix_current(app, id);
    rebuild(app);
}

/// Polecenia z menu listy.
pub fn command(app: &mut App, id: &str) {
    let list = app.active_id();
    let sel = selected_sorted(app);
    match id {
        "play" => {
            if let Some(&i) = sel.first() {
                crate::player::play(app, list, i, None);
            }
        }
        "play-next" | "enqueue" => {
            // Z innej karty utwory są dopisywane do karty „Kolejka”, a kolejność grania
            // ustawia lista kolejki (app.queue).
            let in_queue = app.store.lists.get(app.store.active).is_some_and(|l| l.name == QUEUE_NAME);
            let (qlist, idx) = if in_queue || sel.is_empty() {
                (list, sel.clone())
            } else {
                let tracks = selected_tracks(app);
                append_to_queue(app, tracks)
            };
            if id == "play-next" {
                for (n, &i) in idx.iter().enumerate() {
                    app.queue.insert(n, (qlist, i));
                }
                app.toast(tr!("Will play next"));
            } else {
                for &i in &idx {
                    app.queue.push((qlist, i));
                }
                app.toast(tr!("In queue: {}", app.queue.len()));
            }
            if qlist != list {
                rebuild(app);
                crate::libview::rebuild_nav(app);
            }
            refresh_playing(app);
            update_queue_panel(app);
            crate::player::refresh_next(app);
        }
        "remove" => remove_selected(app),
        "crop" => {
            let keep: std::collections::HashSet<usize> = app.sel.clone();
            let mut i = 0;
            modify_active(app, |v| {
                v.retain(|_| {
                    let k = keep.contains(&i);
                    i += 1;
                    k
                })
            });
        }
        "dedupe" => modify_active(app, |v| {
            let mut seen = std::collections::HashSet::new();
            v.retain(|t| seen.insert(t.path.to_lowercase()));
        }),
        "missing" => modify_active(app, |v| v.retain(|t| meta::is_url(&t.path) || Path::new(&t.path).exists())),
        "clear" => modify_active(app, |v| v.clear()),
        "sort-album" => modify_active(app, |v| v.sort_by(openmp_core::library::album_order)),
        "sort-title" => modify_active(app, |v| v.sort_by_key(|t| t.display_title().to_lowercase())),
        "sort-artist" => modify_active(app, |v| v.sort_by_key(|t| (t.display_artist().to_lowercase(), t.display_title().to_lowercase()))),
        "sort-duration" => modify_active(app, |v| v.sort_by(|a, b| a.duration.total_cmp(&b.duration))),
        "sort-path" => modify_active(app, |v| v.sort_by_key(|t| t.path.to_lowercase())),
        "sort-added" => modify_active(app, |v| v.sort_by_key(|t| std::cmp::Reverse(t.modified))),
        "reverse" => modify_active(app, |v| v.reverse()),
        "randomize" => modify_active(app, |v| v.shuffle(&mut rand::rng())),
        "fav" => {
            let tracks = selected_tracks(app);
            let all_fav = tracks.iter().all(|t| app.stats.get(&t.path).rating >= 5);
            for t in &tracks {
                app.stats.entry(&t.path).rating = if all_fav { 0 } else { 5 };
            }
            app.dirty = true;
            rebuild(app);
            if let Some(c) = &app.cur {
                let f = app.stats.get(&c.info.path).rating >= 5;
                if let Some(ui) = app.ui() {
                    ui.global::<Player>().set_fav(f);
                }
            }
        }
        c if c.starts_with("rate-") => {
            let n = c[5..].parse::<u8>().unwrap_or(0).min(5);
            let tracks = selected_tracks(app);
            if tracks.is_empty() {
                return;
            }
            for t in &tracks {
                app.stats.entry(&t.path).rating = n;
            }
            app.dirty = true;
            app.toast(if n == 0 { tr!("Rating removed").to_string() } else { tr!("Rating: {}", stars(n)) });
            rebuild(app);
            if let Some(c) = &app.cur {
                let f = app.stats.get(&c.info.path).rating >= 5;
                if let Some(ui) = app.ui() {
                    ui.global::<Player>().set_fav(f);
                }
            }
        }
        "explorer" => {
            if let Some(t) = selected_tracks(app).first() {
                crate::win::reveal(&t.path);
            }
        }
        "tags" => {
            let paths: Vec<String> = selected_tracks(app).into_iter().filter(|t| !meta::is_url(&t.path)).map(|t| t.path).collect();
            crate::dialogs::open_tag_editor(app, paths);
        }
        "info" => {
            if let Some(ui) = app.ui() {
                app.s.show_lyrics = true;
                ui.global::<Prefs>().set_side_panel(2);
            }
        }
        "new-from-sel" => {
            let tracks = selected_tracks(app);
            let id = create_list(app, &tr!("List {}", app.store.lists.len() + 1));
            if let Some(li) = app.list_index(id) {
                app.store.lists[li].items = tracks;
            }
            rebuild(app);
            crate::libview::rebuild_nav(app);
        }
        "select-all" => {
            app.sel = app.row_of.keys().copied().collect();
            refresh_selection(app);
            update_summary_only(app);
        }
        _ => {}
    }
}

pub fn row_menu(app: &mut App, x: f32, y: f32) {
    let n = app.sel.len();
    let tracks = selected_tracks(app);
    let all_fav = !tracks.is_empty() && tracks.iter().all(|t| app.stats.get(&t.path).rating >= 5);
    let items = vec![
        crate::menu::item("play", tr!("Play"), "play", "Enter"),
        crate::menu::item("play-next", tr!("Play next"), "queue", ""),
        crate::menu::item("enqueue", tr!("Add to queue"), "add", "Q"),
        crate::menu::sep(),
        crate::menu::item("fav", if all_fav { tr!("Remove from favorites") } else { tr!("Add to favorites") }, "heart", "Ctrl+D"),
        crate::menu::item_en("tags", if n > 1 { tr!("Edit tags of selected…") } else { tr!("Edit tags…") }, "edit", "F2", n > 0),
        crate::menu::item("info", tr!("Track info"), "info", "Ctrl+I"),
        crate::menu::item("explorer", tr!("Show in Explorer"), "folder-open", ""),
        crate::menu::sep(),
        crate::menu::item("new-from-sel", tr!("New list from selection"), "playlist", ""),
        crate::menu::item("remove", tr!("Remove from list"), "delete", "Del"),
        crate::menu::item("crop", tr!("Keep only selected"), "delete", ""),
        crate::menu::sep(),
        crate::menu::item("sort-album", tr!("Sort: artist, album, number"), "sort", ""),
        crate::menu::item("sort-title", tr!("Sort: title"), "sort", ""),
        crate::menu::item("sort-artist", tr!("Sort: artist"), "sort", ""),
        crate::menu::item("sort-duration", tr!("Sort: length"), "sort", ""),
        crate::menu::item("sort-path", tr!("Sort: file path"), "sort", ""),
        crate::menu::item("reverse", tr!("Reverse order"), "sort", ""),
        crate::menu::item("randomize", tr!("Shuffle list"), "shuffle", ""),
        crate::menu::sep(),
        crate::menu::item("dedupe", tr!("Remove duplicates"), "delete", ""),
        crate::menu::item("missing", tr!("Remove missing files"), "delete", ""),
    ];
    crate::menu::open(app, MenuCtx::List, items, x, y);
}

pub fn tab_menu(app: &mut App, i: usize, x: f32, y: f32) {
    if x < 0.0 {
        crate::dialogs::rename(app, RenameCtx::Tab(i), tr!("List name"), &app.store.lists[i].name.clone());
        return;
    }
    let items = vec![
        crate::menu::item("tab-rename", tr!("Rename…"), "edit", ""),
        crate::menu::item("tab-export", tr!("Save as M3U8…"), "download", "Ctrl+S"),
        crate::menu::item("tab-import", tr!("Import a playlist into this tab…"), "file", ""),
        crate::menu::sep(),
        crate::menu::item("tab-clear", tr!("Clear list"), "delete", ""),
        crate::menu::item("tab-close", tr!("Close tab"), "close", ""),
    ];
    crate::menu::open(app, MenuCtx::Tab(i), items, x, y);
}

pub fn tab_command(app: &mut App, i: usize, id: &str) {
    match id {
        "tab-rename" => {
            let name = app.store.lists.get(i).map(|l| l.name.clone()).unwrap_or_default();
            crate::dialogs::rename(app, RenameCtx::Tab(i), tr!("List name"), &name);
        }
        "tab-export" => {
            switch_tab(app, i);
            crate::dialogs::export_list(app);
        }
        "tab-import" => {
            switch_tab(app, i);
            crate::dialogs::import_list(app, false);
        }
        "tab-clear" => {
            switch_tab(app, i);
            command(app, "clear");
        }
        "tab-close" => close_tab(app, i),
        _ => {}
    }
}

pub fn update_queue_panel(app: &mut App) {
    let rows: Vec<TrackRow> = app
        .queue
        .iter()
        .filter_map(|&(l, i)| crate::player::track_at(app, l, i).map(|t| track_row(&t, i, String::new(), false, false, false, String::new())))
        .collect();
    if let Some(ui) = app.ui() {
        ui.global::<Player>().set_queue(slint::ModelRc::new(slint::VecModel::from(rows)));
    }
}

/// Po edycji tagów: nowe dane we wszystkich listach.
pub fn refresh_infos(app: &mut App, fresh: &HashMap<String, TrackInfo>) {
    for l in &mut app.store.lists {
        for t in &mut l.items {
            if let Some(n) = fresh.get(&t.path.to_lowercase()) {
                *t = n.clone();
            }
        }
    }
    for t in &mut app.lib.tracks {
        if let Some(n) = fresh.get(&t.path.to_lowercase()) {
            *t = n.clone();
        }
    }
    if let Some(c) = &mut app.cur {
        if let Some(n) = fresh.get(&c.info.path.to_lowercase()) {
            c.info = n.clone();
        }
    }
    app.dirty = true;
    rebuild(app);
}
