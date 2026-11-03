//! Kolekcje AI: ręcznie uruchamiana analiza biblioteki (język, nastrój, tematy, energia)
//! i listy tworzone z opisu. Wszystko na kluczu API użytkownika – bez klucza sekcji nie ma.

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use openmp_core::ai::{self, Config, Plan, TrackTags};
use openmp_core::library;
use openmp_core::meta::TrackInfo;
use slint::{Model, ModelRc, VecModel};

use crate::app::*;
use crate::*;

/// Analiza w toku i prośba o jej przerwanie.
static RUNNING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Utworów w jednym zapytaniu – mniej zapytań, a odpowiedź wciąż krótka.
const BATCH: usize = 50;

pub fn enabled(app: &App) -> bool {
    !app.s.ai_key.trim().is_empty() || (app.s.ai_provider == 2 && !app.s.ai_url.trim().is_empty())
}

/// Sekcja w panelu: gdy AI jest ustawione albo zostały wyniki wcześniejszej analizy.
pub fn visible(app: &App) -> bool {
    enabled(app) || !app.ai_tags.map.is_empty()
}

fn config(app: &App) -> Config {
    Config {
        provider: app.s.ai_provider,
        key: app.s.ai_key.clone(),
        url: app.s.ai_url.clone(),
        model: app.s.ai_model.clone(),
        temperature: app.s.ai_temperature,
    }
}

fn set_status(app: &App, text: &str) {
    if let Some(ui) = app.ui() {
        ui.global::<Prefs>().set_ai_status(ss(text));
    }
}

/// Licznik w ustawieniach: ile utworów muzycznych ma już wynik analizy.
pub fn publish(app: &App) {
    let Some(ui) = app.ui() else { return };
    let p = ui.global::<Prefs>();
    let music: Vec<&TrackInfo> = app.lib.tracks.iter().filter(|t| !t.video).collect();
    let done = music.iter().filter(|t| app.ai_tags.has(&t.path)).count();
    p.set_ai_count(ss(tr!("Analyzed {} of {} tracks", done, music.len())));
    p.set_ai_busy(RUNNING.load(Ordering::SeqCst));
    if p.get_ai_models().row_count() == 0 && !app.s.ai_model.is_empty() {
        p.set_ai_models(ModelRc::new(VecModel::from(vec![ss(&app.s.ai_model)])));
    }
}

pub fn fetch_models(app: &mut App) {
    if !enabled(app) {
        set_status(app, tr!("Enter the API key first."));
        return;
    }
    set_status(app, tr!("Loading models…"));
    let cfg = config(app);
    std::thread::spawn(move || {
        let res = ai::list_models(&cfg);
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(models) => {
                    if app.s.ai_model.is_empty() || !models.contains(&app.s.ai_model) {
                        app.s.ai_model = models[0].clone();
                        app.s.save();
                    }
                    if let Some(ui) = app.ui() {
                        let p = ui.global::<Prefs>();
                        let list: Vec<slint::SharedString> = models.iter().map(ss).collect();
                        p.set_ai_models(ModelRc::new(VecModel::from(list)));
                        p.set_ai_model(ss(&app.s.ai_model));
                    }
                    set_status(app, &trn!("{n} model available", "{n} models available", models.len()));
                }
                Err(e) => set_status(app, &tr!("Could not load models: {}", e)),
            });
        });
    });
}

/// Analiza utworów, które jeszcze nie mają wyniku (albo wszystkich – `again`).
pub fn analyze(app: &mut App, again: bool) {
    if RUNNING.load(Ordering::SeqCst) {
        return;
    }
    if !enabled(app) || app.s.ai_model.trim().is_empty() {
        set_status(app, tr!("Enter the API key and choose a model first."));
        return;
    }
    let todo: Vec<TrackInfo> = app.lib.tracks.iter().filter(|t| !t.video && (again || !app.ai_tags.has(&t.path))).cloned().collect();
    if todo.is_empty() {
        set_status(app, tr!("Everything is already analyzed."));
        return;
    }
    RUNNING.store(true, Ordering::SeqCst);
    CANCEL.store(false, Ordering::SeqCst);
    publish(app);
    let total = todo.len();
    set_status(app, &tr!("Analyzing… 0 of {}", total));
    let cfg = config(app);
    std::thread::spawn(move || {
        let mut done = 0;
        let mut fails = 0;
        for chunk in todo.chunks(BATCH) {
            if CANCEL.load(Ordering::SeqCst) {
                break;
            }
            let res = ai::analyze(&cfg, chunk);
            let found: Vec<(String, TrackTags)> = match &res {
                Ok(v) => v.iter().map(|(i, t)| (chunk[*i].path.to_lowercase(), t.clone())).collect(),
                Err(_) => Vec::new(),
            };
            done += chunk.len();
            let err = res.err();
            fails = if err.is_some() { fails + 1 } else { 0 };
            let stop = fails >= 3;
            let _ = slint::invoke_from_event_loop(move || {
                later(move |app| {
                    app.ai_tags.map.extend(found);
                    app.ai_tags.save();
                    publish(app);
                    match err {
                        Some(e) if stop => set_status(app, &tr!("Analysis stopped: {}", e)),
                        Some(e) => set_status(app, &tr!("Error, continuing: {}", e)),
                        None => set_status(app, &tr!("Analyzing… {} of {}", done, total)),
                    }
                });
            });
            if stop {
                break;
            }
        }
        RUNNING.store(false, Ordering::SeqCst);
        let cancelled = CANCEL.load(Ordering::SeqCst);
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                publish(app);
                if cancelled {
                    set_status(app, tr!("Analysis stopped."));
                } else if done >= total {
                    set_status(app, tr!("Analysis finished."));
                    app.toast(tr!("Library analysis finished"));
                }
                crate::libview::rebuild_nav(app);
                if app.libv.kind.starts_with("ai") {
                    crate::libview::show(app);
                }
            });
        });
    });
}

pub fn stop(_app: &mut App) {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Usuwa wszystkie wyniki analizy.
pub fn forget(app: &mut App) {
    if RUNNING.load(Ordering::SeqCst) {
        return;
    }
    app.ai_tags.map.clear();
    app.ai_tags.save();
    publish(app);
    set_status(app, tr!("Analysis results removed."));
    crate::libview::rebuild_nav(app);
}

/// Nazwa nastroju w języku interfejsu.
pub fn mood_label(m: &str) -> String {
    match m {
        "calm" => trc!("mood", "calm"),
        "happy" => trc!("mood", "happy"),
        "sad" => trc!("mood", "sad"),
        "energetic" => trc!("mood", "energetic"),
        "romantic" => trc!("mood", "romantic"),
        "melancholic" => trc!("mood", "melancholic"),
        "angry" => trc!("mood", "angry"),
        "dreamy" => trc!("mood", "dreamy"),
        "nostalgic" => trc!("mood", "nostalgic"),
        "dark" => trc!("mood", "dark"),
        "uplifting" => trc!("mood", "uplifting"),
        "party" => trc!("mood", "party"),
        "focused" => trc!("mood", "focused"),
        "epic" => trc!("mood", "epic"),
        other => other,
    }
    .to_string()
}

fn energy_label(e: u8) -> String {
    match e {
        1 => tr!("Calm"),
        2 => tr!("Moderate"),
        _ => tr!("Energetic"),
    }
    .to_string()
}

/// Wartości grupy utworu: język, nastroje, tematy albo energia (klucze – nazwy kafelków).
fn values(kind: &str, t: &TrackTags) -> Vec<String> {
    match kind {
        "ai-lang" => (!t.lang.is_empty()).then(|| t.lang.clone()).into_iter().collect(),
        "ai-mood" => t.moods.iter().map(|m| mood_label(m)).collect(),
        "ai-theme" => t.themes.clone(),
        _ => (t.energy > 0).then(|| energy_label(t.energy)).into_iter().collect(),
    }
}

/// Kafelki kolekcji AI: od najliczniejszych; tematy – tylko powtarzające się.
pub fn groups(app: &App, source: &[TrackInfo], kind: &str) -> Vec<library::Group> {
    let mut map: BTreeMap<String, library::Group> = BTreeMap::new();
    for t in source.iter().filter(|t| !t.video) {
        let Some(tags) = app.ai_tags.get(&t.path) else { continue };
        for v in values(kind, tags) {
            let g = map.entry(v.clone()).or_insert_with(|| library::Group { name: v, first: t.path.clone(), ..Default::default() });
            g.count += 1;
            g.duration += t.duration;
        }
    }
    let mut v: Vec<library::Group> = map.into_values().collect();
    if kind == "ai-theme" {
        v.retain(|g| g.count >= 2);
    }
    v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    v
}

pub fn in_group(app: &App, source: &[TrackInfo], kind: &str, name: &str) -> Vec<TrackInfo> {
    let mut v: Vec<TrackInfo> = source.iter().filter(|t| !t.video && app.ai_tags.get(&t.path).is_some_and(|g| values(kind, g).iter().any(|x| x == name))).cloned().collect();
    v.sort_by(library::album_order);
    v
}

/// Utwory listy z opisu: najlepiej pasujące, w kolejności dopasowania.
pub fn plan_tracks(app: &App, source: &[TrackInfo], plan: &Plan) -> Vec<TrackInfo> {
    let mut v: Vec<(f32, &TrackInfo)> = source.iter().filter(|t| !t.video).map(|t| (ai::score(plan, t, app.ai_tags.get(&t.path)), t)).filter(|x| x.0 > 0.0).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().take(plan.limit.max(5)).map(|x| x.1.clone()).collect()
}

/// Nowa lista z opisu (okno z polem tekstowym).
pub fn ask_new(app: &mut App) {
    if !enabled(app) || app.s.ai_model.trim().is_empty() {
        app.toast(tr!("Set up AI in Settings → AI first"));
        crate::keys::open_settings(app, SETTINGS_PAGE);
        return;
    }
    crate::dialogs::rename(app, RenameCtx::AiList, tr!("Describe the playlist, e.g. calm songs about the sea for the evening"), "");
}

/// Strona „AI” w ustawieniach.
pub const SETTINGS_PAGE: i32 = 11;

pub fn create_list(app: &mut App, description: String) {
    let cfg = config(app);
    // Słownik z biblioteki – model trafia w istniejące języki, tematy i gatunki.
    let mut langs: HashSet<String> = HashSet::new();
    let mut themes: BTreeMap<String, usize> = BTreeMap::new();
    for t in app.ai_tags.map.values() {
        if !t.lang.is_empty() {
            langs.insert(t.lang.clone());
        }
        for th in &t.themes {
            *themes.entry(th.clone()).or_default() += 1;
        }
    }
    let mut themes: Vec<(String, usize)> = themes.into_iter().collect();
    themes.sort_by(|a, b| b.1.cmp(&a.1));
    let themes: Vec<String> = themes.into_iter().map(|x| x.0).collect();
    let langs: Vec<String> = langs.into_iter().collect();
    let genres: Vec<String> = library::group_by(&app.lib.tracks, "genre").into_iter().map(|g| g.name).filter(|g| g != "(nieznane)").collect();
    app.toast(tr!("Creating the playlist…"));
    std::thread::spawn(move || {
        let res = ai::plan(&cfg, &description, &langs, &themes, &genres);
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(plan) => {
                    app.s.ai_lists.push(plan);
                    app.s.save();
                    let id = format!("ai:{}", app.s.ai_lists.len() - 1);
                    crate::libview::rebuild_nav(app);
                    crate::libview::nav_clicked(app, &id);
                }
                Err(e) => app.toast(tr!("Could not create the playlist: {}", e)),
            });
        });
    });
}
