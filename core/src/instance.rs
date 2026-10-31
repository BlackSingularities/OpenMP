//! Jedna instancja: kolejne uruchomienia (podwójne kliknięcie pliku) oddają ścieżki działającemu oknu.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const END: &str = "\u{4}";

fn port_file() -> std::path::PathBuf {
    crate::paths::cache().join("instance.port")
}

/// Wysyła argumenty do działającej instancji; `true` – ktoś je przyjął.
pub fn send(args: &[String]) -> bool {
    let Ok(port) = std::fs::read_to_string(port_file()) else { return false };
    let Ok(port) = port.trim().parse::<u16>() else { return false };
    let Ok(mut s) = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(400)) else { return false };
    let _ = s.set_write_timeout(Some(Duration::from_millis(800)));
    let mut msg = String::from("OPENMP\n");
    for a in args {
        msg.push_str(&a.replace('\n', " "));
        msg.push('\n');
    }
    msg.push_str(END);
    msg.push('\n');
    s.write_all(msg.as_bytes()).is_ok()
}

/// Nasłuch; `on_args` dostaje listę ścieżek (pusta – tylko „pokaż okno”).
pub fn listen(on_args: impl Fn(Vec<String>) + Send + 'static) {
    let Ok(l) = TcpListener::bind(("127.0.0.1", 0)) else { return };
    let Ok(addr) = l.local_addr() else { return };
    let _ = std::fs::write(port_file(), addr.port().to_string());
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            let mut lines = BufReader::new(s).lines();
            if lines.next().and_then(|l| l.ok()).as_deref() != Some("OPENMP") {
                continue;
            }
            let mut args = Vec::new();
            for l in lines.map_while(Result::ok) {
                if l == END {
                    break;
                }
                args.push(l);
            }
            on_args(args);
        }
    });
}
