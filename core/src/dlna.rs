//! Odtwarzanie na urządzeniach DLNA/UPnP (telewizory, amplitunery, głośniki sieciowe):
//! wyszukanie odbiorników (SSDP), sterowanie AVTransport (SOAP) i mały serwer HTTP,
//! z którego odbiornik pobiera lokalny plik.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Renderer {
    pub name: String,
    /// Adres sterowania usługi AVTransport.
    pub control: String,
}

const AVT: &str = "urn:schemas-upnp-org:service:AVTransport:1";

/// Odbiorniki w sieci lokalnej (szukanie trwa około `wait`).
pub fn discover(wait: Duration) -> Vec<Renderer> {
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else { return Vec::new() };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));
    let msg = format!("M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {AVT}\r\n\r\n");
    for _ in 0..2 {
        let _ = sock.send_to(msg.as_bytes(), "239.255.255.250:1900");
    }
    let mut locations: Vec<String> = Vec::new();
    let end = Instant::now() + wait;
    let mut buf = [0u8; 2048];
    while Instant::now() < end {
        let Ok((n, _)) = sock.recv_from(&mut buf) else { continue };
        let text = String::from_utf8_lossy(&buf[..n]);
        for line in text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("location") {
                    let v = v.trim().to_string();
                    if !locations.contains(&v) {
                        locations.push(v);
                    }
                }
            }
        }
    }
    let mut out: Vec<Renderer> = locations.iter().filter_map(|l| describe(l)).collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.dedup_by(|a, b| a.control == b.control);
    out
}

/// Opis urządzenia: nazwa i adres sterowania AVTransport.
fn describe(location: &str) -> Option<Renderer> {
    let xml = ureq::get(location).timeout(Duration::from_secs(4)).call().ok()?.into_string().ok()?;
    let name = between(&xml, "<friendlyName>", "</friendlyName>").unwrap_or("DLNA").trim().to_string();
    // Blok <service> z usługą AVTransport.
    let mut rest = xml.as_str();
    while let Some(i) = rest.find("<service>") {
        let block_end = rest[i..].find("</service>").map(|e| i + e)?;
        let block = &rest[i..block_end];
        if block.contains("AVTransport") {
            let ctl = between(block, "<controlURL>", "</controlURL>")?.trim();
            return Some(Renderer { name, control: resolve(location, ctl) });
        }
        rest = &rest[block_end..];
    }
    None
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

/// Adres względny względem adresu opisu („http://host:port”).
fn resolve(base: &str, rel: &str) -> String {
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    let root_end = base.find("://").and_then(|p| base[p + 3..].find('/').map(|e| p + 3 + e)).unwrap_or(base.len());
    let root = &base[..root_end];
    if rel.starts_with('/') {
        format!("{root}{rel}")
    } else {
        format!("{root}/{rel}")
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn soap(r: &Renderer, action: &str, args: &str) -> Result<(), String> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{AVT}\"><InstanceID>0</InstanceID>{args}</u:{action}></s:Body></s:Envelope>"
    );
    ureq::post(&r.control)
        .set("Content-Type", "text/xml; charset=\"utf-8\"")
        .set("SOAPACTION", &format!("\"{AVT}#{action}\""))
        .timeout(Duration::from_secs(8))
        .send_string(&body)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Odtwarza adres `url` na odbiorniku.
pub fn play(r: &Renderer, url: &str, title: &str, mime: &str) -> Result<(), String> {
    let class = if mime.starts_with("video") { "object.item.videoItem" } else { "object.item.audioItem.musicTrack" };
    let didl = format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\"><item id=\"0\" parentID=\"-1\" restricted=\"1\"><dc:title>{}</dc:title><upnp:class>{class}</upnp:class><res protocolInfo=\"http-get:*:{mime}:*\">{}</res></item></DIDL-Lite>",
        xml_escape(title),
        xml_escape(url)
    );
    let _ = soap(r, "Stop", "");
    soap(r, "SetAVTransportURI", &format!("<CurrentURI>{}</CurrentURI><CurrentURIMetaData>{}</CurrentURIMetaData>", xml_escape(url), xml_escape(&didl)))?;
    soap(r, "Play", "<Speed>1</Speed>")
}

pub fn pause(r: &Renderer) -> Result<(), String> {
    soap(r, "Pause", "")
}

pub fn resume(r: &Renderer) -> Result<(), String> {
    soap(r, "Play", "<Speed>1</Speed>")
}

pub fn stop(r: &Renderer) -> Result<(), String> {
    soap(r, "Stop", "")
}

/// Typ MIME pliku po rozszerzeniu.
pub fn mime(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "m4a" | "aac" => "audio/mp4",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "wma" => "audio/x-ms-wma",
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "ts" => "video/mp2t",
        _ => "application/octet-stream",
    }
}

// ---------- serwer plików ----------

struct Server {
    port: u16,
    file: Mutex<Option<PathBuf>>,
}

static SERVER: OnceLock<Option<Server>> = OnceLock::new();

/// Udostępnia lokalny plik odbiornikowi; zwraca adres HTTP widoczny z sieci (adres tego
/// komputera w tej samej sieci co odbiornik).
pub fn serve(path: &str, r: &Renderer) -> Option<String> {
    let srv = SERVER
        .get_or_init(|| {
            let l = TcpListener::bind("0.0.0.0:0").ok()?;
            let port = l.local_addr().ok()?.port();
            std::thread::spawn(move || {
                for s in l.incoming().flatten() {
                    std::thread::spawn(move || {
                        let _ = handle(s);
                    });
                }
            });
            Some(Server { port, file: Mutex::new(None) })
        })
        .as_ref()?;
    *srv.file.lock().ok()? = Some(PathBuf::from(path));
    // Lokalny adres na trasie do odbiornika.
    let host = r.control.split("://").nth(1)?.split('/').next()?;
    let target: SocketAddr = std::net::ToSocketAddrs::to_socket_addrs(host).ok()?.next()?;
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    probe.connect(target).ok()?;
    let ip = probe.local_addr().ok()?.ip();
    let ext = path.rsplit('.').next().unwrap_or("bin");
    // Znacznik czasu w adresie – odbiornik nie bierze starego pliku z własnej pamięci podręcznej.
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    Some(format!("http://{ip}:{}/media/{n}.{ext}", srv.port))
}

fn handle(mut s: TcpStream) -> std::io::Result<()> {
    let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
    let mut reader = BufReader::new(s.try_clone()?);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut range: Option<(u64, Option<u64>)> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("range") {
                if let Some(r) = v.trim().strip_prefix("bytes=") {
                    let (a, b) = r.split_once('-').unwrap_or((r, ""));
                    range = a.trim().parse().ok().map(|a| (a, b.trim().parse().ok()));
                }
            }
        }
    }
    let head = first.starts_with("HEAD ");
    let path = SERVER.get().and_then(|s| s.as_ref()).and_then(|s| s.file.lock().ok().and_then(|f| f.clone()));
    let Some(path) = path.filter(|_| first.contains(" /media/")) else {
        s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
        return Ok(());
    };
    let mut f = std::fs::File::open(&path)?;
    let size = f.metadata()?.len();
    let mime = mime(&path.to_string_lossy());
    let (start, end) = match range {
        Some((a, b)) if a < size => (a, b.unwrap_or(size - 1).min(size - 1)),
        _ => (0, size.saturating_sub(1)),
    };
    let len = if size == 0 { 0 } else { end - start + 1 };
    let status = if range.is_some() { "206 Partial Content" } else { "200 OK" };
    let mut hdr = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {len}\r\nAccept-Ranges: bytes\r\ntransferMode.dlna.org: Streaming\r\nConnection: close\r\n"
    );
    if range.is_some() {
        hdr.push_str(&format!("Content-Range: bytes {start}-{end}/{size}\r\n"));
    }
    hdr.push_str("\r\n");
    s.write_all(hdr.as_bytes())?;
    if head || len == 0 {
        return Ok(());
    }
    f.seek(SeekFrom::Start(start))?;
    std::io::copy(&mut f.take(len), &mut s)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(resolve("http://10.0.0.5:8080/desc.xml", "/upnp/control/AVT"), "http://10.0.0.5:8080/upnp/control/AVT");
        assert_eq!(resolve("http://10.0.0.5:8080/desc.xml", "AVT_ctl"), "http://10.0.0.5:8080/AVT_ctl");
        assert_eq!(mime("a/b.FLAC"), "audio/flac");
    }
}
