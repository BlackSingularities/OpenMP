//! Teksty piosenek z LRCLIB (lrclib.net) – wersja z sygnaturami czasu, gdy jest, inaczej zwykły tekst.
//!
//! Wyniki trafiają do pamięci podręcznej; brak tekstu też jest zapamiętywany (pusty plik),
//! żeby nie pytać serwera przy każdym odtworzeniu tego samego utworu.

use std::path::PathBuf;

use sha2::{Digest, Sha256};

const API: &str = "https://lrclib.net/api/get";
const UA: &str = "OpenMP (https://redzikowski.com)";

fn cache_file(artist: &str, title: &str) -> PathBuf {
    let key = format!("{}\u{1f}{}", artist.trim().to_lowercase(), title.trim().to_lowercase());
    let h = Sha256::digest(key.as_bytes());
    let name: String = h.iter().take(12).map(|b| format!("{b:02x}")).collect();
    let d = crate::paths::cache().join("lyrics");
    let _ = std::fs::create_dir_all(&d);
    // „v2” – wyniki sprzed szukania wersji z czasami (często sam tekst) pobieramy jeszcze raz.
    d.join(format!("{name}.v2.lrc"))
}

/// Tekst z sygnaturami czasu (LRC), a nie sam tekst.
pub fn is_synced(text: &str) -> bool {
    !crate::meta::parse_lrc(text).is_empty()
}

/// Tekst z pamięci podręcznej: `Some("")` oznacza, że serwer go nie zna.
pub fn cached(artist: &str, title: &str) -> Option<String> {
    std::fs::read(cache_file(artist, title)).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Tytuł bez dopisków wydania: „ - Remastered 2021”, „(Radio Edit)”, „[Live]”, „(feat. …)”.
fn clean_title(t: &str) -> String {
    let mut s = t.trim().to_string();
    let lower = s.to_lowercase();
    for sep in [" - ", " – ", " — "] {
        if let Some(i) = lower.find(sep) {
            let tail = &lower[i + sep.len()..];
            if ["remaster", "live", "edit", "version", "mix", "mono", "stereo", "bonus", "demo", "acoustic", "feat", "radio"].iter().any(|w| tail.contains(w)) {
                s.truncate(i);
                break;
            }
        }
    }
    for (o, c) in [('(', ')'), ('[', ']')] {
        while let (Some(i), Some(j)) = (s.rfind(o), s.rfind(c)) {
            if j < i || i == 0 {
                break;
            }
            s = format!("{}{}", s[..i].trim_end(), &s[j + c.len_utf8()..]);
        }
    }
    s.trim().to_string()
}

/// Pierwszy wykonawca z „A & B”, „A feat. B”, „A, B”.
fn main_artist(a: &str) -> String {
    let l = a.to_lowercase();
    let cut = [" feat.", " ft.", " featuring ", " & ", ", ", " x ", " and "].iter().filter_map(|s| l.find(s)).min();
    match cut {
        Some(i) => a[..i].trim().to_string(),
        None => a.trim().to_string(),
    }
}

enum Got {
    Found { synced: String, plain: String },
    Instrumental,
    Missing,
    NetError,
}

fn get(artist: &str, title: &str, album: &str, duration: f64) -> Got {
    let mut req = ureq::get(API)
        .set("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(12))
        .query("artist_name", artist)
        .query("track_name", title);
    if !album.is_empty() {
        req = req.query("album_name", album);
    }
    if duration > 0.0 {
        req = req.query("duration", &format!("{}", duration.round() as i64));
    }
    match req.call() {
        Ok(r) => {
            let Ok(v) = r.into_json::<serde_json::Value>() else { return Got::NetError };
            if v["instrumental"].as_bool() == Some(true) {
                return Got::Instrumental;
            }
            Got::Found {
                synced: v["syncedLyrics"].as_str().unwrap_or("").trim().to_string(),
                plain: v["plainLyrics"].as_str().unwrap_or("").trim().to_string(),
            }
        }
        Err(ureq::Error::Status(404, _)) => Got::Missing,
        Err(_) => Got::NetError,
    }
}

/// Wyszukiwanie: najbliższy czasem wynik z sygnaturami, inaczej pierwszy z samym tekstem.
fn search(artist: &str, title: &str, duration: f64) -> Option<(String, String)> {
    let r = ureq::get(&API.replace("/get", "/search"))
        .set("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(12))
        .query("artist_name", artist)
        .query("track_name", title)
        .call()
        .ok()?;
    let v: serde_json::Value = r.into_json().ok()?;
    let items = v.as_array()?;
    let diff = |x: &serde_json::Value| if duration > 0.0 { (x["duration"].as_f64().unwrap_or(0.0) - duration).abs() } else { 0.0 };
    let synced = items
        .iter()
        .filter(|x| !x["syncedLyrics"].as_str().unwrap_or("").trim().is_empty())
        .filter(|x| duration <= 0.0 || diff(x) <= 8.0)
        .min_by(|a, b| diff(a).total_cmp(&diff(b)))
        .and_then(|x| x["syncedLyrics"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let plain = items.iter().find_map(|x| x["plainLyrics"].as_str().filter(|s| !s.trim().is_empty())).unwrap_or("").trim().to_string();
    (!synced.is_empty() || !plain.is_empty()).then_some((synced, plain))
}

/// Zapytanie do LRCLIB (blokujące – wywoływać poza wątkiem interfejsu). Najpierw wersja
/// z sygnaturami czasu: dokładne dopasowanie, potem bez albumu, potem z oczyszczonym tytułem
/// i pierwszym wykonawcą, na końcu wyszukiwanie. Sam tekst – tylko gdy nigdzie nie ma czasów.
pub fn fetch(artist: &str, title: &str, album: &str, duration: f64) -> Option<String> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return None;
    }
    if let Some(c) = cached(artist, title) {
        return (!c.trim().is_empty()).then_some(c);
    }
    let (a0, t0, al) = (artist.trim().to_string(), title.trim().to_string(), album.trim().to_string());
    let (a1, t1) = (main_artist(&a0), clean_title(&t0));
    let mut attempts: Vec<(String, String, String)> = vec![(a0.clone(), t0.clone(), al.clone()), (a0.clone(), t0.clone(), String::new())];
    for (a, t) in [(a0.clone(), t1.clone()), (a1.clone(), t0.clone()), (a1.clone(), t1.clone())] {
        if !attempts.iter().any(|x| x.0 == a && x.1 == t) {
            attempts.push((a, t, String::new()));
        }
    }
    let mut plain = String::new();
    let mut any_answer = false;
    for (a, t, alb) in &attempts {
        match get(a, t, alb, duration) {
            Got::Found { synced, plain: p } => {
                any_answer = true;
                if !synced.is_empty() {
                    return store(artist, title, synced);
                }
                if plain.is_empty() {
                    plain = p;
                }
            }
            Got::Instrumental => return store(artist, title, String::new()).or(None),
            Got::Missing => any_answer = true,
            Got::NetError => {}
        }
    }
    let mut searched = false;
    for (a, t) in [(a0.clone(), t1.clone()), (a1.clone(), t1.clone())] {
        if let Some((s, p)) = search(&a, &t, duration) {
            searched = true;
            if !s.is_empty() {
                return store(artist, title, s);
            }
            if plain.is_empty() {
                plain = p;
            }
        }
        if a0 == a1 {
            break;
        }
    }
    // Bez żadnej odpowiedzi serwera (sieć) – nic nie zapamiętujemy, spróbujemy następnym razem.
    if !any_answer && !searched && plain.is_empty() {
        return None;
    }
    store(artist, title, plain)
}

fn store(artist: &str, title: &str, text: String) -> Option<String> {
    crate::paths::write_atomic(&cache_file(artist, title), text.as_bytes());
    (!text.is_empty()).then_some(text)
}

/// Zapis pobranego tekstu jako .lrc obok pliku (polecenie „Zapisz tekst obok utworu”).
pub fn save_next_to(path: &std::path::Path, text: &str) -> bool {
    std::fs::write(path.with_extension("lrc"), text.as_bytes()).is_ok()
}
