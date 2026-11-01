//! OpenSubtitles: pobieranie napisów jednym kliknięciem (klucz API z opensubtitles.com).

use std::path::{Path, PathBuf};
use std::sync::RwLock;

static KEY: RwLock<String> = RwLock::new(String::new());

pub fn set_key(k: &str) {
    if let Ok(mut g) = KEY.write() {
        *g = k.trim().to_string();
    }
}

pub fn has_key() -> bool {
    KEY.read().map(|k| !k.is_empty()).unwrap_or(false)
}

const API: &str = "https://api.opensubtitles.com/api/v1";
const UA: &str = "OpenMP v1";

/// Najlepsze napisy w języku `lang` (np. „pl”) dla filmu – zapisane obok pliku (albo w pamięci
/// podręcznej, gdy folder filmu jest tylko do odczytu). Zwraca ścieżkę do napisów.
pub fn fetch(file: &Path, lang: &str) -> Result<PathBuf, String> {
    let key = KEY.read().map(|k| k.clone()).unwrap_or_default();
    if key.is_empty() {
        return Err(crate::tr!("No OpenSubtitles key – enter it in Settings → Library").into());
    }
    let (title, year, _) = crate::tmdb::guess(file);
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ep = episode(&stem);
    let mut url = format!("{API}/subtitles?languages={lang}&query={}", enc(&title.to_lowercase()));
    if let Some(y) = year {
        url.push_str(&format!("&year={y}"));
    }
    if let Some((s, e)) = ep {
        url.push_str(&format!("&season_number={s}&episode_number={e}"));
    }
    let res: serde_json::Value = ureq::get(&url)
        .set("Api-Key", &key)
        .set("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| format!("wyszukiwanie: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    // Najwięcej pobrań – zwykle najlepiej dopasowane.
    let mut best: Option<(u64, u64)> = None;
    for d in res["data"].as_array().into_iter().flatten() {
        let a = &d["attributes"];
        let dl = a["download_count"].as_u64().unwrap_or(0);
        if let Some(fid) = a["files"].get(0).and_then(|f| f["file_id"].as_u64()) {
            if best.map_or(true, |b| dl > b.1) {
                best = Some((fid, dl));
            }
        }
    }
    let (fid, _) = best.ok_or(crate::tr!("No subtitles found"))?;
    let link: serde_json::Value = ureq::post(&format!("{API}/download"))
        .set("Api-Key", &key)
        .set("User-Agent", UA)
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .send_json(serde_json::json!({ "file_id": fid }))
        .map_err(|e| format!("pobieranie: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let link = link["link"].as_str().ok_or(crate::tr!("OpenSubtitles returned no link (download limit?)"))?;
    let body = ureq::get(link).timeout(std::time::Duration::from_secs(20)).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    let name = format!("{stem}.{lang}.srt");
    let near = file.with_file_name(&name);
    if std::fs::write(&near, &body).is_ok() {
        return Ok(near);
    }
    let dir = crate::paths::cache().join("subs");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(name);
    std::fs::write(&p, &body).map_err(|e| e.to_string())?;
    Ok(p)
}

fn episode(s: &str) -> Option<(u32, u32)> {
    let l = s.to_lowercase();
    let b = l.as_bytes();
    for i in 0..b.len().saturating_sub(3) {
        if b[i] == b's' && b[i + 1].is_ascii_digit() {
            let rest = &l[i + 1..];
            let sn: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let after = &rest[sn.len()..];
            if let Some(e) = after.strip_prefix('e') {
                let en: String = e.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let (Ok(s), Ok(e)) = (sn.parse(), en.parse()) {
                    return Some((s, e));
                }
            }
        }
    }
    None
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
