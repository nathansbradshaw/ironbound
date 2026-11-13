//! Explicit current-level assumptions. These never modify base eligibility or
//! infer that a player owns a potion or landed a special attack.
use crate::{account::Member, planner::Monster};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Boosts {
    pub melee: String,
    pub ranged: String,
    pub magic: String,
    pub defence: String,
    pub encounter: String,
}
impl Default for Boosts {
    fn default() -> Self {
        Self {
            melee: "None".into(),
            ranged: "None".into(),
            magic: "None".into(),
            defence: "None".into(),
            encounter: "None".into(),
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub num: i64,
    pub den: i64,
    pub flat: i64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoostDefinition {
    pub id: String,
    pub label: String,
    pub channel: String,
    pub stats: BTreeMap<String, Change>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Floor {
    ids: Vec<i32>,
    floor: i32,
    #[serde(default)]
    base_num: i64,
    #[serde(default = "one")]
    base_den: i64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reduction {
    id: String,
    kind: String,
    stats: Vec<String>,
    #[serde(default)]
    num: i64,
    #[serde(default = "one")]
    den: i64,
    #[serde(default)]
    demon_num: i64,
    #[serde(default)]
    flat: i64,
    #[serde(default)]
    source_stat: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    #[serde(default)]
    sources: Vec<String>,
    boosts: Vec<BoostDefinition>,
    floors: Vec<Floor>,
    reductions: Vec<Reduction>,
}
impl Data {
    fn validated(self) -> Result<Self, String> {
        let bad = || "Invalid bundled combat-modifier metadata.".to_string();
        if self
            .sources
            .iter()
            .any(|s| !s.starts_with("https://") || s.len() > 2048)
        {
            return Err(bad());
        }
        let mut boost_ids = BTreeSet::new();
        for b in &self.boosts {
            let allowed: &[&str] = match b.channel.as_str() {
                "melee" => &["Attack", "Strength", "Defence"],
                "ranged" => &["Ranged"],
                "magic" => &["Magic", "Attack", "Strength", "Defence"],
                "defence" => &["Defence"],
                "encounter" => &["Attack", "Strength", "Defence", "Ranged", "Magic"],
                _ => return Err(bad()),
            };
            if b.id.trim().is_empty()
                || b.id == "None"
                || b.id.len() > 80
                || b.label.trim().is_empty()
                || !boost_ids.insert(&b.id)
                || b.stats.is_empty()
                || b.stats.iter().any(|(stat, c)| {
                    !allowed.contains(&stat.as_str())
                        || !(1..=1000).contains(&c.den)
                        || !(-c.den..=c.den).contains(&c.num)
                        || !(-255..=255).contains(&c.flat)
                })
            {
                return Err(bad());
            }
        }
        if boost_ids.is_empty() {
            return Err(bad());
        }
        let mut floor_ids = BTreeSet::new();
        for f in &self.floors {
            if f.ids.is_empty()
                || f.ids.iter().any(|id| *id <= 0 || !floor_ids.insert(*id))
                || !(0..=10000).contains(&f.floor)
                || !(1..=1000).contains(&f.base_den)
                || !(0..=f.base_den).contains(&f.base_num)
            {
                return Err(bad());
            }
        }
        // These are the public input fields, so a typo or omitted selector
        // cannot silently disable a bundled rule.
        let inputs = serde_json::to_value(Reductions::default()).map_err(|_| bad())?;
        let known = inputs.as_object().ok_or_else(bad)?;
        let mut reduction_ids = BTreeSet::new();
        for r in &self.reductions {
            let kind = r.kind.as_str();
            if !known.contains_key(&r.id)
                || !reduction_ids.insert(&r.id)
                || ![
                    "keep_current",
                    "subtract_current",
                    "subtract_base",
                    "subtract_stat",
                    "flat_level",
                    "flat_spill",
                    "bonus_flat",
                ]
                .contains(&kind)
                || r.stats.is_empty()
                || r.stats.iter().collect::<BTreeSet<_>>().len() != r.stats.len()
                || r.stats
                    .iter()
                    .any(|s| !["def", "str", "atk", "magic", "ranged"].contains(&s.as_str()))
                || !(1..=1000).contains(&r.den)
                || !(0..=r.den).contains(&r.num)
                || !(0..=r.den).contains(&r.demon_num)
                || !(0..=10000).contains(&r.flat)
                || (kind == "subtract_stat"
                    && !["def", "str", "atk", "magic", "ranged"].contains(&r.source_stat.as_str()))
                || (kind != "subtract_stat" && !r.source_stat.is_empty())
            {
                return Err(bad());
            }
        }
        if reduction_ids.len() != known.len() {
            return Err(bad());
        }
        Ok(self)
    }
}
fn one() -> i64 {
    1
}
fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str::<Data>(include_str!("../assets/combat-modifiers.json"))
            .expect("valid combat-modifier schema")
            .validated()
            .expect("valid combat-modifier semantics")
    })
}
pub fn boost_options(channel: &str) -> Vec<&'static BoostDefinition> {
    data()
        .boosts
        .iter()
        .filter(|b| b.channel == channel)
        .collect()
}
impl Boosts {
    pub fn valid(&self) -> bool {
        let known = [
            ("melee", &self.melee),
            ("ranged", &self.ranged),
            ("magic", &self.magic),
            ("defence", &self.defence),
            ("encounter", &self.encounter),
        ]
        .iter()
        .all(|(channel, id)| {
            id.as_str() == "None" || boost_options(channel).iter().any(|b| b.id == **id)
        });
        // The brew drains current melee stats. Without a drinking order,
        // combining it with a melee/raid boost has no unique current level.
        known
            && !(self.magic == "Forgotten brew"
                && (self.melee != "None" || self.defence != "None" || self.encounter != "None"))
    }
}
pub fn level(member: &Member, skill: &str, boosts: &Boosts) -> Option<u32> {
    if !boosts.valid() {
        return None;
    }
    let base = i64::from(member.level(skill)?);
    // Direct library callers can bypass imported-account validation. Permit
    // effective-level reference fixtures, but reject impossible unbounded input.
    if base > 255 {
        return None;
    }
    let mut highest = base;
    let mut drain = 0;
    for id in [
        &boosts.melee,
        &boosts.ranged,
        &boosts.magic,
        &boosts.defence,
        &boosts.encounter,
    ] {
        if let Some(change) = data()
            .boosts
            .iter()
            .find(|b| b.id == *id)
            .and_then(|b| b.stats.get(skill))
        {
            let delta = (base * change.num).div_euclid(change.den) + change.flat;
            if delta < 0 {
                drain += delta;
            } else {
                highest = highest.max(base + delta);
            }
        }
    }
    Some((highest + drain).clamp(0, u32::MAX as i64) as u32)
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Reductions {
    pub accursed: bool,
    pub vulnerability: bool,
    pub elder_maul: u32,
    pub dwh: u32,
    pub arclight: u32,
    pub emberlight: u32,
    pub tonalztic: u32,
    pub seercull: u32,
    pub bgs: u32,
    pub ayak: u32,
}
impl Reductions {
    pub fn valid(&self) -> bool {
        [
            self.elder_maul,
            self.dwh,
            self.arclight,
            self.emberlight,
            self.tonalztic,
        ]
        .iter()
        .all(|n| *n <= 20)
            && self.bgs <= 10_000
            && self.seercull <= 10_000
            && self.ayak <= 10_000
    }
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
    fn amount(&self, id: &str) -> u32 {
        match id {
            "accursed" => u32::from(self.accursed),
            "vulnerability" => u32::from(self.vulnerability && !self.accursed),
            "elder_maul" => self.elder_maul,
            "dwh" => self.dwh,
            "arclight" => self.arclight,
            "emberlight" => self.emberlight,
            "tonalztic" => self.tonalztic,
            "seercull" => self.seercull,
            "bgs" => self.bgs,
            "ayak" => self.ayak,
            _ => 0,
        }
    }
}
/// Source-NPC metadata owns floors; unknown special modes never inherit them.
pub fn reduce(monster: &Monster, reductions: &Reductions) -> Result<Monster, String> {
    if !reductions.valid() {
        return Err("Invalid NPC stat reductions.".into());
    }
    if reductions.is_empty() {
        return Ok(monster.clone());
    }
    if crate::encounters::window(monster.id).is_some_and(|w| !w.outgoing_supported) {
        return Err("Stat reductions are not verified for this special encounter mode.".into());
    }
    let source = crate::encounters::source_npc_id(monster.id);
    let floor_rule = data().floors.iter().find(|f| f.ids.contains(&source));
    if crate::planner::monster(monster.id).is_none()
        || floor_rule.is_none() && crate::encounters::limitation(monster).is_some()
    {
        return Err("Stat reductions are not verified for this encounter.".into());
    }
    let base = monster.skills.clone();
    let base_def = i64::from(
        base.get("def")
            .copied()
            .ok_or("NPC Defence level is unknown.")?,
    );
    let floor = floor_rule
        .map(|f| i64::from(f.floor).max(base_def * f.base_num / f.base_den))
        .unwrap_or(0)
        .min(base_def)
        .max(0) as i32;
    let mut output = monster.clone();
    for rule in &data().reductions {
        let amount = reductions.amount(&rule.id);
        if amount == 0 {
            continue;
        }
        if rule.kind == "bonus_flat" {
            for stat in &rule.stats {
                let before = output.defensive.get(stat).copied().unwrap_or(0);
                if before > 0 {
                    output
                        .defensive
                        .insert(stat.clone(), (before - amount as i32).max(0));
                }
            }
        } else if rule.kind == "flat_level" {
            for stat in &rule.stats {
                let before = output.skills.get(stat).copied().unwrap_or(0);
                output.skills.insert(
                    stat.clone(),
                    (before - amount as i32).max(if stat == "def" { floor } else { 0 }),
                );
            }
        } else if rule.kind == "flat_spill" {
            let mut remaining = amount as i32;
            for stat in &rule.stats {
                if remaining == 0 {
                    break;
                }
                let before = output.skills.get(stat).copied().unwrap_or(0);
                let after = (before - remaining).max(if stat == "def" { floor } else { 0 });
                output.skills.insert(stat.clone(), after);
                // Any positive floor stops BGS from spilling into another stat.
                if after > 0 {
                    remaining = 0;
                } else {
                    remaining = (remaining - before).max(0);
                }
            }
        } else {
            for _ in 0..amount {
                for stat in &rule.stats {
                    let current = i64::from(output.skills.get(stat).copied().unwrap_or(0));
                    let original = i64::from(base.get(stat).copied().unwrap_or(0));
                    let num =
                        if rule.demon_num > 0 && monster.attributes.iter().any(|a| a == "demon") {
                            rule.demon_num
                        } else {
                            rule.num
                        };
                    let after = match rule.kind.as_str() {
                        "keep_current" => current * num / rule.den,
                        "subtract_current" => current - current * num / rule.den,
                        "subtract_base" => current - (original * num / rule.den + rule.flat),
                        "subtract_stat" => {
                            current
                                - i64::from(
                                    output.skills.get(&rule.source_stat).copied().unwrap_or(0),
                                ) * num
                                    / rule.den
                        }
                        _ => return Err("Unknown stat-reduction rule.".into()),
                    };
                    output.skills.insert(
                        stat.clone(),
                        after.max(if stat == "def" { i64::from(floor) } else { 0 }) as i32,
                    );
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use serde_json::{Value, json};
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../assets/combat-modifiers.json")).unwrap()
    }
    #[test]
    fn bundled_metadata_schema_rejects_unknown_nested_fields() {
        for path in [
            "",
            "/boosts/0",
            "/boosts/0/stats/Attack",
            "/floors/0",
            "/reductions/0",
        ] {
            let mut value = fixture();
            value
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("misspelled_field".into(), json!(true));
            assert!(serde_json::from_value::<Data>(value).is_err(), "{path}");
        }
        let mut optional_sources = fixture();
        optional_sources.as_object_mut().unwrap().remove("sources");
        assert!(
            serde_json::from_value::<Data>(optional_sources)
                .unwrap()
                .validated()
                .is_ok()
        );
    }
    #[test]
    fn bundled_metadata_semantics_reject_unsafe_math_and_unknown_selectors() {
        assert!(
            serde_json::from_value::<Data>(fixture())
                .unwrap()
                .validated()
                .is_ok()
        );
        for (path, replacement) in [
            ("/boosts/0/id", json!("None")),
            ("/boosts/0/channel", json!("unknown")),
            ("/boosts/0/stats/Attack/den", json!(0)),
            ("/boosts/0/stats/Attack/num", json!(i64::MAX)),
            ("/boosts/0/stats/Attack/flat", json!(i64::MAX)),
            (
                "/boosts/0/stats",
                json!({"Hitpoints":{"num":1,"den":1,"flat":0}}),
            ),
            ("/floors/0/ids/0", json!(0)),
            ("/floors/0/base_den", json!(0)),
            ("/floors/0/base_num", json!(2)),
            ("/reductions/0/id", json!("accursed_typo")),
            ("/reductions/0/kind", json!("unknown")),
            ("/reductions/0/den", json!(0)),
            ("/reductions/0/num", json!(i64::MAX)),
            ("/reductions/0/stats", json!(["hitpoints"])),
            ("/reductions/0/source_stat", json!("magic")),
            ("/sources/0", json!("not-a-source-url")),
        ] {
            let mut value = fixture();
            if let Some(field) = value.pointer_mut(path) {
                *field = replacement;
            } else {
                let (parent, key) = path.rsplit_once('/').unwrap();
                value
                    .pointer_mut(parent)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert(key.into(), replacement);
            }
            assert!(
                serde_json::from_value::<Data>(value)
                    .unwrap()
                    .validated()
                    .is_err(),
                "{path}"
            );
        }
        for key in ["boosts", "floors", "reductions"] {
            let mut duplicate = fixture();
            let rows = duplicate[key].as_array_mut().unwrap();
            rows.push(rows[0].clone());
            assert!(
                serde_json::from_value::<Data>(duplicate)
                    .unwrap()
                    .validated()
                    .is_err(),
                "duplicate {key}"
            );
        }
        let mut missing = fixture();
        missing["reductions"].as_array_mut().unwrap().pop();
        assert!(
            serde_json::from_value::<Data>(missing)
                .unwrap()
                .validated()
                .is_err()
        );
    }
}
