//! Foldery programu: dane w %APPDATA%\OpenMP, pamięć podręczna w %LOCALAPPDATA%\OpenMP.

use std::path::{Path, PathBuf};

pub fn data() -> PathBuf {
    let d = dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("OpenMP");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn cache() -> PathBuf {
    let d = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("OpenMP");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Zapis przez plik tymczasowy – awaria w trakcie nie zostawi uciętego JSON-a.
pub fn write_atomic(path: &Path, bytes: &[u8]) {
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

pub fn load_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read(data().join(name)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save_json<T: serde::Serialize>(name: &str, v: &T) {
    write_atomic(&data().join(name), &serde_json::to_vec(v).unwrap_or_default());
}

/// Dziennik programu (awarie, zamknięcie) w %LOCALAPPDATA%\OpenMP\openmp.log – ostatnie ~256 KB.
pub fn log(msg: &str) {
    use std::io::Write;
    let path = cache().join("openmp.log");
    if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{t}] [{}] {msg}", std::process::id());
    }
}
