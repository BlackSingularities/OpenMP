//! Biblioteka na ekranie: muzyka (wykonawcy, albumy, gatunki, lata, foldery), wideo (wszystkie
//! filmy, foldery, kontynuowanie oglądania), zbiory (ulubione, najczęściej, historia),
//! panel nawigacji i skanowanie w tle – także folderów na dyskach sieciowych.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use openmp_core::library::{self, Progress};
use openmp_core::meta::{self, TrackInfo};
use slint::{Image, Model};

use crate::app::*;
use crate::*;

fn nav(id: &str, label: &str, icon: &str, count: String) -> NavItem {
    NavItem { id: ss(id), label: ss(label), icon: ss(icon), count: ss(count), section: false, depth: 0, expander: 0 }
}

fn section(id: &str, label: &str) -> NavItem {
    NavItem { id: ss(id), label: ss(label), icon: ss(""), count: ss(""), section: true, depth: 0, expander: 0 }
}

fn n(v: usize) -> String {
    if v == 0 {
        String::new()
    } else {
        v.to_string()
    }
}

/// Utwory i filmy leżące w folderze biblioteki (z podfolderami).
fn under_root<'a>(app: &'a App, root: &str) -> impl Iterator<Item = &'a TrackInfo> + 'a {
    let prefix = root.trim_end_matches(['\\', '/']).to_lowercase();
    app.lib.tracks.iter().chain(app.lib.videos.iter()).filter(move |t| {
        let p = t.path.to_lowercase();
        p.len() > prefix.len() && p.starts_with(&prefix) && matches!(p.as_bytes()[prefix.len()], b'\\' | b'/')
    })
}

fn folder_name(root: &str) -> String {
    let t = root.trim_end_matches(['\\', '/']);
    std::path::Path::new(t).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_else(|| t.to_string())
}

pub fn rebuild_nav(app: &mut App) {
    let lists: usize = app.store.lists.len();
    let mut v = vec![
        nav("playlist", tr!("Playlists"), "playlist", n(lists)),
        section("sec-music", tr!("MUSIC")),
        nav("artist", tr!("Artists"), "artist", String::new()),
        nav("album", tr!("Albums"), "album", String::new()),
        nav("genre", tr!("Genres"), "tag", String::new()),
        nav("year", tr!("Years"), "calendar", String::new()),
        nav("all", tr!("All tracks"), "note", n(app.lib.tracks.len())),
        section("sec-video", tr!("VIDEO")),
        nav("v-all", tr!("All videos"), "movie", n(app.lib.videos.len())),
        nav("v-continue", tr!("Continue watching"), "history", String::new()),
        nav("v-recent", tr!("Recently added"), "clock", String::new()),
        section("sec-sets", tr!("COLLECTIONS")),
        nav("fav", tr!("Favorites"), "heart", n(app.stats.map.values().filter(|s| s.rating >= 5).count())),
        nav("rated", tr!("Rated"), "heart", n(app.stats.map.values().filter(|s| s.rating > 0).count())),
        nav("top", tr!("Most played"), "trending", String::new()),
        nav("recent", tr!("Recently added music"), "clock", String::new()),
        nav("history", tr!("History"), "history", String::new()),
        nav("dupes", tr!("Duplicates"), "file", String::new()),
        nav("radio", tr!("Internet radio"), "radio", String::new()),
        nav("stats", tr!("Statistics"), "trending", String::new()),
    ];
    if !app.s.smart_lists.is_empty() {
        v.push(section("sec-smart", tr!("SMART PLAYLISTS")));
        for (i, (name, _)) in app.s.smart_lists.iter().enumerate() {
            v.push(nav(&format!("smart:{i}"), name, "playlist", String::new()));
        }
    }
    v.push(section("sec-podcasts", tr!("PODCASTS")));
    for (i, (_, title)) in app.s.podcasts.iter().enumerate() {
        v.push(nav(&format!("pod:{i}"), title, "radio", String::new()));
    }
    v.push(nav("pod-add", tr!("Add podcast…"), "add", String::new()));
    // Foldery biblioteki (także udziały sieciowe) – skrót do całej zawartości folderu.
    v.push(section("sec-folders", tr!("FOLDERS")));
    let roots: Vec<String> = app.s.library_folders.iter().chain(app.s.video_folders.iter()).cloned().collect();
    for root in roots {
        let count = under_root(app, &root).count();
        let root = root.trim_end_matches(['\\', '/']).to_string();
        nav_tree(app, &mut v, &root, &folder_name(&root), count, 0);
    }
    // Sekcje wyłączone w ustawieniach znikają razem z pozycjami.
    let off = |id: &str| app.s.nav_hidden.iter().any(|h| h == id);
    let mut gone = false;
    v.retain(|it| {
        if it.section {
            gone = off(it.id.as_str());
        } else if it.id.as_str() == "playlist" {
            return !off("playlist");
        }
        !gone
    });
    // Zwinięte sekcje: sam nagłówek, bez pozycji pod nim.
    let mut hidden = false;
    v.retain_mut(|it| {
        if it.section {
            hidden = app.s.nav_collapsed.iter().any(|c| c.as_str() == it.id.as_str());
            it.expander = if hidden { 1 } else { 2 };
            return true;
        }
        !hidden
    });
    app.m.nav.set_vec(v);
}

/// Folder w panelu i – gdy rozwinięty – jego podfoldery, jak w okienku nawigacji Eksploratora.
fn nav_tree(app: &App, v: &mut Vec<NavItem>, dir: &str, label: &str, count: usize, depth: i32) {
    let (children, _) = library::folder_children(&app.lib.tracks, dir);
    let (vchildren, _) = library::folder_children(&app.lib.videos, dir);
    let mut kids: Vec<library::Group> = children;
    for g in vchildren {
        match kids.iter_mut().find(|k| k.name.eq_ignore_ascii_case(&g.name)) {
            Some(k) => k.count += g.count,
            None => kids.push(g),
        }
    }
    kids.sort_by_key(|g| g.name.to_lowercase());
    let open = app.libv.expanded.contains(&dir.to_lowercase());
    let mut item = nav(&format!("root:{dir}"), label, if depth == 0 { "folder-open" } else { "folder" }, n(count));
    item.depth = depth;
    item.expander = if kids.is_empty() { 0 } else if open { 2 } else { 1 };
    v.push(item);
    if open {
        for k in kids {
            nav_tree(app, v, &k.name, &folder_name(&k.name), k.count, depth + 1);
        }
    }
}

/// Strzałka przy folderze w panelu: rozwija albo zwija podfoldery.
pub fn nav_toggle(app: &mut App, id: &str) {
    if id.starts_with("sec-") {
        // Nagłówek sekcji: zwija albo rozwija całą sekcję (zapamiętane w ustawieniach).
        if let Some(i) = app.s.nav_collapsed.iter().position(|c| c == id) {
            app.s.nav_collapsed.remove(i);
        } else {
            app.s.nav_collapsed.push(id.to_string());
        }
        app.dirty = true;
        rebuild_nav(app);
        return;
    }
    let Some(dir) = id.strip_prefix("root:") else { return };
    let key = dir.to_lowercase();
    if !app.libv.expanded.remove(&key) {
        app.libv.expanded.insert(key);
    }
    rebuild_nav(app);
}

/// Kliknięcie w panelu nawigacji.
pub fn nav_clicked(app: &mut App, id: &str) {
    let Some(ui) = app.ui() else { return };
    let l = ui.global::<Lib>();
    l.set_nav_selected(ss(id));
    if let Some(lid) = id.strip_prefix("list:").and_then(|s| s.parse::<u64>().ok()) {
        if let Some(i) = app.list_index(lid) {
            crate::lists::switch_tab(app, i);
        }
        l.set_view(0);
        return;
    }
    if id == "playlist" {
        l.set_view(0);
        return;
    }
    if id == "pod-add" {
        crate::dialogs::rename(app, RenameCtx::Podcast, tr!("Podcast feed address (RSS)"), "https://");
        return;
    }
    if id == "stats" {
        l.set_view(3);
        crate::statsview::show(app);
        return;
    }
    if id == "radio" {
        l.set_view(2);
        crate::radio::open(app);
        return;
    }
    l.set_view(1);
    app.libv.kind = id.to_string();
    app.libv.stack.clear();
    app.libv.sel.clear();
    app.libv.search.clear();
    l.set_search(ss(""));
    show(app);
}

pub fn nav_menu(app: &mut App, id: &str, x: f32, y: f32) {
    if let Some(lid) = id.strip_prefix("list:").and_then(|s| s.parse::<u64>().ok()) {
        if let Some(i) = app.list_index(lid) {
            crate::lists::tab_menu(app, i, x, y);
        }
        return;
    }
    if id.starts_with("sec-") || id == "playlist" {
        return;
    }
    if id.starts_with("pod:") {
        let items = vec![
            crate::menu::item("pod-refresh", tr!("Check for new episodes"), "refresh", ""),
            crate::menu::sep(),
            crate::menu::item("pod-del", tr!("Unsubscribe"), "delete", ""),
        ];
        crate::menu::open(app, MenuCtx::Nav(id.to_string()), items, x, y);
        return;
    }
    if id.starts_with("smart:") {
        let items = vec![
            crate::menu::item("smart-rename", tr!("Rename…"), "edit", ""),
            crate::menu::item("smart-query", tr!("Edit filters…"), "edit", ""),
            crate::menu::sep(),
            crate::menu::item("smart-del", tr!("Delete smart playlist"), "delete", ""),
        ];
        crate::menu::open(app, MenuCtx::Nav(id.to_string()), items, x, y);
        return;
    }
    let items = vec![
        crate::menu::item("rescan", tr!("Refresh library"), "refresh", ""),
        crate::menu::item("folders", tr!("Library folders…"), "folder", ""),
    ];
    crate::menu::open(app, MenuCtx::Nav(id.to_string()), items, x, y);
}

pub fn nav_command(app: &mut App, id: &str, cmd: &str) {
    match cmd {
        "smart-rename" | "smart-query" => {
            let Some(i) = id.strip_prefix("smart:").and_then(|s| s.parse::<usize>().ok()) else { return };
            let Some((name, query)) = app.s.smart_lists.get(i).cloned() else { return };
            if cmd == "smart-rename" {
                crate::dialogs::rename(app, RenameCtx::SmartName(i), tr!("Smart playlist name"), &name);
            } else {
                crate::dialogs::rename(app, RenameCtx::SmartQuery(i), tr!("Filters, e.g. rating>=4 year:1990-1999 genre:rock"), &query);
            }
        }
        "smart-del" => {
            if let Some(i) = id.strip_prefix("smart:").and_then(|s| s.parse::<usize>().ok()) {
                if i < app.s.smart_lists.len() {
                    app.s.smart_lists.remove(i);
                    app.dirty = true;
                    rebuild_nav(app);
                }
            }
        }
        "pod-refresh" | "pod-del" => {
            let Some(i) = id.strip_prefix("pod:").and_then(|s| s.parse::<usize>().ok()) else { return };
            let Some((url, _)) = app.s.podcasts.get(i).cloned() else { return };
            app.podcasts.remove(&url);
            if cmd == "pod-del" {
                app.s.podcasts.remove(i);
                app.s.save();
                rebuild_nav(app);
            }
            if app.libv.kind.starts_with("pod:") {
                show(app);
            }
        }
        "rescan" => start_scan(app),
        "folders" => crate::keys::open_settings(app, 7),
        _ => {}
    }
}

fn lookup<'a>(a: &'a [TrackInfo], b: &'a [TrackInfo]) -> HashMap<String, &'a TrackInfo> {
    a.iter().chain(b.iter()).map(|t| (library::key(&t.path), t)).collect()
}

/// Utwory z mapy statystyk (ulubione, najczęściej, historia): z biblioteki albo czytane z pliku.
/// `a` i `b` – obie części biblioteki (jedna bywa wyjęta z `app` na czas rysowania).
fn from_stats(a: &[TrackInfo], b: &[TrackInfo], mut keys: Vec<(String, u64)>, limit: usize) -> Vec<TrackInfo> {
    keys.sort_by(|a, b| b.1.cmp(&a.1));
    let idx = lookup(a, b);
    keys.into_iter()
        .take(limit)
        .filter_map(|(k, _)| {
            idx.get(&k).map(|t| (*t).clone()).or_else(|| {
                let p = std::path::Path::new(&k);
                p.exists().then(|| meta::read(p))
            })
        })
        .collect()
}

fn title_for(kind: &str) -> &'static str {
    match kind {
        "artist" => tr!("Artists"),
        "album" => tr!("Albums"),
        "genre" => tr!("Genres"),
        "year" => tr!("Years"),
        "folder" => tr!("Folders"),
        "all" => tr!("All tracks"),
        "fav" => tr!("Favorites"),
        "top" => tr!("Most played"),
        "rated" => tr!("Rated"),
        "recent" => tr!("Recently added music"),
        "history" => tr!("History"),
        "dupes" => tr!("Duplicates"),
        "v-all" => tr!("All videos"),
        "v-folder" => tr!("Video folders"),
        "v-continue" => tr!("Continue watching"),
        "v-recent" => tr!("Recently added videos"),
        _ => tr!("Library"),
    }
}

/// Rysuje bieżący widok biblioteki.
pub fn show(app: &mut App) {
    // Bez kopiowania całej biblioteki: na czas rysowania wyjmujemy ją z `app` i oddajemy z powrotem.
    let video = app.libv.kind.starts_with("v-");
    let source = std::mem::take(if video { &mut app.lib.videos } else { &mut app.lib.tracks });
    show_with(app, &source);
    *(if video { &mut app.lib.videos } else { &mut app.lib.tracks }) = source;
}

/// Druga część biblioteki (filmy przy muzyce i odwrotnie) – ta, której `show` nie wyjął.
fn other(app: &App, video_kind: bool) -> &[TrackInfo] {
    if video_kind { &app.lib.tracks } else { &app.lib.videos }
}

fn show_with(app: &mut App, source: &[TrackInfo]) {
    let Some(ui) = app.ui() else { return };
    let l = ui.global::<Lib>();
    let kind = app.libv.kind.clone();
    let video_kind = kind.starts_with("v-");
    l.set_can_back(!app.libv.stack.is_empty());
    let empty_lib = if video_kind {
        tr!("No videos in the library.\nAdd video folders (network drives work too) in Settings → Library.")
    } else {
        tr!("The library is empty.\nAdd music folders (network drives work too) in Settings → Library.")
    };
    // Wyszukiwanie obejmuje całą bibliotekę – muzykę i filmy.
    if !app.libv.search.trim().is_empty() {
        let q = library::Query::parse(&app.libv.search);
        let found: Vec<TrackInfo> = source.iter().chain(other(app, video_kind).iter()).filter(|t| q.matches(t, &app.stats.get(&t.path))).take(2000).cloned().collect();
        l.set_title(ss(tr!("Results: “{}”", app.libv.search.trim())));
        l.set_subtitle(ss(trn!("{n} item", "{n} items", found.len())));
        l.set_empty_text(ss(tr!("Nothing found.\nFilters: rating>=4, year:1990-1999, genre:rock, artist:…, album:…, plays>=10, format:flac, added<30, length>6")));
        set_tracks(app, found);
        return;
    }
    // Wejście w głąb: ostatni element stosu to wybrana grupa.
    if let Some((gk, name, sub)) = app.libv.stack.last().cloned() {
        if kind == "artist" && gk == "artist" {
            // Albumy wykonawcy (siatka), potem utwory albumu.
            let mine: Vec<TrackInfo> = library::in_group(&source, "artist", &name, "").into_iter().cloned().collect();
            let groups = library::group_by(&mine, "album");
            l.set_title(ss(&name));
            l.set_subtitle(ss(format!("{} · {}", trn!("{n} album", "{n} albums", groups.len()), trn!("{n} track", "{n} tracks", mine.len()))));
            app.libv.tracks = mine;
            set_tiles(app, groups, "album", true);
            return;
        }
        if gk == "dir" || gk == "v-dir" {
            let video = gk == "v-dir";
            // Drzewo otwarte z sekcji FOLDERY może być innego rodzaju niż bieżący widok.
            if video == video_kind {
                show_dir(app, source, &name, video);
            } else {
                let src = other(app, video_kind).to_vec();
                show_dir(app, &src, &name, video);
            }
            return;
        }
        let gk = match gk.as_str() {
            "files" => "folder".to_string(),
            "v-files" => "v-folder".to_string(),
            _ => gk,
        };
        let gkind = if gk == "v-folder" { "folder" } else { gk.as_str() };
        let mut tracks: Vec<TrackInfo> = library::in_group(&source, gkind, &name, &sub).into_iter().cloned().collect();
        if video_kind {
            tracks.sort_by_key(|t| natural(&t.path));
        }
        let dur: f64 = tracks.iter().map(|t| t.duration).sum();
        l.set_title(ss(if gk == "folder" || gk == "v-folder" { std::path::Path::new(&name).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(name.clone()) } else { name.clone() }));
        let mut s = vec![];
        if !sub.is_empty() {
            s.push(sub.clone());
        }
        s.push(trn!("{n} item", "{n} items", tracks.len()));
        if dur > 0.0 {
            s.push(meta::fmt_total(dur));
        }
        if gk == "folder" || gk == "v-folder" {
            s.push(name.clone());
        }
        l.set_subtitle(ss(s.join("  ·  ")));
        l.set_empty_text(ss(tr!("Empty.")));
        set_tracks(app, tracks);
        return;
    }
    l.set_title(ss(title_for(&kind)));
    l.set_empty_text(ss(empty_lib));
    match kind.as_str() {
        "artist" | "genre" | "year" => {
            let groups = library::group_by(&source, &kind);
            l.set_subtitle(ss(format!("{} · {}", groups.len(), trn!("{n} track", "{n} tracks", source.len()))));
            set_tiles(app, groups, &kind, false);
        }
        "folder" | "v-folder" => {
            // Drzewo folderów: od folderów biblioteki w głąb (jedyny folder – od razu jego zawartość).
            let roots = if video_kind { app.s.video_folders.clone() } else { app.s.library_folders.clone() };
            if roots.len() == 1 {
                show_dir(app, &source, &roots[0], video_kind);
                return;
            }
            let groups: Vec<library::Group> = roots
                .iter()
                .map(|r| {
                    let t = library::in_tree(&source, r);
                    library::Group {
                        name: r.trim_end_matches(['\\', '/']).to_string(),
                        count: t.len(),
                        duration: t.iter().map(|t| t.duration).sum(),
                        first: t.first().map(|t| t.path.clone()).unwrap_or_default(),
                        ..Default::default()
                    }
                })
                .collect();
            let what = if video_kind { trn!("{n} video", "{n} videos", source.len()) } else { trn!("{n} track", "{n} tracks", source.len()) };
            l.set_subtitle(ss(format!("{} · {what}", trn!("{n} folder", "{n} folders", groups.len()))));
            set_tiles(app, groups, if video_kind { "v-dir" } else { "dir" }, true);
        }
        "album" => {
            let groups = library::group_by(&source, "album");
            l.set_subtitle(ss(format!("{} · {}", trn!("{n} album", "{n} albums", groups.len()), trn!("{n} track", "{n} tracks", source.len()))));
            set_tiles(app, groups, "album", true);
        }
        "all" | "v-all" => {
            let mut t = source.to_vec();
            if video_kind {
                t.sort_by_key(|t| natural(&t.path));
            }
            l.set_subtitle(ss(trn!("{n} item", "{n} items", t.len())));
            set_tracks(app, t);
        }
        "recent" | "v-recent" => {
            let mut t = source.to_vec();
            t.sort_by_key(|t| std::cmp::Reverse(t.modified));
            t.truncate(300);
            l.set_subtitle(ss(tr!("By file date")));
            set_tracks(app, t);
        }
        "v-continue" => {
            let keys: Vec<(String, u64)> = app.stats.map.iter().filter(|(k, s)| s.resume > 10.0 && meta::is_video(std::path::Path::new(k.as_str()))).map(|(k, s)| (k.clone(), s.last)).collect();
            let t = from_stats(source, other(app, video_kind), keys, 100);
            l.set_subtitle(ss(tr!("Movies stopped midway – they will start where you left off")));
            l.set_empty_text(ss(tr!("No unfinished movies.")));
            set_tracks(app, t);
        }
        "fav" => {
            let keys: Vec<(String, u64)> = app.stats.map.iter().filter(|(_, s)| s.rating >= 5).map(|(k, s)| (k.clone(), s.last)).collect();
            let t = from_stats(source, other(app, video_kind), keys, 5000);
            l.set_subtitle(ss(format!("{} · {}", trn!("{n} item", "{n} items", t.len()), tr!("the heart next to a track adds it here"))));
            l.set_empty_text(ss(tr!("No favorites. Click the heart next to a track or press Ctrl+D.")));
            set_tracks(app, t);
        }
        "rated" => {
            // Najpierw wyższa ocena, w niej ostatnio odtwarzane.
            let keys: Vec<(String, u64)> = app.stats.map.iter().filter(|(_, s)| s.rating > 0).map(|(k, s)| (k.clone(), ((s.rating as u64) << 40) | (s.last & 0xff_ffff_ffff))).collect();
            let t = from_stats(source, other(app, video_kind), keys, 5000);
            l.set_subtitle(ss(format!("{} · {}", trn!("{n} item", "{n} items", t.len()), tr!("Ctrl+1…5 rates the selection"))));
            l.set_empty_text(ss(tr!("Nothing is rated yet. Select tracks and press Ctrl+1…5.")));
            set_tracks(app, t);
        }
        "dupes" => {
            let t = duplicates(&app.lib.tracks);
            let groups = t.iter().map(|t| dupe_key(t).0).collect::<std::collections::HashSet<_>>().len();
            l.set_subtitle(ss(format!("{} · {}", trn!("{n} group", "{n} groups", groups), tr!("the same artist and title, similar length"))));
            l.set_empty_text(ss(tr!("No duplicates in the music library.")));
            set_tracks(app, t);
        }
        "top" => {
            let keys: Vec<(String, u64)> = app.stats.map.iter().filter(|(_, s)| s.plays > 0).map(|(k, s)| (k.clone(), s.plays as u64)).collect();
            let t = from_stats(source, other(app, video_kind), keys, 200);
            l.set_subtitle(ss(tr!("By play count")));
            l.set_empty_text(ss(tr!("Nothing has been played yet.")));
            set_tracks(app, t);
        }
        k if k.starts_with("root:") => {
            // Folder biblioteki jako drzewo: podfoldery, a w nich pliki.
            let root = k[5..].to_string();
            let video = app.s.video_folders.iter().any(|f| {
                let f = f.trim_end_matches(['\\', '/']).to_lowercase();
                let r = root.to_lowercase();
                r == f || r.starts_with(&format!("{f}\\")) || r.starts_with(&format!("{f}/"))
            });
            if video == video_kind {
                show_dir(app, source, &root, video);
            } else {
                let src = other(app, video_kind).to_vec();
                show_dir(app, &src, &root, video);
            }
        }
        k if k.starts_with("pod:") => {
            let Some((url, title)) = k[4..].parse::<usize>().ok().and_then(|i| app.s.podcasts.get(i).cloned()) else {
                set_tracks(app, Vec::new());
                return;
            };
            l.set_title(ss(&title));
            match app.podcasts.get(&url).cloned() {
                Some(eps) => {
                    l.set_subtitle(ss(trn!("{n} episode", "{n} episodes", eps.len())));
                    l.set_empty_text(ss(tr!("No episodes – the feed could not be loaded or is empty.")));
                    set_tracks(app, eps);
                }
                None => {
                    l.set_subtitle(ss(tr!("Loading episodes…")));
                    l.set_empty_text(ss(tr!("Loading episodes…")));
                    set_tracks(app, Vec::new());
                    fetch_podcast(url, false);
                }
            }
        }
        k if k.starts_with("smart:") => {
            let Some((name, query)) = k[6..].parse::<usize>().ok().and_then(|i| app.s.smart_lists.get(i).cloned()) else {
                set_tracks(app, Vec::new());
                return;
            };
            let q = library::Query::parse(&query);
            let mut t: Vec<TrackInfo> = source.iter().chain(other(app, video_kind).iter()).filter(|t| q.matches(t, &app.stats.get(&t.path))).take(5000).cloned().collect();
            t.sort_by(library::album_order);
            l.set_title(ss(&name));
            l.set_subtitle(ss(format!("{}  ·  {query}", trn!("{n} item", "{n} items", t.len()))));
            l.set_empty_text(ss(tr!("Nothing matches this playlist.")));
            set_tracks(app, t);
        }
        "history" => {
            let keys: Vec<(String, u64)> = app.stats.map.iter().filter(|(_, s)| s.last > 0).map(|(k, s)| (k.clone(), s.last)).collect();
            let t = from_stats(source, other(app, video_kind), keys, 300);
            l.set_subtitle(ss(tr!("Recently played")));
            l.set_empty_text(ss(tr!("History is empty.")));
            set_tracks(app, t);
        }
        _ => set_tracks(app, Vec::new()),
    }
}

/// Zawartość jednego folderu: tylko pliki leżące bezpośrednio w nim – podfoldery rozwija się
/// w panelu nawigacji, jak w Eksploratorze.
fn show_dir(app: &mut App, source: &[TrackInfo], dir: &str, _video: bool) {
    let Some(ui) = app.ui() else { return };
    let l = ui.global::<Lib>();
    let (groups, direct) = library::folder_children(source, dir);
    let mut t: Vec<TrackInfo> = direct.into_iter().cloned().collect();
    t.sort_by_key(|t| natural(&t.path));
    let dur: f64 = t.iter().map(|t| t.duration).sum();
    l.set_title(ss(folder_name(dir)));
    let mut s = vec![trn!("{n} item", "{n} items", t.len())];
    if dur > 0.0 {
        s.push(meta::fmt_total(dur));
    }
    if !groups.is_empty() {
        s.push(trn!("{n} subfolder – expand in the panel", "{n} subfolders – expand in the panel", groups.len()));
    }
    s.push(dir.trim_end_matches(['\\', '/']).to_string());
    l.set_subtitle(ss(s.join("  ·  ")));
    l.set_empty_text(ss(if groups.is_empty() {
        tr!("No files in this folder, or the folder is unavailable (e.g. the NAS server is off).")
    } else {
        tr!("This folder only has subfolders – expand it with the arrow in the panel on the left.")
    }));
    set_tracks(app, t);
}

/// Znacznik grupy plików leżących wprost w otwartym folderze (nie nazwa do wyświetlenia).
const FILES_HERE: &str = "\u{1}files-here";

/// Odcinki kanału jako pozycje listy (adres pliku – odtwarzany jak strumień).
fn episodes(feed: &openmp_core::podcast::Feed) -> Vec<TrackInfo> {
    feed.episodes
        .iter()
        .map(|e| TrackInfo {
            path: e.url.clone(),
            title: e.title.clone(),
            artist: feed.title.clone(),
            album: e.date.clone(),
            duration: e.duration,
            ..Default::default()
        })
        .collect()
}

/// Pobranie kanału w tle; `subscribe` – nowa subskrypcja (zapis i przejście do niej).
fn fetch_podcast(url: String, subscribe: bool) {
    std::thread::spawn(move || {
        let r = openmp_core::podcast::fetch(&url);
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match r {
                Ok(feed) => {
                    let title = if feed.title.trim().is_empty() { url.clone() } else { feed.title.trim().to_string() };
                    app.podcasts.insert(url.clone(), episodes(&feed));
                    if subscribe {
                        if !app.s.podcasts.iter().any(|(u, _)| *u == url) {
                            app.s.podcasts.push((url.clone(), title.clone()));
                            app.s.save();
                        }
                        rebuild_nav(app);
                        let i = app.s.podcasts.iter().position(|(u, _)| *u == url).unwrap_or(0);
                        nav_clicked(app, &format!("pod:{i}"));
                        app.toast(tr!("Subscribed to “{}”", title));
                    } else if app.libv.kind.starts_with("pod:") {
                        show(app);
                    }
                }
                Err(e) => {
                    if !subscribe {
                        app.podcasts.insert(url.clone(), Vec::new());
                        if app.libv.kind.starts_with("pod:") {
                            show(app);
                        }
                    }
                    app.toast(tr!("Could not load the podcast: {}", e));
                }
            });
        });
    });
}

/// Nowa subskrypcja z adresu kanału RSS (sprawdzana pobraniem).
pub fn add_podcast(app: &mut App, url: String) {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() < 12 {
        return;
    }
    app.toast(tr!("Loading the podcast…"));
    fetch_podcast(url, true);
}

/// Klucz duplikatu: wykonawca i tytuł bez wielkości liter, znaków i dopisków w nawiasach
/// („(Remastered)”, „[Live]”), oraz długość w sekundach.
fn dupe_key(t: &TrackInfo) -> (String, f64) {
    let clean = |s: &str| -> String {
        let mut out = String::new();
        let mut depth = 0;
        for c in s.chars() {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' => depth = (depth - 1).max(0),
                c if depth == 0 && c.is_alphanumeric() => out.extend(c.to_lowercase()),
                _ => {}
            }
        }
        out
    };
    (format!("{}\u{1}{}", clean(&t.display_artist()), clean(&t.display_title())), t.duration)
}

/// Utwory, które są w bibliotece więcej niż raz (ten sam klucz, długość różna najwyżej o 3 s),
/// pogrupowane obok siebie.
fn duplicates(tracks: &[TrackInfo]) -> Vec<TrackInfo> {
    let mut by: HashMap<String, Vec<&TrackInfo>> = HashMap::new();
    for t in tracks {
        let (k, _) = dupe_key(t);
        if !k.starts_with('\u{1}') && !k.ends_with('\u{1}') {
            by.entry(k).or_default().push(t);
        }
    }
    let mut groups: Vec<(String, Vec<&TrackInfo>)> = by.into_iter().filter(|(_, v)| v.len() > 1).collect();
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    for (_, mut v) in groups {
        v.sort_by(|a, b| a.duration.total_cmp(&b.duration));
        // Tylko pliki o podobnej długości – inna wersja (np. koncertowa) nie jest duplikatem.
        let close: Vec<&TrackInfo> = v.iter().enumerate().filter(|(i, t)| v.iter().enumerate().any(|(j, o)| j != *i && (o.duration - t.duration).abs() <= 3.0)).map(|(_, t)| *t).collect();
        out.extend(close.into_iter().cloned());
    }
    out
}

/// Porządek „naturalny”: odcinek 2 przed odcinkiem 10.
fn natural(s: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut num = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((std::mem::take(&mut text), num.parse().unwrap_or(0)));
                num.clear();
            }
            text.push(c);
        }
    }
    out.push((text, num.parse().unwrap_or(0)));
    out
}

fn set_tiles(app: &mut App, groups: Vec<library::Group>, kind: &str, grid: bool) {
    let Some(ui) = app.ui() else { return };
    let l = ui.global::<Lib>();
    l.set_show_tiles(true);
    l.set_grid(grid);
    app.libv.tile_keys = groups.iter().map(|g| (kind.to_string(), g.name.clone(), g.sub.clone())).collect();
    let with_cover = matches!(kind, "album" | "artist" | "folder" | "v-folder" | "dir" | "v-dir");
    let tiles: Vec<Tile> = groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let cover = if with_cover { crate::covers::get(&g.first) } else { None };
            let video = kind == "v-folder" || kind == "v-dir";
            let what = if video { trn!("{n} video", "{n} videos", g.count) } else { trn!("{n} track", "{n} tracks", g.count) };
            if kind == "dir" || kind == "v-dir" {
                let name = if g.sub == FILES_HERE { tr!("Files in this folder").to_string() } else { folder_name(&g.name) };
                return Tile {
                    key: ss(i.to_string()),
                    name: ss(name),
                    sub: ss(&what),
                    count: ss(&what),
                    has_cover: cover.is_some(),
                    cover: cover.unwrap_or_default(),
                };
            }
            let name = if kind == "folder" || video { std::path::Path::new(&g.name).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(g.name.clone()) } else { g.name.clone() };
            let sub = match kind {
                "album" => {
                    if g.sub.is_empty() {
                        trn!("{n} track", "{n} tracks", g.count)
                    } else {
                        g.sub.clone()
                    }
                }
                "folder" | "v-folder" => std::path::Path::new(&g.name).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
                _ => String::new(),
            };
            Tile {
                key: ss(i.to_string()),
                name: ss(name),
                sub: ss(if grid && kind == "v-folder" { what.clone() } else { sub }),
                count: ss(&what),
                has_cover: cover.is_some(),
                cover: cover.unwrap_or_default(),
            }
        })
        .collect();
    app.m.tiles.set_vec(tiles);
    app.m.lib_rows.set_vec(Vec::new());
}

fn set_tracks(app: &mut App, tracks: Vec<TrackInfo>) {
    let Some(ui) = app.ui() else { return };
    let l = ui.global::<Lib>();
    l.set_show_tiles(false);
    app.libv.sel.clear();
    app.libv.anchor = None;
    app.libv.tracks = tracks;
    rows(app);
}

fn rows(app: &mut App) {
    let playing = app.cur.as_ref().filter(|c| c.item != 0).map(|c| c.info.path.to_lowercase());
    let qpaths = crate::lists::queue_paths(app);
    let grid = app.s.track_grid;
    let rows: Vec<TrackRow> = app
        .libv
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let st = app.stats.get(&t.path);
            let mut r = crate::lists::track_row(t, i, (i + 1).to_string(), playing.as_deref() == Some(&t.path.to_lowercase()), app.libv.sel.contains(&i), st.rating >= 5, String::new());
            r.in_queue = qpaths.contains(&t.path.to_lowercase());
            crate::lists::with_rating(&mut r, st.rating);
            crate::lists::with_progress(&mut r, t, &st);
            if grid {
                if let Some(img) = crate::covers::get(&t.path) {
                    r.h_cover = img;
                    r.h_has_cover = true;
                }
            }
            if t.video && st.resume > 10.0 {
                let pct = if t.duration > 0.0 { format!("{}%", (st.resume / t.duration * 100.0).round()) } else { meta::fmt_time(st.resume) };
                r.info = ss(tr!("watched {}", pct));
            }
            r
        })
        .collect();
    // Seriale (lista wierszy): nagłówek nad każdym sezonem – „Serial · Sezon 1 · 8 odcinków”.
    let rows = if grid { rows } else { with_seasons(&app.libv.tracks, rows) };
    app.m.lib_rows.set_vec(rows);
}

fn with_seasons(tracks: &[TrackInfo], rows: Vec<TrackRow>) -> Vec<TrackRow> {
    let season = |t: &TrackInfo| -> Option<(String, u32)> {
        let p = std::path::Path::new(&t.path);
        if !(t.video || meta::is_video(p)) {
            return None;
        }
        let (s, _) = crate::lists::episode(&p.file_stem()?.to_string_lossy())?;
        Some((openmp_core::tmdb::guess(p).0, s))
    };
    let keys: Vec<Option<(String, u32)>> = tracks.iter().map(season).collect();
    if keys.iter().all(|k| k.is_none()) {
        return rows;
    }
    let mut out = Vec::with_capacity(rows.len() + 8);
    for (i, r) in rows.into_iter().enumerate() {
        if let Some(k) = &keys[i] {
            if i == 0 || keys[i - 1].as_ref() != Some(k) {
                let n = keys[i..].iter().take_while(|x| x.as_ref() == Some(k)).count();
                let name = if k.0.is_empty() { String::new() } else { format!("{}  ·  ", k.0) };
                out.push(TrackRow {
                    header: true,
                    idx: -1,
                    h_title: ss(format!("{name}{}", tr!("Season {}", k.1))),
                    h_sub: ss(trn!("{n} episode", "{n} episodes", n)),
                    ..Default::default()
                });
            }
        }
        out.push(r);
    }
    out
}

pub fn refresh_playing(app: &mut App) {
    let playing = app.cur.as_ref().filter(|c| c.item != 0).map(|c| c.info.path.to_lowercase());
    let qpaths = crate::lists::queue_paths(app);
    for r in 0..app.m.lib_rows.row_count() {
        let Some(mut row) = app.m.lib_rows.row_data(r) else { continue };
        if row.header {
            continue;
        }
        let path = app.libv.tracks.get(row.idx as usize).map(|t| t.path.to_lowercase());
        let p = path.is_some() && playing == path;
        let iq = path.as_ref().is_some_and(|p| qpaths.contains(p));
        if row.playing != p || row.in_queue != iq {
            row.playing = p;
            row.in_queue = iq;
            app.m.lib_rows.set_row_data(r, row);
        }
    }
}

/// Okładki w kafelkach utworów biblioteki (po włączeniu siatki – bez przebudowy widoku).
pub fn refresh_grid_covers(app: &mut App) {
    if !app.s.track_grid {
        return;
    }
    for r in 0..app.m.lib_rows.row_count() {
        let Some(mut row) = app.m.lib_rows.row_data(r) else { continue };
        if row.h_has_cover {
            continue;
        }
        let Some(p) = app.libv.tracks.get(row.idx as usize).map(|t| t.path.clone()) else { continue };
        if let Some(img) = crate::covers::get(&p) {
            row.h_cover = img;
            row.h_has_cover = true;
            app.m.lib_rows.set_row_data(r, row);
        }
    }
}

pub fn refresh_covers(app: &mut App, key: &str) {
    if app.s.track_grid {
        for r in 0..app.m.lib_rows.row_count() {
            let Some(mut row) = app.m.lib_rows.row_data(r) else { continue };
            if row.h_has_cover {
                continue;
            }
            let Some(p) = app.libv.tracks.get(row.idx as usize).map(|t| t.path.clone()) else { continue };
            if crate::covers::key(&p) == key {
                if let Some(img) = crate::covers::get(&p) {
                    row.h_cover = img;
                    row.h_has_cover = true;
                    app.m.lib_rows.set_row_data(r, row);
                }
            }
        }
    }
    for r in 0..app.m.tiles.row_count() {
        let Some(mut t) = app.m.tiles.row_data(r) else { continue };
        if t.has_cover {
            continue;
        }
        let Some((kind, name, sub)) = app.libv.tile_keys.get(r).cloned() else { continue };
        if !matches!(kind.as_str(), "album" | "artist" | "folder" | "v-folder" | "dir" | "v-dir" | "files" | "v-files") {
            continue;
        }
        let first = group_tracks_of(app, &kind, &name, &sub).first().map(|t| t.path.clone());
        if let Some(p) = first {
            if crate::covers::key(&p) == key {
                if let Some(img) = crate::covers::get(&p) {
                    t.cover = img;
                    t.has_cover = true;
                    app.m.tiles.set_row_data(r, t);
                }
            }
        }
    }
    let _ = Image::default();
}

pub fn tile_clicked(app: &mut App, key: &str) {
    let Ok(i) = key.parse::<usize>() else { return };
    let Some((kind, name, sub)) = app.libv.tile_keys.get(i).cloned() else { return };
    app.libv.stack.push((kind, name, sub));
    show(app);
}

/// Pliki kafelka; folder w drzewie obejmuje też wszystkie podfoldery.
fn group_tracks_of(app: &App, kind: &str, name: &str, sub: &str) -> Vec<TrackInfo> {
    let video = matches!(kind, "v-folder" | "v-dir" | "v-files");
    let src = if video {
        &app.lib.videos
    } else if app.libv.kind == "artist" && !app.libv.stack.is_empty() {
        &app.libv.tracks
    } else {
        &app.lib.tracks
    };
    let mut v: Vec<TrackInfo> = match kind {
        "dir" | "v-dir" => library::in_tree(src, name).into_iter().cloned().collect(),
        "files" | "v-files" | "v-folder" => library::in_group(src, "folder", name, sub).into_iter().cloned().collect(),
        _ => library::in_group(src, kind, name, sub).into_iter().cloned().collect(),
    };
    if matches!(kind, "dir" | "v-dir" | "files" | "v-files" | "v-folder") {
        v.sort_by_key(|t| natural(&t.path));
    }
    v
}

fn group_tracks(app: &App, key: &str) -> Vec<TrackInfo> {
    let Ok(i) = key.parse::<usize>() else { return Vec::new() };
    let Some((kind, name, sub)) = app.libv.tile_keys.get(i).cloned() else { return Vec::new() };
    group_tracks_of(app, &kind, &name, &sub)
}

pub fn tile_double(app: &mut App, key: &str) {
    let t = group_tracks(app, key);
    play_tracks(app, t, 0, false);
}

pub fn tile_menu(app: &mut App, key: &str, x: f32, y: f32) {
    let items = vec![
        crate::menu::item("t-play", tr!("Play"), "play", ""),
        crate::menu::item("t-shuffle", tr!("Shuffle play"), "shuffle", ""),
        crate::menu::item("t-add", tr!("Add to the current list"), "add", ""),
        crate::menu::item("t-new", tr!("New list from this"), "playlist", ""),
        crate::menu::sep(),
        crate::menu::item("t-open", tr!("Open"), "chevron-right", ""),
    ];
    crate::menu::open(app, MenuCtx::Tile(key.to_string()), items, x, y);
}

pub fn tile_command(app: &mut App, key: &str, cmd: &str) {
    let t = group_tracks(app, key);
    match cmd {
        "t-play" => play_tracks(app, t, 0, false),
        "t-shuffle" => play_tracks(app, t, 0, true),
        "t-add" => add_to_active(app, t),
        "t-new" => {
            let name = app.libv.tile_keys.get(key.parse::<usize>().unwrap_or(usize::MAX)).map(|k| k.1.clone()).unwrap_or_else(|| tr!("List").into());
            let name = std::path::Path::new(&name).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(name);
            let id = crate::lists::create_list(app, &name);
            if let Some(li) = app.list_index(id) {
                app.store.lists[li].items = t;
            }
            crate::lists::rebuild(app);
            rebuild_nav(app);
        }
        "t-open" => tile_clicked(app, key),
        _ => {}
    }
}

pub fn back(app: &mut App) {
    if app.libv.stack.pop().is_some() {
        show(app);
    }
}

/// Granie z biblioteki: utwory (i filmy) są dopisywane na koniec karty „Kolejka” – bieżące
/// odtwarzanie – i od razu gra wybrany; następny/poprzedni idą dalej po kolejce.
pub fn play_tracks(app: &mut App, tracks: Vec<TrackInfo>, start: usize, shuffle: bool) {
    play_tracks_at(app, tracks, start, shuffle, None);
}

/// Jak `play_tracks`, z początkiem od podanej sekundy (klik w wiersz tekstu w zaznaczeniu).
pub fn play_tracks_at(app: &mut App, mut tracks: Vec<TrackInfo>, start: usize, shuffle: bool, pos: Option<f64>) {
    if tracks.is_empty() {
        return;
    }
    let mut start = start.min(tracks.len() - 1);
    if shuffle {
        use rand::seq::SliceRandom;
        tracks.shuffle(&mut rand::rng());
        start = 0;
    }
    let id = crate::lists::queue_list(app);
    let li = app.list_index(id).unwrap();
    // Dopisane na koniec – to, co już było w kolejce, zostaje.
    let base = app.store.lists[li].items.len();
    app.store.lists[li].items.extend(tracks);
    app.shuffle_len = 0;
    app.dirty = true;
    crate::lists::fix_current(app, id);
    if app.store.active == li {
        app.sel.clear();
    }
    crate::lists::rebuild(app);
    rebuild_nav(app);
    crate::player::play(app, id, base + start, pos);
}

fn add_to_active(app: &mut App, tracks: Vec<TrackInfo>) {
    let n = tracks.len();
    let li = app.store.active;
    app.store.lists[li].items.extend(tracks);
    app.dirty = true;
    crate::lists::rebuild(app);
    rebuild_nav(app);
    crate::player::refresh_next(app);
    let name = app.store.lists[li].name.clone();
    app.toast(tr!("Added {} to the list “{}”", n, name));
}

/// Zaznaczenie prostokątem w siatce okładek.
pub fn row_band(app: &mut App, band: (i32, i32, i32, i32, i32), ctrl: bool) {
    let items = crate::lists::band_items(&app.m.lib_rows, band);
    app.libv.sel.clear();
    if ctrl {
        if let Some(a) = app.libv.anchor {
            app.libv.sel.insert(a);
        }
    }
    app.libv.sel.extend(items);
    for r in 0..app.m.lib_rows.row_count() {
        let Some(mut row) = app.m.lib_rows.row_data(r) else { continue };
        let s = app.libv.sel.contains(&(row.idx as usize));
        if row.selected != s {
            row.selected = s;
            app.m.lib_rows.set_row_data(r, row);
        }
    }
}

pub fn row_clicked(app: &mut App, i: usize, ctrl: bool, shift: bool) {
    if shift {
        let a = app.libv.anchor.unwrap_or(i);
        if !ctrl {
            app.libv.sel.clear();
        }
        for k in a.min(i)..=a.max(i) {
            app.libv.sel.insert(k);
        }
    } else if ctrl {
        if !app.libv.sel.remove(&i) {
            app.libv.sel.insert(i);
        }
        app.libv.anchor = Some(i);
    } else {
        app.libv.sel.clear();
        app.libv.sel.insert(i);
        app.libv.anchor = Some(i);
    }
    for r in 0..app.m.lib_rows.row_count() {
        let Some(mut row) = app.m.lib_rows.row_data(r) else { continue };
        let s = app.libv.sel.contains(&(row.idx as usize));
        if row.selected != s {
            row.selected = s;
            app.m.lib_rows.set_row_data(r, row);
        }
    }
}

pub fn row_double(app: &mut App, i: usize) {
    let t = app.libv.tracks.clone();
    play_tracks(app, t, i, false);
}

pub fn selected(app: &App) -> Vec<TrackInfo> {
    let mut v: Vec<usize> = app.libv.sel.iter().copied().collect();
    v.sort();
    v.into_iter().filter_map(|i| app.libv.tracks.get(i).cloned()).collect()
}

pub fn row_menu(app: &mut App, x: f32, y: f32) {
    let n = app.libv.sel.len();
    let items = vec![
        crate::menu::item("l-play", tr!("Play"), "play", "Enter"),
        crate::menu::item("l-add", tr!("Add to the current list"), "add", ""),
        crate::menu::item("l-new", tr!("New list from selection"), "playlist", ""),
        crate::menu::sep(),
        crate::menu::item("l-fav", tr!("Favorite (toggle)"), "heart", ""),
        crate::menu::item_en("l-tags", tr!("Edit tags…"), "edit", "", n > 0),
        crate::menu::item("l-explorer", tr!("Show in Explorer"), "folder-open", ""),
        crate::menu::sep(),
        crate::menu::item("l-resume-reset", tr!("Forget the resume position"), "history", ""),
        crate::menu::item("l-watched", tr!("Watched / unwatched (toggle)"), "check", ""),
        crate::menu::item_en("l-tmdb", tr!("Fix TMDB match…"), "edit", "", n == 1 && !app.s.tmdb_key.is_empty()),
        crate::menu::item_en("l-save-smart", tr!("Save search as a smart playlist"), "playlist", "", !app.libv.search.trim().is_empty()),
    ];
    crate::menu::open(app, MenuCtx::Lib, items, x, y);
}

pub fn command(app: &mut App, cmd: &str) {
    let sel = selected(app);
    match cmd {
        "l-play" => {
            let first = app.libv.sel.iter().min().copied().unwrap_or(0);
            if sel.len() > 1 {
                play_tracks(app, sel, 0, false);
            } else {
                row_double(app, first);
            }
        }
        "l-add" => add_to_active(app, sel),
        "l-new" => {
            let id = crate::lists::create_list(app, &tr!("List {}", app.store.lists.len() + 1));
            if let Some(li) = app.list_index(id) {
                app.store.lists[li].items = sel;
            }
            crate::lists::rebuild(app);
            rebuild_nav(app);
        }
        "l-fav" => {
            let all = sel.iter().all(|t| app.stats.get(&t.path).rating >= 5);
            for t in &sel {
                app.stats.entry(&t.path).rating = if all { 0 } else { 5 };
            }
            app.dirty = true;
            rows(app);
            rebuild_nav(app);
        }
        "l-tags" => {
            let p = sel.into_iter().map(|t| t.path).collect();
            crate::dialogs::open_tag_editor(app, p);
        }
        "l-explorer" => {
            if let Some(t) = sel.first() {
                crate::win::reveal(&t.path);
            }
        }
        "l-save-smart" => {
            let q = app.libv.search.trim().to_string();
            if !q.is_empty() && !app.s.smart_lists.iter().any(|(_, s)| s == &q) {
                app.s.smart_lists.push((q.clone(), q.clone()));
                app.dirty = true;
                rebuild_nav(app);
                app.toast(tr!("Smart playlist “{}” saved", q));
            }
        }
        "l-tmdb" => {
            if let Some(t) = sel.first() {
                let q = openmp_core::tmdb::query_of(std::path::Path::new(&t.path));
                crate::dialogs::rename(app, RenameCtx::Tmdb(t.path.clone()), tr!("Movie title on TMDB (with the year, e.g. Angels & Demons 2009)"), &q);
            }
        }
        "l-watched" => {
            // Wszystkie zaznaczone obejrzane – odznaczamy, inaczej oznaczamy wszystkie.
            let all = sel.iter().all(|t| app.stats.get(&t.path).watched);
            for t in &sel {
                let st = app.stats.entry(&t.path);
                st.watched = !all;
                if !all {
                    st.resume = 0.0;
                }
            }
            app.dirty = true;
            show(app);
            app.toast(if all { tr!("Marked as unwatched") } else { tr!("Marked as watched") });
        }
        "l-resume-reset" => {
            for t in &sel {
                app.stats.entry(&t.path).resume = 0.0;
            }
            app.dirty = true;
            show(app);
        }
        _ => {}
    }
}

pub fn toggle_fav_row(app: &mut App, i: usize) {
    if let Some(t) = app.libv.tracks.get(i).cloned() {
        let s = app.stats.entry(&t.path);
        s.rating = if s.rating >= 5 { 0 } else { 5 };
        app.dirty = true;
        rows(app);
        rebuild_nav(app);
    }
}

/// Skanowanie bibliotek muzyki i wideo w tle.
pub fn start_scan(app: &mut App) {
    if app.scan.is_some() {
        return;
    }
    let music = app.s.library_folders.clone();
    let video = app.s.video_folders.clone();
    let old_m = app.lib.tracks.clone();
    let old_v = app.lib.videos.clone();
    let prog = Arc::new(Progress::default());
    app.scan = Some(prog.clone());
    if let Some(ui) = app.ui() {
        ui.global::<Lib>().set_scanning(true);
        ui.global::<Lib>().set_scan_text(ss(tr!("Looking for files…")));
        ui.global::<Lib>().set_scan_progress(0.0);
    }
    std::thread::spawn(move || {
        let tracks = library::scan(&music, &old_m, &prog, false);
        prog.found.store(0, Ordering::Relaxed);
        prog.done.store(0, Ordering::Relaxed);
        let videos = library::scan(&video, &old_v, &prog, true);
        let lib = openmp_core::library::Library { tracks, videos, scanned: library::now() };
        lib.save();
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                let (nt, nv) = (lib.tracks.len(), lib.videos.len());
                app.lib = lib;
                app.scan = None;
                if let Some(ui) = app.ui() {
                    ui.global::<Lib>().set_scanning(false);
                }
                rebuild_nav(app);
                if app.ui().map(|u| u.global::<Lib>().get_view() == 1).unwrap_or(false) {
                    show(app);
                }
                app.toast(tr!("Library: {}, {}", trn!("{n} track", "{n} tracks", nt), trn!("{n} video", "{n} videos", nv)));
            });
        });
    });
}

/// Obserwuje foldery biblioteki – nowe, usunięte i przeniesione pliki trafiają do biblioteki same.
pub fn watch(app: &mut App) {
    use notify::{EventKind, RecursiveMode, Watcher};
    app.watch = None;
    let changed = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let c = changed.clone();
    let w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        let media = ev.paths.iter().any(|p| p.is_dir() || p.extension().is_none() || meta::is_media(p));
        if media && matches!(ev.kind, EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))) {
            c.store(library::now_ms(), Ordering::Relaxed);
        }
    });
    let Ok(mut w) = w else { return };
    for f in app.s.library_folders.iter().chain(app.s.video_folders.iter()) {
        // Udział sieciowy bywa niedostępny – wtedy obserwujemy resztę.
        let _ = w.watch(std::path::Path::new(f), RecursiveMode::Recursive);
    }
    app.watch = Some((w, changed));
}

/// Po 3 s ciszy od zmian na dysku – ponowne skanowanie (z pamięci podręcznej tagów, szybkie).
fn watch_tick(app: &mut App) {
    let Some(c) = app.watch.as_ref().map(|w| w.1.clone()) else { return };
    let t = c.load(Ordering::Relaxed);
    if t == 0 || app.scan.is_some() || library::now_ms().saturating_sub(t) < 3000 {
        return;
    }
    c.store(0, Ordering::Relaxed);
    start_scan(app);
}

/// Postęp skanowania – wołane z zegara.
pub fn scan_tick(app: &mut App) {
    watch_tick(app);
    let Some(p) = app.scan.clone() else { return };
    let (found, done) = (p.found.load(Ordering::Relaxed), p.done.load(Ordering::Relaxed));
    if let Some(ui) = app.ui() {
        let l = ui.global::<Lib>();
        if found > 0 {
            l.set_scan_text(ss(tr!("Scanning library: {} / {}", done, found)));
            l.set_scan_progress(done as f32 / found as f32);
        }
    }
}
