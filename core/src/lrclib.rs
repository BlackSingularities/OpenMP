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
    d.join(format!("{name}.lrc"))
}

/// Tekst z pamięci podręcznej: `Some("")` oznacza, że serwer go nie zna.
pub fn cached(artist: &str, title: &str) -> Option<String> {
    std::fs::read(cache_file(artist, title)).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Zapytanie do LRCLIB (blokujące – wywoływać poza wątkiem interfejsu).
pub fn fetch(artist: &str, title: &str, album: &str, duration: f64) -> Option<String> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return None;
    }
    if let Some(c) = cached(artist, title) {
        return (!c.trim().is_empty()).then_some(c);
    }
    let mut req = ureq::get(API)
        .set("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(12))
        .query("artist_name", artist.trim())
        .query("track_name", title.trim());
    if !album.trim().is_empty() {
        req = req.query("album_name", album.trim());
    }
    if duration > 0.0 {
        req = req.query("duration", &format!("{}", duration.round() as i64));
    }
    let text = match req.call() {
        Ok(r) => {
            let v: serde_json::Value = r.into_json().ok()?;
            let synced = v["syncedLyrics"].as_str().unwrap_or("").trim().to_string();
            let plain = v["plainLyrics"].as_str().unwrap_or("").trim().to_string();
            if v["instrumental"].as_bool() == Some(true) {
                String::new()
            } else if !synced.is_empty() {
                synced
            } else {
                plain
            }
        }
        // 404 – utworu nie ma w bazie; zapamiętujemy. Błąd sieci – spróbujemy następnym razem.
        Err(ureq::Error::Status(404, _)) => String::new(),
        Err(_) => return None,
    };
    crate::paths::write_atomic(&cache_file(artist, title), text.as_bytes());
    (!text.is_empty()).then_some(text)
}

/// Zapis pobranego tekstu jako .lrc obok pliku (polecenie „Zapisz tekst obok utworu”).
pub fn save_next_to(path: &std::path::Path, text: &str) -> bool {
    std::fs::write(path.with_extension("lrc"), text.as_bytes()).is_ok()
}
