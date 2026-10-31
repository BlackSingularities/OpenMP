//! Radio internetowe: katalog radio-browser.info (otwarty, bez klucza) i zapisane stacje.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Station {
    pub name: String,
    pub url: String,
    pub country: String,
    pub codec: String,
    pub bitrate: u32,
    pub tags: String,
    pub favicon: String,
}

impl Station {
    pub fn info(&self) -> String {
        let mut parts = Vec::new();
        if !self.country.is_empty() {
            parts.push(self.country.clone());
        }
        if !self.codec.is_empty() {
            parts.push(if self.bitrate > 0 { format!("{} {} kb/s", self.codec, self.bitrate) } else { self.codec.clone() });
        }
        if !self.tags.is_empty() {
            parts.push(self.tags.split(',').take(4).collect::<Vec<_>>().join(", "));
        }
        parts.join(" · ")
    }
}

const SERVERS: [&str; 3] = ["de1.api.radio-browser.info", "nl1.api.radio-browser.info", "at1.api.radio-browser.info"];

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `source`: 0 – Polska, 1 – świat; `query` – nazwa albo gatunek (puste = najpopularniejsze).
pub fn fetch(source: i32, query: &str) -> Result<Vec<Station>, String> {
    let mut params = String::from("order=clickcount&reverse=true&hidebroken=true&limit=250");
    if source == 0 {
        params.push_str("&countrycode=PL");
    }
    let q = query.trim();
    let mut last = String::new();
    for server in SERVERS {
        let by_name = format!("https://{server}/json/stations/search?{params}{}", if q.is_empty() { String::new() } else { format!("&name={}", enc(q)) });
        match get(&by_name) {
            Ok(mut v) => {
                if !q.is_empty() && v.len() < 20 {
                    // Mało trafień po nazwie – dokładamy wyszukiwanie po gatunku.
                    if let Ok(more) = get(&format!("https://{server}/json/stations/search?{params}&tag={}", enc(&q.to_lowercase()))) {
                        for s in more {
                            if !v.iter().any(|x| x.url == s.url) {
                                v.push(s);
                            }
                        }
                    }
                }
                return Ok(v);
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn get(url: &str) -> Result<Vec<Station>, String> {
    let v: serde_json::Value = ureq::get(url)
        .set("User-Agent", "OpenMP/1.0")
        .timeout(std::time::Duration::from_secs(12))
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    Ok(v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    let url = s["url_resolved"].as_str().filter(|u| !u.is_empty()).or(s["url"].as_str())?.to_string();
                    Some(Station {
                        name: s["name"].as_str().unwrap_or("").trim().to_string(),
                        url,
                        country: s["country"].as_str().unwrap_or("").to_string(),
                        codec: s["codec"].as_str().unwrap_or("").to_string(),
                        bitrate: s["bitrate"].as_u64().unwrap_or(0) as u32,
                        tags: s["tags"].as_str().unwrap_or("").to_string(),
                        favicon: s["favicon"].as_str().unwrap_or("").to_string(),
                    })
                })
                .filter(|s| !s.name.is_empty())
                .collect()
        })
        .unwrap_or_default())
}

pub fn load_saved() -> Vec<Station> {
    crate::paths::load_json("radio.json")
}

pub fn save_saved(v: &Vec<Station>) {
    crate::paths::save_json("radio.json", v);
}
