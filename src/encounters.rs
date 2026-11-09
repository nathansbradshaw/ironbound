//! Public encounter catalog and reusable mechanic coverage, separate from bank ownership.
use crate::planner::{self, Monster, Target};
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct BrowserCatalog {
    pub raids: Vec<String>,
    pub groups: Vec<String>,
    pub popular: Vec<String>,
}
pub fn browser_catalog() -> &'static BrowserCatalog {
    static DATA: std::sync::OnceLock<BrowserCatalog> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/encounter-browser.json"))
            .expect("encounter browser metadata")
    })
}
#[derive(Clone, Debug, Deserialize)]
pub struct Group {
    pub name: String,
    pub aliases: Vec<String>,
    pub targets: Vec<Target>,
    pub note: String,
}
pub fn groups() -> &'static [Group] {
    static DATA: std::sync::OnceLock<Vec<Group>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/encounter-groups.json"))
            .expect("encounter groups")
    })
}
#[derive(Debug, Deserialize)]
pub struct Mechanics {
    pub toa_ids: Vec<i32>,
    pub unsupported_attributes: Vec<String>,
    pub unsupported_names: Vec<String>,
    pub no_bank_equipment: Vec<String>,
    pub unverified_modes: Vec<String>,
}
pub fn mechanics() -> &'static Mechanics {
    static DATA: std::sync::OnceLock<Mechanics> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/encounter-mechanics.json"))
            .expect("encounter mechanics")
    })
}
pub fn limitation(target: &Monster) -> Option<&'static str> {
    let data = mechanics();
    if target
        .attributes
        .iter()
        .any(|a| data.unsupported_attributes.contains(a))
        || data
            .unsupported_names
            .iter()
            .any(|n| target.name.starts_with(n) || target.name.contains(n))
    {
        Some(
            "This encounter needs additional scaling, immunity or phase mechanics. Equipment packing is available, but outgoing DPS is not verified.",
        )
    } else {
        None
    }
}
pub struct Entry {
    pub name: String,
    pub targets: Vec<Target>,
    pub note: String,
    pub bank_equipment: bool,
}
impl Entry {
    pub fn category(&self) -> &'static str {
        let data = browser_catalog();
        if data.raids.iter().any(|raid| self.name.starts_with(raid)) {
            "Raids"
        } else if data.groups.contains(&self.name) {
            "Groups"
        } else {
            "Bosses"
        }
    }
    pub fn coverage(&self) -> &'static str {
        if !self.bank_equipment {
            "Encounter equipment / skilling model needed"
        } else if self.targets.is_empty() {
            if mechanics().unverified_modes.contains(&self.name) {
                "Mode scaling not supported"
            } else {
                "Target mapping missing"
            }
        } else if self
            .targets
            .iter()
            .any(|t| planner::monster(t.id).is_some_and(|m| limitation(m).is_some()))
        {
            "Packing template · DPS partial"
        } else {
            "Basic attack estimates · mechanics partial"
        }
    }
}
fn key(s: &str) -> String {
    s.trim_start_matches("The ")
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}
pub fn catalog() -> &'static [Entry] {
    static DATA: std::sync::OnceLock<Vec<Entry>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let mut names:Vec<String>=serde_json::from_str(include_str!("../assets/bosses.json")).expect("boss names");
        for group in groups(){if !names.contains(&group.name){names.push(group.name.clone());}}
        let mut entries=vec![];
        for name in names {
            let group=groups().iter().find(|g|g.name==name||g.aliases.contains(&name));
            let bank_equipment=!mechanics().no_bank_equipment.contains(&name);
            let unverified_mode=mechanics().unverified_modes.contains(&name);
            let targets=if unverified_mode||!bank_equipment {vec![]} else if let Some(g)=group {g.targets.clone()} else {
                let search=match name.as_str(){"Barrows Chests"=>"Dharok","The Fractured Archive"=>"Ocellus",_=>name.as_str()};
                let mut candidates=planner::monsters().iter().filter(|m| {
                    let base=m.name.split(" (").next().unwrap_or(&m.name);
                    key(base)==key(search)&&!m.name.contains("Nightmare Zone")&&!m.name.contains("LANScape")&&!m.name.contains("(Quest")&&!m.name.contains("Awakened")
                }).collect::<Vec<_>>();
                // One default attack window per boss. Group templates explicitly opt
                // into multiple phases; do not accidentally combine solo/group, normal/
                // awakened, quest/post-quest or event versions into one encounter.
                candidates.sort_by_key(|m| (variant_rank(&m.name),m.id));
                candidates.first().map(|m|vec![Target{id:m.id,style:starting_style(m).into()}]).unwrap_or_default()
            };
            let note=if !bank_equipment {"Uses encounter-specific equipment or skilling mechanics; bank combat-gear optimization is not applicable yet.".into()} else if unverified_mode {"This mode needs dedicated raid scaling. Use the base-mode packing template; mode-specific optimization is not supported yet.".into()} else if let Some(g)=group {g.note.clone()} else if targets.is_empty(){"Listed in the boss catalog, but a verified combat target mapping is still missing. Use monster search when the target becomes available.".into()} else {"One recorded attack window, not a complete phase list. Editable starting styles use recorded defences; check immunities and add other forms through monster search. Basic attack estimates do not imply a fully simulated fight.".into()};
            entries.push(Entry{name,targets,note,bank_equipment});
        }
        entries.sort_by(|a,b|a.name.cmp(&b.name));entries
    })
}
fn variant_rank(name: &str) -> u8 {
    if name.contains("(Solo)") {
        0
    } else if name.contains("(Post-quest") {
        1
    } else if name.contains("(Normal") {
        2
    } else if !name.contains(" (") {
        3
    } else {
        4
    }
}
fn starting_style(m: &Monster) -> &'static str {
    ["Stab", "Slash", "Crush", "Ranged", "Magic"]
        .into_iter()
        .min_by_key(|style| {
            let key = match *style {
                "Stab" => "stab",
                "Slash" => "slash",
                "Crush" => "crush",
                "Ranged" => "standard",
                _ => "magic",
            };
            let level = m
                .skills
                .get(if *style == "Magic" { "magic" } else { "def" })
                .copied()
                .unwrap_or(1);
            (level as i64 + 9) * (m.defensive.get(key).copied().unwrap_or(0) as i64 + 64)
        })
        .unwrap_or("Melee")
}
