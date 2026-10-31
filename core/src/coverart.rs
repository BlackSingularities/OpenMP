//! Brakujące okładki albumów z MusicBrainz i Cover Art Archive.
//!
//! Wyniki trafiają do pamięci podręcznej; brak okładki też jest zapamiętywany (pusty plik),
//! żeby nie pytać serwerów przy każdym otwarciu biblioteki. MusicBrainz pozwala na jedno
//! zapytanie na sekundę – pilnuje tego wspólna blokada.

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const UA: &str = "OpenMP/1.2 (https://redzikowski.com)";

static ENABLED: AtomicBool = AtomicBool::new(true);
static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// Włącza albo wyłącza pobieranie (ustawienie „Pobieraj brakujące okładki”).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

fn cache_file(artist: &str, album: &str) -> PathBuf {
    let key = format!("{}\u{1f}{}", artist.trim().to_lowercase(), album.trim().to_lowercase());
    let h = Sha256::digest(key.as_bytes());
    let name: String = h.iter().take(12).map(|b| format!("{b:02x}")).collect();
    let d = crate::paths::cache().join("covers");
    let _ = std::fs::create_dir_all(&d);
    d.join(format!("{name}.jpg"))
}

/// Odstęp co najmniej sekundy między zapytaniami do MusicBrainz.
fn throttle() {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(t) = *last {
        let wait = Duration::from_millis(1100).saturating_sub(t.elapsed());
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
    }
    *last = Some(Instant::now());
}

/// Fraza wyszukiwania Lucene – cudzysłowy i ukośniki w nazwach nie psują zapytania.
fn quote(s: &str) -> String {
    format!("\"{}\"", s.trim().replace(['"', '\\'], " "))
}

/// Okładka albumu (blokujące – wywoływać poza wątkiem interfejsu).
pub fn fetch(artist: &str, album: &str) -> Option<Vec<u8>> {
    if artist.trim().is_empty() || album.trim().is_empty() {
        return None;
    }
    let file = cache_file(artist, album);
    if let Ok(b) = std::fs::read(&file) {
        return (!b.is_empty()).then_some(b);
    }
    if !ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    throttle();
    let q = format!("release:{} AND artist:{}", quote(album), quote(artist));
    let found = ureq::get("https://musicbrainz.org/ws/2/release/")
        .set("User-Agent", UA)
        .timeout(Duration::from_secs(12))
        .query("query", &q)
        .query("limit", "5")
        .query("fmt", "json")
        .call();
    let v: serde_json::Value = match found {
        Ok(r) => r.into_json().ok()?,
        // Błąd sieci albo przeciążenie – spróbujemy następnym razem, bez zapamiętywania.
        Err(_) => return None,
    };
    let ids: Vec<String> = v["releases"]
        .as_array()
        .map(|a| a.iter().filter(|r| r["score"].as_u64().unwrap_or(0) >= 90).filter_map(|r| r["id"].as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    for id in ids {
        let url = format!("https://coverartarchive.org/release/{id}/front-500");
        let Ok(r) = ureq::get(&url).set("User-Agent", UA).timeout(Duration::from_secs(15)).call() else { continue };
        let mut bytes = Vec::new();
        if r.into_reader().take(8 << 20).read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
            crate::paths::write_atomic(&file, &bytes);
            return Some(bytes);
        }
    }
    // Żadne wydanie nie ma okładki – zapamiętujemy, żeby nie pytać ponownie.
    crate::paths::write_atomic(&file, &[]);
    None
}
