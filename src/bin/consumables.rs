//! Import factual food restoration values from RuneLite's public Java registry.
use serde::Serialize;
use std::{collections::BTreeMap, fs};
#[derive(Clone, Serialize)]
struct Food {
    per_bite: i32,
    bites: u32,
    delayed: i32,
    scaling: String,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let source = fs::read_to_string(
        args.get(1)
            .ok_or("Provide ItemStatChanges.java and gameval/ItemID.java")?,
    )?;
    let constants = fs::read_to_string(args.get(2).ok_or("Provide gameval/ItemID.java")?)?;
    let mut ids = BTreeMap::new();
    for line in constants.lines() {
        if let Some(rest) = line.trim().strip_prefix("public static final int ") {
            if let Some((key, value)) = rest.split_once(" = ") {
                if let Ok(id) = value.trim_end_matches(';').parse::<i32>() {
                    ids.entry(key.to_string()).or_insert(id);
                }
            }
        }
    }
    let mut foods = BTreeMap::new();
    for entry in source.split("add(").skip(1) {
        let Some((effect, items)) = entry.split_once(", ItemID.") else {
            continue;
        };
        if !effect.starts_with("food(")
            && !effect.starts_with("combo(food(")
            && !effect.starts_with("combo(heal(HITPOINTS, ")
        {
            continue;
        }
        let mut heals = Vec::new();
        for text in effect.split("food(").skip(1) {
            if let Some(value) = text.split(')').next().and_then(|v| v.parse::<i32>().ok()) {
                heals.push(value);
            }
        }
        if heals.is_empty() {
            if let Some(value) = effect
                .strip_prefix("combo(heal(HITPOINTS, ")
                .and_then(|v| v.split(')').next())
                .and_then(|v| v.parse::<i32>().ok())
            {
                heals.push(value);
            }
        }
        if heals.is_empty() || heals[0] <= 0 {
            continue;
        }
        let group = items.split(");").next().unwrap_or(items);
        for token in std::iter::once(group).chain(std::iter::empty()) {
            for item in token.split("ItemID.") {
                let key = item
                    .trim()
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if let Some(id) = ids.get(key) {
                    let bites = if key == "CAKE" || key == "CHOCOLATE_CAKE" {
                        3
                    } else if key == "PARTIAL_CAKE"
                        || key == "PARTIAL_CHOCOLATE_CAKE"
                        || ((key.contains("PIZZA") || key.ends_with("_PIE"))
                            && !key.contains("HALF"))
                    {
                        2
                    } else {
                        1
                    };
                    foods.insert(
                        *id,
                        Food {
                            per_bite: heals[0],
                            bites,
                            delayed: heals.iter().skip(1).sum(),
                            scaling: String::new(),
                        },
                    );
                }
            }
        }
    }
    for (key, scaling) in [
        ("ANGLERFISH", "anglerfish"),
        ("BLIGHTED_ANGLERFISH", "anglerfish"),
        ("STRAWBERRY", "strawberry"),
        ("WATERMELON_SLICE", "watermelon"),
    ] {
        if let Some(id) = ids.get(key) {
            foods.insert(
                *id,
                Food {
                    per_bite: 0,
                    bites: 1,
                    delayed: 0,
                    scaling: scaling.into(),
                },
            );
        }
    }
    if foods.len() < 100
        || foods.get(&385).is_none_or(|f| f.per_bite != 20)
        || foods.get(&391).is_none_or(|f| f.per_bite != 22)
    {
        return Err("Public registry shape changed; food import rejected".into());
    }
    fs::write(
        "assets/food-healing.json",
        serde_json::to_vec_pretty(&foods)?,
    )?;
    println!(
        "Imported {} public food profiles; unsupported scaling stays unranked",
        foods.len()
    );
    Ok(())
}
