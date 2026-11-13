//! Refresh a public, account-independent boss portrait pack from the OSRS Wiki.
use std::{collections::BTreeMap, fs, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scratch = std::env::temp_dir().join(format!("ironbound-portraits-{}", std::process::id()));
    fs::create_dir(&scratch)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut pack: BTreeMap<String, String> = fs::read_to_string("assets/boss-portraits.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for entry in ironbound::encounters::catalog() {
            if pack.contains_key(&entry.name) && !std::env::args().any(|a| a == "--refresh") {
                continue;
            }
            let name = match entry.name.as_str() {
                "Tormented Demons" => "Tormented Demon (1)",
                "Barrows Chests" => "Dharok the Wretched",
                "Moons of Peril" | "Lunar Chests" => "Blood Moon",
                "Dagannoth Kings" => "Dagannoth Rex",
                "Chambers of Xeric" | "Chambers of Xeric: Challenge Mode" => "Great Olm",
                "Theatre of Blood" | "Theatre of Blood: Hard Mode" => "Verzik Vitur",
                "Tombs of Amascut" | "Tombs of Amascut: Expert Mode" => "Tumeken's Warden",
                "Fight Caves" => "TzTok-Jad",
                "Inferno" => "TzKal-Zuk",
                "Fortis Colosseum" => "Sol Heredit",
                "The Gauntlet" => "Crystalline Hunllef",
                "The Corrupted Gauntlet" => "Corrupted Hunllef",
                "The Fractured Archive" => "Ocellus",
                "Alchemical Hydra" => "Alchemical Hydra (serpentine)",
                "Crazy Archaeologist" => "Crazy archaeologist",
                "Deranged Archaeologist" => "Deranged archaeologist",
                "Grotesque Guardians" => "Dusk",
                "Kree'Arra" => "Kree'arra",
                "Nightmare" => "The Nightmare",
                "Phantom Muspah" => "Phantom Muspah (ranged)",
                "Phosani's Nightmare" => "The Nightmare",
                "Shellbane Gryphon" => "Shellbane gryphon",
                "The Royal Titans" => "Branda the Fire Queen",
                "Thermonuclear Smoke Devil" => "Thermonuclear smoke devil",
                "Wintertodt" => "Wintertodt prison",
                "Zulrah" => "Zulrah (serpentine)",
                "Sliske's wights" => "The Forsaken Assassin",
                "Vorkath (Quest)" => "Vorkath",
                "Duke Sucellus (Quest)" => "Duke Sucellus",
                "Vardorvis (Quest)" => "Vardorvis",
                "The Leviathan (Quest)" => "The Leviathan",
                "The Whisperer (Quest)" => "The Whisperer",
                other => other,
            };
            let file = format!("{}.png", name.replace(' ', "_").replace('\'', "%27"));
            let raw = scratch.join("raw.png");
            let small = scratch.join("small.png");
            let status = Command::new("curl")
                .args([
                    "-fsSL",
                    "--max-time",
                    "20",
                    &format!("https://oldschool.runescape.wiki/images/{file}"),
                    "-o",
                ])
                .arg(&raw)
                .status()?;
            if !status.success() {
                eprintln!("No portrait: {}", entry.name);
                continue;
            }
            if !fs::read(&raw)?.starts_with(b"\x89PNG\r\n\x1a\n") {
                continue;
            }
            if !Command::new("sips")
                .args(["-Z", "144"])
                .arg(&raw)
                .arg("--out")
                .arg(&small)
                .output()?
                .status
                .success()
            {
                return Err("Portrait resize failed".into());
            }
            let encoded = Command::new("base64").arg("-i").arg(&small).output()?;
            if !encoded.status.success() {
                return Err("Portrait encoding failed".into());
            }
            pack.insert(
                entry.name.clone(),
                String::from_utf8(encoded.stdout)?.replace(['\n', '\r'], ""),
            );
        }
        if pack.len() < 50 {
            return Err(format!(
                "Only {} portraits available; preserving existing pack",
                pack.len()
            )
            .into());
        }
        fs::write("assets/boss-portraits.json", serde_json::to_vec(&pack)?)?;
        println!("Packed {} public boss portraits.", pack.len());
        Ok(())
    })();
    fs::remove_dir_all(scratch)?;
    result
}
