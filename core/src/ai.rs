//! Analiza biblioteki modelem językowym (klucz API użytkownika): język, nastrój, tematy i energia
//! utworu z jego metadanych, a listy „z opisu” – model zamienia opis na filtr, dopasowanie lokalne.
//! Dostawcy: Anthropic, OpenAI i dowolny serwer zgodny z API OpenAI (własny adres).

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::meta::TrackInfo;

/// Nastroje do wyboru przez model – stała lista, żeby kafelki się nie rozdrabniały.
pub const MOODS: [&str; 14] = [
    "calm", "happy", "sad", "energetic", "romantic", "melancholic", "angry", "dreamy", "nostalgic", "dark", "uplifting", "party", "focused", "epic",
];

/// Połączenie z dostawcą.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// 0 – Anthropic, 1 – OpenAI, 2 – zgodny z OpenAI (własny adres).
    pub provider: i32,
    pub key: String,
    pub url: String,
    pub model: String,
    pub temperature: f32,
}

impl Config {
    fn base(&self) -> String {
        match self.provider {
            0 => "https://api.anthropic.com/v1".into(),
            1 => "https://api.openai.com/v1".into(),
            _ => self.url.trim().trim_end_matches('/').to_string(),
        }
    }

    fn get(&self, path: &str) -> ureq::Request {
        let r = ureq::get(&format!("{}{path}", self.base())).timeout(Duration::from_secs(30));
        self.auth(r)
    }

    fn post(&self, path: &str) -> ureq::Request {
        let r = ureq::post(&format!("{}{path}", self.base())).timeout(Duration::from_secs(180));
        self.auth(r)
    }

    fn auth(&self, r: ureq::Request) -> ureq::Request {
        if self.provider == 0 {
            r.set("x-api-key", self.key.trim()).set("anthropic-version", "2023-06-01")
        } else if self.key.trim().is_empty() {
            r
        } else {
            r.set("Authorization", &format!("Bearer {}", self.key.trim()))
        }
    }
}

/// Opis błędu HTTP z treścią odpowiedzi dostawcy (np. zły klucz, brak środków).
fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(str::to_string).or_else(|| v["error"].as_str().map(str::to_string)))
                .unwrap_or(body);
            format!("HTTP {code}: {}", msg.chars().take(300).collect::<String>())
        }
        other => other.to_string(),
    }
}

/// Modele dostępne dla klucza – posortowane, najnowsze nazwy zwykle na górze listy dostawcy.
pub fn list_models(cfg: &Config) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    // Anthropic stronicuje listę; OpenAI zwraca całość od razu.
    for _ in 0..10 {
        let mut req = cfg.get("/models");
        if cfg.provider == 0 {
            req = req.query("limit", "100");
            if let Some(a) = &after {
                req = req.query("after_id", a);
            }
        }
        let v: Value = req.call().map_err(http_err)?.into_json().map_err(|e| e.to_string())?;
        let ids: Vec<String> = v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect()).unwrap_or_default();
        out.extend(ids);
        if cfg.provider == 0 && v["has_more"].as_bool() == Some(true) {
            after = v["last_id"].as_str().map(str::to_string);
            if after.is_none() {
                break;
            }
        } else {
            break;
        }
    }
    // OpenAI: tylko modele czatu (bez osadzeń, mowy, obrazów).
    if cfg.provider == 1 {
        out.retain(|m| {
            let m = m.to_lowercase();
            !["embedding", "whisper", "tts", "dall-e", "moderation", "image", "audio", "realtime", "transcribe", "search", "davinci", "babbage"].iter().any(|x| m.contains(x))
        });
        out.sort();
        out.reverse();
    }
    out.dedup();
    if out.is_empty() {
        return Err("no models".into());
    }
    Ok(out)
}

/// Jedno zapytanie: polecenie systemowe i treść; odpowiedź jako tekst.
pub fn chat(cfg: &Config, system: &str, user: &str) -> Result<String, String> {
    let temp = cfg.temperature.clamp(0.0, 1.0);
    if cfg.provider == 0 {
        let body = json!({
            "model": cfg.model,
            "max_tokens": 8000,
            "temperature": temp,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
        });
        let v: Value = cfg.post("/messages").send_json(body).map_err(http_err)?.into_json().map_err(|e| e.to_string())?;
        let text: String = v["content"].as_array().map(|a| a.iter().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
        return Ok(text);
    }
    let send = |with_temp: bool| {
        let mut body = json!({
            "model": cfg.model,
            "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }],
        });
        if with_temp {
            body["temperature"] = json!(temp);
        }
        cfg.post("/chat/completions").send_json(body)
    };
    // Część modeli (rozumujące) przyjmuje tylko domyślną temperaturę – wtedy ponawiamy bez niej.
    let resp = match send(true) {
        Err(ureq::Error::Status(400, r)) => {
            let body = r.into_string().unwrap_or_default();
            if body.contains("temperature") {
                send(false).map_err(http_err)?
            } else {
                return Err(format!("HTTP 400: {}", body.chars().take(300).collect::<String>()));
            }
        }
        other => other.map_err(http_err)?,
    };
    let v: Value = resp.into_json().map_err(|e| e.to_string())?;
    Ok(v["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string())
}

/// Pierwszy obiekt albo tablica JSON w odpowiedzi (model bywa, że doda zdanie albo ```json).
fn json_in(text: &str) -> Option<Value> {
    let start = text.find(['{', '['])?;
    let open = text.as_bytes()[start];
    let close = if open == b'{' { '}' } else { ']' };
    let end = text.rfind(close)?;
    serde_json::from_str(text.get(start..=end)?).ok()
}

/// Wynik analizy jednego utworu.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrackTags {
    /// Język śpiewu (nazwa w języku interfejsu) albo „instrumental”.
    pub lang: String,
    pub moods: Vec<String>,
    pub themes: Vec<String>,
    /// 1 – spokojne, 2 – umiarkowane, 3 – energetyczne (0 – nieznane).
    pub energy: u8,
}

/// Wyniki analizy całej biblioteki: ścieżka (małymi literami) → znaczniki.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Tags {
    pub map: HashMap<String, TrackTags>,
}

impl Tags {
    pub fn load() -> Self {
        crate::paths::load_json("ai_tags.json")
    }

    pub fn save(&self) {
        crate::paths::save_json("ai_tags.json", self);
    }

    pub fn get(&self, path: &str) -> Option<&TrackTags> {
        self.map.get(&path.to_lowercase())
    }

    pub fn has(&self, path: &str) -> bool {
        self.map.contains_key(&path.to_lowercase())
    }
}

/// Nazwa języka interfejsu dla modelu (wyniki w tym języku).
fn ui_language() -> &'static str {
    match crate::i18n::current() {
        "pl" => "Polish",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" | "pt-BR" => "Portuguese",
        "cs" => "Czech",
        "uk" => "Ukrainian",
        "ru" => "Russian",
        _ => "English",
    }
}

fn describe(t: &TrackInfo) -> String {
    let name = std::path::Path::new(&t.path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut s = format!("{} – {}", t.artist.trim(), if t.title.trim().is_empty() { name.as_str() } else { t.title.trim() });
    if !t.album.trim().is_empty() {
        s.push_str(&format!(" | album: {}", t.album.trim()));
    }
    if !t.genre.trim().is_empty() {
        s.push_str(&format!(" | genre: {}", t.genre.trim()));
    }
    if t.year > 0 {
        s.push_str(&format!(" | {}", t.year));
    }
    s
}

/// Analiza partii utworów (zwykle 40–60). Zwraca znaczniki dla numerów z listy.
pub fn analyze(cfg: &Config, tracks: &[TrackInfo]) -> Result<Vec<(usize, TrackTags)>, String> {
    let lang = ui_language();
    let system = format!(
        "You are a music librarian. For every numbered track (artist – title, album, genre, year) use your knowledge of the song and artist; \
         if you do not know it, infer from the metadata. Answer ONLY with a JSON array, one object per track: \
         {{\"i\": number, \"lang\": language of the lyrics written in {lang} (e.g. a language name), or \"instrumental\", \
         \"moods\": 1-3 values from [{}], \"themes\": 1-3 short lyrical themes in {lang} (lowercase, 1-2 words, e.g. love, travel, city, freedom), \
         \"energy\": 1 calm, 2 moderate or 3 energetic}}.",
        MOODS.join(", ")
    );
    let list: String = tracks.iter().enumerate().map(|(i, t)| format!("{i}. {}\n", describe(t))).collect();
    let text = chat(cfg, &system, &list)?;
    let v = json_in(&text).ok_or_else(|| "invalid answer".to_string())?;
    let arr = v.as_array().cloned().or_else(|| v["tracks"].as_array().cloned()).unwrap_or_default();
    let words = |x: &Value| -> Vec<String> { x.as_array().map(|a| a.iter().filter_map(|s| s.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).take(3).collect()).unwrap_or_default() };
    let mut out = Vec::new();
    for o in arr {
        let Some(i) = o["i"].as_u64().map(|i| i as usize).filter(|i| *i < tracks.len()) else { continue };
        let mut lang = o["lang"].as_str().unwrap_or("").trim().to_string();
        // Wielka litera na początku nazwy języka (model pisze różnie).
        if let Some(c) = lang.chars().next() {
            lang = c.to_uppercase().collect::<String>() + &lang[c.len_utf8()..].to_lowercase();
        }
        let moods = words(&o["moods"]).into_iter().filter(|m| MOODS.contains(&m.as_str())).collect();
        out.push((i, TrackTags { lang, moods, themes: words(&o["themes"]), energy: o["energy"].as_u64().unwrap_or(0).min(3) as u8 }));
    }
    Ok(out)
}

/// Filtr listy z opisu – kryteria dopasowywane lokalnie do biblioteki i wyników analizy.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Plan {
    pub name: String,
    pub description: String,
    pub languages: Vec<String>,
    pub moods: Vec<String>,
    pub themes: Vec<String>,
    pub genres: Vec<String>,
    pub artists: Vec<String>,
    pub energy: Vec<u8>,
    pub year_from: u32,
    pub year_to: u32,
    pub limit: usize,
}

/// Opis słowami → filtr. Model dostaje słownik znaczników z biblioteki, żeby trafiał w istniejące.
pub fn plan(cfg: &Config, description: &str, langs: &[String], themes: &[String], genres: &[String]) -> Result<Plan, String> {
    let lang = ui_language();
    let system = format!(
        "You turn a playlist request into a JSON filter for a local music library. Answer ONLY with one JSON object: \
         {{\"name\": short playlist title in {lang}, \"languages\": [...], \"moods\": [...], \"themes\": [...], \"genres\": [...], \"artists\": [...], \
         \"energy\": [1|2|3], \"year_from\": 0 or year, \"year_to\": 0 or year, \"limit\": number of tracks (default 60)}}. \
         Use only values that fit the request; leave others empty. Prefer values from these lists. \
         Languages: [{}]. Moods: [{}]. Themes: [{}]. Genres: [{}].",
        langs.join(", "),
        MOODS.join(", "),
        themes.iter().take(300).cloned().collect::<Vec<_>>().join(", "),
        genres.iter().take(200).cloned().collect::<Vec<_>>().join(", ")
    );
    let text = chat(cfg, &system, description)?;
    let mut v = json_in(&text).ok_or_else(|| "invalid answer".to_string())?;
    // Puste pola jako null – serde przyjmie wtedy wartości domyślne.
    if let Some(o) = v.as_object_mut() {
        o.retain(|_, x| !x.is_null());
    }
    let mut p: Plan = serde_json::from_value(v).map_err(|e| e.to_string())?;
    p.description = description.to_string();
    if p.name.trim().is_empty() {
        p.name = description.chars().take(40).collect();
    }
    if p.limit == 0 {
        p.limit = 60;
    }
    p.limit = p.limit.clamp(5, 500);
    Ok(p)
}

/// Dopasowanie utworu do filtru: 0 – odpada, więcej – lepiej pasuje.
pub fn score(p: &Plan, t: &TrackInfo, tags: Option<&TrackTags>) -> f32 {
    let low = |v: &[String]| v.iter().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let (langs, moods, themes, genres, artists) = (low(&p.languages), low(&p.moods), low(&p.themes), low(&p.genres), low(&p.artists));
    // Twarde warunki: rok i język.
    if (p.year_from > 0 && (t.year == 0 || t.year < p.year_from)) || (p.year_to > 0 && (t.year == 0 || t.year > p.year_to)) {
        return 0.0;
    }
    if !langs.is_empty() && !tags.is_some_and(|g| langs.contains(&g.lang.to_lowercase())) {
        return 0.0;
    }
    let mut s = 0.0;
    let mut wanted = 0;
    if !langs.is_empty() {
        s += 1.0;
    }
    if !artists.is_empty() {
        wanted += 1;
        let a = t.artist.to_lowercase();
        if artists.iter().any(|x| a.contains(x.as_str())) {
            s += 4.0;
        }
    }
    if !genres.is_empty() {
        wanted += 1;
        let g = t.genre.to_lowercase();
        if !g.is_empty() && genres.iter().any(|x| g.contains(x.as_str()) || x.contains(g.as_str())) {
            s += 2.0;
        }
    }
    if let Some(tg) = tags {
        if !moods.is_empty() {
            wanted += 1;
            s += 2.0 * tg.moods.iter().filter(|m| moods.contains(m)).count() as f32;
        }
        if !themes.is_empty() {
            wanted += 1;
            s += 2.0 * tg.themes.iter().filter(|m| themes.iter().any(|x| m.contains(x.as_str()) || x.contains(m.as_str()))).count() as f32;
        }
        if !p.energy.is_empty() {
            wanted += 1;
            if p.energy.contains(&tg.energy) {
                s += 1.0;
            }
        }
    }
    // Same twarde warunki (np. „polskie z lat 80.”) – każdy pasujący utwór się liczy.
    if wanted == 0 {
        return 1.0;
    }
    if s <= 1.0 && wanted > 0 && langs.is_empty() {
        return 0.0;
    }
    s
}
