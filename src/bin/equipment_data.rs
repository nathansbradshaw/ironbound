//! Import factual NPC size metadata from the pinned public calculator catalog.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .ok_or("Provide public monsters.json")?;
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&fs::read(input)?)?;
    let mut sizes = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for row in rows {
        if let (Some(id), Some(size)) = (row["id"].as_i64(), row["size"].as_u64()) {
            if id > 0 && (1..=100).contains(&size) {
                if sizes.insert(id, size).is_some_and(|old| old != size) {
                    ambiguous.insert(id);
                }
            }
        }
    }
    for id in &ambiguous {
        sizes.remove(id);
    }
    if sizes.is_empty() {
        return Err("No unambiguous target sizes".into());
    }
    fs::write("assets/monster-sizes.json", serde_json::to_vec(&sizes)?)?;
    println!(
        "Imported {} target sizes; withheld {} ambiguous IDs",
        sizes.len(),
        ambiguous.len()
    );
    Ok(())
}
