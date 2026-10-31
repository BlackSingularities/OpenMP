//! Biblioteka muzyki: skanowanie folderów (wielowątkowo, z pamięcią podręczną tagów),
//! statystyki odtworzeń, oceny, ulubione i miejsca wznowienia.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::meta::{self, TrackInfo};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stat {
    pub plays: u32,
    pub last: u64,
    pub rating: u8,
    pub added: u64,
    /// Miejsce, w którym przerwano (wideo, audiobooki).
    pub resume: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub map: HashMap<String, Stat>,
    pub recent: Vec<String>,
}

impl Stats {
    pub fn load() -> Stats {
        crate::paths::load_json("stats.json")
    }
    pub fn save(&self) {
        crate::paths::save_json("stats.json", self);
    }
    pub fn get(&self, p: &str) -> Stat {
        self.map.get(&key(p)).cloned().unwrap_or_default()
    }
    pub fn entry(&mut self, p: &str) -> &mut Stat {
        self.map.entry(key(p)).or_default()
    }
    pub fn played(&mut self, p: &str) {
        let now = now();
        let s = self.entry(p);
        s.plays += 1;
        s.last = now;
    }
    pub fn touch_recent(&mut self, p: &str) {
        self.recent.retain(|r| !r.eq_ignore_ascii_case(p));
        self.recent.insert(0, p.to_string());
        self.recent.truncate(30);
    }
}

pub fn key(p: &str) -> String {
    p.to_lowercase()
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub tracks: Vec<TrackInfo>,
    pub videos: Vec<TrackInfo>,
    pub scanned: u64,
}

fn file() -> PathBuf {
    crate::paths::cache().join("library.json")
}

impl Library {
    pub fn load() -> Library {
        std::fs::read(file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        crate::paths::write_atomic(&file(), &serde_json::to_vec(self).unwrap_or_default());
    }
}

/// Postęp skanowania.
pub struct Progress {
    pub found: AtomicUsize,
    pub done: AtomicUsize,
    pub cancel: AtomicBool,
}

impl Default for Progress {
    fn default() -> Self {
        Progress { found: AtomicUsize::new(0), done: AtomicUsize::new(0), cancel: AtomicBool::new(false) }
    }
}

/// Foldery pomijane przy przeglądaniu (systemowe, kosz, ukryte).
fn skip_dir(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.starts_with('.') || l == "$recycle.bin" || l == "system volume information" || l == "@eadir" || l == "#recycle" || l == "node_modules"
}

/// Pliki w folderach – także udziały sieciowe (\\serwer\udział) i dyski zmapowane.
/// `kind`: 0 – muzyka, 1 – wideo, 2 – wszystko.
pub fn collect_kind(folders: &[String], kind: u8) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for f in folders {
        let walker = walkdir::WalkDir::new(f)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_type().is_dir() || !skip_dir(&e.file_name().to_string_lossy()));
        for e in walker.filter_map(Result::ok) {
            let p = e.path();
            if !e.file_type().is_file() {
                continue;
            }
            let ok = match kind {
                0 => meta::is_audio(p),
                1 => meta::is_video(p),
                _ => meta::is_media(p),
            };
            if ok {
                out.push(p.to_path_buf());
            }
        }
    }
    out
}

/// Wszystkie pliki multimedialne (`audio_only` – bez wideo).
pub fn collect(folders: &[String], audio_only: bool) -> Vec<PathBuf> {
    collect_kind(folders, if audio_only { 0 } else { 2 })
}

/// Czy folder jest teraz osiągalny (dysk sieciowy może być wyłączony).
pub fn reachable(folder: &str) -> bool {
    std::fs::read_dir(folder).is_ok()
}

/// Pełne skanowanie: niezmienione pliki (rozmiar + data) biorą tagi z poprzedniego stanu.
/// Niedostępne foldery (np. wyłączony NAS) zachowują poprzednie wpisy zamiast znikać z biblioteki.
pub fn scan(folders: &[String], old: &[TrackInfo], prog: &Progress, video: bool) -> Vec<TrackInfo> {
    let (online, offline): (Vec<String>, Vec<String>) = folders.iter().cloned().partition(|f| reachable(f));
    let files = collect_kind(&online, if video { 1 } else { 0 });
    prog.found.store(files.len(), Ordering::Relaxed);
    let cache: HashMap<String, &TrackInfo> = old.iter().map(|t| (key(&t.path), t)).collect();
    // Na dyskach sieciowych czas idzie na opóźnienia, nie na procesor – więcej wątków niż rdzeni.
    let threads = (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) * 2).clamp(4, 16);
    let next = AtomicUsize::new(0);
    let results: Vec<Vec<TrackInfo>> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut part = Vec::new();
                    loop {
                        if prog.cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(p) = files.get(i) else { break };
                        let ps = p.to_string_lossy();
                        let md = std::fs::metadata(p).ok();
                        let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
                        let mt = md
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        match cache.get(&key(&ps)) {
                            Some(t) if t.size == size && t.modified == mt => part.push((*t).clone()),
                            _ => part.push(meta::read(p)),
                        }
                        prog.done.fetch_add(1, Ordering::Relaxed);
                    }
                    part
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    let mut all: Vec<TrackInfo> = results.into_iter().flatten().collect();
    for f in &offline {
        let root = key(f);
        all.extend(old.iter().filter(|t| key(&t.path).starts_with(&root)).cloned());
    }
    if video {
        all.sort_by_key(|t| t.path.to_lowercase());
    } else {
        all.sort_by(album_order);
    }
    all
}

/// Porządek albumowy: wykonawca albumu, rok, album, płyta, numer, nazwa pliku.
pub fn album_order(a: &TrackInfo, b: &TrackInfo) -> std::cmp::Ordering {
    let aa = if a.album_artist.is_empty() { &a.artist } else { &a.album_artist };
    let ba = if b.album_artist.is_empty() { &b.artist } else { &b.album_artist };
    aa.to_lowercase()
        .cmp(&ba.to_lowercase())
        .then(a.year.cmp(&b.year))
        .then(a.album.to_lowercase().cmp(&b.album.to_lowercase()))
        .then(a.disc.cmp(&b.disc))
        .then(a.track.cmp(&b.track))
        .then(a.path.cmp(&b.path))
}

/// Grupa w widoku biblioteki (wykonawca, album, gatunek, rok, folder).
#[derive(Clone, Debug, Default)]
pub struct Group {
    pub name: String,
    pub sub: String,
    pub count: usize,
    pub duration: f64,
    pub first: String,
}

pub fn group_by(tracks: &[TrackInfo], kind: &str) -> Vec<Group> {
    let mut map: HashMap<String, Group> = HashMap::new();
    for t in tracks {
        let (k, sub) = match kind {
            "artist" => (if t.album_artist.is_empty() { t.artist.clone() } else { t.album_artist.clone() }, String::new()),
            "album" => (t.album.clone(), if t.album_artist.is_empty() { t.artist.clone() } else { t.album_artist.clone() }),
            "genre" => (t.genre.clone(), String::new()),
            "year" => (if t.year > 0 { t.year.to_string() } else { String::new() }, String::new()),
            _ => (Path::new(&t.path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(), String::new()),
        };
        let k = if k.trim().is_empty() { "(nieznane)".to_string() } else { k };
        let id = format!("{}\u{1}{}", k.to_lowercase(), if kind == "album" { sub.to_lowercase() } else { String::new() });
        let g = map.entry(id).or_insert_with(|| Group { name: k.clone(), sub: sub.clone(), first: t.path.clone(), ..Default::default() });
        g.count += 1;
        g.duration += t.duration;
    }
    let mut v: Vec<Group> = map.into_values().collect();
    if kind == "year" {
        v.sort_by(|a, b| b.name.cmp(&a.name));
    } else {
        v.sort_by_key(|g| g.name.to_lowercase());
    }
    v
}

/// Utwory należące do grupy.
pub fn in_group<'a>(tracks: &'a [TrackInfo], kind: &str, name: &str, sub: &str) -> Vec<&'a TrackInfo> {
    let unknown = name == "(nieznane)";
    let eq = |v: &str| if unknown { v.trim().is_empty() } else { v.eq_ignore_ascii_case(name) };
    tracks
        .iter()
        .filter(|t| match kind {
            "artist" => eq(if t.album_artist.is_empty() { &t.artist } else { &t.album_artist }),
            "album" => eq(&t.album) && (if t.album_artist.is_empty() { &t.artist } else { &t.album_artist }).eq_ignore_ascii_case(sub),
            "genre" => eq(&t.genre),
            "year" => if unknown { t.year == 0 } else { t.year.to_string() == name },
            _ => Path::new(&t.path).parent().map(|p| p.to_string_lossy().eq_ignore_ascii_case(name)).unwrap_or(false),
        })
        .collect()
}

/// Wyszukiwanie po wszystkich polach; słowa muszą wystąpić wszystkie.
pub fn matches(t: &TrackInfo, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let hay = format!("{} {} {} {} {} {} {}", t.title, t.artist, t.album, t.album_artist, t.genre, t.year, t.path).to_lowercase();
    words.iter().all(|w| hay.contains(w.as_str()))
}

pub fn words(q: &str) -> Vec<String> {
    q.to_lowercase().split_whitespace().map(str::to_string).collect()
}
