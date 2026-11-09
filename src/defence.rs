//! Explicit attack scenarios; unknown prayer/special interactions never become zero damage.
use crate::{account::Member, combat, planner, planner::Monster};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub protection: String,
    // Retained for old backups/shared setups; automatic ranking uses Options::ranking.
    pub safety: u32,
    pub offensive: String,
    pub preserve: bool,
    pub prayer_weight: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            protection: "None".into(),
            safety: 1,
            offensive: "None".into(),
            preserve: false,
            prayer_weight: 0,
        }
    }
}
impl Settings {
    pub fn valid(&self) -> bool {
        self.safety <= 3
            && self.prayer_weight <= 3
            && crate::prayer::prayers()
                .iter()
                .any(|p| p.name == self.offensive)
            && ["None", "Melee", "Ranged", "Magic"].contains(&self.protection.as_str())
    }
}
#[derive(Debug, Deserialize)]
pub struct Hit {
    pub max: i32,
    #[serde(default)]
    pub heal_multiplier: i32,
    #[serde(default)]
    pub heal_floor: i32,
    #[serde(default = "cap")]
    pub heal_cap: i32,
}
fn cap() -> i32 {
    i32::MAX
}
#[derive(Debug, Deserialize)]
pub struct Attack {
    pub name: String,
    pub defence: String,
    pub max: i32,
    #[serde(default)]
    pub min: i32,
    pub prayer_through: f64,
    #[serde(default)]
    pub interval: i32,
    #[serde(default)]
    pub healing_fraction: f64,
}
#[derive(Debug, Deserialize)]
pub struct Profile {
    pub ids: Vec<i32>,
    pub kind: String,
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub healing_modelled: bool,
    #[serde(default)]
    pub defence: String,
    #[serde(default)]
    pub interval: i32,
    pub prayer_through: f64,
    #[serde(default)]
    pub hits: Vec<Hit>,
    #[serde(default)]
    pub attacks: Vec<Attack>,
    pub note: String,
}
#[derive(Debug, Deserialize)]
struct GearGuard {
    names: Vec<String>,
    all: bool,
    note: String,
}
#[derive(Debug, Deserialize)]
struct Registry {
    profiles: Vec<Profile>,
    gear_guards: Vec<GearGuard>,
}
fn registry() -> &'static Registry {
    static DATA: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/incoming-rules.json")).expect("incoming rules")
    })
}
pub fn profiles() -> &'static [Profile] {
    &registry().profiles
}
pub fn profile(id: i32) -> Option<&'static Profile> {
    profiles().iter().find(|p| p.ids.contains(&id))
}
/// Verified profiles may expose explicit phase/scenario labels instead of ambiguous NPC styles.
pub fn attacks(target: &Monster) -> Vec<String> {
    match profile(target.id) {
        Some(p) if p.kind == "chained" => vec!["Chained slash".into()],
        Some(p) if !p.attacks.is_empty() => p.attacks.iter().map(|a| a.name.clone()).collect(),
        _ => target.attack_styles.clone(),
    }
}
#[derive(Debug)]
pub struct Estimate {
    pub attack: String,
    pub hit_chance: f64,
    pub damage_per_second: f64,
    pub healing_per_second: f64,
    pub healing_modelled: bool,
    pub note: String,
}
/// A numeric max hit is shared by the listed basic styles. An annotated maximum
/// is accepted only for its stated style, never silently reused for another one.
fn recorded_max(target: &Monster, kind: &str) -> Result<i32, String> {
    let raw = target.max_hit.trim();
    if let Ok(value) = raw.parse::<i32>() {
        return Ok(value);
    }
    if let Some((number, annotation)) = raw.split_once('(') {
        let annotation = annotation.trim_end_matches(')').trim().to_ascii_lowercase();
        let melee = ["stab", "slash", "crush"].contains(&kind);
        if annotation == kind || annotation == "melee" && melee {
            return number
                .trim()
                .parse::<i32>()
                .map_err(|_| "Recorded max-hit annotation is unsupported.".into());
        }
    }
    Err("Per-attack max hit is unverified; a mechanic profile is required.".into())
}
pub fn estimate(
    member: &Member,
    set: &[Option<i32>],
    target: &Monster,
    attack: &str,
    settings: &Settings,
) -> Result<Estimate, String> {
    crate::prayer::eligible_settings(member, settings)?;
    if set.len() != 14 {
        return Err("Equipment needs 14 slots.".into());
    }
    for guard in &registry().gear_guards {
        let present = |name: &String| {
            set.iter()
                .flatten()
                .any(|id| planner::gear(*id).is_some_and(|g| g.name == *name))
        };
        let matches = if guard.all {
            guard.names.iter().all(present)
        } else {
            guard.names.iter().any(present)
        };
        if matches {
            return Err(guard.note.clone());
        }
    }
    let profile = profile(target.id);
    if profile.is_some_and(|p| p.kind == "unverified") {
        return Err(profile.unwrap().note.clone());
    }
    let chained = profile.is_some_and(|p| p.kind == "chained");
    let scenario = profile.and_then(|p| {
        p.attacks
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(attack))
    });
    if !chained
        && scenario.is_none()
        && !target
            .attack_styles
            .iter()
            .any(|s| s.eq_ignore_ascii_case(attack))
    {
        return Err("Choose one of this target's recorded basic attack scenarios.".into());
    }
    let kind = if chained {
        profile.unwrap().defence.as_str()
    } else {
        scenario.map(|a| a.defence.as_str()).unwrap_or(attack)
    };
    let kind = kind.to_ascii_lowercase();
    let basic = match kind.as_str() {
        "stab" => "stab",
        "slash" => "slash",
        "crush" => "crush",
        "magic" => "magic",
        "ranged" => "ranged",
        "melee" => return Err(
            "The NPC's melee defence type is unspecified; stab, slash or crush must be verified."
                .into(),
        ),
        _ => return Err("This incoming attack needs a separate mechanic model.".into()),
    };
    let bonus = combat::bonuses(set);
    let get = |map: &std::collections::BTreeMap<String, i32>, key: &str| {
        map.get(key).copied().unwrap_or(0) as i64
    };
    let prayer = crate::prayer::selected(settings);
    let defence = member.level("Defence").ok_or("Enter your Defence level.")? as i64;
    let defence = defence * (100 + prayer.defence) / 100;
    let effective = if basic == "magic" {
        let magic = member.level("Magic").ok_or("Enter your Magic level.")? as i64;
        let magic = magic * (100 + prayer.magic_defence) / 100;
        7 * magic / 10 + 3 * defence / 10 + 8
    } else {
        defence + 8
    };
    let player_roll = effective * (get(&bonus.defensive, basic) + 64);
    let npc_level = get(
        &target.skills,
        if basic == "magic" {
            "magic"
        } else if basic == "ranged" {
            "ranged"
        } else {
            "atk"
        },
    );
    let npc_roll = (npc_level + 9)
        * (get(
            &target.offensive,
            if basic == "magic" {
                "magic"
            } else if basic == "ranged" {
                "ranged"
            } else {
                "atk"
            },
        ) + 64);
    let chance = combat::accuracy(npc_roll, player_roll);
    let protection = if ["stab", "slash", "crush"].contains(&basic) {
        "Melee"
    } else if basic == "magic" {
        "Magic"
    } else {
        "Ranged"
    };
    let through = if settings.protection == protection {
        scenario.map(|a| a.prayer_through).or_else(|| profile.map(|p| p.prayer_through))
            .ok_or("Protection-prayer interaction is unverified for this encounter; no protected damage estimate is available.")?
    } else {
        1.0
    };
    let ticks = scenario
        .filter(|a| a.interval > 0)
        .map(|a| a.interval)
        .or_else(|| profile.filter(|p| p.interval > 0).map(|p| p.interval))
        .unwrap_or(target.speed);
    if ticks <= 0 {
        return Err("Incoming attack interval unavailable.".into());
    }
    let mut damage = 0.0;
    let mut healing = 0.0;
    if chained {
        for (index, hit) in profile.unwrap().hits.iter().enumerate() {
            let land = chance.powi(index as i32 + 1);
            damage += land * hit.max as f64 / 2.0 * through;
            if hit.heal_multiplier > 0 {
                let mean = (0..=hit.max)
                    .map(|d| (d * hit.heal_multiplier).clamp(hit.heal_floor, hit.heal_cap) as f64)
                    .sum::<f64>()
                    / (hit.max + 1) as f64;
                healing += land * mean;
            }
        }
    } else {
        let max = match scenario {
            Some(a) => a.max,
            None => recorded_max(target, basic)?,
        };
        let min = scenario.map(|a| a.min).unwrap_or(0);
        if min < 0 || max < min || max > 10_000 {
            return Err("Incoming damage distribution is invalid.".into());
        }
        // Prayer reduction is applied to each possible hit before averaging.
        let mean = (min..=max)
            .map(|hit| (hit as f64 * through).floor())
            .sum::<f64>()
            / (max - min + 1) as f64;
        damage = chance * mean;
        healing = damage * scenario.map(|a| a.healing_fraction).unwrap_or(0.0);
    }
    let note = profile.map(|p| p.note.clone()).unwrap_or("Unprotected basic-attack estimate only. Attack choice frequencies, chip damage, protection interactions, scaling and special mechanics are unverified.".into());
    Ok(Estimate {
        attack: if chained {
            "Chained slash".into()
        } else {
            attack.into()
        },
        hit_chance: chance,
        damage_per_second: damage / (ticks as f64 * 0.6),
        healing_per_second: healing / (ticks as f64 * 0.6),
        healing_modelled: profile.is_some_and(|p| p.healing_modelled),
        note: format!(
            "{note} Uses unboosted skills, the selected prayer and +0 defensive-stance bonus. This is one attack scenario, not full-fight incoming DPS."
        ),
    })
}
pub fn score(
    member: &Member,
    set: &[Option<i32>],
    target: &Monster,
    outgoing: f64,
    settings: &Settings,
) -> f64 {
    let drain = crate::prayer::drain_per_minute(crate::prayer::bonus(set), settings);
    // Explicit utility: 0 ignores drain; 1–3 increasingly value prayer economy.
    let prayer_cost = 1.0 + settings.prayer_weight as f64 * drain / 60.0;
    // Mixed attack frequencies must be known before a partial attack scenario can
    // influence automatic defence ranking. Moon chains have a verified interval.
    if !profile(target.id).is_some_and(|p| p.automatic) {
        return outgoing / prayer_cost;
    }
    let attack = attacks(target).into_iter().next().unwrap_or_default();
    match estimate(member, set, target, &attack, settings) {
        Ok(result) => {
            (outgoing - result.healing_per_second).max(0.001)
                / (1.0 + settings.safety as f64 * result.damage_per_second * 0.09)
                / prayer_cost
        }
        // Unsupported incoming passives cannot win by escaping the damage/heal
        // penalty that is correctly applied to fully modelled alternatives.
        Err(_) => 0.001 / prayer_cost,
    }
}
