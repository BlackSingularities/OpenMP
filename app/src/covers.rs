//! Miniatury okładek (albumy, foldery filmów) wczytywane w tle i trzymane w pamięci.
//! Klucz: folder pliku – utwory jednego albumu dzielą okładkę.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::app::*;

thread_local! {
    static CACHE: RefCell<HashMap<String, Option<Image>>> = RefCell::new(HashMap::new());
    static ASKED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

static WORKER: std::sync::OnceLock<crossbeam_channel::Sender<(String, String)>> = std::sync::OnceLock::new();

pub fn key(path: &str) -> String {
    Path::new(path).parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn decode(bytes: &[u8], size: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > size || img.height() > size { img.thumbnail(size, size) } else { img };
    let rgba = img.to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()))
}

fn worker() -> &'static crossbeam_channel::Sender<(String, String)> {
    WORKER.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<(String, String)>();
        // Dwa wątki – okładki z dysku sieciowego nie blokują się nawzajem.
        for _ in 0..2 {
            let rx = rx.clone();
            std::thread::spawn(move || {
                while let Ok((k, path)) = rx.recv() {
                    let p = Path::new(&path);
                    let bytes = if openmp_core::meta::is_video(p) {
                        openmp_core::meta::folder_cover(p).and_then(|c| std::fs::read(c).ok())
                    } else {
                        // Bez okładki w pliku i folderze – z sieci, wg wykonawcy i albumu z tagów.
                        openmp_core::meta::cover(p).or_else(|| {
                            let t = openmp_core::meta::read(p);
                            let artist = if t.album_artist.trim().is_empty() { &t.artist } else { &t.album_artist };
                            openmp_core::coverart::fetch(artist, &t.album)
                        })
                    };
                    let buf = bytes.and_then(|b| decode(&b, 200));
                    let _ = slint::invoke_from_event_loop(move || {
                        let img = buf.map(Image::from_rgba8);
                        CACHE.with(|c| c.borrow_mut().insert(k.clone(), img));
                        later(move |app| {
                            crate::lists::refresh_covers(app, &k);
                            crate::libview::refresh_covers(app, &k);
                        });
                    });
                }
            });
        }
        tx
    })
}

/// Okładka dla pliku: gotowa, albo zlecona do wczytania (wtedy `None`).
pub fn get(path: &str) -> Option<Image> {
    if path.is_empty() || openmp_core::meta::is_url(path) {
        return None;
    }
    let k = key(path);
    if let Some(v) = CACHE.with(|c| c.borrow().get(&k).cloned()) {
        return v;
    }
    let fresh = ASKED.with(|a| a.borrow_mut().insert(k.clone()));
    if fresh {
        let _ = worker().send((k, path.to_string()));
    }
    None
}

/// Zapomina okładkę (po zmianie w edytorze tagów).
pub fn forget(path: &str) {
    let k = key(path);
    CACHE.with(|c| c.borrow_mut().remove(&k));
    ASKED.with(|a| a.borrow_mut().remove(&k));
}
