//! OpenMP – Open Media Player: odtwarzacz muzyki i wideo dla Windows.
//! Pomysł i wykonanie: Adam Rędzikowski · redzikowski.com

#![windows_subsystem = "windows"]

#[macro_use]
extern crate openmp_core;

slint::include_modules!();

mod app;
mod cast;
mod columns;
mod covers;
mod dialogs;
mod dialogwin;
mod display;
mod keys;
mod libview;
mod lists;
mod menu;
mod player;
mod prefs;
mod radio;
mod sleep;
mod sorting;
mod statsview;
mod video;
mod vizpaths;
mod wave;
mod win;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use app::*;
use openmp_core::audio::engine::Engine;
use openmp_core::audio::viz::Analyzer;
use openmp_core::library::{Library, Stats};
use openmp_core::playlist::Store;
use openmp_core::settings::{Session, Settings};
use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, Model, VecModel};

thread_local! {
    static DROPS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Język interfejsu: kod z ustawień albo – gdy pusty – język Windows. Teksty Slint (`@tr`)
/// i teksty z kodu (`tr!`) biorą tłumaczenia z tego samego pliku `lang/<kod>/LC_MESSAGES/openmp.po`.
pub fn apply_language(code: &str) {
    let want = if code.is_empty() { win::system_language() } else { code.to_string() };
    let got = openmp_core::i18n::set_language(&want);
    if let Err(e) = slint::select_bundled_translation(got) {
        openmp_core::paths::log(&format!("tłumaczenie {got}: {e}"));
    }
}

/// Poprzednie uruchomienie skończyło się awarią: raport trafia do `crash-last.txt` (obok dziennika
/// `openmp.log`), a program mówi, gdzie go znaleźć i otwiera folder, żeby można go było przesłać.
fn crash_report(app: &mut App) {
    let dir = openmp_core::paths::cache();
    let fresh = dir.join("crash.txt");
    if !fresh.exists() {
        return;
    }
    let last = dir.join("crash-last.txt");
    let _ = std::fs::remove_file(&last);
    if std::fs::rename(&fresh, &last).is_ok() {
        app.toast(tr!("OpenMP closed unexpectedly last time. The report is in {} – you can send it to the author.", last.display()));
        win::reveal(&last.to_string_lossy());
    }
}

fn main() {
    // Awaria w dowolnym wątku trafia do dziennika – bez konsoli komunikat inaczej by zniknął.
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        openmp_core::paths::log(&format!("PANIC {info}
{bt}"));
        // Raport do pokazania przy następnym uruchomieniu (dopisywany, gdy pada kilka wątków).
        let report = format!("OpenMP {} · Windows {}\n\n{info}\n\n{bt}\n", openmp_core::version(), win::system_language());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(openmp_core::paths::cache().join("crash.txt")) {
            use std::io::Write;
            let _ = f.write_all(report.as_bytes());
        }
    }));
    let argv: Vec<String> = std::env::args().skip(1).collect();
    openmp_core::paths::log(&format!("start {} {:?}", openmp_core::version(), argv));
    let add_only = argv.iter().any(|a| a == "--add");
    let files: Vec<String> = argv.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    // Najpierw język systemu – w nim powstają domyślne ustawienia przy pierwszym uruchomieniu.
    let system = win::system_language();
    openmp_core::i18n::set_language(&system);
    if let Some((_, region)) = system.rsplit_once('-') {
        openmp_core::radio::set_country(region);
    }
    // Ekran powitalny tylko przy pierwszym uruchomieniu – gdy nie ma jeszcze pliku ustawień.
    let first_run = !openmp_core::paths::data().join("settings.json").exists();
    let mut settings = Settings::load();
    if !first_run {
        settings.welcome_done = true;
    }
    // Działa już okno OpenMP – oddajemy mu pliki i kończymy.
    if settings.single_instance {
        let mut msg = files.clone();
        if add_only {
            msg.insert(0, "--add".into());
        }
        if openmp_core::instance::send(&msg) {
            return;
        }
    }

    let ui = AppWindow::new().expect("okno");
    apply_language(&settings.language);
    let session = Session::load();
    let engine = Engine::new(settings.device.clone());
    let (mpv, mpv_err) = match video::create(&settings) {
        Ok(m) => (Some(m), String::new()),
        Err(e) => (None, tr!("Unavailable: {}. Movies and radio need the libmpv-2.dll file next to OpenMP.exe.", e)),
    };
    let models = Models {
        bars: Rc::new(VecModel::default()),
        peaks: Rc::new(VecModel::default()),
        rows: Rc::new(VecModel::default()),
        lib_rows: Rc::new(VecModel::default()),
        tiles: Rc::new(VecModel::default()),
        nav: Rc::new(VecModel::default()),
        tabs: Rc::new(VecModel::default()),
        eq: Rc::new(VecModel::default()),
        lyrics: Rc::new(VecModel::default()),
        stations: Rc::new(VecModel::default()),
    };
    {
        let p = ui.global::<Player>();
        p.set_bars(model(&models.bars));
        p.set_peaks(model(&models.peaks));
        p.set_lyrics(model(&models.lyrics));
        ui.global::<Lists>().set_rows(model(&models.rows));
        ui.global::<Lists>().set_tabs(model(&models.tabs));
        let l = ui.global::<Lib>();
        l.set_rows(model(&models.lib_rows));
        l.set_tiles(model(&models.tiles));
        l.set_nav(model(&models.nav));
        l.set_stations(model(&models.stations));
    }
    let app = App {
        ui: ui.as_weak(),
        s: settings,
        session,
        store: Store::load(),
        lib: Library::load(),
        stats: Stats::load(),
        loudness: openmp_core::audio::loudness::Store::load(),
        measuring: Default::default(),
        engine,
        mpv,
        mpv_err,
        m: models,
        cur: None,
        pending: None,
        seq: 0,
        queue: Vec::new(),
        history: Vec::new(),
        shuffle: Vec::new(),
        shuffle_list: 0,
        shuffle_len: 0,
        sel: Default::default(),
        anchor: None,
        row_of: Default::default(),
        filter: String::new(),
        libv: LibState { kind: "album".into(), ..Default::default() },
        analyzer: Analyzer::new(),
        last_frame: Instant::now(),
        listen_at: Instant::now(),
        menu: MenuCtx::None,
        rename: RenameCtx::None,
        sleep: None,
        sleep_fade: None,
        toast_until: None,
        osd_until: None,
        last_mouse: Instant::now(),
        hover_controls: false,
        seek_hover: false,
        resume_at: Instant::now(),
        next_cancelled: false,
        ab: (None, None),
        lyrics: Vec::new(),
        lyrics_text: String::new(),
        sel_text: String::new(),
        sel_desc: None,
        sub_panel: Vec::new(),
        sub_panel_id: 0,
        sub_live: false,
        lyrics_hits: Vec::new(),
        wave_raw: Vec::new(),
        viz_hist: Default::default(),
        music_geom: None,
        menu_win: None,
        dialogs: None,
        dirty: false,
        scrobbled: 0,
        counted: 0,
        sel_cover_path: String::new(),
        last_save: Instant::now(),
        last_smtc: Instant::now(),
        stations: Vec::new(),
        podcasts: Default::default(),
        dlna_found: Vec::new(),
        dlna: None,
        saved_stations: openmp_core::radio::load_saved(),
        update: None,
        cover_bytes: None,
        tag_paths: Vec::new(),
        tag_mixed: Vec::new(),
        tag_cover: None,
        smtc: None,
        scan: None,
        watch: None,
        video_size: (0, 0),
        chapters: Vec::new(),
        preview: None,
        preview_file: String::new(),
        preview_want: None,
        preview_sent: -1.0,
        preview_at: Instant::now(),
        tint_target: None,
        video_slow: Instant::now(),
        awake: false,
        icy: String::new(),
        pause_after_load: false,
        last_pos: -1.0,
        last_dur: -1.0,
        fail_streak: 0,
    };
    APP.with(|a| *a.borrow_mut() = Some(app));

    with(|app| {
        prefs::publish(app);
        prefs::apply_audio(app);
        lists::rebuild(app);
        libview::rebuild_nav(app);
        if let Some(ui) = app.ui() {
            let id = app.active_id();
            ui.global::<Lib>().set_nav_selected(ss(format!("list:{id}")));
        }
        // Ten sam widok co przy zamknięciu: lista, folder, serial…
        let nav = app.session.nav.clone();
        if !nav.is_empty() && nav != "pod-add" {
            libview::nav_clicked(app, &nav);
            if !app.session.nav_stack.is_empty() {
                app.libv.stack = app.session.nav_stack.clone();
                libview::show(app);
            }
        }
        if let Some(m) = app.mpv.clone() {
            app.preview = video::create_preview();
            if let Some(ui) = app.ui() {
                video::attach(&ui, m, app.preview.clone());
            }
        }
        // Okno jak przy ostatnim zamknięciu.
        if let (Some(ui), Some(g)) = (app.ui(), app.session.window) {
            ui.window().set_size(slint::PhysicalSize::new(g.2.max(420), g.3.max(240)));
        }
    });
    wire(&ui);

    // Pliki z wiersza poleceń albo wznowienie ostatniej sesji.
    with(|app| {
        if !files.is_empty() {
            player::open_paths(app, files.clone(), !add_only, None);
        } else if app.s.resume_on_start {
            restore(app);
        }
        if app.s.single_instance {
            openmp_core::instance::listen(|args| {
                let _ = slint::invoke_from_event_loop(move || {
                    later(move |app| {
                        win::raise(app);
                        let add = args.iter().any(|a| a == "--add");
                        let files: Vec<String> = args.into_iter().filter(|a| !a.starts_with("--")).collect();
                        if !files.is_empty() {
                            player::open_paths(app, files, !add, None);
                        }
                    });
                });
            });
        }
    });

    // Po pokazaniu okna: położenie, zaokrąglone rogi, panel Windows, biblioteka, aktualizacje.
    let w = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(30), move || {
        later(|app| {
            if let Some(ui) = app.ui() {
                if let Some(g) = app.session.window {
                    win::set_geometry(&ui, g);
                }
                if app.session.maximized {
                    ui.window().set_maximized(true);
                    ui.global::<Ui>().set_maximized(true);
                }
                win::round_corners(&ui);
                win::acrylic(win::hwnd(&ui), app.s.theme != "light");
                win::set_ontop(app, app.s.always_on_top);
            }
            win::smtc_init(app);
            if let Some(ui) = app.ui() {
                win::thumb_init(&ui);
            }
            if app.s.tray {
                win::tray(app, true);
            }
            if app.s.global_hotkeys {
                win::hotkeys(app, true);
            }
            libview::watch(app);
            // Pierwsze uruchomienie: ekran powitalny; skanowanie rusza po jego zamknięciu.
            if !app.s.welcome_done {
                if let Some(ui) = app.ui() {
                    ui.global::<Ui>().set_welcome(true);
                }
                crash_report(app);
                return;
            }
            let stale = library::now().saturating_sub(app.lib.scanned) > 6 * 3600;
            let has_folders = !app.s.library_folders.is_empty() || !app.s.video_folders.is_empty();
            if has_folders && (app.lib.tracks.is_empty() && app.lib.videos.is_empty() || stale) {
                libview::start_scan(app);
            }
            if app.s.check_updates {
                dialogs::check_update(app, false);
            }
            crash_report(app);
        });
        drop(w);
    });

    // Zegar: stan odtwarzania i analizator co klatkę, reszta rzadziej.
    let timer = slint::Timer::default();
    let mut slow = Instant::now();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        with(|app| {
            player::tick(app);
            dialogwin::sync_now(app);
            display::frame(app);
            video::tick(app);
            if let Some(t) = app.toast_until {
                if Instant::now() > t {
                    app.toast_until = None;
                    if let Some(ui) = app.ui() {
                        ui.global::<Ui>().set_toast_visible(false);
                    }
                }
            }
            if slow.elapsed() >= Duration::from_millis(500) {
                slow = Instant::now();
                sleep::tick(app);
                radio::icy_tick(app);
                libview::scan_tick(app);
                player::scrobble_tick(app);
                player::played_tick(app);
                win::thumb_state(app);
                if app.last_smtc.elapsed() > Duration::from_secs(2) {
                    app.last_smtc = Instant::now();
                    win::smtc_state(app);
                }
                if app.dirty && app.last_save.elapsed() > Duration::from_secs(8) {
                    app.last_save = Instant::now();
                    app.store.save();
                    app.stats.save();
                    app.s.save();
                    app.dirty = false;
                }
            }
        });
    });

    let r = ui.run();
    openmp_core::paths::log(&format!("koniec pętli zdarzeń: {r:?}"));
    with(|app| keys::save_all(app));
    drop(timer);
    // Stan jest zapisany. Kończymy od razu: zwalnianie bibliotek (mpv, sterowniki dźwięku i grafiki)
    // potrafiło zawiesić proces z widocznym, martwym oknem.
    openmp_core::paths::log("wyjście");
    unsafe {
        let _ = windows::Win32::System::Threading::TerminateProcess(windows::Win32::System::Threading::GetCurrentProcess(), 0);
    }
}

use openmp_core::library;

/// Ostatni utwór na wyświetlaczu, wstrzymany w miejscu, w którym skończyłeś.
fn restore(app: &mut App) {
    let s = app.session.clone();
    if s.path.is_empty() {
        return;
    }
    let Some(li) = app.list_index(s.list_id) else { return };
    let Some(t) = app.store.lists[li].items.get(s.idx).cloned() else { return };
    if t.path != s.path || openmp_core::meta::is_url(&t.path) || openmp_core::meta::is_video(std::path::Path::new(&t.path)) {
        return;
    }
    if !std::path::Path::new(&t.path).exists() {
        return;
    }
    app.store.active = li;
    lists::rebuild(app);
    app.pause_after_load = true;
    player::play(app, s.list_id, s.idx, Some(s.pos));
}

fn drop_files(path: String) {
    DROPS.with(|d| {
        let mut d = d.borrow_mut();
        let first = d.is_empty();
        d.push(path);
        if first {
            // Kilka plików przychodzi osobnymi zdarzeniami – zbieramy je chwilę.
            slint::Timer::single_shot(Duration::from_millis(120), || {
                let paths: Vec<String> = DROPS.with(|d| std::mem::take(&mut *d.borrow_mut()));
                later(move |app| {
                    if let Some(ui) = app.ui() {
                        ui.global::<Ui>().set_drop_hint(false);
                    }
                    // Jeden film – od razu odtwarzamy; reszta trafia na listę.
                    let single_video = paths.len() == 1 && openmp_core::meta::is_video(std::path::Path::new(&paths[0]));
                    let idle = !player::is_playing(app);
                    player::open_paths(app, paths, single_video || idle, None);
                });
            });
        }
    });
}

fn wire(ui: &AppWindow) {
    ui.window().on_winit_window_event(|_, ev| {
        use slint::winit_030::winit::event::WindowEvent;
        match ev {
            WindowEvent::DroppedFile(p) => {
                drop_files(p.to_string_lossy().into_owned());
                return slint::winit_030::EventResult::PreventDefault;
            }
            WindowEvent::HoveredFile(_) => later(|app| {
                if let Some(ui) = app.ui() {
                    ui.global::<Ui>().set_drop_hint(true);
                }
            }),
            WindowEvent::HoveredFileCancelled => later(|app| {
                if let Some(ui) = app.ui() {
                    ui.global::<Ui>().set_drop_hint(false);
                }
            }),
            WindowEvent::Resized(_) => later(|app| {
                if let Some(ui) = app.ui() {
                    let m = ui.window().is_maximized();
                    if ui.global::<Ui>().get_maximized() != m {
                        ui.global::<Ui>().set_maximized(m);
                    }
                }
            }),
            _ => {}
        }
        slint::winit_030::EventResult::Propagate
    });
    ui.window().on_close_requested(|| {
        // Z ikoną w zasobniku zamknięcie tylko chowa okno.
        if win::hide_to_tray() {
            return slint::CloseRequestResponse::KeepWindowShown;
        }
        with(keys::quit);
        slint::CloseRequestResponse::HideWindow
    });

    // Okno
    let u = ui.global::<Ui>();
    u.on_welcome_done(|| later(|app| {
        app.s.welcome_done = true;
        app.s.save();
        if let Some(ui) = app.ui() {
            ui.global::<Ui>().set_welcome(false);
        }
        if !app.s.library_folders.is_empty() || !app.s.video_folders.is_empty() {
            libview::start_scan(app);
        }
    }));
    u.on_drag(|| later(|app| {
        if let Some(ui) = app.ui().filter(|u| !u.window().is_fullscreen()) {
            win::drag(&ui);
        }
    }));
    u.on_resize(|d| later(move |app| {
        if let Some(ui) = app.ui() {
            win::resize(&ui, d);
        }
    }));
    u.on_minimize(|| later(|app| {
        if let Some(ui) = app.ui() {
            ui.window().set_minimized(true);
        }
    }));
    u.on_toggle_maximize(|| later(|app| {
        if let Some(ui) = app.ui().filter(|u| !u.window().is_fullscreen()) {
            let m = !ui.window().is_maximized();
            ui.window().set_maximized(m);
            ui.global::<Ui>().set_maximized(m);
        }
    }));
    u.on_close(|| later(keys::quit));
    u.on_menu_activated(|id| later(move |app| menu::activated(app, &id)));
    u.on_open_main_menu(|x, y| later(move |app| menu::main_menu(app, x, y)));
    u.on_sleep_menu(|x, y| later(move |app| menu::sleep_menu(app, x, y)));
    u.on_device_menu(|x, y| later(move |app| menu::device_menu(app, x, y)));
    u.on_dialog_shown(|shown| later(move |app| dialogwin::shown(app, shown)));
    u.on_set_mode(|m| later(move |app| keys::set_mode(app, m)));
    u.on_toggle_full(|| later(keys::toggle_full));
    u.on_url_accepted(|t| later(move |app| {
        if let Some(ui) = app.ui() {
            ui.global::<Ui>().set_url_dialog(false);
            ui.invoke_focus_keys();
        }
        player::open_url(app, &t);
    }));
    u.on_rename_accepted(|t| later(move |app| dialogs::rename_accepted(app, &t)));
    u.on_tag_save(|v| {
        let v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
        later(move |app| dialogs::tag_save(app, v))
    });
    u.on_tag_pick_cover(|| later(dialogs::tag_pick_cover));
    u.on_update_install(|| later(dialogs::update_install));
    u.on_update_skip(|| later(dialogs::update_skip));
    u.on_key(|t, c, s, a| later(move |app| keys::key(app, &t, c, s, a)));

    // Odtwarzanie
    let p = ui.global::<Player>();
    p.on_play_pause(|| later(player::play_pause));
    p.on_stop(|| later(player::stop));
    p.on_close_current(|| later(display::close_current));
    p.on_next(|| later(player::next));
    p.on_prev(|| later(player::prev));
    p.on_seek(|v| later(move |app| {
        let (_, dur) = player::position(app);
        if dur > 0.0 {
            player::seek_abs(app, v as f64 * dur);
        }
    }));
    p.on_seek_rel(|d| later(move |app| player::seek_rel(app, d as f64)));
    p.on_set_volume(|v| later(move |app| player::set_volume(app, v)));
    p.on_toggle_mute(|| later(player::toggle_mute));
    p.on_toggle_shuffle(|| later(keys::toggle_shuffle));
    p.on_cycle_repeat(|| later(keys::cycle_repeat));
    p.on_cycle_viz(|| later(display::cycle_viz));
    p.on_save_lyrics(|| later(display::save_lyrics));
    p.on_pick_sub_lang(|i| later(move |app| display::pick_sub_panel(app, i)));
    p.on_lyrics_search(|_| later(display::lyrics_search));
    p.on_lyrics_next(|d| later(move |app| display::lyrics_next(app, d)));
    p.on_lyrics_seek(|i| later(move |app| display::lyrics_seek(app, i)));
    p.on_reveal_current(|| later(|app| {
        if let Some(c) = &app.cur {
            if !openmp_core::meta::is_url(&c.info.path) {
                win::reveal(&c.info.path);
            }
        }
    }));
    p.on_toggle_fav(|| later(|app| {
        if let Some(c) = app.cur.clone() {
            let s = app.stats.entry(&c.info.path);
            s.rating = if s.rating >= 5 { 0 } else { 5 };
            let f = s.rating >= 5;
            app.dirty = true;
            if let Some(ui) = app.ui() {
                ui.global::<Player>().set_fav(f);
            }
            lists::rebuild(app);
        }
    }));
    p.on_show_current(|| later(|app| {
        if let Some(l) = app.cur.as_ref().and_then(|c| c.list) {
            if let Some(i) = app.list_index(l) {
                if i != app.store.active {
                    lists::switch_tab(app, i);
                }
                if let Some(ui) = app.ui() {
                    ui.global::<Lib>().set_view(0);
                }
                lists::scroll_to_current(app);
            }
        }
    }));
    p.on_open_cover(|| later(|app| {
        if let Some(c) = &app.cur {
            if let Some(f) = openmp_core::meta::folder_cover(std::path::Path::new(&c.info.path)) {
                let _ = open::that(f);
            }
        }
    }));
    p.on_queue_remove(|i| later(move |app| {
        if (i as usize) < app.queue.len() {
            app.queue.remove(i as usize);
        }
        lists::refresh_playing(app);
        lists::update_queue_panel(app);
        app.pending = None;
        player::refresh_next(app);
    }));
    p.on_queue_clear(|| later(|app| {
        app.queue.clear();
        lists::refresh_playing(app);
        lists::update_queue_panel(app);
        app.pending = None;
        player::refresh_next(app);
    }));

    // Listy
    let l = ui.global::<Lists>();
    l.on_tab_clicked(|i| later(move |app| lists::switch_tab(app, i as usize)));
    l.on_tab_menu(|i, x, y| later(move |app| lists::tab_menu(app, i as usize, x, y)));
    l.on_tab_add(|| later(|app| keys::action(app, "new-list")));
    l.on_tab_close(|i| later(move |app| lists::close_tab(app, i as usize)));
    // Po każdej zmianie zaznaczenia – szczegóły w trybie zaznaczenia (gdy nic nie gra).
    l.on_row_clicked(|i, c, s| later(move |app| {
        lists::select_click(app, i as usize, c, s);
        display::sel_info(app);
    }));
    l.on_row_band(|r0, r1, c0, c1, n, k| later(move |app| {
        lists::select_band(app, (r0, r1, c0, c1, n), k);
        display::sel_info(app);
    }));
    l.on_row_double(|i| later(move |app| {
        let id = app.active_id();
        player::play(app, id, i as usize, None);
    }));
    l.on_row_menu(|_, x, y| later(move |app| lists::row_menu(app, x, y)));
    l.on_columns_menu(|x, y| later(move |app| columns::menu(app, true, x, y)));
    l.on_header_clicked(|i| later(move |app| {
        // Klik w nagłówek zaznacza cały album.
        let li = app.store.active;
        let items = &app.store.lists[li].items;
        let Some(first) = items.get(i as usize) else { return };
        let key = (first.album.to_lowercase(), first.album_artist.to_lowercase());
        app.sel.clear();
        let mut j = i as usize;
        while j < items.len() && (items[j].album.to_lowercase(), items[j].album_artist.to_lowercase()) == key {
            app.sel.insert(j);
            j += 1;
        }
        app.anchor = Some(i as usize);
        lists::rebuild(app);
    }));
    l.on_header_double(|i| later(move |app| {
        let id = app.active_id();
        player::play(app, id, i as usize, None);
    }));
    l.on_filter_changed(|t| later(move |app| {
        app.filter = t.to_string();
        lists::rebuild(app);
    }));
    l.on_fav_toggle(|i| later(move |app| {
        if i < 0 {
            libview::toggle_fav_row(app, (-1 - i) as usize);
            return;
        }
        app.sel.clear();
        app.sel.insert(i as usize);
        lists::command(app, "fav");
        libview::rebuild_nav(app);
    }));
    l.on_open_files(|| later(dialogs::add_files));

    // Biblioteka i radio
    let b = ui.global::<Lib>();
    b.on_nav_clicked(|id| later(move |app| {
        libview::nav_clicked(app, &id);
        display::sel_info(app);
    }));
    b.on_nav_menu(|id, x, y| later(move |app| libview::nav_menu(app, &id, x, y)));
    b.on_nav_toggle(|id| later(move |app| libview::nav_toggle(app, &id)));
    b.on_tile_clicked(|k| later(move |app| {
        libview::tile_clicked(app, &k);
        display::sel_info(app);
    }));
    b.on_tile_double(|k| later(move |app| libview::tile_double(app, &k)));
    b.on_tile_menu(|k, x, y| later(move |app| libview::tile_menu(app, &k, x, y)));
    b.on_back(|| later(|app| {
        libview::back(app);
        display::sel_info(app);
    }));
    b.on_play_all(|| later(|app| {
        let t = app.libv.tracks.clone();
        libview::play_tracks(app, t, 0, false);
    }));
    b.on_shuffle_all(|| later(|app| {
        let t = app.libv.tracks.clone();
        libview::play_tracks(app, t, 0, true);
    }));
    b.on_add_all(|| later(|app| {
        app.libv.sel = (0..app.libv.tracks.len()).collect();
        libview::command(app, "l-add");
    }));
    b.on_row_clicked(|i, c, s| later(move |app| {
        libview::row_clicked(app, i as usize, c, s);
        display::sel_info(app);
    }));
    b.on_row_band(|r0, r1, c0, c1, n, k| later(move |app| {
        libview::row_band(app, (r0, r1, c0, c1, n), k);
        display::sel_info(app);
    }));
    b.on_row_double(|i| later(move |app| libview::row_double(app, i as usize)));
    b.on_row_menu(|_, x, y| later(move |app| libview::row_menu(app, x, y)));
    b.on_columns_menu(|x, y| later(move |app| columns::menu(app, false, x, y)));
    b.on_sort_menu(|x, y| later(move |app| sorting::menu(app, x, y)));
    b.on_search_changed(|t| later(move |app| {
        app.libv.search = t.to_string();
        libview::show(app);
    }));
    b.on_kind_filter_changed(|k| later(move |app| {
        app.s.coll_kind = k;
        app.s.save();
        libview::show(app);
    }));
    b.on_rescan(|| later(libview::start_scan));
    b.on_radio_search(|q| later(move |app| {
        let src = app.ui().map(|u| u.global::<Lib>().get_radio_source()).unwrap_or(0);
        radio::load(app, src, q.to_string());
    }));
    b.on_radio_source_changed(|s| later(move |app| {
        let q = app.ui().map(|u| u.global::<Lib>().get_radio_query().to_string()).unwrap_or_default();
        radio::load(app, s, q);
    }));
    b.on_radio_play(|i| later(move |app| radio::play_index(app, i as usize)));
    b.on_radio_save(|i| later(move |app| radio::toggle_save(app, i as usize)));
    b.on_radio_add_url(|| later(|app| dialogs::rename(app, RenameCtx::RadioUrl, tr!("Radio stream URL"), "https://")));

    // Wideo
    let v = ui.global::<Video>();
    v.on_mouse_moved(|| later(|app| {
        app.last_mouse = Instant::now();
        if let Some(ui) = app.ui() {
            let v = ui.global::<Video>();
            if !v.get_controls() {
                v.set_controls(true);
            }
            if !v.get_cursor() {
                v.set_cursor(true);
            }
        }
    }));
    v.on_toggle_fullscreen(|| later(video::toggle_fullscreen));
    v.on_toggle_ontop(|| later(video::toggle_ontop));
    v.on_back_to_music(|| later(video::leave_view));
    v.on_open_view(|| later(video::open_view));
    v.on_leave_view(|| later(video::leave_view));
    v.on_show_picture(|| later(video::open_view));
    v.on_hover_seek(|f| later(move |app| video::hover_seek(app, f)));
    v.on_set_zoom(|z| later(move |app| video::set(app, "zoom", z)));
    v.on_skip(|| later(video::skip));
    v.on_next_cancel(|| later(|app| app.next_cancelled = true));
    v.on_rotate_view(|| later(video::rotate));
    v.on_reset_view(|| later(video::reset_view));
    v.on_menu(|x, y, above| later(move |app| crate::menu::video_menu(app, x, y, above)));
    v.on_mini(|| later(|app| keys::set_mode(app, 2)));
    v.on_panel_page_changed(|p| later(move |app| {
        if p == 4 {
            video::open_panel(app, 4);
        }
    }));
    v.on_select_audio(|id| later(move |app| video::select(app, "aid", &id)));
    v.on_select_sub(|id| later(move |app| video::select(app, "sid", &id)));
    v.on_load_sub(|| later(video::load_sub));
    v.on_fetch_sub(|| later(video::fetch_sub));
    v.on_goto_chapter(|id| later(move |app| video::goto_chapter(app, &id)));
    v.on_set_speed(|s| later(move |app| video::set(app, "speed", s)));
    v.on_set_sub_delay(|s| later(move |app| video::set(app, "sub-delay", s)));
    v.on_set_audio_delay(|s| later(move |app| video::set(app, "audio-delay", s)));
    v.on_set_eq(|n, s| later(move |app| video::set(app, &n, s)));
    v.on_set_sub_scale(|s| later(move |app| video::set(app, "sub-scale", s)));
    v.on_set_aspect(|i| later(move |app| video::set_aspect(app, i)));
    v.on_screenshot(|| later(|app| video::screenshot(app, true)));
    v.on_frame_step(|b| later(move |app| video::frame_step(app, b)));
    v.on_ab_loop(|| later(video::ab_loop));
    v.on_hover_controls(|h| later(move |app| app.hover_controls = h));

    // Ustawienia
    let pr = ui.global::<Prefs>();
    pr.on_changed(|k| later(move |app| prefs::changed(app, &k)));
    pr.on_eq_band(|i, v| later(move |app| prefs::eq_band(app, i as usize, v)));
    pr.on_eq_preset(|i| later(move |app| prefs::eq_preset(app, i as usize)));
    pr.on_eq_save_preset(|| later(|app| dialogs::rename(app, RenameCtx::EqPreset, tr!("Equalizer preset name"), tr!("Mine"))));
    pr.on_eq_reset(|| later(prefs::eq_reset));
    pr.on_add_folder(|k| later(move |app| prefs::add_folder(app, k)));
    pr.on_add_network_folder(|k| later(move |app| prefs::add_network_folder(app, k)));
    pr.on_remove_folder(|k, i| later(move |app| prefs::remove_folder(app, k, i as usize)));
    pr.on_pick_screenshot_dir(|| later(prefs::pick_screenshot_dir));
    pr.on_backup_export(|| later(prefs::backup_export));
    pr.on_custom_key(|id, t, c, s, a| later(move |app| keys::set_custom(app, &id, &t, c, s, a)));
    pr.on_backup_import(|| later(prefs::backup_import));
    pr.on_check_update(|| later(|app| dialogs::check_update(app, true)));
    pr.on_open_url(|u| {
        let _ = open::that(u.as_str());
    });
    pr.on_associate(|| {
        let _ = open::that("ms-settings:defaultapps");
    });
    let _ = Model::row_count(&VecModel::<i32>::default());
}
