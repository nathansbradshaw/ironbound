//! Disposable manual browser QA server. Normal mode and `/?storage=blocked`
//! share the build; CSP gives the latter an opaque origin where Storage throws.
//! No mocks, browser-setting changes or private account fixtures are required.
//! rustc tools/browser-check.rs -o /tmp/ironbound-browser-check
//! /tmp/ironbound-browser-check dist 3046
use std::{fs, io::{Read, Write}, net::TcpListener, path::{Component, PathBuf}};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = PathBuf::from(args.get(1).ok_or("Provide generated dist directory")?).canonicalize()?;
    let port: u16 = args.get(2).ok_or("Provide an unused preview port")?.parse()?;
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("Browser QA: http://127.0.0.1:{port}/ (append ?storage=blocked for real storage failure)");
    for stream in listener.incoming() {
        let mut stream = stream?;
        let root = root.clone();
        std::thread::spawn(move || {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut request = [0; 8192];
            let n = stream.read(&mut request).unwrap_or(0);
            let request = String::from_utf8_lossy(&request[..n]);
            let url = request.lines().next().and_then(|r| r.split_whitespace().nth(1)).unwrap_or("/");
            let path = url.split('?').next().unwrap_or("/").trim_start_matches('/');
            let path = if path.is_empty() { "index.html" } else { path };
            let relative = PathBuf::from(path);
            if relative.components().any(|c| !matches!(c, Component::Normal(_))) { return; }
            let bytes = fs::read(root.join(&relative));
            let (status, bytes) = match bytes { Ok(b) => ("200 OK", b), Err(_) => ("404 Not Found", b"Not found".to_vec()) };
            let mime = match relative.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8", "js" => "text/javascript", "wasm" => "application/wasm",
                "css" => "text/css", "png" => "image/png", "gif" => "image/gif", "jpg" => "image/jpeg",
                "woff" => "font/woff", "ttf" => "font/ttf", _ => "text/plain",
            };
            let sandbox = if path == "index.html" && url.contains("storage=blocked") {
                "Content-Security-Policy: sandbox allow-scripts allow-downloads\r\n"
            } else { "" };
            let header = format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n{sandbox}Connection: close\r\n\r\n", bytes.len());
            let _ = stream.write_all(header.as_bytes()).and_then(|_| stream.write_all(&bytes));
        });
    }
    Ok(())
}
