//! TMDB: polskie tytuły, opisy, gatunki, oceny i plakaty filmów oraz seriali.
//! Działa po wpisaniu klucza API (themoviedb.org) w ustawieniach. Wyniki trzymane w pamięci
//! podręcznej – każdy plik pyta TMDB tylko raz.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

static KEY: RwLock<String> = RwLock::new(String::new());

pub fn set_key(k: &str) {
    if let Ok(mut g) = KEY.write() {
        *g = k.trim().to_string();
    }
}

fn key() -> Option<String> {
    KEY.read().ok().map(|k| k.clone()).filter(|k| !k.is_empty())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Info {
    pub found: bool,
    pub title: String,
    pub year: String,
    pub genres: String,
    pub rating: f32,
    pub runtime: u32,
    pub overview: String,
    pub poster: String,
    pub director: String,
    pub cast: String,
}

impl Info {
    /// „Reżyseria: … · Obsada: …”
    pub fn credits(&self) -> String {
        let mut v = Vec::new();
        if !self.director.is_empty() {
            v.push(format!("Reżyseria: {}", self.director));
        }
        if !self.cast.is_empty() {
            v.push(format!("Obsada: {}", self.cast));
        }
        v.join("  ·  ")
    }

    /// „2009 · Thriller, Kryminał · ★ 6,7 · 2 h 18 min”
    pub fn line(&self) -> String {
        let mut v = Vec::new();
        if !self.year.is_empty() {
            v.push(self.year.clone());
        }
        if !self.genres.is_empty() {
            v.push(self.genres.clone());
        }
        if self.rating > 0.0 {
            v.push(format!("★ {:.1}", self.rating).replace('.', ","));
        }
        if self.runtime > 0 {
            v.push(if self.runtime >= 60 { format!("{} h {:02} min", self.runtime / 60, self.runtime % 60) } else { format!("{} min", self.runtime) });
        }
        v.join("  ·  ")
    }
}

fn cache_dir() -> PathBuf {
    let d = crate::paths::cache().join("tmdb");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.to_lowercase().hash(&mut h);
    format!("{:016x}", h.finish())
}

fn overrides() -> std::collections::HashMap<String, String> {
    std::fs::read(cache_dir().join("overrides.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Wpisany ręcznie tytuł (z rokiem) dla pliku – poprzednie dopasowanie i plakat są zapominane.
pub fn set_query(file: &Path, query: &str) {
    let k = file.to_string_lossy().to_lowercase();
    let mut o = overrides();
    if query.trim().is_empty() {
        o.remove(&k);
    } else {
        o.insert(k, query.trim().to_string());
    }
    let _ = std::fs::write(cache_dir().join("overrides.json"), serde_json::to_vec(&o).unwrap_or_default());
    let _ = std::fs::remove_file(cache_dir().join(format!("{}.json", hash(&file.to_string_lossy()))));
}

/// Tytuł, którego program szuka dla pliku (do poprawienia ręcznie).
pub fn query_of(file: &Path) -> String {
    if let Some(q) = overrides().get(&file.to_string_lossy().to_lowercase()) {
        return q.clone();
    }
    let (t, y, _) = guess(file);
    match y {
        Some(y) => format!("{t} {y}"),
        None => t,
    }
}

/// Tytuł, rok i czy to serial – z nazwy pliku („Aniolowie.i.Demony.2009.1080p.BluRay”).
pub fn guess(file: &Path) -> (String, Option<u32>, bool) {
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let clean: String = stem.chars().map(|c| if matches!(c, '.' | '_' | '[' | ']' | '(' | ')') { ' ' } else { c }).collect();
    let mut words = Vec::new();
    let mut year = None;
    let mut series = false;
    const TAGS: [&str; 18] = ["1080p", "720p", "2160p", "480p", "4k", "x264", "x265", "h264", "h265", "hevc", "bluray", "brrip", "bdrip", "webrip", "web-dl", "webdl", "dvdrip", "hdtv"];
    for w in clean.split_whitespace() {
        let l = w.to_lowercase();
        if TAGS.contains(&l.as_str()) || l == "pl" || l == "lektor" || l == "dubbing" || l == "napisy" {
            break;
        }
        if l.len() == 4 && (l.starts_with("19") || l.starts_with("20")) && l.chars().all(|c| c.is_ascii_digit()) && !words.is_empty() {
            year = l.parse().ok();
            break;
        }
        let b = l.as_bytes();
        if b.len() >= 4 && b[0] == b's' && b[1].is_ascii_digit() && l.contains('e') {
            series = true;
            break;
        }
        words.push(w.to_string());
    }
    (words.join(" ").trim_matches('-').trim().to_string(), year, series)
}

fn get(url: &str) -> Option<serde_json::Value> {
    ureq::get(url).set("User-Agent", "OpenMP").timeout(std::time::Duration::from_secs(10)).call().ok()?.into_json().ok()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Opis filmu albo serialu dla pliku (z pamięci podręcznej albo z TMDB).
pub fn lookup(file: &Path) -> Option<Info> {
    let cfile = cache_dir().join(format!("{}.json", hash(&file.to_string_lossy())));
    if let Ok(b) = std::fs::read(&cfile) {
        if let Ok(i) = serde_json::from_slice::<Info>(&b) {
            return i.found.then_some(i);
        }
    }
    let key = key()?;
    let (mut title, mut year, series) = guess(file);
    if let Some(q) = overrides().get(&file.to_string_lossy().to_lowercase()) {
        // „Tytuł 2009” – rok na końcu jest osobno.
        let (t, y) = match q.rsplit_once(' ') {
            Some((t, y)) if y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()) => (t.to_string(), y.parse().ok()),
            _ => (q.clone(), None),
        };
        title = t;
        year = y;
    }
    if title.is_empty() {
        return None;
    }
    let kind = if series { "tv" } else { "movie" };
    let mut url = format!("https://api.themoviedb.org/3/search/{kind}?api_key={key}&language=pl-PL&query={}", enc(&title));
    if let Some(y) = year {
        url.push_str(&format!("&{}={y}", if series { "first_air_date_year" } else { "year" }));
    }
    let res = get(&url)?;
    let mut info = Info::default();
    if let Some(id) = res["results"].get(0).and_then(|r| r["id"].as_u64()) {
        if let Some(d) = get(&format!("https://api.themoviedb.org/3/{kind}/{id}?api_key={key}&language=pl-PL&append_to_response=credits")) {
            info.found = true;
            info.title = d[if series { "name" } else { "title" }].as_str().unwrap_or_default().to_string();
            let date = d[if series { "first_air_date" } else { "release_date" }].as_str().unwrap_or_default();
            info.year = date.get(..4).unwrap_or_default().to_string();
            info.genres = d["genres"].as_array().map(|g| g.iter().filter_map(|x| x["name"].as_str()).take(3).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            info.rating = d["vote_average"].as_f64().unwrap_or(0.0) as f32;
            info.runtime = d["runtime"].as_u64().unwrap_or(0) as u32;
            info.overview = d["overview"].as_str().unwrap_or_default().to_string();
            info.poster = d["poster_path"].as_str().unwrap_or_default().to_string();
            let names = |v: &serde_json::Value, n: usize| v.as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str()).take(n).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            info.cast = names(&d["credits"]["cast"], 4);
            info.director = if series {
                names(&d["created_by"], 2)
            } else {
                d["credits"]["crew"].as_array().map(|a| a.iter().filter(|x| x["job"] == "Director").filter_map(|x| x["name"].as_str()).take(2).collect::<Vec<_>>().join(", ")).unwrap_or_default()
            };
        }
    }
    // Zapamiętane także „nie znaleziono” – bez ponownych zapytań o ten plik.
    let _ = std::fs::write(&cfile, serde_json::to_vec(&info).unwrap_or_default());
    info.found.then_some(info)
}

/// Plakat (JPEG) dla pliku – pobrany raz i zapisany w pamięci podręcznej.
pub fn poster(file: &Path) -> Option<Vec<u8>> {
    let info = lookup(file)?;
    if info.poster.is_empty() {
        return None;
    }
    let pfile = cache_dir().join(format!("{}-w780.jpg", hash(&info.poster)));
    if let Ok(b) = std::fs::read(&pfile) {
        return Some(b);
    }
    let resp = ureq::get(&format!("https://image.tmdb.org/t/p/w780{}", info.poster)).timeout(std::time::Duration::from_secs(15)).call().ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_reader(), &mut buf).ok()?;
    if buf.len() < 1000 {
        return None;
    }
    let _ = std::fs::write(&pfile, &buf);
    Some(buf)
}
