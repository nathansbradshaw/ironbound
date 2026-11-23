//! Ironman construction bills. Prepared parts are credited before expanding the
//! remainder into planks, bars and other materials. Planning never grants stock.
use crate::{
    account::Snapshot,
    equipment_view::escape as esc,
    planner::{self, Options},
    sailing::{Boat, Settings},
    tooltip::item_icon,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Recipe {
    pub name: String,
    pub quantity: u32,
    pub materials: Vec<(i32, u32)>,
    pub skills: BTreeMap<String, u32>,
    pub source: String,
    pub tools: String,
    pub notes: String,
}
#[derive(Deserialize)]
pub struct Catalog {
    pub reviewed: String,
    pub recipes: BTreeMap<String, Recipe>,
    pub items: BTreeMap<i32, Recipe>,
    pub aliases: BTreeMap<i32, i32>,
    pub sources: BTreeMap<String, u64>,
}
pub fn catalog() -> &'static Catalog {
    static DATA: OnceLock<Catalog> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/sailing-materials.json"))
            .expect("pinned boat recipes")
    })
}
pub fn recipe(b: &Settings, key: &str) -> Option<&'static Recipe> {
    if !b.valid() {
        return None;
    }
    let k = match key {
        "hull" => format!("hull:{}:{}", b.boat.index(), b.hull),
        "keel" if b.boat != Boat::Raft => format!("keel:{}:{}", b.boat.index(), b.keel),
        "cargo" => format!("cargo:{}", b.cargo?),
        "cannon" => format!("cannon:{}", b.cannon?),
        "cannon2" if b.cannon_count == 2 => {
            format!("cannon:{}", b.second_cannon.unwrap_or(b.cannon?))
        }
        "attractor" if b.attractor => "attractor".into(),
        "perildance" if b.perildance => "perildance".into(),
        "workbench" if b.repair_workbench => "workbench".into(),
        _ => return None,
    };
    catalog().recipes.get(&k)
}
pub fn changed(b: &Settings, old: Option<&Settings>) -> Vec<&'static str> {
    let old = old.filter(|p| p.boat == b.boat);
    [
        "hull",
        "keel",
        "cargo",
        "cannon",
        "cannon2",
        "attractor",
        "perildance",
        "workbench",
    ]
    .into_iter()
    .filter(|key| {
        recipe(b, key).is_some()
            && old.is_none_or(|p| match *key {
                "hull" => b.hull != p.hull,
                "keel" => b.keel != p.keel,
                "cargo" => b.cargo != p.cargo,
                "cannon" => b.cannon != p.cannon,
                "cannon2" => {
                    p.cannon.is_none()
                        || p.cannon_count != 2
                        || b.second_cannon.unwrap_or(b.cannon.unwrap_or(0))
                            != p.second_cannon.unwrap_or(p.cannon.unwrap_or(0))
                }
                "attractor" => !p.attractor,
                "perildance" => !p.perildance,
                "workbench" => !p.repair_workbench,
                _ => false,
            })
    })
    .collect()
}
pub fn stock(snapshot: &Snapshot, o: &Options) -> BTreeMap<i32, u32> {
    let mut out = BTreeMap::<i32, u32>::new();
    let mut available = snapshot.available(o.shared);
    if let Some(m) = snapshot.selected() {
        for (id, q) in m.sailing_owned.loaded_stock() {
            let n = available.entry(id).or_default();
            *n = n.saturating_add(q);
        }
    }
    for (id, q) in available {
        let id = catalog().aliases.get(&id).copied().unwrap_or(id);
        let entry = out.entry(id).or_default();
        *entry = entry.saturating_add(q);
    }
    out
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub id: i32,
    pub need: u32,
    pub have: u32,
    pub missing: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Bill {
    pub direct: Vec<Line>,
    pub raw: Vec<Line>,
    pub crafts: BTreeMap<i32, u32>,
    pub skills: BTreeMap<String, u32>,
    pub tools: BTreeSet<String>,
}
fn add_requirements(bill: &mut Bill, r: &Recipe) {
    for (skill, n) in &r.skills {
        let entry = bill.skills.entry(skill.clone()).or_default();
        *entry = (*entry).max(*n);
    }
    for tool in r.tools.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        bill.tools.insert(tool.into());
    }
}
fn fulfill(
    id: i32,
    n: u32,
    store: &mut BTreeMap<i32, u32>,
    raw: &mut BTreeMap<i32, (u32, u32)>,
    bill: &mut Bill,
    depth: usize,
) {
    assert!(depth < 8, "acyclic pinned material recipes");
    let available = store.entry(id).or_default();
    let used = (*available).min(n);
    *available -= used;
    let missing = n - used;
    if let Some(r) = catalog().items.get(&id) {
        if missing == 0 {
            return;
        }
        let batches = missing.div_ceil(r.quantity);
        *bill.crafts.entry(id).or_default() += batches * r.quantity;
        add_requirements(bill, r);
        for (ingredient, q) in &r.materials {
            fulfill(*ingredient, q * batches, store, raw, bill, depth + 1);
        }
        *store.entry(id).or_default() += batches * r.quantity - missing;
    } else {
        let entry = raw.entry(id).or_default();
        entry.0 += n;
        entry.1 += missing;
    }
}
pub fn bill(recipes: &[&Recipe], extra: &[(i32, u32)], stock: &BTreeMap<i32, u32>) -> Bill {
    let mut bill = Bill::default();
    let mut direct = BTreeMap::<i32, u32>::new();
    for r in recipes {
        add_requirements(&mut bill, r);
        for (id, q) in &r.materials {
            *direct.entry(*id).or_default() += *q;
        }
    }
    for (id, q) in extra.iter().filter(|(_, q)| *q > 0) {
        *direct.entry(*id).or_default() += *q;
    }
    let mut store = stock.clone();
    let mut raw = BTreeMap::new();
    for (id, q) in direct {
        bill.direct.push(Line {
            id,
            need: q,
            have: stock.get(&id).copied().unwrap_or(0),
            missing: q.saturating_sub(stock.get(&id).copied().unwrap_or(0)),
        });
        fulfill(id, q, &mut store, &mut raw, &mut bill, 0);
    }
    bill.raw = raw
        .into_iter()
        .map(|(id, (need, missing))| Line {
            id,
            need,
            missing,
            have: stock.get(&id).copied().unwrap_or(0),
        })
        .collect();
    bill
}
pub fn upgrades(b: &Settings, old: Option<&Settings>, stock: &BTreeMap<i32, u32>) -> Bill {
    let recipes: Vec<_> = changed(b, old)
        .iter()
        .filter_map(|key| recipe(b, key))
        .collect();
    bill(&recipes, &[], stock)
}
fn table(lines: &[Line]) -> String {
    let mut h="<table class=\"material-table\"><thead><tr><th>Material</th><th>Need</th><th>Have</th><th>Missing</th></tr></thead><tbody>".to_string();
    for l in lines {
        h.push_str(&format!(
            r#"<tr><th scope="row">{} {}</th><td>{}</td><td>{}</td><td class="{}">{}</td></tr>"#,
            item_icon(l.id),
            esc(&planner::item_name(l.id)),
            l.need,
            l.have,
            if l.missing == 0 {
                "material-ready"
            } else {
                "material-missing"
            },
            if l.missing == 0 {
                "✓".into()
            } else {
                l.missing.to_string()
            }
        ));
    }
    h.push_str("</tbody></table>");
    h
}
pub fn render_bill(b: &Bill, snapshot: &Snapshot) -> String {
    if b.direct.is_empty() {
        return "<p>No construction materials needed for these changes.</p>".into();
    }
    let mut h = if b.raw.iter().all(|l| l.missing == 0) {
        "<p class=\"material-ready\">Materials ready</p>".to_string()
    } else {
        String::new()
    };
    if !b.raw.is_empty() {
        h.push_str(&table(&b.raw));
    }
    if !b.crafts.is_empty() {
        h.push_str("<details><summary>Parts to make</summary><ul>");
        for (id, q) in &b.crafts {
            h.push_str(&format!("<li>{q} × {}</li>", esc(&planner::item_name(*id))));
        }
        h.push_str("</ul></details>");
    }
    h.push_str("<details><summary>Fitting ingredients</summary>");
    h.push_str(&table(&b.direct));
    h.push_str("</details><p class=\"material-skills\">");
    for (skill, level) in &b.skills {
        let have = snapshot.selected().and_then(|m| m.level(skill));
        h.push_str(&format!(
            "<span class=\"{}\">{} {}{}</span> ",
            if have.is_some_and(|n| n >= *level) {
                "material-ready"
            } else {
                "material-missing"
            },
            esc(skill),
            level,
            have.map(|n| format!(" (you: {n})"))
                .unwrap_or_else(|| " (unknown)".into())
        ));
    }
    h.push_str("</p>");
    if !b.tools.is_empty() {
        h.push_str(&format!(
            "<p class=\"small-note\">Reusable tools: {}.</p>",
            esc(&b.tools.iter().cloned().collect::<Vec<_>>().join(", "))
        ));
    }
    h
}
pub fn choice_status(b: &Settings, key: &str, value: &str, stock: &BTreeMap<i32, u32>) -> String {
    if !["hull", "keel", "cargo", "cannon", "cannon2"].contains(&key) {
        return String::new();
    }
    let mut candidate = b.clone();
    if !candidate.update(key, value) {
        return String::new();
    }
    // A second cannon selection is one fitting, independently of the count control.
    if key == "cannon2" {
        candidate.cannon_count = 2;
    }
    let Some(r) = recipe(&candidate, key) else {
        return "<small>No material cost</small>".into();
    };
    let b = bill(&[r], &[], stock);
    let missing = b.raw.iter().filter(|l| l.missing > 0).count();
    format!(
        "<small class=\"{}\">{}</small>",
        if missing == 0 {
            "material-ready"
        } else {
            "material-missing"
        },
        if missing == 0 {
            "Materials ready".into()
        } else {
            format!("{missing} shortfall{}", if missing == 1 { "" } else { "s" })
        }
    )
}
pub fn wizard_cost(d: &crate::sailing_owned::Draft, snapshot: &Snapshot, o: &Options) -> String {
    let stock = stock(snapshot, o);
    let old = d.original.as_ref().map(|p| &p.settings);
    let mut h = "<section class=\"boat-materials\"><h3>".to_string();
    h.push_str(if old.is_some() {
        "Upgrade materials"
    } else {
        "Fitting materials"
    });
    h.push_str("</h3>");
    let b = upgrades(&d.settings, old, &stock);
    h.push_str(&render_bill(&b, snapshot));
    h.push_str("<p class=\"small-note\">Credits imported parts, nails and materials once. Existing fittings cost nothing; removed fittings give no material refund. Helm and mast are outside this combat planner.</p><details><summary>Recipe sources &amp; unlocks</summary><ul>");
    for key in changed(&d.settings, old) {
        if let Some(r) = recipe(&d.settings, key) {
            h.push_str(&format!(
                "<li><a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">{}</a>{}</li>",
                esc(&r.source),
                esc(&r.name),
                if r.notes.is_empty() {
                    String::new()
                } else {
                    format!(
                        " · {}",
                        esc(&r
                            .notes
                            .replace("{{plink|", "")
                            .replace("}}", "")
                            .replace("[[", "")
                            .replace("]]", ""))
                    )
                }
            ));
        }
    }
    h.push_str("</ul></details></section>");
    h
}
pub fn fleet_cost(snapshot: &Snapshot, o: &Options) -> String {
    let Some(m) = snapshot.selected() else {
        return String::new();
    };
    let mut recipes = Vec::new();
    for p in &m.sailing_owned.profiles {
        if let Some(upgrade) = &p.upgrade {
            for key in changed(upgrade, Some(&p.settings)) {
                if let Some(r) = recipe(upgrade, key) {
                    recipes.push(r);
                }
            }
        }
    }
    if recipes.is_empty() {
        return String::new();
    }
    let b = bill(&recipes, &[], &stock(snapshot, o));
    format!(
        "<details class=\"panel fleet-materials\"><summary>Fleet upgrade materials</summary>{}<p class=\"small-note\">Combined saved plans. Materials are shared across this list and are not reserved or removed from your bank.</p></details>",
        render_bill(&b, snapshot)
    )
}
pub fn supply_cost(snapshot: &Snapshot, o: &Options) -> String {
    let b = &o.sailing;
    let extra = [
        (crate::sailing_owned::BALL_IDS[b.cannonball], b.cannonballs),
        (crate::sailing_owned::KIT_IDS[b.repair_tier], b.repair_kits),
    ];
    let bill = bill(&[], &extra, &stock(snapshot, o));
    format!(
        "<details class=\"material-supplies\"><summary>Materials for missing trip supplies</summary>{}<p class=\"small-note\">Uses the requested quantities above. Dragon cannonballs must be gathered; they cannot be smithed from sheets.</p></details>",
        render_bill(&bill, snapshot)
    )
}
