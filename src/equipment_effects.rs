//! Independently implemented equipment arithmetic. Reference revision and limits
//! are recorded in docs/equipment-backlog.md; no reference engine is embedded.
use crate::{defence::Settings, rules};

pub fn notes(set: &[Option<i32>], settings: &Settings) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(weapon) = set.get(3).copied().flatten().and_then(rules::weapon) {
        if weapon.has(rules::Effect::Soulreaper) {
            notes.push(format!("Soulreaper: {} held stacks. Build-up, stack loss, self-damage, healing and the special attack are excluded.", settings.soulreaper_stacks));
        }
        if weapon.has(rules::Effect::Scythe) {
            notes.push("Scythe: size-based independent hits; verify charges. Movement, multi-target swings and special encounter transforms remain partial.".into());
        }
        if weapon.has(rules::Effect::Macuahuitl) {
            notes.push("Dual macuahuitl: the second hit requires the first accuracy roll to succeed. Full Blood Moon uses expected sustained attack timing; specials are excluded.".into());
        }
        if weapon.has(rules::Effect::StrengthRanged) {
            notes.push("Atlatl: Ranged accuracy/prayers, Strength level and melee strength bonuses for damage. Full Eclipse Moon uses delayed sustained burn DPS with five-stack limits; finite-HP burn timing remains unavailable.".into());
        }
    }
    if set.get(3) == Some(&Some(28988)) {
        notes.push(format!("Blue Moon spear: elemental, binding, Ice and Grasp spells, plus melee. Free melee: {} (requires the in-game automatic melee option and melee range).",settings.blue_moon_melee));
    }
    if rules::equipped_effect(set, "bulwark") {
        notes.push(format!("Dinh's Block: {}. Active Block prevents attacks. Only settled ordinary unprotected incoming hits are supported; activation delay and offensive defence scaling are excluded.", if settings.bulwark_block { "active" } else { "inactive" }));
    }
    if set
        .get(12)
        .copied()
        .flatten()
        .is_some_and(|id| crate::planner::gear(id).is_some_and(|g| g.name == "Lightbearer"))
    {
        notes.push("Lightbearer energy recovery and special-attack rotations are not included in gear scores.".into());
    }
    if set
        .get(13)
        .copied()
        .flatten()
        .is_some_and(|id| rules::ammunition(id) == rules::AmmoEffect::Onyx)
    {
        notes.push("Onyx: direct proc damage only; healing and sustain are not simulated.".into());
    }
    notes
}

pub fn has_defensive_passive(set: &[Option<i32>]) -> bool {
    ["elysian", "bulwark", "rondache"]
        .iter()
        .any(|e| rules::equipped_effect(set, e))
        || rules::set_effect(set, "justiciar")
}

/// Expected damage from one accurate, unprotected ordinary hit. Every proc
/// branch is floored separately before averaging; zero never becomes negative.
pub fn incoming_hit(hit: i32, defence_bonus: i32, set: &[Option<i32>], settings: &Settings) -> f64 {
    let transform = |mut damage: i32| {
        if settings.bulwark_block && rules::equipped_effect(set, "bulwark") {
            damage = damage * 4 / 5;
        }
        if rules::set_effect(set, "justiciar") {
            damage = crate::defence::justiciar_hit(damage, i64::from(defence_bonus));
        }
        if rules::equipped_effect(set, "rondache") {
            damage = (damage - 2).max(0);
        }
        f64::from(damage)
    };
    if rules::equipped_effect(set, "elysian") {
        0.3 * transform(hit) + 0.7 * transform((hit - (hit / 4).max(1)).max(0))
    } else {
        transform(hit)
    }
}

/// Independent splats form one attack. Compact equal totals after every
/// convolution so finite-HP calculation remains bounded for multi-hit weapons.
pub fn combine_hits(parts: &[Vec<(i32, f64)>]) -> Vec<(i32, f64)> {
    let mut totals = vec![(0, 1.0)];
    for part in parts {
        let mut next = std::collections::BTreeMap::new();
        for (left, p) in &totals {
            for (right, q) in part {
                *next.entry(left + right).or_insert(0.0) += p * q;
            }
        }
        totals = next.into_iter().collect();
    }
    totals
}

pub fn target_size(id: i32) -> Option<u32> {
    static SIZES: std::sync::OnceLock<std::collections::BTreeMap<i32, u32>> =
        std::sync::OnceLock::new();
    SIZES
        .get_or_init(|| {
            serde_json::from_str(include_str!("../assets/monster-sizes.json"))
                .expect("monster sizes")
        })
        .get(&crate::encounters::source_npc_id(id))
        .copied()
}
