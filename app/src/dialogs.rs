//! Okna wyboru plików, edytor tagów, zmiana nazwy, adres URL, aktualizacje.

use std::collections::HashMap;
use std::path::Path;

use openmp_core::meta::{self, TagEdit};
use slint::{Image, ModelRc, VecModel};

use crate::app::*;
use crate::*;

/// Okno nadrzędne dla systemowego okna wyboru (po uchwycie – da się je przekazać do wątku).
struct Owner(isize);

impl raw_window_handle::HasWindowHandle for Owner {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let h = std::num::NonZeroIsize::new(self.0).ok_or(raw_window_handle::HandleError::Unavailable)?;
        let raw = raw_window_handle::RawWindowHandle::Win32(raw_window_handle::Win32WindowHandle::new(h));
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
    }
}

impl raw_window_handle::HasDisplayHandle for Owner {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let raw = raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new());
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(raw) })
    }
}

/// Systemowe okno wyboru w osobnym wątku: program działa dalej, a anulowanie niczego nie blokuje.
/// Okno wyboru jest podpięte pod otwarty dialog albo okno programu, więc zawsze jest na wierzchu.
pub fn ask<T: Send + 'static>(
    app: &App,
    make: impl FnOnce(rfd::FileDialog) -> Option<T> + Send + 'static,
    done: impl FnOnce(&mut App, T) + Send + 'static,
) {
    let owner = app
        .dialogs
        .as_ref()
        .filter(|d| d.open)
        .map(|d| crate::win::hwnd_of(d.win.window()))
        .or_else(|| app.ui().map(|u| crate::win::hwnd(&u)))
        .unwrap_or(0);
    std::thread::spawn(move || {
        let mut d = rfd::FileDialog::new();
        if owner != 0 {
            d = d.set_parent(&Owner(owner));
        }
        if let Some(v) = make(d) {
            let _ = slint::invoke_from_event_loop(move || later(move |app| done(app, v)));
        }
    });
}

fn media_filters(d: rfd::FileDialog) -> rfd::FileDialog {
    let mut all: Vec<&str> = meta::AUDIO.to_vec();
    all.extend_from_slice(meta::VIDEO);
    all.extend_from_slice(meta::PLAYLISTS);
    d.add_filter(tr!("Media and playlists"), &all)
        .add_filter(tr!("Music"), meta::AUDIO)
        .add_filter(tr!("Video"), meta::VIDEO)
        .add_filter(tr!("Playlists"), meta::PLAYLISTS)
        .add_filter(tr!("All files"), &["*"])
}

pub fn open_files(app: &mut App) {
    ask(app, |d| media_filters(d.set_title(tr!("Open files"))).pick_files(), |app, files| {
        let paths: Vec<String> = files.into_iter().map(|p| p.to_string_lossy().into_owned()).collect();
        crate::player::open_paths(app, paths, true, None);
    });
}

pub fn add_files(app: &mut App) {
    ask(app, |d| media_filters(d.set_title(tr!("Add to list"))).pick_files(), |app, files| {
        let paths: Vec<String> = files.into_iter().map(|p| p.to_string_lossy().into_owned()).collect();
        crate::player::open_paths(app, paths, false, None);
    });
}

pub fn open_folder(app: &mut App) {
    ask(app, |d| d.set_title(tr!("Open folder")).pick_folder(), |app, dir| {
        crate::player::open_paths(app, vec![dir.to_string_lossy().into_owned()], true, None);
    });
}

pub fn import_list(app: &mut App, new_tab: bool) {
    ask(app, |d| d.set_title(tr!("Import playlist")).add_filter(tr!("Playlists"), meta::PLAYLISTS).pick_file(), move |app, f| {
        let name = f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| tr!("List").into());
        crate::player::open_paths(app, vec![f.to_string_lossy().into_owned()], false, new_tab.then_some(name));
    });
}

pub fn export_list(app: &mut App) {
    let li = app.store.active;
    let name = app.store.lists[li].name.clone();
    ask(app, move |d| d.set_title(tr!("Save playlist")).set_file_name(format!("{name}.m3u8")).add_filter(tr!("M3U8 playlist"), &["m3u8", "m3u"]).save_file(), move |app, f| export_to(app, li, f));
}

fn export_to(app: &mut App, li: usize, f: std::path::PathBuf) {
    if li >= app.store.lists.len() {
        return;
    }
    match openmp_core::playlist::export_m3u(&f, &app.store.lists[li].items) {
        Ok(()) => app.toast(tr!("Saved “{}”", f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())),
        Err(e) => app.toast(tr!("Could not save: {}", e)),
    }
}

pub fn rename(app: &mut App, ctx: RenameCtx, title: &str, value: &str) {
    app.rename = ctx;
    if let Some(ui) = app.ui() {
        let u = ui.global::<Ui>();
        u.set_rename_title(ss(title));
        u.set_rename_text(ss(value));
        u.set_rename_dialog(true);
    }
}

pub fn rename_accepted(app: &mut App, text: &str) {
    let text = text.trim().to_string();
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_rename_dialog(false);
        ui.invoke_focus_keys();
    }
    if text.is_empty() {
        return;
    }
    match std::mem::take(&mut app.rename) {
        RenameCtx::Tab(i) => {
            if let Some(l) = app.store.lists.get_mut(i) {
                l.name = text;
                app.dirty = true;
            }
            crate::lists::rebuild_tabs(app);
            crate::libview::rebuild_nav(app);
        }
        RenameCtx::EqPreset => {
            let gains = app.s.dsp.gains.clone();
            app.s.custom_eq.retain(|(n, _)| *n != text);
            app.s.custom_eq.push((text.clone(), gains));
            app.s.eq_preset = text;
            crate::prefs::publish_presets(app);
            app.s.save();
            app.toast(tr!("Equalizer preset saved"));
        }
        RenameCtx::RadioUrl => crate::radio::add_url(app, &text),
        RenameCtx::Podcast => crate::libview::add_podcast(app, text),
        RenameCtx::AiList => crate::ai::create_list(app, text),
        RenameCtx::Tmdb(path) => {
            openmp_core::tmdb::set_query(std::path::Path::new(&path), &text);
            crate::covers::forget(&path);
            crate::libview::show(app);
            if app.cur.as_ref().is_some_and(|c| c.info.path == path) {
                crate::display::show_track(app);
            }
            app.toast(tr!("Searching TMDB: “{}”", text));
        }
        RenameCtx::NetFolder(kind) => crate::prefs::add_folder_path(app, kind, text),
        ctx @ (RenameCtx::SmartName(_) | RenameCtx::SmartQuery(_)) => {
            let (i, query) = match ctx {
                RenameCtx::SmartQuery(i) => (i, true),
                RenameCtx::SmartName(i) => (i, false),
                _ => return,
            };
            if let Some(s) = app.s.smart_lists.get_mut(i) {
                if query {
                    // Nowa kolekcja: dopóki bez nazwy – nazwą są filtry.
                    if s.0.trim().is_empty() {
                        s.0 = text.clone();
                    }
                    s.1 = text;
                } else {
                    s.0 = text;
                }
                app.dirty = true;
            }
            crate::libview::rebuild_nav(app);
            if app.libv.kind == format!("smart:{i}") {
                crate::libview::show(app);
            }
        }
        RenameCtx::None => {}
    }
}

/// Edytor tagów dla jednego lub wielu plików (pola różniące się zostają bez zmian).
pub fn open_tag_editor(app: &mut App, paths: Vec<String>) {
    let paths: Vec<String> = paths.into_iter().filter(|p| !meta::is_url(p) && Path::new(p).exists()).collect();
    if paths.is_empty() {
        return;
    }
    let infos: Vec<_> = paths.iter().map(|p| meta::read(Path::new(p))).collect();
    let fields = |t: &meta::TrackInfo| -> [String; 10] {
        [
            t.title.clone(),
            t.artist.clone(),
            t.album.clone(),
            t.album_artist.clone(),
            t.genre.clone(),
            if t.year > 0 { t.year.to_string() } else { String::new() },
            if t.track > 0 { t.track.to_string() } else { String::new() },
            if t.disc > 0 { t.disc.to_string() } else { String::new() },
            t.composer.clone(),
            t.comment.clone(),
        ]
    };
    let first = fields(&infos[0]);
    let mut mixed = vec![false; 10];
    for t in &infos[1..] {
        let f = fields(t);
        for i in 0..10 {
            if f[i] != first[i] {
                mixed[i] = true;
            }
        }
    }
    let values: Vec<String> = (0..10).map(|i| if mixed[i] { String::new() } else { first[i].clone() }).collect();
    app.tag_paths = paths.clone();
    app.tag_mixed = mixed.clone();
    app.tag_cover = None;
    let cover = meta::cover(Path::new(&paths[0])).and_then(|b| crate::covers::decode(&b, 300));
    if let Some(ui) = app.ui() {
        let u = ui.global::<Ui>();
        u.set_tag_title(ss(if paths.len() == 1 { paths[0].clone() } else { trn!("{n} file", "{n} files", paths.len()) }));
        u.set_tag_values(ModelRc::new(VecModel::from(values.iter().map(ss).collect::<Vec<_>>())));
        u.set_tag_mixed(ModelRc::new(VecModel::from(mixed)));
        u.set_tag_has_cover(cover.is_some());
        u.set_tag_cover(cover.map(Image::from_rgba8).unwrap_or_default());
        u.set_tag_busy(false);
        u.set_tag_editor(true);
    }
}

pub fn tag_pick_cover(app: &mut App) {
    ask(app, |d| d.set_title(tr!("Cover")).add_filter(tr!("Images"), &["jpg", "jpeg", "png", "webp", "bmp"]).pick_file(), tag_cover_from);
}

fn tag_cover_from(app: &mut App, f: std::path::PathBuf) {
    let Ok(bytes) = std::fs::read(&f) else { return };
    // Do pliku zapisujemy JPEG/PNG – inne formaty konwertujemy.
    let bytes = match meta::ext(&f).as_str() {
        "jpg" | "jpeg" | "png" => bytes,
        _ => match image::load_from_memory(&bytes) {
            Ok(img) => {
                let mut out = std::io::Cursor::new(Vec::new());
                if img.to_rgb8().write_to(&mut out, image::ImageFormat::Jpeg).is_err() {
                    return;
                }
                out.into_inner()
            }
            Err(_) => return,
        },
    };
    if let Some(buf) = crate::covers::decode(&bytes, 300) {
        if let Some(ui) = app.ui() {
            ui.global::<Ui>().set_tag_cover(Image::from_rgba8(buf));
            ui.global::<Ui>().set_tag_has_cover(true);
        }
    }
    app.tag_cover = Some(bytes);
}

pub fn tag_save(app: &mut App, v: Vec<String>) {
    let paths = app.tag_paths.clone();
    let mixed = app.tag_mixed.clone();
    let cover = app.tag_cover.clone();
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_tag_busy(true);
    }
    // Plik grany przez mpv bywa zablokowany – zwalniamy go na czas zapisu.
    std::thread::spawn(move || {
        let mut errors = Vec::new();
        let mut fresh = HashMap::new();
        for p in &paths {
            let cur = meta::read(Path::new(p));
            let keep = |i: usize, old: String| if mixed.get(i).copied().unwrap_or(false) && v[i].trim().is_empty() { old } else { v[i].clone() };
            let e = TagEdit {
                title: keep(0, cur.title.clone()),
                artist: keep(1, cur.artist.clone()),
                album: keep(2, cur.album.clone()),
                album_artist: keep(3, cur.album_artist.clone()),
                genre: keep(4, cur.genre.clone()),
                year: keep(5, if cur.year > 0 { cur.year.to_string() } else { String::new() }),
                track: keep(6, if cur.track > 0 { cur.track.to_string() } else { String::new() }),
                disc: keep(7, if cur.disc > 0 { cur.disc.to_string() } else { String::new() }),
                composer: keep(8, cur.composer.clone()),
                comment: keep(9, cur.comment.clone()),
                cover: cover.clone(),
            };
            match meta::write(Path::new(p), &e) {
                Ok(()) => {
                    fresh.insert(p.to_lowercase(), meta::read(Path::new(p)));
                }
                Err(err) => errors.push(format!("{}: {err}", Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())),
            }
        }
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| {
                if let Some(ui) = app.ui() {
                    ui.global::<Ui>().set_tag_busy(false);
                    ui.global::<Ui>().set_tag_editor(false);
                    ui.invoke_focus_keys();
                }
                for p in fresh.keys() {
                    crate::covers::forget(p);
                }
                let n = fresh.len();
                crate::lists::refresh_infos(app, &fresh);
                if app.cur.as_ref().is_some_and(|c| fresh.contains_key(&c.info.path.to_lowercase())) {
                    crate::display::show_track(app);
                }
                if errors.is_empty() {
                    app.toast(tr!("Tags saved ({})", n));
                } else {
                    app.toast(tr!("Save errors: {}", errors.join("; ")));
                }
            });
        });
    });
}

/// Sprawdzenie aktualizacji (`manual` – komunikat także, gdy nic nowego).
pub fn check_update(app: &mut App, manual: bool) {
    let skip = app.s.skip_version.clone();
    std::thread::spawn(move || {
        let res = openmp_core::update::check();
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(Some(rel)) if manual || rel.version != skip => {
                    if let Some(ui) = app.ui() {
                        let u = ui.global::<Ui>();
                        u.set_update_version(ss(&rel.version));
                        u.set_update_notes(ss(rel.notes.trim()));
                        u.set_update_progress(-1.0);
                        u.set_update_status(ss(""));
                        u.set_update_portable(!openmp_core::update::installed());
                        u.set_update_dialog(true);
                    }
                    app.update = Some(rel);
                }
                Ok(_) => {
                    if manual {
                        app.toast(tr!("You have the latest version ({})", openmp_core::version()));
                    }
                }
                Err(e) => {
                    if manual {
                        app.toast(tr!("Could not check for updates: {}", e));
                    }
                }
            });
        });
    });
}

pub fn update_install(app: &mut App) {
    let Some(rel) = app.update.clone() else { return };
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_update_progress(0.0);
        ui.global::<Ui>().set_update_status(ss(tr!("Downloading…")));
    }
    let weak = app.ui.clone();
    std::thread::spawn(move || {
        let w2 = weak.clone();
        let res = openmp_core::update::download(&rel, move |done, total| {
            let w = w2.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = w.upgrade() {
                    ui.global::<Ui>().set_update_progress(if total > 0 { done as f32 / total as f32 } else { 0.0 });
                }
            });
        });
        let _ = slint::invoke_from_event_loop(move || {
            later(move |app| match res {
                Ok(path) => {
                    if let Some(ui) = app.ui() {
                        ui.global::<Ui>().set_update_status(ss(tr!("Installing – OpenMP will restart.")));
                    }
                    match openmp_core::update::apply(&path) {
                        Ok(()) => crate::keys::quit(app),
                        Err(e) => app.toast(tr!("Could not start the installer: {}", e)),
                    }
                }
                Err(e) => {
                    if let Some(ui) = app.ui() {
                        ui.global::<Ui>().set_update_progress(-1.0);
                        ui.global::<Ui>().set_update_status(ss(tr!("Error: {}", e)));
                    }
                }
            });
        });
    });
}

pub fn update_skip(app: &mut App) {
    if let Some(r) = &app.update {
        app.s.skip_version = r.version.clone();
        app.s.save();
    }
    if let Some(ui) = app.ui() {
        ui.global::<Ui>().set_update_dialog(false);
    }
}
