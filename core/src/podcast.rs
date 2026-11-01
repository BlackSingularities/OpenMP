//! Podcasty: kanał RSS (z rozszerzeniami iTunes) – tytuł i odcinki z plikiem dźwięku.
//! Prosty odczyt bez parsera XML: kanały podcastów mają przewidywalną budowę.

use std::io::Read;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Episode {
    pub title: String,
    /// Adres pliku (enclosure).
    pub url: String,
    /// Data publikacji, np. „10 Jun 2025”.
    pub date: String,
    /// Długość w sekundach (0 – nieznana).
    pub duration: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    pub title: String,
    pub episodes: Vec<Episode>,
}

/// Pobiera i odczytuje kanał.
pub fn fetch(url: &str) -> Result<Feed, String> {
    let resp = ureq::get(url.trim())
        .set("User-Agent", "OpenMP (https://redzikowski.com)")
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .map_err(|e| e.to_string())?;
    let mut body = String::new();
    resp.into_reader().take(64 << 20).read_to_string(&mut body).map_err(|e| e.to_string())?;
    parse(&body).ok_or_else(|| crate::tr!("This is not a podcast feed (RSS)").to_string())
}

/// Odczyt treści kanału; `None`, gdy to nie jest RSS z odcinkami.
pub fn parse(xml: &str) -> Option<Feed> {
    if !xml.contains("<rss") && !xml.contains("<channel") {
        return None;
    }
    let first = find_item(xml, 0).unwrap_or(xml.len());
    let title = tag(&xml[..first], "title").unwrap_or_default();
    let mut episodes = Vec::new();
    let mut pos = first;
    while let Some(start) = find_item(xml, pos) {
        let end = xml[start..].find("</item>").map(|e| start + e).unwrap_or(xml.len());
        let block = &xml[start..end];
        if let Some(url) = attr(block, "enclosure", "url") {
            episodes.push(Episode {
                title: tag(block, "title").unwrap_or_default(),
                url,
                date: tag(block, "pubDate").map(|d| short_date(&d)).unwrap_or_default(),
                duration: tag(block, "itunes:duration").map(|d| duration(&d)).unwrap_or(0.0),
            });
        }
        pos = end.max(start + 5);
    }
    Some(Feed { title, episodes })
}

/// Początek następnego `<item>` (ale nie np. `<itemType>`).
fn find_item(xml: &str, from: usize) -> Option<usize> {
    let mut i = from;
    while let Some(p) = xml.get(i..)?.find("<item") {
        let at = i + p;
        match xml[at + 5..].chars().next() {
            Some('>') | Some(' ') | Some('\n') | Some('\r') | Some('\t') => return Some(at),
            _ => i = at + 5,
        }
    }
    None
}

/// Treść pierwszego znacznika `<name …>…</name>`, bez CDATA i encji.
fn tag(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}");
    let mut i = 0;
    while let Some(p) = block[i..].find(&open) {
        let at = i + p;
        let rest = &block[at + open.len()..];
        if rest.starts_with('>') || rest.starts_with(' ') {
            let body_start = at + open.len() + rest.find('>')? + 1;
            let close = format!("</{name}>");
            let body_end = body_start + block[body_start..].find(&close)?;
            let raw = block[body_start..body_end].trim();
            let raw = raw.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")).unwrap_or(raw);
            return Some(unescape(raw.trim()));
        }
        i = at + open.len();
    }
    None
}

/// Wartość atrybutu z pierwszego znacznika `<name …>`.
fn attr(block: &str, name: &str, key: &str) -> Option<String> {
    let at = block.find(&format!("<{name}"))?;
    let end = at + block[at..].find('>')?;
    let t = &block[at..end];
    for q in ['"', '\''] {
        let k = format!("{key}={q}");
        if let Some(p) = t.find(&k) {
            let v = &t[p + k.len()..];
            return v.find(q).map(|e| unescape(&v[..e]));
        }
    }
    None
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// „Tue, 10 Jun 2025 10:00:00 +0000” → „10 Jun 2025”.
fn short_date(d: &str) -> String {
    let parts: Vec<&str> = d.split_whitespace().collect();
    let skip = usize::from(parts.first().is_some_and(|p| p.ends_with(',')));
    parts.iter().skip(skip).take(3).cloned().collect::<Vec<_>>().join(" ")
}

/// „1:02:03”, „45:10” albo liczba sekund.
fn duration(d: &str) -> f64 {
    d.trim().split(':').fold(0.0, |acc, p| acc * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_feed() {
        let xml = r#"<?xml version="1.0"?><rss><channel><title><![CDATA[Mój &amp; podcast]]></title>
<itunes:image href="x"/>
<item><title>Odcinek 2</title><pubDate>Tue, 10 Jun 2025 10:00:00 +0000</pubDate>
<enclosure url="https://ex.com/a.mp3?x=1&amp;y=2" type="audio/mpeg" length="1"/><itunes:duration>1:02:03</itunes:duration></item>
<item><title>Bez pliku</title></item>
<item>
<title>Odcinek 1 &#8211; start</title><enclosure type='audio/mpeg' url='https://ex.com/b.mp3'/><itunes:duration>95</itunes:duration></item>
</channel></rss>"#;
        let f = parse(xml).unwrap();
        assert_eq!(f.title, "Mój & podcast");
        assert_eq!(f.episodes.len(), 2);
        assert_eq!(f.episodes[0].url, "https://ex.com/a.mp3?x=1&y=2");
        assert_eq!(f.episodes[0].date, "10 Jun 2025");
        assert_eq!(f.episodes[0].duration, 3723.0);
        assert_eq!(f.episodes[1].title, "Odcinek 1 – start");
        assert_eq!(f.episodes[1].duration, 95.0);
        assert!(parse("<html></html>").is_none());
    }
}
