//! Join whole public catalogs; never read an account or request bank-specific items.
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 7 {
        return Err(
            "official-prices.json market.json items.json ItemID.java ItemMapping.java YYYY-MM-DD"
                .into(),
        );
    }
    let ge: Vec<Value> = serde_json::from_str(&fs::read_to_string(&a[1])?)?;
    let market: Value = serde_json::from_str(&fs::read_to_string(&a[2])?)?;
    let meta: BTreeMap<String, Value> = serde_json::from_str(&fs::read_to_string(&a[3])?)?;
    let ids = fs::read_to_string(&a[4])?;
    let mapping = fs::read_to_string(&a[5])?;
    let mut constants = BTreeMap::<String, i32>::new();
    for line in ids.lines() {
        if let Some(s) = line.trim().strip_prefix("public static final int ") {
            if let Some((name, v)) = s.split_once(" = ") {
                if let Ok(id) = v.trim_end_matches(';').parse() {
                    constants.entry(name.into()).or_insert(id);
                }
            }
        }
    }
    let official: BTreeMap<i32, u64> = ge
        .iter()
        .filter_map(|v| {
            Some((
                v["id"].as_i64()? as i32,
                v["price"].as_u64().filter(|n| *n > 0)?,
            ))
        })
        .collect();
    let mut aliases = BTreeMap::<i32, Vec<(i32, u64)>>::new();
    for line in mapping.lines() {
        let Some((_, args)) = line.trim().split_once('(') else {
            continue;
        };
        if !line.trim().starts_with("ITEM_") {
            continue;
        }
        let args = args.split(')').next().unwrap_or("");
        let parts: Vec<_> = args.split(',').map(str::trim).collect();
        let named: Vec<_> = parts
            .iter()
            .filter_map(|s| {
                s.strip_prefix("ItemID.")
                    .and_then(|k| constants.get(k))
                    .copied()
            })
            .collect();
        if named.len() < 2 {
            continue;
        }
        let quantity = parts
            .iter()
            .find_map(|s| s.strip_suffix('L').and_then(|s| s.parse::<u64>().ok()))
            .unwrap_or(1);
        for id in &named[1..] {
            if *id != named[0] {
                aliases.entry(*id).or_default().push((named[0], quantity));
            }
        }
    }
    let mut items = BTreeMap::new();
    for (raw, v) in &meta {
        let Ok(id) = raw.parse::<i32>() else { continue };
        if v["placeholder"] == true {
            continue;
        }
        let canonical = v["linked_id_item"]
            .as_i64()
            .filter(|_| v["noted"] == true)
            .map(|n| n as i32)
            .unwrap_or(id);
        let m = &market["data"][canonical.to_string()];
        let price = m["high"]
            .as_u64()
            .into_iter()
            .chain(m["low"].as_u64())
            .filter(|n| *n > 0)
            .max()
            .or_else(|| official.get(&canonical).copied());
        let protect = official
            .get(&canonical)
            .copied()
            .filter(|_| v["tradeable_on_ge"] == true);
        items.insert(id,json!({"market":price,"protect":protect,"stackable":v["stackable"],"noted":v["noted"],"canonical":canonical}));
    }
    // Current GE items can be newer than the historical metadata snapshot.
    for (&id, &price) in &official {
        items.entry(id).or_insert_with(||{let m=&market["data"][id.to_string()];json!({"market":m["high"].as_u64().into_iter().chain(m["low"].as_u64()).filter(|n|*n>0).max().unwrap_or(price),"protect":price,"stackable":null,"noted":false,"canonical":id})});
    }
    // RuneLite component mappings provide replacement costs, never automatic protection.
    for (&id, parts) in &aliases {
        let mut value = 0u64;
        let mut complete = true;
        for (base, q) in parts {
            let market = items.get(base).and_then(|v| v["market"].as_u64());
            if let Some(n) = market {
                value = value.saturating_add(n.saturating_mul(*q));
            } else {
                complete = false;
            }
        }
        let row = items.entry(id).or_insert_with(
            || json!({"market":null,"protect":null,"stackable":false,"noted":false,"canonical":id}),
        );
        if complete && value > 0 && row["market"].is_null() {
            row["market"] = json!(value);
        }
    }
    // Barrows protects according to its broken version in PvP. Replacement stays separate.
    for (name, &id) in &constants {
        if !name.starts_with("BARROWS_") || name.contains("ORNAMENT") || name.ends_with("BROKEN") {
            continue;
        }
        let base = name
            .rsplit_once('_')
            .filter(|(_, suffix)| ["25", "50", "75", "100"].contains(suffix))
            .map(|(base, _)| base)
            .unwrap_or(name);
        if let Some(broken) = constants.get(&format!("{base}_BROKEN")) {
            if let Some(price) = official.get(broken) {
                if let Some(row) = items.get_mut(&id) {
                    row["protect"] = json!(price);
                }
            }
        }
    }
    items.insert(
        995,
        json!({"market":1,"protect":1,"stackable":true,"canonical":995}),
    );
    items.insert(
        13204,
        json!({"market":1000,"protect":1000,"stackable":true,"canonical":13204}),
    );
    if official.len() < 3000 || items.len() < 15000 {
        return Err("Public catalog incomplete".into());
    }
    fs::write(
        "assets/death-risk-values.json",
        serde_json::to_vec(&json!({"captured_at":a[6],"items":items}))?,
    )?;
    println!(
        "Built {} public item values; protection and replacement prices remain separate.",
        items.len()
    );
    Ok(())
}
