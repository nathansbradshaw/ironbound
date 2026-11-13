//! Build a public, account-independent gear-price snapshot from complete Wiki responses.
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let latest: Value = serde_json::from_str(&fs::read_to_string(
        args.get(1).ok_or("latest.json required")?,
    )?)?;
    let mapping: Vec<Value> = serde_json::from_str(&fs::read_to_string(
        args.get(2).ok_or("mapping.json required")?,
    )?)?;
    let date = args.get(3).ok_or("snapshot date required")?;
    let ids: std::collections::BTreeSet<_> = ironbound::planner::equipment()
        .iter()
        .map(|g| g.id)
        .collect();
    let mut prices = BTreeMap::new();
    for item in mapping {
        let Some(id) = item["id"].as_i64().map(|v| v as i32) else {
            continue;
        };
        if !ids.contains(&id) {
            continue;
        }
        let row = &latest["data"][id.to_string()];
        let price = row["high"]
            .as_u64()
            .into_iter()
            .chain(row["low"].as_u64())
            .chain(item["value"].as_u64())
            .max();
        if let Some(price) = price.filter(|p| *p > 0) {
            prices.insert(id, price);
        }
    }
    if prices.len() < 1000 {
        return Err("Incomplete gear price snapshot; preserving existing data".into());
    }
    fs::write(
        "assets/wilderness-prices.json",
        serde_json::to_vec(&json!({"captured_at":date,"prices":prices}))?,
    )?;
    println!(
        "Stored {} public gear values; unknown/untradeable variants consume valuable-item slots.",
        prices.len()
    );
    Ok(())
}
