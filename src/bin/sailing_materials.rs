//! Pin boat recipes from public MediaWiki action=query revision exports.
//! cargo run --bin sailing_materials -- YYYY-MM-DD core.json facilities.json intermediates.json
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
fn templates(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find("{{Recipe") {
        let start = at + start;
        let bytes = text.as_bytes();
        let mut depth = 0;
        let mut end = start;
        while end + 1 < bytes.len() {
            if &bytes[end..end + 2] == b"{{" {
                depth += 1;
                end += 2;
            } else if &bytes[end..end + 2] == b"}}" {
                depth -= 1;
                end += 2;
                if depth == 0 {
                    break;
                }
            } else {
                end += 1;
            }
        }
        assert_eq!(depth, 0, "unclosed Recipe");
        out.push(&text[start + 8..end - 2]);
        at = end;
    }
    out
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let date = args
        .next()
        .ok_or("Provide review date and Wiki revision exports")?;
    let names: BTreeMap<i32, String> =
        serde_json::from_str(&fs::read_to_string("assets/item-names.json")?)?;
    let id_for = |name: &str| {
        names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
            .ok_or_else(|| format!("Unknown material: {name}"))
    };
    let mut recipes = BTreeMap::new();
    let mut items = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut aliases = BTreeMap::new();
    let woods = [
        "Wooden", "Oak", "Teak", "Mahogany", "Camphor", "Ironwood", "Rosewood",
    ];
    let metals = [
        "Bronze", "Iron", "Steel", "Mithril", "Adamant", "Rune", "Dragon",
    ];
    let mut register = |id: i32| {
        if names.get(&(id + 1)) == names.get(&id) {
            aliases.insert(id + 1, id);
        }
    };
    for path in args {
        let export: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        for page in export["query"]["pages"]
            .as_object()
            .ok_or("No Wiki pages")?
            .values()
        {
            let title = page["title"].as_str().ok_or("No title")?;
            if page.get("missing").is_some() {
                return Err(format!("Missing Wiki page: {title}").into());
            }
            let revision = &page["revisions"][0];
            let revid = revision["revid"].as_u64().ok_or("No revision ID")?;
            sources.insert(title.to_string(), revid);
            let source = format!(
                "https://oldschool.runescape.wiki/w/{}?oldid={revid}",
                title
                    .replace(' ', "_")
                    .replace('\'', "%27")
                    .replace('(', "%28")
                    .replace(')', "%29")
            );
            for template in templates(
                revision["slots"]["main"]["*"]
                    .as_str()
                    .ok_or("No wikitext")?,
            ) {
                let fields: BTreeMap<_, _> = template
                    .lines()
                    .filter_map(|l| l.trim().strip_prefix('|')?.split_once('='))
                    .map(|(k, v)| (k.trim(), v.trim()))
                    .collect();
                let Some(output) = fields.get("output1") else {
                    continue;
                };
                let quantity = fields
                    .get("output1quantity")
                    .unwrap_or(&"1")
                    .parse::<u32>()?;
                let mut materials = Vec::new();
                for n in 1..=12 {
                    if let Some(name) = fields.get(format!("mat{n}").as_str()) {
                        let id = id_for(name)?;
                        register(id);
                        let q = fields
                            .get(format!("mat{n}quantity").as_str())
                            .unwrap_or(&"1")
                            .parse::<u32>()?;
                        materials.push((id, q));
                    }
                }
                if materials.is_empty() {
                    continue;
                }
                let mut skills = BTreeMap::new();
                for n in 1..=4 {
                    if let Some(skill) = fields.get(format!("skill{n}").as_str()) {
                        skills.insert(
                            *skill,
                            fields
                                .get(format!("skill{n}lvl").as_str())
                                .ok_or("No skill level")?
                                .parse::<u32>()?,
                        );
                    }
                }
                let recipe = json!({"name":output,"quantity":quantity,"materials":materials,"skills":skills,"source":source,"tools":fields.get("tools").unwrap_or(&""),"notes":fields.get("notes").unwrap_or(&"")});
                if let Some(tier) = woods.iter().position(|w| title == format!("{w} hull")) {
                    let boat = fields
                        .get("output1subtxt")
                        .and_then(|v| ["Raft", "Skiff", "Sloop"].iter().position(|b| b == v))
                        .ok_or("Unknown hull variant")?;
                    recipes.insert(format!("hull:{boat}:{tier}"), recipe);
                } else if let Some(tier) = metals.iter().position(|m| title == format!("{m} keel"))
                {
                    let boat = fields
                        .get("output1subtxt")
                        .and_then(|v| ["Raft", "Skiff", "Sloop"].iter().position(|b| b == v))
                        .ok_or("Unknown keel variant")?;
                    recipes.insert(format!("keel:{boat}:{tier}"), recipe);
                } else if let Some(tier) = [
                    "Basic", "Oak", "Teak", "Mahogany", "Camphor", "Ironwood", "Rosewood",
                ]
                .iter()
                .position(|w| title == format!("{w} cargo hold"))
                {
                    recipes.insert(format!("cargo:{tier}"), recipe);
                } else if let Some(tier) =
                    metals.iter().position(|m| title == format!("{m} cannon"))
                {
                    recipes.insert(format!("cannon:{tier}"), recipe);
                } else if let Some(key) = [
                    ("Ballistic attractor", "attractor"),
                    ("Bosun's workbench", "workbench"),
                    ("Keg (facility)", "perildance"),
                ]
                .iter()
                .find(|(name, _)| *name == title)
                .map(|(_, k)| *k)
                {
                    recipes.entry(key.to_string()).or_insert(recipe);
                } else {
                    let id = id_for(output)?;
                    register(id);
                    items.entry(id).or_insert(recipe);
                }
            }
        }
    }
    // Filling the keg consumes 25 ales; the barrel stand is a reusable tool.
    recipes.get_mut("perildance").ok_or("Missing keg recipe")?["materials"]
        .as_array_mut()
        .unwrap()
        .push(json!([31817, 25]));
    register(31817);
    if recipes.len() != 52 || items.len() != 48 {
        return Err(format!(
            "Recipe coverage changed: {} fittings, {} intermediates",
            recipes.len(),
            items.len()
        )
        .into());
    }
    fs::write(
        "assets/sailing-materials.json",
        serde_json::to_string(
            &json!({"reviewed":date,"recipes":recipes,"items":items,"aliases":aliases,"sources":sources}),
        )?,
    )?;
    println!("Pinned 52 fitting recipes and 48 component/supply recipes, with revision IDs.");
    Ok(())
}
