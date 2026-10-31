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
