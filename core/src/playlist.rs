//! Listy odtwarzania: zapis w folderze danych, import M3U/M3U8/PLS/XSPF/CUE i eksport M3U8.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::meta::{self, TrackInfo};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub items: Vec<TrackInfo>,
    /// Pozycja bieżącego utworu (do wznowienia po uruchomieniu).
    pub current: Option<usize>,
    /// Lista „inteligentna” – odświeżana z biblioteki według reguły.
    pub smart: Option<String>,
}

impl Playlist {
    pub fn total(&self) -> f64 {
        self.items.iter().map(|t| t.duration).sum()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub lists: Vec<Playlist>,
    pub active: usize,
}

fn file() -> PathBuf {
    crate::paths::data().join("playlists.json")
}

impl Store {
    pub fn load() -> Store {
        let mut s: Store = crate::paths::read_sealed(&file()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if s.lists.is_empty() {
            s.lists.push(Playlist { id: 1, name: "Domyślna".into(), ..Default::default() });
        }
        s.active = s.active.min(s.lists.len() - 1);
        s
    }

    pub fn save(&self) {
        crate::paths::write_sealed(&file(), &serde_json::to_vec(self).unwrap_or_default());
    }

    pub fn next_id(&self) -> u64 {
        self.lists.iter().map(|l| l.id).max().unwrap_or(0) + 1
    }
}

fn resolve(base: &Path, entry: &str) -> String {
    let e = entry.trim();
    if meta::is_url(e) {
        return e.to_string();
    }
    let e = e.strip_prefix("file:///").map(|s| s.replace("%20", " ")).unwrap_or_else(|| e.to_string());
    let p = Path::new(&e);
    if p.is_absolute() {
        e.replace('/', "\\")
    } else {
        base.join(p).to_string_lossy().replace('/', "\\")
    }
}

/// Wpisy z pliku listy (ścieżki i adresy) z tytułami, jeśli lista je podaje.
pub fn import(path: &Path) -> Vec<(String, Option<String>, f64)> {
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    let text = match String::from_utf8(bytes.clone()) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        // Stare listy M3U bywają w Windows-1250.
        Err(_) => bytes.iter().map(|&b| cp1250(b)).collect(),
    };
    let base = path.parent().unwrap_or(Path::new("."));
    let mut out = Vec::new();
    match meta::ext(path).as_str() {
        "pls" => {
            let mut files: Vec<(u32, String)> = Vec::new();
            let mut titles: std::collections::HashMap<u32, String> = Default::default();
            for l in text.lines() {
                let l = l.trim();
                if let Some((k, v)) = l.split_once('=') {
                    let kl = k.to_ascii_lowercase();
                    if let Some(n) = kl.strip_prefix("file").and_then(|n| n.parse().ok()) {
                        files.push((n, resolve(base, v)));
                    } else if let Some(n) = kl.strip_prefix("title").and_then(|n| n.parse().ok()) {
                        titles.insert(n, v.to_string());
                    }
                }
            }
            files.sort_by_key(|f| f.0);
            for (n, f) in files {
                out.push((f, titles.get(&n).cloned(), 0.0));
            }
        }
        "xspf" => {
            for part in text.split("<track>").skip(1) {
                let get = |tag: &str| {
                    let a = part.find(&format!("<{tag}>"))? + tag.len() + 2;
                    let b = part[a..].find(&format!("</{tag}>"))? + a;
                    Some(unescape(&part[a..b]))
                };
                if let Some(loc) = get("location") {
                    let dur = get("duration").and_then(|d| d.parse::<f64>().ok()).map(|d| d / 1000.0).unwrap_or(0.0);
                    out.push((resolve(base, &loc), get("title"), dur));
                }
            }
        }
        "cue" => {
            for l in text.lines() {
                let l = l.trim();
                if let Some(rest) = l.strip_prefix("FILE ") {
                    let name = rest.rsplit_once(' ').map(|(n, _)| n).unwrap_or(rest).trim_matches('"');
                    let p = resolve(base, name);
                    if !out.iter().any(|o: &(String, Option<String>, f64)| o.0 == p) {
                        out.push((p, None, 0.0));
                    }
                }
            }
        }
        _ => {
            let mut title: Option<String> = None;
            let mut dur = 0.0;
            for l in text.lines() {
                let l = l.trim();
                if let Some(info) = l.strip_prefix("#EXTINF:") {
                    if let Some((d, t)) = info.split_once(',') {
                        dur = d.trim().parse().unwrap_or(0.0f64).max(0.0);
                        title = Some(t.trim().to_string());
                    }
                } else if !l.is_empty() && !l.starts_with('#') {
                    out.push((resolve(base, l), title.take(), dur));
                    dur = 0.0;
                }
            }
        }
    }
    out
}

pub fn export_m3u(path: &Path, items: &[TrackInfo]) -> Result<(), String> {
    let mut s = String::from("#EXTM3U\r\n");
    for t in items {
        let who = t.display_artist();
        let name = if who.is_empty() { t.display_title() } else { format!("{who} - {}", t.display_title()) };
        s.push_str(&format!("#EXTINF:{},{}\r\n{}\r\n", t.duration.round() as i64, name, t.path));
    }
    std::fs::write(path, s).map_err(|e| e.to_string())
}

fn unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("%20", " ")
}

fn cp1250(b: u8) -> char {
    const HI: [char; 128] = [
        '€', '?', '‚', '?', '„', '…', '†', '‡', '?', '‰', 'Š', '‹', 'Ś', 'Ť', 'Ž', 'Ź', '?', '‘', '’', '“', '”', '•', '–', '—', '?', '™', 'š',
        '›', 'ś', 'ť', 'ž', 'ź', ' ', 'ˇ', '˘', 'Ł', '¤', 'Ą', '¦', '§', '¨', '©', 'Ş', '«', '¬', '-', '®', 'Ż', '°', '±', '˛', 'ł', '´', 'µ',
        '¶', '·', '¸', 'ą', 'ş', '»', 'Ľ', '˝', 'ľ', 'ż', 'Ŕ', 'Á', 'Â', 'Ă', 'Ä', 'Ĺ', 'Ć', 'Ç', 'Č', 'É', 'Ę', 'Ë', 'Ě', 'Í', 'Î', 'Ď', 'Đ',
        'Ń', 'Ň', 'Ó', 'Ô', 'Ő', 'Ö', '×', 'Ř', 'Ů', 'Ú', 'Ű', 'Ü', 'Ý', 'Ţ', 'ß', 'ŕ', 'á', 'â', 'ă', 'ä', 'ĺ', 'ć', 'ç', 'č', 'é', 'ę', 'ë',
        'ě', 'í', 'î', 'ď', 'đ', 'ń', 'ň', 'ó', 'ô', 'ő', 'ö', '÷', 'ř', 'ů', 'ú', 'ű', 'ü', 'ý', 'ţ', '˙',
    ];
    if b < 128 {
        b as char
    } else {
        HI[(b - 128) as usize]
    }
}
