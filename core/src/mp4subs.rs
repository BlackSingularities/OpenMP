//! Napisy tekstowe zapisane w pliku MP4/MOV/M4V (mov_text, tx3g) – bez ffmpeg. Czyta tylko
//! atom moov z tabelami próbek i same próbki napisów, obrazu i dźwięku nie dotyka.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Atomy (typ, zawartość) w bloku danych.
fn boxes(b: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut v = Vec::new();
    let mut i = 0;
    while i + 8 <= b.len() {
        let Some(size) = be32(b, i) else { break };
        let (head, size) = match size {
            1 => match be64(b, i + 8) {
                Some(s) => (16, s as usize),
                None => break,
            },
            0 => (8, b.len() - i),
            s => (8, s as usize),
        };
        if size < head || i + size > b.len() {
            break;
        }
        v.push((&b[i + 4..i + 8], &b[i + head..i + size]));
        i += size;
    }
    v
}

fn child<'a>(b: &'a [u8], kind: &[u8]) -> Option<&'a [u8]> {
    boxes(b).into_iter().find(|(k, _)| *k == kind).map(|(_, d)| d)
}

fn path<'a>(b: &'a [u8], kinds: &[&[u8]]) -> Option<&'a [u8]> {
    kinds.iter().try_fold(b, |b, k| child(b, k))
}

/// Atom moov w całości – przeskakuje mdat i inne duże atomy.
fn read_moov(f: &mut BufReader<File>) -> Option<Vec<u8>> {
    let len = f.seek(SeekFrom::End(0)).ok()?;
    let mut pos = 0u64;
    while pos + 8 <= len {
        f.seek(SeekFrom::Start(pos)).ok()?;
        let mut h = [0u8; 16];
        f.read_exact(&mut h[..8]).ok()?;
        let mut size = u32::from_be_bytes(h[..4].try_into().ok()?) as u64;
        let mut head = 8;
        if size == 1 {
            f.read_exact(&mut h[8..]).ok()?;
            size = u64::from_be_bytes(h[8..].try_into().ok()?);
            head = 16;
        } else if size == 0 {
            size = len - pos;
        }
        if size < head {
            return None;
        }
        if &h[4..8] == b"moov" {
            // Rozsądna granica – moov nawet długich filmów ma kilka-kilkanaście MB.
            if size > 256 << 20 {
                return None;
            }
            let mut b = vec![0u8; (size - head) as usize];
            f.read_exact(&mut b).ok()?;
            return Some(b);
        }
        pos += size;
    }
    None
}

struct Track<'a> {
    id: u32,
    scale: u32,
    stbl: &'a [u8],
}

fn text_tracks(moov: &[u8]) -> Vec<Track<'_>> {
    let mut v = Vec::new();
    for (k, trak) in boxes(moov) {
        if k != b"trak" {
            continue;
        }
        let Some(tkhd) = child(trak, b"tkhd") else { continue };
        let id = if tkhd.first() == Some(&1) { be32(tkhd, 20) } else { be32(tkhd, 12) };
        let Some(mdia) = child(trak, b"mdia") else { continue };
        let Some(mdhd) = child(mdia, b"mdhd") else { continue };
        let scale = if mdhd.first() == Some(&1) { be32(mdhd, 20) } else { be32(mdhd, 12) };
        let Some(stbl) = path(mdia, &[b"minf", b"stbl"]) else { continue };
        // Pierwszy opis próbek: stsd = wersja(4) + liczba(4) + [rozmiar(4) + format(4)].
        let is_text = child(stbl, b"stsd").and_then(|d| d.get(12..16)).is_some_and(|f| f == b"tx3g");
        if let (Some(id), Some(scale), true) = (id, scale, is_text) {
            if scale > 0 {
                v.push(Track { id, scale, stbl });
            }
        }
    }
    v
}

/// Tabele próbek: (czas startu w sekundach, przesunięcie w pliku, rozmiar).
fn samples(t: &Track) -> Option<Vec<(f64, u64, u32)>> {
    let stts = child(t.stbl, b"stts")?;
    let stsz = child(t.stbl, b"stsz")?;
    let stsc = child(t.stbl, b"stsc")?;
    let (chunks, wide) = match child(t.stbl, b"stco") {
        Some(c) => (c, false),
        None => (child(t.stbl, b"co64")?, true),
    };
    let fixed = be32(stsz, 4)?;
    let count = be32(stsz, 8)? as usize;
    let size = |i: usize| if fixed != 0 { Some(fixed) } else { be32(stsz, 12 + i * 4) };
    // Czasy startu z stts (liczba próbek, długość).
    let mut times = Vec::with_capacity(count);
    let mut tick = 0u64;
    for e in 0..be32(stts, 4)? as usize {
        let n = be32(stts, 8 + e * 8)?;
        let d = be32(stts, 12 + e * 8)? as u64;
        for _ in 0..n {
            times.push(tick as f64 / t.scale as f64);
            tick += d;
        }
    }
    let n_chunks = be32(chunks, 4)? as usize;
    let chunk_off = |c: usize| if wide { be64(chunks, 8 + c * 8) } else { be32(chunks, 8 + c * 4).map(u64::from) };
    // stsc: (pierwszy fragment od 1, próbek we fragmencie, opis).
    let n_stsc = be32(stsc, 4)? as usize;
    let mut out = Vec::with_capacity(count);
    let mut s = 0usize;
    for e in 0..n_stsc {
        let first = be32(stsc, 8 + e * 12)? as usize;
        let per = be32(stsc, 12 + e * 12)? as usize;
        let last = if e + 1 < n_stsc { be32(stsc, 8 + (e + 1) * 12)? as usize } else { n_chunks + 1 };
        for c in first..last {
            let mut off = chunk_off(c.checked_sub(1)?)?;
            for _ in 0..per {
                if s >= count {
                    return Some(out);
                }
                let sz = size(s)?;
                out.push((*times.get(s)?, off, sz));
                off += sz as u64;
                s += 1;
            }
        }
    }
    Some(out)
}

/// Kwestie (czas startu w sekundach, tekst) ścieżki o danym numerze (track_ID). Gdy numer
/// nie pasuje, a ścieżka tekstowa jest jedna – ta.
pub fn subtitles(path_: &Path, track: u64, stop: &dyn Fn() -> bool) -> Option<Vec<(f64, String)>> {
    let mut f = BufReader::new(File::open(path_).ok()?);
    let moov = read_moov(&mut f)?;
    let tracks = text_tracks(&moov);
    let t = match tracks.iter().find(|t| t.id as u64 == track) {
        Some(t) => t,
        None if tracks.len() == 1 => &tracks[0],
        None => return None,
    };
    let mut out = Vec::new();
    for (i, (start, off, size)) in samples(t)?.into_iter().enumerate() {
        if i % 64 == 0 && stop() {
            return None;
        }
        // Próbka: długość tekstu (2 bajty) + tekst UTF-8, potem ewentualne style.
        if size < 2 || size > 64 << 10 {
            continue;
        }
        let mut b = vec![0u8; size as usize];
        f.seek(SeekFrom::Start(off)).ok()?;
        f.read_exact(&mut b).ok()?;
        let n = u16::from_be_bytes([b[0], b[1]]) as usize;
        let Some(raw) = b.get(2..2 + n) else { continue };
        let text = String::from_utf8_lossy(raw);
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            out.push((start, text));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some(out)
}
