//! Sortowanie list utworów w bibliotece: każdy widok ma swoje domyślne (np. „Ostatnio dodane”
//! – wg daty pliku), a wybór użytkownika jest zapamiętany osobno dla widoku.

use openmp_core::meta::TrackInfo;

use crate::app::*;
use crate::*;

/// 0 – kolejność widoku (bez sortowania), dalej kolumny jak w `names`.
fn names() -> Vec<String> {
    vec![
        tr!("Default order").into(),
        tr!("Title").into(),
        tr!("Artist").into(),
        tr!("Album").into(),
        tr!("Year").into(),
        tr!("Length").into(),
        tr!("File date").into(),
        tr!("Plays").into(),
        tr!("Last played").into(),
        tr!("Rating").into(),
        tr!("Size").into(),
    ]
}

/// Domyślne sortowanie widoku – to, wg którego widok i tak jest ułożony.
fn default_for(key: &str) -> i32 {
    match key {
        "recent" | "v-recent" => 6,
        "top" => 7,
        "history" | "fav" | "v-continue" => 8,
        "rated" => 9,
        _ => 0,
    }
}

fn current(app: &App) -> (String, i32) {
    let k = crate::columns::key(app, false);
    let s = app.s.sorts.get(&k).copied().unwrap_or_else(|| default_for(&k));
    (k, s)
}

/// Układa utwory wg sortowania bieżącego widoku (0 i domyślne – kolejność widoku zostaje).
pub fn apply(app: &App, t: &mut [TrackInfo]) {
    let (k, s) = current(app);
    if s == 0 || s == default_for(&k) {
        return;
    }
    let low = |x: &str| x.to_lowercase();
    match s {
        1 => t.sort_by_cached_key(|t| low(&t.display_title())),
        2 => t.sort_by_cached_key(|t| (low(&t.display_artist()), low(&t.album), t.disc, t.track)),
        3 => t.sort_by_cached_key(|t| (low(&t.album), t.disc, t.track)),
        4 => t.sort_by_key(|t| std::cmp::Reverse(t.year)),
        5 => t.sort_by(|a, b| b.duration.total_cmp(&a.duration)),
        6 => t.sort_by_key(|t| std::cmp::Reverse(t.modified)),
        7 => t.sort_by_cached_key(|t| std::cmp::Reverse(app.stats.get(&t.path).plays)),
        8 => t.sort_by_cached_key(|t| std::cmp::Reverse(app.stats.get(&t.path).last)),
        9 => t.sort_by_cached_key(|t| std::cmp::Reverse(app.stats.get(&t.path).rating)),
        10 => t.sort_by_key(|t| std::cmp::Reverse(t.size)),
        _ => {}
    }
}

/// Napis na przełączniku: bieżące sortowanie widoku.
pub fn publish(app: &App) {
    let Some(ui) = app.ui() else { return };
    let (_, s) = current(app);
    let n = names();
    ui.global::<Lib>().set_sort_label(ss(n.get(s as usize).cloned().unwrap_or_default()));
}

pub fn menu(app: &mut App, x: f32, y: f32) {
    let (k, s) = current(app);
    let def = default_for(&k);
    let items: Vec<_> = names()
        .into_iter()
        .enumerate()
        // „Kolejność widoku” ma sens tylko tam, gdzie domyślne nie jest już inną kolumną.
        .filter(|(i, _)| *i != 0 || def == 0)
        .map(|(i, n)| {
            let label = if i as i32 == def && def != 0 { format!("{n}  ·  {}", tr!("default")) } else { n };
            crate::menu::check(&format!("sort-{i}"), &label, s == i as i32, "")
        })
        .collect();
    crate::menu::open(app, MenuCtx::Sort, items, x, y);
}

pub fn command(app: &mut App, id: &str) {
    let Some(s) = id.strip_prefix("sort-").and_then(|s| s.parse::<i32>().ok()) else { return };
    let k = crate::columns::key(app, false);
    if s == default_for(&k) {
        app.s.sorts.remove(&k);
    } else {
        app.s.sorts.insert(k, s);
    }
    app.s.save();
    crate::libview::show(app);
}
