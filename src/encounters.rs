//! Public encounter catalog and reusable mechanic coverage, separate from bank ownership.
use crate::planner::{self, Monster, Target};
use serde::Deserialize;
use std::collections::BTreeMap;
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
    #[serde(default)]
    pub targets: Vec<Target>,
    pub note: String,
}
pub fn groups() -> &'static [Group] {
    static DATA: std::sync::OnceLock<Vec<Group>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let mut groups: Vec<Group> =
            serde_json::from_str(include_str!("../assets/encounter-groups.json"))
                .expect("encounter groups");
        for group in &mut groups {
            if let Some(raid) = crate::raids::raids()
                .iter()
                .find(|raid| raid.name == group.name)
            {
                group.targets = raid.default_targets();
            }
        }
        groups
    })
}
#[derive(Debug, Deserialize)]
pub struct Mechanics {
    #[serde(default)]
    pub slayer_monster_ids: Vec<i32>,
    #[serde(default)]
    pub magic_defence_uses_defence: Vec<i32>,
    pub toa_ids: Vec<i32>,
    pub unsupported_attributes: Vec<String>,
    pub unsupported_names: Vec<String>,
    pub no_bank_equipment: Vec<String>,
    pub unverified_modes: Vec<String>,
    #[serde(default)]
    pub windows: Vec<Window>,
    #[serde(default)]
    pub demonbane_vulnerability: BTreeMap<i32, u32>,
    #[serde(default)]
    pub cycles: Vec<AlternatingCycle>,
}
#[derive(Debug, Deserialize)]
pub struct AlternatingCycle {
    pub id: String,
    pub hp: u32,
    pub occurrences: u32,
    pub first_style: String,
}
/// Catalog target keys may describe different states of the same game NPC.
/// `npc_id` is the real source NPC; `id` is a shareable internal attack-window key.
#[derive(Debug, Deserialize)]
pub struct Window {
    #[serde(default)]
    pub damage_model: DamageModel,
    pub id: i32,
    pub npc_id: i32,
    #[serde(default)]
    pub label: String,
    pub phase: String,
    pub description: String,
    #[serde(default)]
    pub progress_hp: BTreeMap<String, f64>,
    /// Alternative states share this workload instead of each counting a kill.
    #[serde(default)]
    pub progress_group: Option<String>,
    #[serde(default)]
    pub start_hp: Option<u32>,
    #[serde(default)]
    pub comparison_probe: bool,
    #[serde(default = "supported")]
    pub outgoing_supported: bool,
    #[serde(default)]
    pub default_style: Option<String>,
    #[serde(default)]
    pub hit_rule: Option<HitRule>,
    #[serde(default)]
    pub cycle: Option<String>,
    #[serde(default = "one_entity")]
    pub entity_count: u32,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DamageModel {
    #[default]
    Standard,
    VerifiedStandard,
    WardensP2,
    WardenCore,
    HalfDamage,
}
#[derive(Debug, Deserialize)]
pub struct HitRule {
    pub always_max: bool,
    pub allowed_styles: Vec<String>,
    #[serde(default)]
    pub mark_damage: bool,
}
fn one_entity() -> u32 {
    1
}
fn supported() -> bool {
    true
}
pub fn mechanics() -> &'static Mechanics {
    static DATA: std::sync::OnceLock<Mechanics> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/encounter-mechanics.json"))
            .expect("encounter mechanics")
    })
}
pub fn window(id: i32) -> Option<&'static Window> {
    mechanics().windows.iter().find(|window| window.id == id)
}
pub fn hit_rule(id: i32) -> Option<&'static HitRule> {
    window(id)?.hit_rule.as_ref()
}
pub fn alternating_cycle(id: &str) -> Option<&'static AlternatingCycle> {
    mechanics().cycles.iter().find(|cycle| cycle.id == id)
}
/// Explicit progress budgets prevent each state from counting an entire boss kill.
pub fn progress_hp(id: i32, style: &str) -> Option<f64> {
    window(id)?
        .progress_hp
        .get(crate::rules::base_style(style))
        .copied()
}
pub fn demonbane_vulnerability(id: i32) -> u32 {
    let source = source_npc_id(id);
    mechanics()
        .demonbane_vulnerability
        .get(&source)
        .copied()
        .unwrap_or(100)
}
pub fn source_npc_id(id: i32) -> i32 {
    window(id).map(|w| w.npc_id).unwrap_or(id)
}
pub fn slayer_eligible(id: i32) -> bool {
    let source = source_npc_id(id);
    mechanics()
        .slayer_monster_ids
        .binary_search(&source)
        .is_ok()
}
pub fn limitation(target: &Monster) -> Option<&'static str> {
    if window(target.id).is_some_and(|window| !window.outgoing_supported) {
        return Some(
            "This attack window needs reactive protection, guaranteed-hit or contract mechanics. Its equipment packing is available; outgoing DPS is not verified.",
        );
    }
    if window(target.id)
        .is_some_and(|w| w.outgoing_supported && w.damage_model != DamageModel::Standard)
    {
        return None;
    }
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
    /// Equipment style changes do not change which boss is being fought.
    pub fn matches(&self, targets: &[Target]) -> bool {
        !self.targets.is_empty()
            && self.targets.len() == targets.len()
            && targets.iter().all(Target::valid)
            && self.targets.iter().zip(targets).all(|(a, b)| a.id == b.id)
    }
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
    if let Some(style) = window(m.id).and_then(|window| window.default_style.as_deref()) {
        if crate::rules::STYLES.contains(&style) {
            return style;
        }
    }
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
