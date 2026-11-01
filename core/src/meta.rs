//! Metadane plików: tagi, parametry techniczne, okładki, ReplayGain, teksty piosenek; zapis tagów.

use std::path::{Path, PathBuf};

use lofty::config::WriteOptions;
use lofty::picture::{Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};
use serde::{Deserialize, Serialize};

/// Rozszerzenia plików dźwiękowych.
pub const AUDIO: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "m4b", "aac", "alac", "wav", "wave", "aif", "aiff", "aifc", "wma", "ape", "wv", "mpc",
    "tta", "dsf", "dff", "mka", "caf", "mp2", "mp1", "ac3", "dts", "amr", "spx", "mid", "midi", "mod", "xm", "it", "s3m", "w64",
];
/// Rozszerzenia plików wideo.
pub const VIDEO: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "webm", "flv", "m4v", "mpg", "mpeg", "m2ts", "mts", "ts", "vob", "3gp", "3g2", "ogv", "divx",
    "xvid", "rm", "rmvb", "asf", "f4v", "hevc", "h264", "y4m", "dav",
];
/// Listy odtwarzania.
pub const PLAYLISTS: &[&str] = &["m3u", "m3u8", "pls", "cue", "xspf"];
/// Napisy.
pub const SUBTITLES: &[&str] = &["srt", "ass", "ssa", "sub", "vtt", "idx", "sup", "smi", "txt"];

pub fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}
pub fn is_audio(p: &Path) -> bool {
    AUDIO.contains(&ext(p).as_str())
}
pub fn is_video(p: &Path) -> bool {
    VIDEO.contains(&ext(p).as_str())
}
pub fn is_media(p: &Path) -> bool {
    is_audio(p) || is_video(p)
}
pub fn is_url(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    ["http://", "https://", "rtsp://", "rtmp://", "mms://", "udp://", "ftp://", "ytdl://"].iter().any(|p| l.starts_with(p))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrackInfo {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: u32,
    pub track: u32,
    pub disc: u32,
    pub duration: f64,
    pub bitrate: u32,
    pub rate: u32,
    pub channels: u8,
    pub bits: u8,
    pub size: u64,
    pub modified: u64,
    pub rg_track: Option<f32>,
    pub rg_album: Option<f32>,
    pub rg_peak: Option<f32>,
    pub composer: String,
    pub comment: String,
    pub video: bool,
}

impl TrackInfo {
    /// Tytuł do wyświetlenia – z tagu albo z nazwy pliku.
    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.clone();
        }
        if is_url(&self.path) {
            return self.path.clone();
        }
        Path::new(&self.path).file_stem().map(|s| s.to_string_lossy().replace('_', " ")).unwrap_or_default()
    }

    pub fn display_artist(&self) -> String {
        if !self.artist.trim().is_empty() {
            self.artist.clone()
        } else {
            self.album_artist.clone()
        }
    }

    pub fn format(&self) -> String {
        if is_url(&self.path) {
            return "STREAM".into();
        }
        ext(Path::new(&self.path)).to_uppercase()
    }
}

fn parse_gain(s: &str) -> Option<f32> {
    s.trim().trim_end_matches("dB").trim_end_matches("db").trim().replace(',', ".").parse().ok()
}

fn mtime(p: &Path) -> (u64, u64) {
    let m = std::fs::metadata(p).ok();
    let size = m.as_ref().map(|m| m.len()).unwrap_or(0);
    let mt = m
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (size, mt)
}

/// Odczyt informacji o pliku. Nie przerywa się na błędach – zawsze coś zwraca.
pub fn read(path: &Path) -> TrackInfo {
    let (size, modified) = mtime(path);
    let mut t = TrackInfo { path: path.to_string_lossy().into_owned(), size, modified, video: is_video(path), ..Default::default() };
    let Ok(f) = lofty::read_from_path(path) else {
        return t;
    };
    if is_video(path) {
        // Kontenery MP4/MOV – długość z nagłówka; tytuł z tagu, jeśli jest.
        t.duration = f.properties().duration().as_secs_f64();
        if let Some(tag) = f.primary_tag().or_else(|| f.first_tag()) {
            t.title = tag.title().map(|s| s.trim().to_string()).unwrap_or_default();
            t.year = tag.year().unwrap_or(0);
            t.genre = tag.genre().map(|s| s.trim().to_string()).unwrap_or_default();
        }
        return t;
    }
    let p = f.properties();
    t.duration = p.duration().as_secs_f64();
    t.bitrate = p.audio_bitrate().or(p.overall_bitrate()).unwrap_or(0);
    t.rate = p.sample_rate().unwrap_or(0);
    t.channels = p.channels().unwrap_or(0);
    t.bits = p.bit_depth().unwrap_or(0);
    if let Some(tag) = f.primary_tag().or_else(|| f.first_tag()) {
        t.title = tag.title().map(|s| s.trim().to_string()).unwrap_or_default();
        t.artist = tag.artist().map(|s| s.trim().to_string()).unwrap_or_default();
        t.album = tag.album().map(|s| s.trim().to_string()).unwrap_or_default();
        t.genre = tag.genre().map(|s| s.trim().to_string()).unwrap_or_default();
        t.comment = tag.comment().map(|s| s.trim().to_string()).unwrap_or_default();
        t.year = tag.year().unwrap_or(0);
        if t.year == 0 {
            t.year = tag
                .get_string(&ItemKey::RecordingDate)
                .or_else(|| tag.get_string(&ItemKey::Year))
                .and_then(|d| d.get(..4)?.parse().ok())
                .unwrap_or(0);
        }
        t.track = tag.track().unwrap_or(0);
        t.disc = tag.disk().unwrap_or(0);
        t.album_artist = tag.get_string(&ItemKey::AlbumArtist).unwrap_or("").trim().to_string();
        t.composer = tag.get_string(&ItemKey::Composer).unwrap_or("").trim().to_string();
        t.rg_track = tag.get_string(&ItemKey::ReplayGainTrackGain).and_then(parse_gain);
        t.rg_album = tag.get_string(&ItemKey::ReplayGainAlbumGain).and_then(parse_gain);
        t.rg_peak = tag.get_string(&ItemKey::ReplayGainTrackPeak).and_then(parse_gain);
    }
    t
}

/// Okładka: osadzona w pliku albo cover/folder/front.jpg w katalogu.
pub fn cover(path: &Path) -> Option<Vec<u8>> {
    if let Ok(f) = lofty::read_from_path(path) {
        for tag in f.tags() {
            let pics = tag.pictures();
            if let Some(p) = pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or(pics.first()) {
                return Some(p.data().to_vec());
            }
        }
    }
    folder_cover(path).and_then(|p| std::fs::read(p).ok())
}

pub fn folder_cover(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let names = ["cover", "folder", "front", "album", "albumart", "albumartsmall", "artwork", "poster", "thumb", "fanart"];
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        let ex = ext(&p);
        if !["jpg", "jpeg", "png", "webp", "bmp"].contains(&ex.as_str()) {
            continue;
        }
        let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        if names.iter().any(|n| stem == *n) {
            return Some(p);
        }
        if found.is_none() && (stem.contains("cover") || stem.contains("front")) {
            found = Some(p);
        }
    }
    found
}

/// Tekst piosenki: plik .lrc obok, potem tag USLT/LYRICS.
pub fn lyrics(path: &Path) -> Option<String> {
    let lrc = path.with_extension("lrc");
    if let Ok(b) = std::fs::read(&lrc) {
        return Some(String::from_utf8_lossy(&b).into_owned());
    }
    let f = lofty::read_from_path(path).ok()?;
    for tag in f.tags() {
        if let Some(s) = tag.get_string(&ItemKey::Lyrics) {
            if !s.trim().is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Zmiany tagów z edytora; puste pole czyści wartość.
#[derive(Clone, Debug, Default)]
pub struct TagEdit {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: String,
    pub track: String,
    pub disc: String,
    pub composer: String,
    pub comment: String,
    pub cover: Option<Vec<u8>>,
}

pub fn write(path: &Path, e: &TagEdit) -> Result<(), String> {
    let mut f = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    if f.primary_tag().is_none() {
        let tt = f.primary_tag_type();
        f.insert_tag(Tag::new(tt));
    }
    let tag = f.primary_tag_mut().ok_or("Brak tagu")?;
    let set = |tag: &mut Tag, key: ItemKey, v: &str| {
        if v.trim().is_empty() {
            tag.remove_key(&key);
        } else {
            tag.insert_text(key, v.trim().to_string());
        }
    };
    set(tag, ItemKey::TrackTitle, &e.title);
    set(tag, ItemKey::TrackArtist, &e.artist);
    set(tag, ItemKey::AlbumTitle, &e.album);
    set(tag, ItemKey::AlbumArtist, &e.album_artist);
    set(tag, ItemKey::Genre, &e.genre);
    set(tag, ItemKey::Composer, &e.composer);
    set(tag, ItemKey::Comment, &e.comment);
    match e.year.trim().parse::<u32>() {
        Ok(y) => tag.set_year(y),
        Err(_) => tag.remove_year(),
    }
    match e.track.trim().parse::<u32>() {
        Ok(n) => tag.set_track(n),
        Err(_) => tag.remove_track(),
    }
    match e.disc.trim().parse::<u32>() {
        Ok(n) => tag.set_disk(n),
        Err(_) => tag.remove_disk(),
    }
    if let Some(data) = &e.cover {
        tag.remove_picture_type(PictureType::CoverFront);
        if let Ok(mut p) = Picture::from_reader(&mut std::io::Cursor::new(data.clone())) {
            p.set_pic_type(PictureType::CoverFront);
            tag.push_picture(p);
        }
    }
    tag.save_to_path(path, WriteOptions::default()).map_err(|e| e.to_string())
}

/// Czas w formacie m:ss albo h:mm:ss.
pub fn fmt_time(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "0:00".into();
    }
    let s = secs as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// Czas łączny listy, np. „2 h 14 min”.
pub fn fmt_total(secs: f64) -> String {
    let s = secs as u64;
    let (d, h, m) = (s / 86400, s / 3600 % 24, s / 60 % 60);
    if d > 0 {
        format!("{d} d {h} h {m} min")
    } else if h > 0 {
        format!("{h} h {m} min")
    } else {
        format!("{m} min {} s", s % 60)
    }
}

/// Synchronizowany tekst LRC: (czas w s, linia). Pusty – tekst bez znaczników.
pub fn parse_lrc(text: &str) -> Vec<(f64, String)> {
    let mut out = Vec::new();
    let mut offset = 0.0;
    for line in text.lines() {
        let mut rest = line.trim();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let tag = &stripped[..end];
            if let Some(o) = tag.strip_prefix("offset:") {
                offset = o.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            } else if let Some((m, s)) = tag.split_once(':') {
                if let (Ok(m), Ok(s)) = (m.trim().parse::<f64>(), s.trim().replace(',', ".").parse::<f64>()) {
                    stamps.push(m * 60.0 + s);
                }
            }
            rest = stripped[end + 1..].trim_start();
        }
        for t in stamps {
            out.push(((t - offset).max(0.0), rest.to_string()));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// Tekst kwestii bez znaczników ASS/HTML – też dla napisów z kontenera (mkvsubs).
pub fn clean_sub(s: &str) -> String {
    let s = s.replace("\\N", " ").replace("\\n", " ");
    let mut out = String::new();
    let mut skip = None;
    for ch in s.chars() {
        match (skip, ch) {
            (None, '{') => skip = Some('}'),
            (None, '<') => skip = Some('>'),
            (Some(e), c) if c == e => skip = None,
            (Some(_), _) => {}
            (None, c) => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Napisy filmu z pliku SRT, VTT albo ASS/SSA: (czas w s, linia). Puste – nieznany format.
pub fn parse_subs(text: &str) -> Vec<(f64, String)> {
    fn stamp(s: &str) -> Option<f64> {
        let s = s.trim().replace(',', ".");
        let p: Vec<&str> = s.split(':').collect();
        let n = |x: &str| x.trim().parse::<f64>().ok();
        match p.len() {
            3 => Some(n(p[0])? * 3600.0 + n(p[1])? * 60.0 + n(p[2])?),
            2 => Some(n(p[0])? * 60.0 + n(p[1])?),
            _ => None,
        }
    }
    let clean = clean_sub;
    let text = text.trim_start_matches('\u{feff}');
    let mut out = Vec::new();
    if text.contains("[Events]") {
        // Pola: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("Dialogue:") else { continue };
            let f: Vec<&str> = rest.splitn(10, ',').collect();
            if f.len() == 10 {
                if let Some(t) = stamp(f[1]) {
                    let l = clean(f[9]);
                    if !l.is_empty() {
                        out.push((t, l));
                    }
                }
            }
        }
    } else {
        let mut lines = text.lines().peekable();
        while let Some(line) = lines.next() {
            let Some((a, _)) = line.split_once("-->") else { continue };
            let Some(t) = stamp(a) else { continue };
            let mut parts = Vec::new();
            while let Some(l) = lines.peek() {
                if l.trim().is_empty() {
                    break;
                }
                parts.push(clean(l));
                lines.next();
            }
            let l = parts.join(" ").trim().to_string();
            if !l.is_empty() {
                out.push((t, l));
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}
