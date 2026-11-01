//! Foldery programu: dane w %APPDATA%\OpenMP, pamięć podręczna w %LOCALAPPDATA%\OpenMP.
//! Pliki danych (ustawienia, listy, statystyki, biblioteka) są skompresowane i zaszyfrowane.

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
    read_sealed(&data().join(name)).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save_json<T: serde::Serialize>(name: &str, v: &T) {
    write_sealed(&data().join(name), &serde_json::to_vec(v).unwrap_or_default());
}

/// Nagłówek pliku zapisanego przez `seal`: skompresowany (LZ4) i zaszyfrowany (DPAPI).
const MAGIC: &[u8; 4] = b"OMP1";

/// Dane programu na dysku: kompresja LZ4 (szybki odczyt) i szyfrowanie kluczem konta Windows.
pub fn seal(plain: &[u8]) -> Vec<u8> {
    let packed = lz4_flex::compress_prepend_size(plain);
    match dpapi::protect(&packed) {
        Some(enc) => {
            let mut out = MAGIC.to_vec();
            out.extend_from_slice(&enc);
            out
        }
        // Bez szyfrowania (system go nie udostępnia) – zwykły JSON, czytelny dla starszych wersji.
        None => plain.to_vec(),
    }
}

/// Odwrotność `seal`; stary, zwykły JSON przechodzi bez zmian.
pub fn unseal(bytes: &[u8]) -> Option<Vec<u8>> {
    let Some(enc) = bytes.strip_prefix(MAGIC.as_slice()) else { return Some(bytes.to_vec()) };
    let packed = dpapi::unprotect(enc)?;
    lz4_flex::decompress_size_prepended(&packed).ok()
}

/// Odczyt pliku danych. Gdy nie da się go odszyfrować (inne konto, inny komputer), zostaje kopia
/// `.bak`, a program zaczyna od ustawień domyślnych zamiast nadpisać plik bez śladu.
pub fn read_sealed(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let out = unseal(&bytes);
    if out.is_none() {
        let _ = std::fs::copy(path, path.with_extension("bak"));
        log(&format!("nie udało się odczytać {}", path.display()));
    }
    out
}

pub fn write_sealed(path: &Path, plain: &[u8]) {
    write_atomic(path, &seal(plain));
}

const BACKUP_FORMAT: &str = "openmp-backup-1";

/// Kopia danych (ustawienia, listy, statystyki…) w jednym zwykłym pliku JSON. Pliki w folderze
/// danych są zaszyfrowane kluczem konta Windows, więc kopia zapisuje je odszyfrowane – da się ją
/// przenieść na inny komputer. Zwraca liczbę zapisanych plików.
pub fn export_backup(dest: &Path) -> Result<usize, String> {
    let mut files = serde_json::Map::new();
    let dir = std::fs::read_dir(data()).map_err(|e| e.to_string())?;
    for e in dir.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "json") {
            let Some(value) = read_sealed(&p).and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else { continue };
            files.insert(e.file_name().to_string_lossy().into_owned(), value);
        }
    }
    let n = files.len();
    let doc = serde_json::json!({ "format": BACKUP_FORMAT, "version": crate::version(), "files": files });
    std::fs::write(dest, serde_json::to_vec_pretty(&doc).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(n)
}

/// Przywraca kopię z `export_backup` – nadpisuje pliki danych (zaszyfrowane od nowa).
/// Program trzeba potem uruchomić ponownie, bez zapisywania stanu z pamięci.
pub fn import_backup(src: &Path) -> Result<usize, String> {
    let bytes = std::fs::read(src).map_err(|e| e.to_string())?;
    let doc: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if doc["format"] != BACKUP_FORMAT {
        return Err(crate::tr!("This is not an OpenMP backup").to_string());
    }
    let Some(files) = doc["files"].as_object() else { return Err(crate::tr!("This is not an OpenMP backup").to_string()) };
    let mut n = 0;
    for (name, value) in files {
        // Tylko zwykłe nazwy plików .json – bez ścieżek prowadzących poza folder danych.
        let ok = name.ends_with(".json") && !name.contains(['/', '\\', ':']) && !name.starts_with('.');
        if ok {
            write_sealed(&data().join(name), &serde_json::to_vec(value).unwrap_or_default());
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(windows)]
mod dpapi {
    use std::ffi::c_void;

    #[repr(C)]
    struct Blob {
        cb: u32,
        pb: *mut u8,
    }

    const UI_FORBIDDEN: u32 = 0x1;

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(din: *const Blob, descr: *const u16, entropy: *const Blob, reserved: *mut c_void, prompt: *mut c_void, flags: u32, dout: *mut Blob) -> i32;
        fn CryptUnprotectData(din: *const Blob, descr: *mut *mut u16, entropy: *const Blob, reserved: *mut c_void, prompt: *mut c_void, flags: u32, dout: *mut Blob) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(h: *mut c_void) -> *mut c_void;
    }

    /// Stała „sól” programu – dane innego programu tego samego konta nie odczytają się jako nasze.
    const ENTROPY: &[u8] = b"OpenMP/data/v1";

    fn run(data: &[u8], encrypt: bool) -> Option<Vec<u8>> {
        let din = Blob { cb: data.len() as u32, pb: data.as_ptr() as *mut u8 };
        let ent = Blob { cb: ENTROPY.len() as u32, pb: ENTROPY.as_ptr() as *mut u8 };
        let mut dout = Blob { cb: 0, pb: std::ptr::null_mut() };
        let ok = unsafe {
            if encrypt {
                CryptProtectData(&din, std::ptr::null(), &ent, std::ptr::null_mut(), std::ptr::null_mut(), UI_FORBIDDEN, &mut dout)
            } else {
                CryptUnprotectData(&din, std::ptr::null_mut(), &ent, std::ptr::null_mut(), std::ptr::null_mut(), UI_FORBIDDEN, &mut dout)
            }
        };
        if ok == 0 || dout.pb.is_null() {
            return None;
        }
        let out = unsafe { std::slice::from_raw_parts(dout.pb, dout.cb as usize).to_vec() };
        unsafe { LocalFree(dout.pb as *mut c_void) };
        Some(out)
    }

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, true)
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        run(data, false)
    }
}

#[cfg(not(windows))]
mod dpapi {
    pub fn protect(_: &[u8]) -> Option<Vec<u8>> {
        None
    }
    pub fn unprotect(_: &[u8]) -> Option<Vec<u8>> {
        None
    }
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
