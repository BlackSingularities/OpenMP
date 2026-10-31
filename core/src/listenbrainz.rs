//! Scrobbling do ListenBrainz: wysłuchany utwór (połowa albo 4 minuty) trafia do profilu
//! użytkownika. Wystarczy token z https://listenbrainz.org/settings/.

use serde_json::json;

use crate::meta::TrackInfo;

const URL: &str = "https://api.listenbrainz.org/1/submit-listens";

/// Czy utwór jest już „wysłuchany” wg zasad ListenBrainz/Last.fm.
pub fn due(pos: f64, duration: f64) -> bool {
    duration >= 30.0 && pos >= (duration / 2.0).min(240.0)
}

/// Wysyła odsłuch w tle; błędy (brak sieci, zły token) zapisuje tylko do dziennika.
pub fn submit(token: &str, t: &TrackInfo, listened_at: u64) {
    let artist = t.display_artist();
    let title = t.display_title();
    if token.trim().is_empty() || artist.trim().is_empty() || title.trim().is_empty() {
        return;
    }
    let mut info = json!({ "media_player": "OpenMP", "submission_client": "OpenMP" });
    if t.duration > 0.0 {
        info["duration_ms"] = json!((t.duration * 1000.0) as u64);
    }
    let mut meta = json!({ "artist_name": artist, "track_name": title, "additional_info": info });
    if !t.album.trim().is_empty() {
        meta["release_name"] = json!(t.album);
    }
    let body = json!({
        "listen_type": "single",
        "payload": [{ "listened_at": listened_at, "track_metadata": meta }],
    });
    let auth = format!("Token {}", token.trim());
    std::thread::spawn(move || {
        if let Err(e) = ureq::post(URL).set("Authorization", &auth).timeout(std::time::Duration::from_secs(15)).send_json(body) {
            crate::paths::log(&format!("ListenBrainz: {e}"));
        }
    });
}
