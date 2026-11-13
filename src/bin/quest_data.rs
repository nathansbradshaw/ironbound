//! Refresh the reviewed quest-body metadata after editing assets/quest-bosses.json.
use serde_json::{Value, json};
use std::fs;
// Retain unchanged rows from the tracked catalog so a factual update does not
// reformat thousands of unrelated equipment and encounter records.
fn source(file: &str) -> String {
    std::process::Command::new("git")
        .args(["show", &format!("HEAD:{file}")])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| fs::read_to_string(file).expect("catalog source"))
}
fn rows(original: &str) -> std::collections::BTreeMap<String, String> {
    original
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            let value: Value = serde_json::from_str(line).ok()?;
            value
                .is_object()
                .then(|| (serde_json::to_string(&value).unwrap(), line.to_string()))
        })
        .collect()
}
fn array(values: &[Value], original: &str, indent: &str) -> String {
    let old = rows(original);
    values
        .iter()
        .map(|v| {
            let key = serde_json::to_string(v).unwrap();
            format!("{indent}{}", old.get(&key).unwrap_or(&key))
        })
        .collect::<Vec<_>>()
        .join(",\n")
}
fn save(file: &str, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let original = source(file);
    let base: Value = serde_json::from_str(&original)?;
    let lines: Vec<_> = original.lines().collect();
    let fields: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            line.strip_prefix("  \"")
                .and_then(|s| s.split('"').next())
                .map(|key| (i, key))
        })
        .collect();
    let mut output = Vec::new();
    for (index, (start, key)) in fields.iter().enumerate() {
        let end = fields
            .get(index + 1)
            .map(|(i, _)| *i)
            .unwrap_or(lines.len() - 1);
        let raw = lines[*start..end].join("\n");
        if data[*key] == base[*key] {
            output.push(raw.trim_end().trim_end_matches(',').to_string());
        } else if let Some(values) = data[*key].as_array() {
            output.push(format!(
                "  \"{key}\": [\n{}\n  ]",
                array(values, &raw, "    ")
            ));
        } else if *key == "families" {
            let old: std::collections::BTreeMap<_, _> = raw
                .lines()
                .filter_map(|line| {
                    let line = line.trim().trim_end_matches(',');
                    let v: Value = serde_json::from_str(&format!("{{{line}}}")).ok()?;
                    Some((serde_json::to_string(&v).ok()?, line.to_string()))
                })
                .collect();
            let children = data[*key]
                .as_object()
                .ok_or("families")?
                .iter()
                .map(|(name, value)| {
                    let single = json!({name:value});
                    let key = serde_json::to_string(&single).unwrap();
                    let row = old.get(&key).cloned().unwrap_or_else(|| {
                        format!("\"{name}\":{}", serde_json::to_string(value).unwrap())
                    });
                    format!("    {row}")
                })
                .collect::<Vec<_>>()
                .join(",\n");
            output.push(format!("  \"{key}\": {{\n{children}\n  }}"));
        } else {
            output.push(format!(
                "  \"{key}\": {}",
                serde_json::to_string(&data[*key])?
            ));
        }
    }
    fs::write(file, format!("{{\n{}\n}}\n", output.join(",\n")))?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profiles = &serde_json::from_slice::<Vec<ironbound::quest_bosses::Profile>>(&fs::read(
        "assets/quest-bosses.json",
    )?)?;
    let mut mechanics: Value =
        serde_json::from_slice(&fs::read("assets/encounter-mechanics.json")?)?;
    let windows = mechanics["windows"].as_array_mut().ok_or("windows")?;
    for p in profiles {
        for id in &p.ids {
            let index = if let Some(index) = windows.iter().position(|w| w["id"] == *id) {
                index
            } else {
                windows.push(
                    json!({"id":id,"npc_id":id,"phase":format!("{} · ordinary quest body",p.name)}),
                );
                windows.len() - 1
            };
            let w = &mut windows[index];
            w["outgoing_supported"] = json!(true);
            w["damage_model"] = json!("verified_standard");
            w["description"] = json!(p.note);
            if *id == 12224 {
                w["hp_scaling"] =
                    json!({"hitpoints":500,"skills":{"def":[180,130],"str":[210,280]}});
            }
        }
    }
    mechanics["demonbane_vulnerability"]["12195"] = json!(70);
    let mut groups: Value = serde_json::from_slice(&fs::read("assets/encounter-groups.json")?)?;
    for group in groups.as_array_mut().ok_or("groups")? {
        if let Some(p) = profiles.iter().find(|p| group["name"] == p.name) {
            group["note"] = json!(format!("Quest boss · {}. {}", p.quest, p.note));
            if !p.weapons.is_empty() {
                group["targets"][0]["style"] = json!(if p.ids.contains(&16204) {
                    "Slash"
                } else {
                    "Crush"
                });
            }
        }
    }
    let mut npc: Value = serde_json::from_slice(&fs::read("assets/monster-rules.json")?)?;
    for p in profiles {
        for id in &p.ids {
            let rules = npc["profiles"].as_array_mut().ok_or("NPC profiles")?;
            if rules.iter().any(|r| {
                r["ids"]
                    .as_array()
                    .is_some_and(|ids| ids.contains(&json!(id)))
            }) {
                continue;
            }
            let styles = if !p.weapons.is_empty() {
                vec!["Melee"]
            } else if *id == 13482 {
                vec!["Magic"]
            } else if [12215, 8096, 8097].contains(id) {
                vec!["Ranged", "Magic"]
            } else {
                vec!["Melee", "Ranged", "Magic"]
            };
            rules.push(json!({"ids":[id],"kind":"standard","allowed_styles":styles}));
        }
    }
    let mut weapons: Value = serde_json::from_slice(&fs::read("assets/weapon-rules.json")?)?;
    weapons["families"]["quest_flail"] =
        json!({"attack_types":["slash"],"formula":"standard","style":"Melee"});
    for (ids, family, level) in [
        (vec![22398], "crush", 40),
        (vec![24699], "crush", 50),
        (vec![33718], "quest_flail", 50),
    ] {
        if !weapons["weapons"]
            .as_array()
            .ok_or("weapons")?
            .iter()
            .any(|w| {
                w["ids"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(ids[0])))
            })
        {
            weapons["weapons"]
                .as_array_mut()
                .ok_or("weapons")?
                .push(json!({"ids":ids,"family":family,"requirements":{"Attack":level}}));
        }
    }
    for (file, data) in [
        ("encounter-mechanics.json", mechanics),
        ("monster-rules.json", npc),
        ("weapon-rules.json", weapons),
    ] {
        save(&format!("assets/{file}"), &data)?;
    }
    // Keep the established one-entry-per-line layout for encounter groups.
    fs::write(
        "assets/encounter-groups.json",
        format!(
            "[\n{}\n]\n",
            array(
                groups.as_array().ok_or("groups")?,
                &source("assets/encounter-groups.json"),
                " "
            )
        ),
    )?;
    println!(
        "Updated {} quest-body profiles, weapon mappings and source notes.",
        profiles.len()
    );
    Ok(())
}
