//! Dodatkowe kolumny list utworów: osobny wybór dla każdego widoku (listy odtwarzania,
//! foldery, albumy, zbiory, filmy…), zapamiętany w ustawieniach.

use openmp_core::library::Stat;
use openmp_core::meta::TrackInfo;
use slint::{ModelRc, SharedString, VecModel};

use crate::app::*;
use crate::*;

/// Numery kolumn – kolejność jak w `TrackList` (proporcje i wyrównanie).
const COUNT: usize = 12;

pub fn names() -> Vec<String> {
    vec![
        tr!("Artist").into(),
        tr!("Album").into(),
        tr!("Genre").into(),
        tr!("Year").into(),
        tr!("Format").into(),
        tr!("Size").into(),
        tr!("Plays").into(),
        tr!("Rating").into(),
        tr!("Last played").into(),
        tr!("File date").into(),
        tr!("Composer").into(),
        tr!("Folder").into(),
    ]
}

/// Klucz widoku: listy odtwarzania mają jeden wspólny, biblioteka – rodzaj widoku
/// (a w głębi: rodzaj grupy, np. album wykonawcy, folder, serial).
pub fn key(app: &App, playlist: bool) -> String {
    if playlist {
        return "list".into();
    }
    let kind = app.libv.kind.as_str();
    match app.libv.stack.last() {
        _ if kind.starts_with("root:") => "folder".into(),
        _ if kind.starts_with("smart:") => "smart".into(),
        _ if kind.starts_with("pod:") => "podcast".into(),
        Some((g, _, _)) => format!("{kind}>{g}"),
        None => kind.to_string(),
    }
}

fn default_for(key: &str) -> Vec<i32> {
    if key.starts_with("v-") || key == "podcast" {
        vec![0, 4]
    } else {
        vec![0, 1, 4]
    }
}

pub fn get(app: &App, key: &str) -> Vec<i32> {
    app.s.columns.get(key).cloned().unwrap_or_else(|| default_for(key))
}

fn date(t: u64) -> String {
    if t == 0 {
        return String::new();
    }
    // Dzień od 1970 → rok-miesiąc-dzień (kalendarz gregoriański).
    let z = (t / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y}-{m:02}-{d:02}")
}

fn size(b: u64) -> String {
    match b {
        0 => String::new(),
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b => format!("{} KB", (b + 1023) / 1024),
    }
}

/// Wartości wszystkich kolumn wiersza (pokazywane są tylko wybrane).
pub fn values(t: &TrackInfo, st: &Stat, artist: &str, info: &str) -> ModelRc<SharedString> {
    let folder = std::path::Path::new(&t.path).parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let v: [String; COUNT] = [
        artist.to_string(),
        t.album.clone(),
        t.genre.clone(),
        if t.year > 0 { t.year.to_string() } else { String::new() },
        info.to_string(),
        size(t.size),
        if st.plays > 0 { st.plays.to_string() } else { String::new() },
        if st.rating > 0 { crate::lists::stars(st.rating) } else { String::new() },
        date(st.last),
        date(t.modified),
        t.composer.clone(),
        folder,
    ];
    ModelRc::new(VecModel::from(v.into_iter().map(SharedString::from).collect::<Vec<_>>()))
}

fn ints(v: &[i32]) -> ModelRc<i32> {
    ModelRc::new(VecModel::from(v.to_vec()))
}

/// Kolumny bieżących widoków do interfejsu (po zmianie widoku albo wyboru).
pub fn publish(app: &App) {
    let Some(ui) = app.ui() else { return };
    let names: Vec<SharedString> = names().into_iter().map(SharedString::from).collect();
    let l = ui.global::<Lib>();
    l.set_col_names(ModelRc::new(VecModel::from(names)));
    l.set_columns(ints(&get(app, &key(app, false))));
    ui.global::<Lists>().set_columns(ints(&get(app, &key(app, true))));
}

/// Menu wyboru kolumn: zaznaczone są pokazywane, w kolejności z tego menu.
pub fn menu(app: &mut App, playlist: bool, x: f32, y: f32) {
    let on = get(app, &key(app, playlist));
    let mut items: Vec<_> = names().iter().enumerate().map(|(i, n)| crate::menu::check(&format!("col-{i}"), n, on.contains(&(i as i32)), "")).collect();
    items.push(crate::menu::sep());
    items.push(crate::menu::item("col-reset", tr!("Default columns"), "refresh", ""));
    crate::menu::open(app, MenuCtx::Columns(playlist), items, x, y);
}

pub fn command(app: &mut App, playlist: bool, id: &str) {
    let k = key(app, playlist);
    if id == "col-reset" {
        app.s.columns.remove(&k);
    } else if let Some(c) = id.strip_prefix("col-").and_then(|s| s.parse::<i32>().ok()) {
        let mut v = get(app, &k);
        if let Some(p) = v.iter().position(|x| *x == c) {
            v.remove(p);
        } else {
            v.push(c);
            v.sort();
        }
        app.s.columns.insert(k, v);
    }
    app.s.save();
    publish(app);
}
