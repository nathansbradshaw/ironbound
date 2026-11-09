use std::{
    fs,
    io::{IsTerminal, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
fn run(program: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if !Command::new(program).args(args).status()?.success() {
        return Err(format!("{program} failed").into());
    }
    Ok(())
}
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("compact-data") => {
            let catalog = ironbound::planner::equipment();
            fs::write(
                "assets/equipment.json",
                serde_json::to_string_pretty(catalog)?,
            )?;
            println!(
                "Retained {} equipment IDs and only the fields this planner uses.",
                catalog.len()
            );
        }
        Some("import-group") => {
            let group = args
                .get(2)
                .ok_or("Usage: cargo run --bin site -- import-group 'Your Group'")?;
            eprint!("GroupIron token (not saved): ");
            std::io::stderr().flush()?;
            let terminal = std::io::stdin().is_terminal();
            if terminal {
                Command::new("stty").arg("-echo").status()?;
            }
            let mut token = String::new();
            let read = std::io::stdin().read_line(&mut token);
            if terminal {
                Command::new("stty").arg("echo").status()?;
                eprintln!();
            }
            read?;
            if token.trim().is_empty() || token.trim().chars().any(char::is_control) {
                return Err("Token is empty or contains invalid characters".into());
            }
            let group_path = group
                .bytes()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                        char::from(b).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect::<String>();
            let url = format!(
                "https://groupiron.men/api/group/{group_path}/get-group-data?from_time=1970-01-01T00%3A00%3A00.000Z"
            );
            // Header passes through a private pipe, never argv, a file, or the source repository.
            let mut child = Command::new("curl")
                .args([
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--max-time",
                    "20",
                    "--max-filesize",
                    "2000000",
                    "--proto",
                    "=https",
                    "--header",
                    "@-",
                    &url,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child
                .stdin
                .take()
                .ok_or("Could not open the credential pipe")?
                .write_all(format!("Authorization: {}\n", token.trim()).as_bytes())?;
            drop(token);
            let result = child.wait_with_output()?;
            if !result.status.success() {
                return Err("GroupIron rejected the request or could not be reached; no snapshot was written.".into());
            }
            let raw = String::from_utf8(result.stdout)?;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let snapshot = ironbound::account::Snapshot::parse(&raw, format!("Unix time {time}"))?;
            fs::create_dir_all(".local")?;
            fs::write(
                ".local/groupiron.json",
                serde_json::to_vec_pretty(&snapshot)?,
            )?;
            println!(
                "Snapshot saved to .local/groupiron.json. Import it through Your account → Choose a JSON file. The token was not saved."
            );
        }
        Some("build") => {
            run(
                "cargo",
                &[
                    "build",
                    "--release",
                    "--lib",
                    "--target",
                    "wasm32-unknown-unknown",
                    "--locked",
                    "-j",
                    "2",
                ],
            )?;
            fs::create_dir_all("dist")?;
            run(
                "wasm-bindgen",
                &[
                    "--target",
                    "web",
                    "--out-dir",
                    "dist/pkg",
                    "--out-name",
                    "ironbound",
                    "target/wasm32-unknown-unknown/release/ironbound.wasm",
                ],
            )?;
            fs::copy("index.html", "dist/index.html")?;
            fs::copy("style.css", "dist/style.css")?;
            copy_tree(Path::new("assets"), Path::new("dist/assets"))?;
            println!("Built static Rust/Wasm site in dist/");
        }
        Some("serve") => {
            let port = args.get(2).map(String::as_str).unwrap_or("3030");
            let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
            println!("Ironbound preview: http://127.0.0.1:{port}");
            for mut stream in listener.incoming().flatten() {
                stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
                let mut buf = [0; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let path = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .split('?')
                    .next()
                    .unwrap_or("/");
                if path.contains("..") || path.contains('%') || path.contains('\\') {
                    let _ =
                        stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
                    continue;
                }
                let file = PathBuf::from("dist").join(if path == "/" {
                    "index.html"
                } else {
                    path.trim_start_matches('/')
                });
                match fs::read(&file) {
                    Ok(bytes) => {
                        let mime = match file.extension().and_then(|s| s.to_str()) {
                            Some("wasm") => "application/wasm",
                            Some("js") => "text/javascript",
                            Some("css") => "text/css",
                            Some("png") => "image/png",
                            Some("ttf") => "font/ttf",
                            Some("json") => "application/json",
                            _ => "text/html; charset=utf-8",
                        };
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                            bytes.len()
                        );
                        let _ = stream.write_all(&bytes);
                    }
                    Err(_) => {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    }
                }
            }
        }
        Some("assets") => {
            fs::create_dir_all("assets/items")?;
            let mut ids = std::collections::BTreeSet::new();
            ids.extend([
                2347, 4151, 952, 385, 3144, 379, 373, 8007, 2434, 139, 141, 143, 3024, 3026, 3028,
                3030, 6685, 3040, 2444, 2440, 2442, 2436, 12625, 556, 554, 560, 565, 562, 563,
                9242, 9243, 9144, 9143, 9142, 892, 890, 1359, 1275, 1271, 1269, 4708, 4716, 4724,
                4732, 4745, 4753, 25979, 26219, 6737, 11905, 6570, 10828, 7462, 1712, 10499, 4091,
                4093, 9185, 12954, 3105, 2550, 12002, 6585, 11832, 11834, 2503, 2497, 2413, 1381,
                1215,
            ]);
            ids.extend(ironbound::planner::catalog_ids());
            for id in ids {
                let file = format!("assets/items/{id}.png");
                if !Path::new(&file).exists() {
                    run(
                        "curl",
                        &[
                            "-fsSL",
                            "--max-time",
                            "15",
                            &format!("https://static.runelite.net/cache/item/icon/{id}.png"),
                            "-o",
                            &file,
                        ],
                    )?;
                }
            }
        }
        Some("fixture") => {
            let account = ironbound::account::demo();
            let options = ironbound::planner::Options {
                boss: if args.get(2).is_some_and(|s| s == "toa") {
                    ironbound::planner::Boss::Toa
                } else {
                    ironbound::planner::Boss::Barrows
                },
                ..Default::default()
            };
            let plan = ironbound::planner::plan(&account, &options);
            println!(
                "{}",
                ironbound::export::inventory_setup(&plan, &options, &account.selected)?
            );
        }
        _ => println!(
            "cargo run --bin site -- [build | serve [port] | assets | fixture [toa] | import-group GROUP]"
        ),
    }
    Ok(())
}
