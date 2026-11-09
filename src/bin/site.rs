use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
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
fn pack_icons() -> Result<(), Box<dyn std::error::Error>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut icons = std::collections::BTreeMap::new();
    for entry in fs::read_dir("assets/items")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        let Some(id) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<i32>().ok())
        else {
            continue;
        };
        let bytes = fs::read(path)?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            continue;
        }
        let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let n = ((chunk[0] as u32) << 16)
                | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
                | chunk.get(2).copied().unwrap_or(0) as u32;
            encoded.push(TABLE[(n >> 18) as usize] as char);
            encoded.push(TABLE[((n >> 12) & 63) as usize] as char);
            encoded.push(if chunk.len() > 1 {
                TABLE[((n >> 6) & 63) as usize] as char
            } else {
                '='
            });
            encoded.push(if chunk.len() > 2 {
                TABLE[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        icons.insert(id, encoded);
    }
    let output = serde_json::to_vec(&icons)?;
    if fs::read("assets/item-icons-pack.json").ok().as_deref() != Some(output.as_slice()) {
        fs::write("assets/item-icons-pack.json", output)?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("catalog-bosses") => {
            let source = fs::read_to_string(
                args.get(2)
                    .ok_or("Provide public RuneLite HiscoreSkill.java")?,
            )?;
            let names: Vec<_> = source
                .lines()
                .filter(|l| l.contains(", BOSS,"))
                .filter_map(|l| l.split('"').nth(1))
                .collect();
            if names.len() < 50 {
                return Err("Incomplete public boss catalog".into());
            }
            fs::write("assets/bosses.json", serde_json::to_vec_pretty(&names)?)?;
        }
        Some("item-weights") => {
            let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(
                args.get(2)
                    .ok_or("Provide public OSRSReboxed items-complete.json")?,
            )?)?;
            let weights: std::collections::BTreeMap<i32, f64> = ironbound::planner::equipment()
                .iter()
                .filter_map(|gear| {
                    let item = data.get(gear.id.to_string())?;
                    if item.get("id")?.as_i64()? != i64::from(gear.id)
                        || item.get("noted")?.as_bool()?
                    {
                        return None;
                    }
                    let weight = item.get("weight")?.as_f64()?;
                    (weight.is_finite() && weight.abs() <= 1000.0).then_some((gear.id, weight))
                })
                .collect();
            if weights.len() < 100 {
                return Err("Incomplete public item weight data".into());
            }
            fs::write(
                "assets/item-weights.json",
                serde_json::to_vec_pretty(&weights)?,
            )?;
            println!(
                "Imported exact weights for {} / {} equipment IDs. Missing weights remain unknown.",
                weights.len(),
                ironbound::planner::equipment().len()
            );
        }
        Some("pack-icons") => pack_icons()?,
        Some("assets-all") => {
            // Always fetch the public catalog, never an account-derived item list.
            fs::create_dir_all("assets/items")?;
            let queue = std::sync::Arc::new(std::sync::Mutex::new(
                ironbound::planner::equipment()
                    .iter()
                    .map(|g| g.id)
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>(),
            ));
            let failures = std::sync::Arc::new(std::sync::Mutex::new(0usize));
            let mut workers = vec![];
            for _ in 0..8 {
                let queue = queue.clone();
                let failures = failures.clone();
                workers.push(std::thread::spawn(move || {
                    loop {
                        let Some(id) = queue.lock().unwrap().pop() else {
                            break;
                        };
                        let file = format!("assets/items/{id}.png");
                        if Path::new(&file).exists() {
                            continue;
                        }
                        let temporary = format!("{file}.part");
                        let result = Command::new("curl")
                            .args([
                                "-fsSL",
                                "--retry",
                                "1",
                                "--max-time",
                                "15",
                                &format!("https://static.runelite.net/cache/item/icon/{id}.png"),
                                "-o",
                                &temporary,
                            ])
                            .output();
                        if result.is_ok_and(|o| o.status.success())
                            && fs::read(&temporary)
                                .is_ok_and(|b| b.starts_with(b"\x89PNG\r\n\x1a\n"))
                        {
                            let _ = fs::rename(&temporary, &file);
                        } else {
                            let _ = fs::remove_file(&temporary);
                            *failures.lock().unwrap() += 1;
                        }
                    }
                }));
            }
            for worker in workers {
                let _ = worker.join();
            }
            let mut icons: Vec<i32> = fs::read_dir("assets/items")?
                .filter_map(|e| e.ok()?.path().file_stem()?.to_str()?.parse().ok())
                .collect();
            icons.sort_unstable();
            fs::write("assets/item-icons.json", serde_json::to_vec(&icons)?)?;
            println!(
                "Cached {} public equipment icons. {} unavailable icons use placeholders. No account data was read.",
                icons.len(),
                failures.lock().unwrap()
            );
        }
        Some("compact-monsters") => {
            let raw = fs::read_to_string(args.get(2).ok_or("Provide the public monster catalog")?)?;
            let data: Vec<serde_json::Value> = serde_json::from_str(&raw)?;
            let mut result = std::collections::BTreeMap::new();
            let mut ranks = std::collections::BTreeMap::new();
            for m in data {
                if let (Some(id), Some(name)) = (m["id"].as_i64(), m["name"].as_str()) {
                    let version = m["version"].as_str().unwrap_or("");
                    let entry = ironbound::planner::Monster {
                        id: id as i32,
                        name: if version.is_empty() {
                            name.into()
                        } else {
                            format!("{name} ({version})")
                        },
                        skills: serde_json::from_value(m["skills"].clone())?,
                        defensive: serde_json::from_value(m["defensive"].clone())?,
                        weakness: serde_json::from_value(m["weakness"].clone())?,
                        attributes: serde_json::from_value(m["attributes"].clone())?,
                        offensive: serde_json::from_value(m["offensive"].clone())?,
                        speed: m["speed"].as_i64().unwrap_or(0) as i32,
                        attack_styles: serde_json::from_value(m["style"].clone())
                            .unwrap_or_default(),
                        max_hit: m["max_hit"].as_str().unwrap_or("").into(),
                    };
                    // A shared NPC ID can have different calculator variants. Prefer its
                    // regular attack window over clone/parry or other special variants.
                    let rank = if version == "Regular" {
                        100
                    } else if version.starts_with("Post-quest") {
                        90
                    } else if version.starts_with("Normal") {
                        80
                    } else if version.is_empty() {
                        60
                    } else if version.contains("Awakened")
                        || version.contains("Quest")
                        || version.contains("Clone")
                    {
                        0
                    } else {
                        20
                    };
                    if ranks.get(&id).is_none_or(|r| rank > *r) {
                        ranks.insert(id, rank);
                        result.insert(id, entry);
                    }
                }
            }
            fs::write(
                "assets/monsters.json",
                serde_json::to_vec(&result.into_values().collect::<Vec<_>>())?,
            )?;
        }
        Some("compact-combat") => {
            let data: Vec<ironbound::combat::EquipmentStats> = serde_json::from_str(
                &fs::read_to_string(args.get(2).ok_or("Provide public equipment data")?)?,
            )?;
            fs::write("assets/combat-equipment.json", serde_json::to_vec(&data)?)?;
        }
        Some("compact-variants") => {
            let source: Vec<serde_json::Value> = serde_json::from_str(&fs::read_to_string(
                args.get(2)
                    .ok_or("Provide the public Wiki equipment data")?,
            )?)?;
            let variants: std::collections::BTreeMap<_, _> = source
                .iter()
                .filter_map(|v| {
                    Some((
                        v["id"].as_i64()? as i32,
                        v["version"].as_str().unwrap_or("").to_string(),
                    ))
                })
                .collect();
            let mut gear = ironbound::planner::equipment().to_vec();
            for g in &mut gear {
                g.version = variants.get(&g.id).cloned().unwrap_or_default();
            }
            fs::write("assets/equipment.json", serde_json::to_vec(&gear)?)?;
        }
        Some("compact-requirements") => {
            let source: serde_json::Value = serde_json::from_str(&fs::read_to_string(
                args.get(2).ok_or("Provide public requirements data")?,
            )?)?;
            let mut gear = ironbound::planner::equipment().to_vec();
            for g in &mut gear {
                let entry = &source[g.id.to_string()];
                if entry["equipment"].is_object() {
                    let mut req = std::collections::BTreeMap::new();
                    if let Some(fields) = entry["equipment"]["requirements"].as_object() {
                        for (name, level) in fields {
                            if let (Some(skill), Some(level)) = (
                                ironbound::account::SKILLS
                                    .iter()
                                    .find(|s| s.eq_ignore_ascii_case(name)),
                                level.as_u64(),
                            ) {
                                req.insert((*skill).to_string(), level as u32);
                            }
                        }
                    }
                    g.requirements = Some(req);
                }
            }
            fs::write("assets/equipment.json", serde_json::to_vec(&gear)?)?;
        }
        Some("publish-pages") => {
            fn git(dir: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
                let out = Command::new("git")
                    .current_dir(dir)
                    .args(["-c", "credential.helper=!gh auth git-credential"])
                    .args(args)
                    .output()?;
                if !out.status.success() {
                    return Err(String::from_utf8_lossy(&out.stderr).to_string().into());
                }
                Ok(String::from_utf8(out.stdout)?.trim().into())
            }
            let root = std::env::current_dir()?;
            if !git(&root, &["status", "--porcelain"])?.is_empty() {
                return Err("Commit and merge the source before publishing".into());
            }
            if git(&root, &["branch", "--show-current"])? != "main" {
                return Err("Publish from main".into());
            }
            let remote = git(&root, &["remote", "get-url", "origin"])?;
            if remote != "https://github.com/nathansbradshaw/ironbound.git" {
                return Err("Unexpected deployment remote".into());
            }
            let source = git(&root, &["rev-parse", "HEAD"])?;
            git(&root, &["push", "origin", "main"])?;
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis();
            let staging = std::env::temp_dir().join(format!("ironbound-pages-{unique}"));
            fs::create_dir(&staging)?;
            git(&staging, &["init", "--initial-branch=gh-pages"])?;
            git(&staging, &["remote", "add", "origin", &remote])?;
            if !git(&root, &["ls-remote", "origin", "refs/heads/gh-pages"])?.is_empty() {
                git(&staging, &["fetch", "--depth=1", "origin", "gh-pages"])?;
                git(&staging, &["checkout", "-B", "gh-pages", "FETCH_HEAD"])?;
                git(&staging, &["rm", "-r", "--ignore-unmatch", "."])?;
            }
            copy_tree(&root.join("dist"), &staging)?;
            fs::write(staging.join(".nojekyll"), "")?;
            fs::write(staging.join("source-commit.txt"), format!("{source}\n"))?;
            git(&staging, &["add", "--all"])?;
            git(
                &staging,
                &["commit", "-m", &format!("Deploy Ironbound {source}")],
            )?;
            git(&staging, &["push", "origin", "gh-pages"])?;
            let deployed = git(&staging, &["rev-parse", "HEAD"])?;
            fs::remove_dir_all(&staging)?;
            println!(
                "Pushed source {source} to main and static release {deployed} to gh-pages. Removed the owned staging directory. GitHub Pages must finish publishing before the release is live."
            );
        }
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
        Some("private-check") => {
            let bank = fs::read_to_string(
                args.get(2)
                    .ok_or("Provide a bank file and hiscores JSON file")?,
            )?;
            let levels = fs::read_to_string(args.get(3).ok_or("Provide a hiscores JSON file")?)?;
            let mut snapshot = ironbound::account::Snapshot::from_bank_tsv(
                &bank,
                "Local test account",
                "Private local test".into(),
            )?;
            snapshot.members[0].apply_hiscores(&levels)?;
            fs::create_dir_all(".local")?;
            fs::write(
                ".local/test-account.json",
                serde_json::to_vec_pretty(&snapshot)?,
            )?;
            for boss in [
                ironbound::planner::Boss::Barrows,
                ironbound::planner::Boss::Toa,
            ] {
                let p = ironbound::planner::plan(
                    &snapshot,
                    &ironbound::planner::Options {
                        boss,
                        ..Default::default()
                    },
                );
                println!(
                    "{}: {} bank item types, {} supported gear IDs, {} carried slots, {} preparation gaps. No quest checks. Skill levels loaded.",
                    boss.name(),
                    p.owned,
                    p.supported,
                    p.inventory.len(),
                    p.missing.len()
                );
                let budget = ironbound::planner::plan(
                    &snapshot,
                    &ironbound::planner::Options {
                        boss,
                        max_switches: Some(4),
                        switch_slots: 8,
                        ..Default::default()
                    },
                );
                if let Some(report) = budget.optimization {
                    println!(
                        "4-item mode: maximum {} changed slots, {} style-switch inventory slots, {} total carried slots.",
                        report.max_changes,
                        report.gear_slots,
                        budget.inventory.len()
                    );
                }
            }
            println!(
                "Private test snapshot saved under ignored .local/. No bank data was transmitted."
            );
        }
        Some("build") => {
            pack_icons()?;
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
            // All browser code and CSS share a content-derived cache key. A Pages
            // deployment must not combine a fresh page with cached older Wasm.
            let mut version = 0xcbf29ce484222325u64;
            for file in [
                "dist/pkg/ironbound_bg.wasm",
                "dist/pkg/ironbound.js",
                "style.css",
            ] {
                for byte in fs::read(file)? {
                    version = (version ^ byte as u64).wrapping_mul(0x100000001b3);
                }
            }
            let version = format!("{version:016x}");
            let index=fs::read_to_string("index.html")?
                .replace("./style.css",&format!("./style.css?v={version}"))
                .replace("./pkg/ironbound.js",&format!("./pkg/ironbound.js?v={version}"))
                .replace("init().catch",&format!("init({{module_or_path: new URL('./pkg/ironbound_bg.wasm?v={version}', location.href)}}).catch"));
            if !index.contains("module_or_path") {
                return Err(
                    "The module bootstrap changed; update asset versioning before publishing"
                        .into(),
                );
            }
            fs::write("dist/index.html", index)?;
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
                            Some("gif") => "image/gif",
                            Some("jpg") => "image/jpeg",
                            Some("woff") => "font/woff",
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
                9242, 9243, 9144, 9143, 9142, 892, 890, 1273, 1275, 1271, 1269, 4587, 25928, 4708,
                4716, 4724, 4732, 4745, 4753, 25979, 26219, 6737, 11905, 6570, 10828, 7462, 1712,
                10499, 4091, 4093, 9185, 12954, 3105, 2550, 12002, 6585, 11832, 11834, 2503, 2497,
                2413, 1381, 1215,
            ]);
            ids.extend(ironbound::planner::catalog_ids());
            // Public, account-independent artwork coverage for common alternatives.
            for g in ironbound::planner::equipment() {
                if [
                    "Blue wizard hat",
                    "Black wizard hat",
                    "Farseer helm",
                    "Warrior helm",
                    "Berserker helm",
                    "Archer helm",
                    "Mystic boots",
                    "Beacon ring",
                    "Ring of shadows",
                    "Ardougne cloak 1",
                    "Ardougne cloak 2",
                    "Ardougne cloak 3",
                    "Ardougne cloak 4",
                    "Explorer's ring 1",
                    "Explorer's ring 2",
                    "Explorer's ring 3",
                    "Explorer's ring 4",
                    "Fire cape",
                    "Mythical cape",
                ]
                .contains(&g.name.as_str())
                    || g.name.starts_with("Imbued ") && g.name.ends_with("cape")
                    || g.name.ends_with("d'hide boots")
                {
                    ids.insert(g.id);
                }
            }
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
            let icons: Vec<i32> = fs::read_dir("assets/items")?
                .filter_map(|e| e.ok()?.path().file_stem()?.to_str()?.parse().ok())
                .collect();
            fs::write("assets/item-icons.json", serde_json::to_vec(&icons)?)?;
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
            "cargo run --bin site -- [build | serve [port] | assets | fixture [toa] | private-check BANK HISCORES]"
        ),
    }
    Ok(())
}
