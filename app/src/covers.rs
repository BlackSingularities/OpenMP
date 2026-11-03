//! Miniatury okładek (albumy, foldery filmów) wczytywane w tle i trzymane w pamięci.
//! Klucz: sam plik – w jednym folderze bywają różne albumy. Te same obrazy (okładka
//! folderu, wspólna okładka albumu) są dekodowane raz i współdzielone.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::app::*;

thread_local! {
    static CACHE: RefCell<HashMap<String, Option<Image>>> = RefCell::new(HashMap::new());
    static ASKED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Kolejka zleceń jako stos: najpierw ostatnio zlecone – po przewinięciu widoczne kafelki
/// nie czekają na wszystkie wcześniej przewinięte.
struct Jobs {
    stack: std::sync::Mutex<Vec<(String, String)>>,
    ready: std::sync::Condvar,
}

static WORKER: std::sync::OnceLock<std::sync::Arc<Jobs>> = std::sync::OnceLock::new();

/// Zdekodowane miniatury wg skrótu treści – utwory jednego albumu nie dekodują jej po kilka razy.
static DECODED: std::sync::Mutex<Option<HashMap<u64, SharedPixelBuffer<Rgba8Pixel>>>> = std::sync::Mutex::new(None);

/// Klucz okładki: sam plik (folder może mieścić utwory z różnych albumów).
pub fn key(path: &str) -> String {
    path.to_lowercase()
}

fn decode_shared(bytes: &[u8]) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(bytes, &mut h);
    let id = std::hash::Hasher::finish(&h);
    if let Some(b) = DECODED.lock().ok().and_then(|g| g.as_ref().and_then(|m| m.get(&id).cloned())) {
        return Some(b);
    }
    let buf = decode(bytes, 200)?;
    if let Ok(mut g) = DECODED.lock() {
        g.get_or_insert_with(HashMap::new).insert(id, buf.clone());
    }
    Some(buf)
}

/// Miniatura filmu: obraz obok pliku (nazwa.jpg, nazwa-poster.jpg), zapisana wcześniej klatka
/// albo nowa klatka z ~10% filmu (mpv zapisuje ją do pamięci podręcznej).
fn video_thumb(p: &Path) -> Option<Vec<u8>> {
    let stem = p.file_stem()?.to_string_lossy().into_owned();
    let dir = p.parent()?;
    for name in [format!("{stem}.jpg"), format!("{stem}.png"), format!("{stem}-poster.jpg"), format!("{stem}-thumb.jpg")] {
        if let Ok(b) = std::fs::read(dir.join(name)) {
            return Some(b);
        }
    }
    if let Some(b) = openmp_core::tmdb::poster(p) {
        return Some(b);
    }
    let cache = openmp_core::paths::cache().join("thumbs");
    let _ = std::fs::create_dir_all(&cache);
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&p.to_string_lossy().to_lowercase(), &mut h);
    let file = cache.join(format!("{:016x}.jpg", std::hash::Hasher::finish(&h)));
    if let Ok(b) = std::fs::read(&file) {
        return Some(b);
    }
    // Osobny folder na zadanie – mpv nadaje klatce stałą nazwę.
    let tmp = cache.join(format!("tmp-{:?}", std::thread::current().id()).replace(['(', ')'], ""));
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::create_dir_all(&tmp);
    let out = tmp.to_string_lossy().into_owned();
    let m = openmp_core::mpv::Mpv::new(&[
        ("vo", "image"),
        ("vo-image-format", "jpg"),
        ("vo-image-jpeg-quality", "85"),
        ("vo-image-outdir", &out),
        ("ao", "null"),
        ("aid", "no"),
        ("sid", "no"),
        ("frames", "1"),
        ("start", "10%"),
        ("hwdec", "no"),
        ("vf", "scale=360:-2"),
        ("idle", "yes"),
        ("config", "no"),
        ("load-scripts", "no"),
        ("terminal", "no"),
        ("ytdl", "no"),
    ])
    .ok()?;
    m.command(&["loadfile", &p.to_string_lossy()]).ok()?;
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut got = None;
    while std::time::Instant::now() < until {
        if let Some(f) = std::fs::read_dir(&tmp).ok().and_then(|d| d.filter_map(Result::ok).map(|e| e.path()).find(|e| e.extension().is_some_and(|x| x == "jpg"))) {
            // Plik mógł być jeszcze zapisywany – chwila na dokończenie.
            std::thread::sleep(std::time::Duration::from_millis(150));
            got = std::fs::read(&f).ok();
            break;
        }
        let ended = m.poll().iter().any(|e| matches!(e, openmp_core::mpv::Event::EndFile { .. } | openmp_core::mpv::Event::Shutdown));
        if ended {
            std::thread::sleep(std::time::Duration::from_millis(150));
            continue;
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    drop(m);
    let _ = std::fs::remove_dir_all(&tmp);
    if let Some(b) = &got {
        let _ = std::fs::write(&file, b);
    }
    got
}

/// Obraz okładki w pełnej rozdzielczości (blokujące – poza wątkiem interfejsu).
pub fn source(path: &str) -> Option<Vec<u8>> {
    let p = Path::new(path);
    if openmp_core::meta::is_video(p) {
        video_thumb(p).or_else(|| openmp_core::meta::folder_cover(p).and_then(|c| std::fs::read(c).ok()))
    } else {
        // Bez okładki w pliku i folderze – z sieci, wg wykonawcy i albumu z tagów.
        openmp_core::meta::cover(p).or_else(|| {
            let t = openmp_core::meta::read(p);
            let artist = if t.album_artist.trim().is_empty() { &t.artist } else { &t.album_artist };
            openmp_core::coverart::fetch(artist, &t.album)
        })
    }
}

pub fn decode(bytes: &[u8], size: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > size || img.height() > size { img.thumbnail(size, size) } else { img };
    let rgba = img.to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()))
}

fn worker() -> &'static std::sync::Arc<Jobs> {
    WORKER.get_or_init(|| {
        let jobs = std::sync::Arc::new(Jobs { stack: std::sync::Mutex::new(Vec::new()), ready: std::sync::Condvar::new() });
        // Kilka wątków – okładki z dysku sieciowego nie blokują się nawzajem.
        for _ in 0..4 {
            let jobs = jobs.clone();
            std::thread::spawn(move || loop {
                let (k, path) = {
                    let Ok(mut st) = jobs.stack.lock() else { return };
                    loop {
                        if let Some(j) = st.pop() {
                            break j;
                        }
                        let Ok(g) = jobs.ready.wait(st) else { return };
                        st = g;
                    }
                };
                let bytes = source(&path);
                let buf = bytes.and_then(|b| decode_shared(&b));
                let _ = slint::invoke_from_event_loop(move || {
                    let img = buf.map(Image::from_rgba8);
                    CACHE.with(|c| c.borrow_mut().insert(k.clone(), img));
                    later(move |app| {
                        crate::lists::refresh_covers(app, &k);
                        crate::libview::refresh_covers(app, &k);
                    });
                });
            });
        }
        jobs
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
        let w = worker();
        if let Ok(mut st) = w.stack.lock() {
            st.push((k, path.to_string()));
        }
        w.ready.notify_one();
    }
    None
}

/// Zapomina okładkę (po zmianie w edytorze tagów).
pub fn forget(path: &str) {
    let k = key(path);
    CACHE.with(|c| c.borrow_mut().remove(&k));
    ASKED.with(|a| a.borrow_mut().remove(&k));
}
