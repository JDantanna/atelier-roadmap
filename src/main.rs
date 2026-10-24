#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};
#[cfg(windows)]
mod desktop;
mod notifications;
#[cfg(windows)]
mod tray;
mod updates;
const HTML: &str = include_str!("../web/index.html");
const MODEL: &str = include_str!("../web/model.js");
const PROJECTS: &str = include_str!("../web/projects.js");
const MAX: usize = 4 * 1024 * 1024;
fn token() -> String {
    let mut a = RandomState::new().build_hasher();
    a.write_u8(1);
    let mut b = RandomState::new().build_hasher();
    b.write_u8(2);
    format!("{:016x}{:016x}", a.finish(), b.finish())
}
fn directory() -> PathBuf {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        });
    root.join("AtelierRoadmap")
}
fn save(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    if path.exists() {
        std::fs::copy(path, path.with_extension("backup.json"))?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(tmp, path)
    }
}
fn reply(s: &mut TcpStream, code: &str, mime: &str, data: &[u8]) {
    let header=format!("HTTP/1.1 {code}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'\r\n\r\n",data.len());
    let _ = s.write_all(header.as_bytes());
    let _ = s.write_all(data);
}
fn handle(
    mut s: TcpStream,
    base: &str,
    origin: &str,
    path: &Path,
    updates: &updates::Shared,
    notifications: &notifications::Hub,
) -> bool {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let end = loop {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return false,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        if let Some(i) = buf.windows(4).position(|x| x == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 16384 {
            reply(
                &mut s,
                "431 Request Header Fields Too Large",
                "text/plain",
                b"Header limit",
            );
            return false;
        }
    };
    let headers = String::from_utf8_lossy(&buf[..end]).to_string();
    let mut lines = headers.lines();
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let route = parts.next().unwrap_or("");
    if !route.starts_with(base) {
        reply(&mut s, "404 Not Found", "text/plain", b"Not found");
        return false;
    }
    if let Some(o) = headers.lines().find_map(|x| {
        x.split_once(':')
            .filter(|(k, _)| k.eq_ignore_ascii_case("origin"))
            .map(|(_, v)| v.trim())
    }) {
        if o != origin {
            reply(&mut s, "403 Forbidden", "text/plain", b"Origin denied");
            return false;
        }
    }
    let suffix = &route[base.len()..];
    if method == "GET" && suffix.is_empty() {
        reply(
            &mut s,
            "200 OK",
            "text/html; charset=utf-8",
            HTML.replace("__ATELIER_VERSION__", env!("CARGO_PKG_VERSION"))
                .as_bytes(),
        );
    } else if method == "GET" && suffix == "model.js" {
        reply(
            &mut s,
            "200 OK",
            "text/javascript; charset=utf-8",
            MODEL.as_bytes(),
        );
    } else if method == "GET" && suffix == "projects.js" {
        reply(
            &mut s,
            "200 OK",
            "text/javascript; charset=utf-8",
            PROJECTS.as_bytes(),
        );
    } else if method == "GET" && suffix == "notifications" {
        reply(
            &mut s,
            "200 OK",
            "application/json",
            &serde_json::to_vec(&notifications.list()).unwrap(),
        );
    } else if method == "POST" && suffix == "notifications/test" {
        match notifications.test() {
            Ok(()) => reply(&mut s, "200 OK", "application/json", b"{}"),
            Err(e) => reply(
                &mut s,
                "500 Internal Server Error",
                "text/plain",
                e.as_bytes(),
            ),
        }
    } else if method == "POST" && suffix == "notifications/action" {
        let len = headers
            .lines()
            .find_map(|x| {
                x.split_once(':')
                    .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, v)| v.trim().parse::<usize>().ok())
            })
            .unwrap_or(MAX + 1);
        if len > MAX {
            reply(&mut s, "400 Bad Request", "text/plain", b"Invalid body");
            return false;
        }
        while buf.len() < end + len {
            match s.read(&mut chunk) {
                Ok(0) | Err(_) => return false,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        }
        match serde_json::from_slice::<serde_json::Value>(&buf[end..end + len]) {
            Ok(v) => match notifications.action(
                v["id"].as_str().unwrap_or(""),
                v["action"].as_str().unwrap_or(""),
                v["minutes"].as_i64().unwrap_or(10),
                chrono::Utc::now().timestamp_millis(),
            ) {
                Ok(()) => reply(&mut s, "200 OK", "application/json", b"{}"),
                Err(e) => reply(
                    &mut s,
                    "400 Bad Request",
                    "text/plain; charset=utf-8",
                    e.as_bytes(),
                ),
            },
            Err(_) => reply(&mut s, "400 Bad Request", "text/plain", b"Invalid JSON"),
        }
    } else if method == "GET" && suffix == "updates" {
        let data = serde_json::to_vec(&*updates.lock().unwrap()).unwrap();
        reply(&mut s, "200 OK", "application/json", &data);
    } else if method == "POST" && suffix == "update-check" {
        updates::check(updates.clone());
        reply(&mut s, "200 OK", "application/json", b"{}");
    } else if method == "POST" && suffix == "update-download" {
        updates::download(updates.clone(), path.parent().unwrap().join("updates"));
        reply(&mut s, "200 OK", "application/json", b"{}");
    } else if method == "POST" && suffix == "update-install" {
        match updates::install(updates) {
            Ok(()) => {
                reply(&mut s, "200 OK", "application/json", b"{}");
                return true;
            }
            Err(e) => reply(&mut s, "400 Bad Request", "text/plain", e.as_bytes()),
        }
    } else if method == "GET" && suffix == "mentor-plan" {
        match updates::mentor_plan() {
            Ok(data) => reply(
                &mut s,
                "200 OK",
                "application/json",
                &serde_json::to_vec(&data).unwrap(),
            ),
            Err(e) => reply(
                &mut s,
                "502 Bad Gateway",
                "text/plain; charset=utf-8",
                e.as_bytes(),
            ),
        }
    } else if method == "GET" && suffix == "ping" {
        reply(&mut s, "200 OK", "text/plain", b"atelier-roadmap-v1");
    } else if method == "GET" && suffix == "state" {
        let data =
            std::fs::read(path).or_else(|_| std::fs::read(path.with_extension("backup.json")));
        match data {
            Ok(b) => reply(&mut s, "200 OK", "application/json", &b),
            Err(_) => reply(&mut s, "200 OK", "application/json", b"null"),
        }
    } else if method == "POST" && suffix == "state" {
        let content_type = headers.lines().any(|x| {
            x.to_ascii_lowercase()
                .starts_with("content-type: application/json")
        });
        let len = headers.lines().find_map(|x| {
            x.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        });
        if !content_type || len.is_none() || len.unwrap_or(MAX + 1) > MAX {
            reply(&mut s, "400 Bad Request", "text/plain", b"Invalid payload");
            return false;
        }
        let len = len.unwrap();
        while buf.len() < end + len {
            match s.read(&mut chunk) {
                Ok(0) | Err(_) => return false,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        }
        let data = &buf[end..end + len];
        if !serde_json::from_slice::<serde_json::Value>(data)
            .map(|v| v.is_object())
            .unwrap_or(false)
        {
            reply(
                &mut s,
                "400 Bad Request",
                "text/plain",
                b"Expected JSON object",
            );
            return false;
        }
        match save(path, data) {
            Ok(()) => reply(&mut s, "200 OK", "application/json", b"{\"ok\":true}"),
            Err(_) => reply(
                &mut s,
                "500 Internal Server Error",
                "text/plain",
                b"Save failed",
            ),
        }
    } else if method == "POST" && suffix == "quit" {
        reply(&mut s, "200 OK", "text/plain", b"Closed");
        return true;
    } else {
        reply(&mut s, "404 Not Found", "text/plain", b"Not found");
    }
    false
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    if !desktop::single_instance() {
        return Ok(());
    }
    let dir = directory();
    std::fs::create_dir_all(&dir)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let origin = format!("http://{}", listener.local_addr()?);
    let base = format!("/{}/", token());
    let url = format!("{origin}{base}");
    let state = updates::new();
    let (notification_sender, notification_receiver) = std::sync::mpsc::channel();
    let hub = notifications::Hub::new(dir.join("notifications.json"), notification_sender)?;
    hub.start(dir.join("agenda.json"));
    let serve = move || {
        for stream in listener.incoming().flatten() {
            if handle(
                stream,
                &base,
                &origin,
                &dir.join("agenda.json"),
                &state,
                &hub,
            ) {
                break;
            }
        }
    };
    if std::env::var_os("ATELIER_HEADLESS").is_some() {
        println!("{url}");
        serve();
        return Ok(());
    }
    #[cfg(windows)]
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            serve();
            let _ = sender.send(());
        });
        desktop::run(
            url,
            receiver,
            notification_receiver,
            directory().join("webview"),
        )?;
    }
    #[cfg(not(windows))]
    {
        drop(notification_receiver);
        println!("{url}");
        serve();
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        #[cfg(windows)] desktop::error(&format!("Não foi possível abrir o Atelier.\n{e}\nSe necessário, execute novamente o instalador para preparar o WebView2."));
        #[cfg(not(windows))]
        eprintln!("{e}");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_varies() {
        assert_ne!(token(), token());
    }
    #[test]
    fn write_keeps_previous_backup() {
        let p = std::env::temp_dir().join(format!("atelier-test-{}.json", token()));
        save(&p, b"{\"v\":1}").unwrap();
        save(&p, b"{\"v\":2}").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"{\"v\":2}");
        assert_eq!(
            std::fs::read(p.with_extension("backup.json")).unwrap(),
            b"{\"v\":1}"
        );
        let _ = std::fs::remove_file(p.with_extension("backup.json"));
        let _ = std::fs::remove_file(p);
    }
}
