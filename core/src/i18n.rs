//! Tłumaczenia tekstów z kodu Rust.
//!
//! Teksty w kodzie są po angielsku (język źródłowy), a każdy inny język to jeden plik
//! `lang/<kod>/LC_MESSAGES/openmp.po` – ten sam, z którego Slint bierze teksty interfejsu
//! (`@tr(...)`). Pliki są wbudowywane w program przy kompilacji (build.rs), więc nowy język
//! to tylko nowy plik .po – bez zmian w kodzie.
//!
//! - `tr!("Text")` – tłumaczenie (`&'static str`),
//! - `tr!("Track {} of {}", a, b)` – z wartościami w miejsce `{}` (albo `{0}`, `{1}`…),
//! - `trn!("{n} track", "{n} tracks", n)` – z odmianą przez liczbę (`{n}` to liczba),
//! - `trc!("context", "Text")` – ten sam angielski tekst z różnym tłumaczeniem zależnie od miejsca.

use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::sync::RwLock;

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

/// Kod języka źródłowego (teksty w kodzie).
pub const SOURCE_LANG: &str = "en";

struct Catalog {
    lang: &'static str,
    /// (kontekst, msgid) → tłumaczenia (jedno albo formy liczby mnogiej).
    map: HashMap<(&'static str, &'static str), Vec<&'static str>>,
    plural: fn(u64) -> usize,
}

static CURRENT: RwLock<Option<&'static Catalog>> = RwLock::new(None);

/// Dostępne języki: (kod, nazwa w tym języku). Pierwszy to język źródłowy.
pub fn languages() -> Vec<(&'static str, &'static str)> {
    let mut v = vec![(SOURCE_LANG, "English")];
    for &(code, po) in CATALOGS {
        v.push((code, header(po, "X-Language-Name").unwrap_or(code)));
    }
    v
}

/// Kod bieżącego języka.
pub fn current() -> &'static str {
    CURRENT.read().ok().and_then(|c| c.map(|c| c.lang)).unwrap_or(SOURCE_LANG)
}

/// Wybiera język z kodu (np. `pl`, `pl-PL`, `pt_BR`); nieznany – angielski. Zwraca wybrany kod.
pub fn set_language(code: &str) -> &'static str {
    let code = code.trim().replace('_', "-");
    let base = code.split('-').next().unwrap_or("").to_ascii_lowercase();
    let found = CATALOGS
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(&code))
        .or_else(|| CATALOGS.iter().find(|(c, _)| c.eq_ignore_ascii_case(&base)));
    let cat: Option<&'static Catalog> = found.map(|&(c, po)| &*Box::leak(Box::new(parse(c, po))));
    if let Ok(mut cur) = CURRENT.write() {
        *cur = cat;
    }
    cat.map(|c| c.lang).unwrap_or(SOURCE_LANG)
}

fn lookup(ctx: &'static str, id: &'static str, n: Option<u64>) -> Option<&'static str> {
    let cur = CURRENT.read().ok()?;
    let cat = (*cur)?;
    let forms = cat.map.get(&(ctx, id))?;
    let i = n.map(|n| (cat.plural)(n)).unwrap_or(0);
    forms.get(i).or(forms.first()).copied().filter(|s| !s.is_empty())
}

#[doc(hidden)]
pub fn tr(id: &'static str) -> &'static str {
    lookup("", id, None).unwrap_or(id)
}

#[doc(hidden)]
pub fn trc(ctx: &'static str, id: &'static str) -> &'static str {
    lookup(ctx, id, None).unwrap_or(id)
}

#[doc(hidden)]
pub fn trn(one: &'static str, many: &'static str, n: u64) -> &'static str {
    lookup("", one, Some(n)).unwrap_or(if n == 1 { one } else { many })
}

/// Liczba dziesiętna z kropką w zapisie bieżącego języka (np. „1,5” po polsku).
pub fn decimal(s: impl AsRef<str>) -> String {
    s.as_ref().replace('.', trc("decimal separator", "."))
}

/// Wstawia wartości w `{}` (kolejno) albo `{0}`, `{1}`… oraz liczbę w `{n}`; `{{` i `}}` to klamry.
#[doc(hidden)]
pub fn format(template: &str, n: Option<u64>, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut it = template.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        match c {
            '{' if it.peek().map(|p| p.1) == Some('{') => {
                it.next();
                out.push('{');
            }
            '}' if it.peek().map(|p| p.1) == Some('}') => {
                it.next();
                out.push('}');
            }
            '{' => {
                let rest = &template[i + 1..];
                let Some(end) = rest.find('}') else {
                    out.push('{');
                    continue;
                };
                let key = &rest[..end];
                let arg = if key.is_empty() {
                    next += 1;
                    args.get(next - 1).map(|a| a.to_string())
                } else if key == "n" {
                    n.map(|n| n.to_string())
                } else {
                    key.parse::<usize>().ok().and_then(|k| args.get(k)).map(|a| a.to_string())
                };
                match arg {
                    Some(a) => out.push_str(&a),
                    None => {
                        let _ = write!(out, "{{{key}}}");
                    }
                }
                for _ in rest[..=end].chars() {
                    it.next();
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Tłumaczenie tekstu z kodu. Z wartościami zwraca `String`, bez – `&'static str`.
#[macro_export]
macro_rules! tr {
    ($id:literal) => {
        $crate::i18n::tr($id)
    };
    ($id:literal, $($a:expr),+ $(,)?) => {
        $crate::i18n::format($crate::i18n::tr($id), None, &[$(&$a as &dyn ::std::fmt::Display),+])
    };
}

/// Tłumaczenie z kontekstem – gdy ten sam angielski tekst ma w innym języku różne formy.
#[macro_export]
macro_rules! trc {
    ($ctx:literal, $id:literal) => {
        $crate::i18n::trc($ctx, $id)
    };
    ($ctx:literal, $id:literal, $($a:expr),+ $(,)?) => {
        $crate::i18n::format($crate::i18n::trc($ctx, $id), None, &[$(&$a as &dyn ::std::fmt::Display),+])
    };
}

/// Tekst odmieniany przez liczbę: `trn!("{n} track", "{n} tracks", n)`; dalsze wartości w `{}`.
#[macro_export]
macro_rules! trn {
    ($one:literal, $many:literal, $n:expr $(, $a:expr)* $(,)?) => {{
        let n = ($n) as u64;
        $crate::i18n::format($crate::i18n::trn($one, $many, n), Some(n), &[$(&$a as &dyn ::std::fmt::Display),*])
    }};
}

// ---------- pliki .po ----------

fn header(po: &'static str, key: &str) -> Option<&'static str> {
    // Nagłówek to pierwsza pozycja (msgid ""): linie "Klucz: wartość\n".
    let start = po.find("msgid \"\"")?;
    let head = &po[start..po[start..].find("\n\n").map(|e| start + e).unwrap_or(po.len())];
    head.lines().find_map(|l| {
        let l = l.trim().strip_prefix('"')?.strip_suffix('"')?;
        let l = l.strip_suffix("\\n").unwrap_or(l);
        let (k, v) = l.split_once(':')?;
        (k.trim() == key).then(|| v.trim())
    })
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn parse(lang: &'static str, po: &'static str) -> Catalog {
    #[derive(Default)]
    struct Entry {
        ctx: Option<String>,
        id: Option<String>,
        strs: Vec<String>,
        fuzzy: bool,
    }
    let mut map = HashMap::new();
    let mut flush = |e: &mut Entry| {
        if let Some(id) = e.id.take() {
            if !id.is_empty() && !e.fuzzy && e.strs.iter().any(|s| !s.is_empty()) {
                let ctx = leak(e.ctx.take().unwrap_or_default());
                let strs = std::mem::take(&mut e.strs).into_iter().map(leak).collect();
                map.insert((ctx, leak(id)), strs);
            }
        }
        *e = Entry::default();
    };
    let mut e = Entry::default();
    // Do którego pola dopisywać kolejne linie "…".
    enum Field {
        None,
        Ctx,
        Id,
        Plural,
        Str(usize),
    }
    let mut field = Field::None;
    for line in po.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(c) = l.strip_prefix("#,") {
            if e.id.is_some() {
                flush(&mut e);
            }
            e.fuzzy = c.split(',').any(|f| f.trim() == "fuzzy");
            continue;
        }
        if l.starts_with('#') {
            continue;
        }
        let (key, rest) = match l.find(' ') {
            Some(i) if !l.starts_with('"') => (&l[..i], l[i..].trim()),
            _ => ("", l),
        };
        let val = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')).map(unescape).unwrap_or_default();
        match key {
            "msgctxt" => {
                if e.id.is_some() {
                    flush(&mut e);
                }
                e.ctx = Some(val);
                field = Field::Ctx;
            }
            "msgid" => {
                if e.id.is_some() {
                    flush(&mut e);
                }
                e.id = Some(val);
                field = Field::Id;
            }
            "msgid_plural" => field = Field::Plural,
            "msgstr" => {
                e.strs = vec![val];
                field = Field::Str(0);
            }
            k if k.starts_with("msgstr[") => {
                let i: usize = k[7..k.len().saturating_sub(1)].parse().unwrap_or(0);
                if e.strs.len() <= i {
                    e.strs.resize(i + 1, String::new());
                }
                e.strs[i] = val;
                field = Field::Str(i);
            }
            "" => match field {
                Field::Ctx => e.ctx.get_or_insert_with(String::new).push_str(&val),
                Field::Id => e.id.get_or_insert_with(String::new).push_str(&val),
                Field::Str(i) => {
                    if let Some(s) = e.strs.get_mut(i) {
                        s.push_str(&val);
                    }
                }
                Field::Plural | Field::None => {}
            },
            _ => {}
        }
    }
    flush(&mut e);
    Catalog { lang, map, plural: plural_rule(lang) }
}

/// Reguła wyboru formy liczby mnogiej (indeks `msgstr[i]`) – taka sama jak `Plural-Forms` w pliku .po.
/// Nowy język z inną odmianą niż angielska potrzebuje tu jednej linii.
fn plural_rule(lang: &str) -> fn(u64) -> usize {
    match lang.split('-').next().unwrap_or(lang) {
        "pl" => |n| {
            if n == 1 {
                0
            } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
                1
            } else {
                2
            }
        },
        "ru" | "uk" | "be" | "sr" | "hr" | "bs" => |n| {
            if n % 10 == 1 && n % 100 != 11 {
                0
            } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
                1
            } else {
                2
            }
        },
        "cs" | "sk" => |n| match n {
            1 => 0,
            2..=4 => 1,
            _ => 2,
        },
        "fr" | "pt" => |n| usize::from(n > 1),
        "ja" | "zh" | "ko" | "vi" | "th" | "id" => |_| 0,
        _ => |n| usize::from(n != 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(format("{} of {}", None, &[&1, &"x"]), "1 of x");
        assert_eq!(format("{1} {0}", None, &[&"a", &"b"]), "b a");
        assert_eq!(format("{n} tracks · {}", Some(3), &[&"a"]), "3 tracks · a");
        assert_eq!(format("{{x}}", None, &[]), "{x}");
    }

    #[test]
    fn po_parsing() {
        let po: &'static str = "msgid \"\"\nmsgstr \"\"\n\"X-Language-Name: Test\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Cześć\"\n\nmsgctxt \"c\"\nmsgid \"Off\"\nmsgstr \"Wył.\"\n\nmsgid \"{n} track\"\nmsgid_plural \"{n} tracks\"\nmsgstr[0] \"{n} utwór\"\nmsgstr[1] \"{n} utwory\"\nmsgstr[2] \"{n} utworów\"\n\n#, fuzzy\nmsgid \"Fuzzy\"\nmsgstr \"Nie\"\n";
        let c = parse("pl", po);
        assert_eq!(c.map.get(&("", "Hello")).unwrap()[0], "Cześć");
        assert_eq!(c.map.get(&("c", "Off")).unwrap()[0], "Wył.");
        assert_eq!(c.map.get(&("", "{n} track")).unwrap()[2], "{n} utworów");
        assert!(c.map.get(&("", "Fuzzy")).is_none());
        assert_eq!((c.plural)(22), 1);
        assert_eq!((c.plural)(12), 2);
        assert_eq!(header(po, "X-Language-Name"), Some("Test"));
    }

    #[test]
    fn bundled_catalogs_parse() {
        for &(code, po) in CATALOGS {
            let c = parse(code, po);
            assert!(!c.map.is_empty(), "pusty katalog {code}");
        }
    }
}
