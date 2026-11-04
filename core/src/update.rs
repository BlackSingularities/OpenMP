//! Aktualizacje z portfolio redzikowski.com – serwer podaje wydania w formacie GitHub API,
//! więc program nie potrzebuje żadnego klucza.
//!
//! Wydanie zawiera `OpenMP-Setup-<wersja>.exe` i `SHA256SUMS.txt`. Instalator pobieramy do
//! pamięci podręcznej, sprawdzamy sumę i uruchamiamy po cichu – sam zamyka i uruchamia program.

use std::io::{Read, Write};
use std::path::PathBuf;

use sha2::{Digest, Sha256};

const FEED: &str = "https://redzikowski.com/api/github/repos/BlackSingularities/OpenMP/releases/latest";
const UA: &str = "OpenMP";

fn get(url: &str, accept: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("User-Agent", UA)
        .set("Accept", accept)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => crate::tr!("No releases to download").to_string(),
            e => e.to_string(),
        })
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    asset: String,
    name: String,
    sums: Option<String>,
}

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn newer(a: &str, b: &str) -> bool {
    parse(a) > parse(b)
}

/// Najnowsze wydanie, jeśli nowsze od tej wersji.
pub fn check() -> Result<Option<Release>, String> {
    let r: serde_json::Value = get(FEED, "application/vnd.github+json")?.into_json().map_err(|e| e.to_string())?;
    let version = r["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    if version.is_empty() || !newer(&version, crate::version()) {
        return Ok(None);
    }
    let assets = r["assets"].as_array().cloned().unwrap_or_default();
    let find = |pred: &dyn Fn(&str) -> bool| assets.iter().find(|a| a["name"].as_str().is_some_and(|n| pred(&n.to_lowercase()))).cloned();
    let Some(exe) = find(&|n| n.starts_with("openmp-setup-") && n.ends_with(".exe")) else { return Ok(None) };
    Ok(Some(Release {
        version,
        notes: String::new(),
        asset: exe["browser_download_url"].as_str().unwrap_or_default().to_string(),
        name: exe["name"].as_str().unwrap_or("OpenMP-Setup.exe").to_string(),
        sums: find(&|n| n == "sha256sums.txt").and_then(|a| a["browser_download_url"].as_str().map(str::to_string)),
    }))
}

/// Pobiera instalator i sprawdza sumę SHA-256. `progress(pobrane, całość)`.
pub fn download(rel: &Release, progress: impl Fn(u64, u64)) -> Result<PathBuf, String> {
    let expected = match &rel.sums {
        Some(url) => get(url, "application/octet-stream")?
            .into_string()
            .map_err(|e| e.to_string())?
            .lines()
            .find(|l| l.trim_end().ends_with(&rel.name))
            .and_then(|l| l.split_whitespace().next())
            .map(|h| h.to_lowercase()),
        None => None,
    };
    let expected = expected.ok_or("Wydanie nie ma sumy kontrolnej instalatora")?;
    let resp = get(&rel.asset, "application/octet-stream")?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let folder = crate::paths::cache().join("update");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(&rel.name);
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut rd = resp.into_reader();
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut done = 0u64;
    loop {
        let n = rd.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        h.update(&buf[..n]);
        done += n as u64;
        progress(done, total.max(done));
    }
    drop(f);
    if format!("{:x}", h.finalize()) != expected {
        let _ = std::fs::remove_file(&path);
        return Err(crate::tr!("The downloaded file's checksum does not match – update aborted").into());
    }
    Ok(path)
}

/// Uruchamia instalator po cichu; on zamyka OpenMP, podmienia pliki i uruchamia program ponownie.
pub fn apply(setup: &std::path::Path) -> Result<(), String> {
    std::process::Command::new(setup)
        .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Program zainstalowany instalatorem (aktualizacja możliwa) czy przenośny.
pub fn installed() -> bool {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("unins000.exe").exists())).unwrap_or(false)
}
