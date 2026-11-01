//! Napisy tekstowe zapisane w pliku Matroska (MKV/WebM) – bez ffmpeg. Czytnik przechodzi po
//! elementach EBML, wchodzi tylko do potrzebnych kontenerów, a bloki obrazu i dźwięku
//! przeskakuje po nagłówku, więc czyta z dysku głównie napisy. Obsługuje SRT (S_TEXT/UTF8),
//! ASS/SSA i WebVTT bez kompresji ścieżki.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const EBML: u32 = 0x1A45_DFA3;
const SEGMENT: u32 = 0x1853_8067;
const INFO: u32 = 0x1549_A966;
const TIMECODE_SCALE: u32 = 0x2A_D7B1;
const TRACKS: u32 = 0x1654_AE6B;
const TRACK_ENTRY: u32 = 0xAE;
const TRACK_NUMBER: u32 = 0xD7;
const CODEC_ID: u32 = 0x86;
const CONTENT_ENCODINGS: u32 = 0x6D80;
const CLUSTER: u32 = 0x1F43_B675;
const CLUSTER_TIMECODE: u32 = 0xE7;
const BLOCK_GROUP: u32 = 0xA0;
const BLOCK: u32 = 0xA1;
const SIMPLE_BLOCK: u32 = 0xA3;

#[derive(Default)]
struct Entry {
    number: u64,
    codec: String,
    encoded: bool,
}

struct Reader<R> {
    r: R,
}

impl<R: Read + Seek> Reader<R> {
    fn byte(&mut self) -> Option<u8> {
        let mut b = [0u8; 1];
        self.r.read_exact(&mut b).ok()?;
        Some(b[0])
    }

    fn bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        let mut v = vec![0u8; n];
        self.r.read_exact(&mut v).ok()?;
        Some(v)
    }

    fn skip(&mut self, n: u64) -> Option<()> {
        self.r.seek(SeekFrom::Current(n as i64)).ok()?;
        Some(())
    }

    /// Identyfikator elementu (ze znacznikiem długości, jak w specyfikacji).
    fn id(&mut self) -> Option<u32> {
        let b = self.byte()?;
        let len = b.leading_zeros() as usize + 1;
        if len > 4 {
            return None;
        }
        let mut id = b as u32;
        for _ in 1..len {
            id = (id << 8) | self.byte()? as u32;
        }
        Some(id)
    }

    /// Liczba o zmiennej długości bez znacznika; None w środku – rozmiar nieznany.
    fn vint(&mut self) -> Option<(Option<u64>, usize)> {
        let b = self.byte()?;
        let len = b.leading_zeros() as usize + 1;
        if len > 8 {
            return None;
        }
        let mask = if len == 8 { 0 } else { 0xFFu8 >> len };
        let mut v = (b & mask) as u64;
        let mut all = (b & mask) == mask;
        for _ in 1..len {
            let n = self.byte()?;
            all &= n == 0xFF;
            v = (v << 8) | n as u64;
        }
        Some((if all { None } else { Some(v) }, len))
    }

    fn uint(&mut self, size: u64) -> Option<u64> {
        let b = self.bytes(size.min(8) as usize)?;
        self.skip(size.saturating_sub(8))?;
        Some(b.iter().fold(0u64, |a, &x| (a << 8) | x as u64))
    }
}

/// Kwestie ścieżki o numerze `track` (TrackNumber z pliku, w mpv: src-id): (czas w s, tekst).
/// `stop` przerywa czytanie (np. inny film). None – to nie Matroska albo ścieżka nieobsługiwana.
pub fn subtitles(path: &Path, track: u64, stop: &dyn Fn() -> bool) -> Option<Vec<(f64, String)>> {
    let f = File::open(path).ok()?;
    let mut r = Reader { r: BufReader::with_capacity(1 << 16, f) };
    if r.id()? != EBML {
        return None;
    }
    let (size, _) = r.vint()?;
    r.skip(size?)?;
    let mut scale: u64 = 1_000_000;
    let mut entries: Vec<Entry> = Vec::new();
    let mut codec: Option<String> = None;
    let mut cluster_tc: u64 = 0;
    let mut out: Vec<(f64, String)> = Vec::new();
    let mut n = 0u32;
    loop {
        n += 1;
        if n % 4096 == 0 && stop() {
            return None;
        }
        let Some(id) = r.id() else { break };
        let Some((size, _)) = r.vint() else { break };
        match id {
            // Kontenery – wchodzimy do środka (także o nieznanym rozmiarze).
            SEGMENT | INFO | TRACKS | CLUSTER | BLOCK_GROUP => {}
            TRACK_ENTRY => entries.push(Entry::default()),
            TIMECODE_SCALE => scale = r.uint(size?)?.max(1),
            TRACK_NUMBER => {
                let v = r.uint(size?)?;
                if let Some(e) = entries.last_mut() {
                    e.number = v;
                }
            }
            CODEC_ID => {
                let b = r.bytes(size? as usize)?;
                if let Some(e) = entries.last_mut() {
                    e.codec = String::from_utf8_lossy(&b).trim_end_matches('\0').to_string();
                }
            }
            CONTENT_ENCODINGS => {
                if let Some(e) = entries.last_mut() {
                    e.encoded = true;
                }
                r.skip(size?)?;
            }
            CLUSTER_TIMECODE => {
                // Pierwszy klaster: ścieżki już znane – sprawdzamy, czy umiemy ją czytać.
                if codec.is_none() {
                    let e = entries.iter().find(|e| e.number == track)?;
                    if e.encoded || !matches!(e.codec.as_str(), "S_TEXT/UTF8" | "S_TEXT/ASS" | "S_TEXT/SSA" | "S_ASS" | "S_SSA" | "S_TEXT/WEBVTT") {
                        return None;
                    }
                    codec = Some(e.codec.clone());
                }
                cluster_tc = r.uint(size?)?;
            }
            SIMPLE_BLOCK | BLOCK => {
                let size = size?;
                let (num, len) = r.vint()?;
                if num != Some(track) || codec.is_none() {
                    r.skip(size.saturating_sub(len as u64))?;
                    continue;
                }
                let b = r.bytes(size.saturating_sub(len as u64) as usize)?;
                if b.len() < 3 || b[2] & 0x06 != 0 {
                    continue;
                }
                let rel = i16::from_be_bytes([b[0], b[1]]) as i64;
                let t = (cluster_tc as i64 + rel).max(0) as f64 * scale as f64 / 1e9;
                let raw = String::from_utf8_lossy(&b[3..]).to_string();
                // ASS w Matroce: ReadOrder,Layer,Style,Name,MarginL,MarginR,MarginV,Effect,Text
                let text = if codec.as_deref().is_some_and(|c| c.contains("SSA") || c.contains("ASS")) {
                    raw.splitn(9, ',').nth(8).unwrap_or("").to_string()
                } else {
                    raw
                };
                let text = crate::meta::clean_sub(&text);
                if !text.is_empty() {
                    out.push((t, text));
                }
            }
            _ => match size {
                Some(s) => r.skip(s)?,
                None => break,
            },
        }
    }
    codec.as_ref()?;
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01 && a.1 == b.1);
    Some(out)
}
