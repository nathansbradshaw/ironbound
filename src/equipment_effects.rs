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
            notes.push("Atlatl: Ranged accuracy/prayers, Strength level and melee strength bonuses for damage. Full Eclipse Moon finite-HP timing starts empty and simulates buildup, timed pulses, five-stack blocking, expiry and target death. Full-fight phase clearing is an explicit scenario assumption.".into());
        }
        if weapon.has(rules::Effect::Twinflame) {
            notes.push("Twinflame: six-tick standard casts, 10% standard-spell accuracy/damage and a correlated 40% second hit for elemental Bolt/Blast/Wave spells. Strike, Surge and binding spells receive no second hit. Guaranteed-hit/P2 extra-hit transforms remain unavailable.".into());
        }
        if weapon.has(rules::Effect::Obligator) {
            notes.push(format!("Obligator: {}. Charged Thrust uses one accuracy roll, then keeps the best of 2–4 damage rolls; size 2/3+ adds 20%/40% to max hit. Uncharged and Smash receive no passive. Inquisitor/minimum-hit ordering, multiple targets and charge consumption remain unverified.", settings.obligator_style));
        }
        if weapon.has(rules::Effect::Ascension) {
            notes.push("Ascension crossbows: one bolt per attack, two ticks on Rapid and three on Accurate. Ascension bolts only; diamond/onyx proc rates are halved, with the Kandarin multiplier applied. Onyx healing and ammunition recovery are excluded.".into());
        }
        if weapon.weapon.family == "ayak" {
            notes.push("Charged powered weapon: normal attacks only. Verify loaded charges; special attacks, empowered follow-ups and energy rotations are excluded.".into());
        }
        if let Some(special) = &weapon.weapon.special {
            notes.push(format!("Special burst: {}% energy available; one activation costs {}%. Normal gear scores and full-fight simulation exclude specials, recovery and rotations.",settings.special_energy,special.energy));
            if special.kind == rules::SpecialKind::Zorya {
                notes.push(format!("Zorya: {} empowered attacks held. A successful opener grants three two-tick attacks at +150% accuracy and +60% damage. Normal attacks remain three ticks. Verify charges; no pre-stacking or reset assumption is inferred.",settings.zorya_empowered_hits));
            } else {
                notes.push("Crimson kisten: Brutal Swing rolls accuracy four times against crush defence, then makes one damage hit in the matching 70–110% through 130–170% band. Attack bonus and stance use the selected attack style.".into());
            }
        }
        if weapon.weapon.family == "dragon_wand" {
            notes.push("Dragon hunter wand: standard and supported Ice casts; dragonbane applies before elemental weakness. Spell rune costs are packed normally; no free runes are inferred.".into());
        }
    }
    if settings.rotation.enabled {
        let cfg = &settings.rotation;
        notes.push(format!("Rotation: {} ticks, {} seeded trials, {}. Seed {}. Initial recovery pulse: {}. Natural recovery restores 10% every {} ticks; full energy pauses recovery. No incoming damage or phase changes.",cfg.window_ticks,cfg.trials,cfg.policy.label(),cfg.seed,if cfg.next_pulse==0 {"fresh interval".into()} else {format!("{} ticks",cfg.next_pulse)},crate::rotation::interval(crate::rotation::lightbearer(set))));
        if let Some(id) = cfg.swap_weapon {
            notes.push(format!("Rotation comparison swap: {} ({}); {} extra ticks per setup change after the prior cooldown. A two-handed swap removes the shield; returning restores the original setup. Finish held Zorya follow-ups before switching away. Carry the swap weapon and displaced-item space separately; this comparison does not add them to the exported bag.",crate::planner::gear(id).map(|g|g.name.as_str()).unwrap_or("Unavailable swap"),if cfg.swap_style.is_empty(){"weapon default"}else{&cfg.swap_style},cfg.swap_delay));
        }
        if let Some(t) = cfg.surge_tick {
            notes.push(format!("Rotation Surge: one owned dose at tick {t}, ready at tick {}. Restores up to 25%, 500-tick reuse cooldown; full-energy attempts consume no dose. Carry this dose separately; no potion is added to the exported bag.",cfg.surge_ready));
        }
        if let Some(t) = cfg.transfer_tick {
            notes.push(format!("Rotation Energy Transfer: one confirmed cooperative arrival at tick {t}, restoring up to 100%. Caster energy/HP costs, travel and availability are not simulated."));
        }
        if let Some(t) = cfg.death_charge {
            notes.push(format!("Rotation Death Charge: confirmed already active for {t} more ticks; one eligible target kill before expiry restores up to 15%. Verify unlocks, runes, range and last hit. No add kills or automatic recasts."));
        }
    }
    if rules::equipped_effect(set, "charged_quiver") {
        notes.push("Charged Dizana quiver: +10 Ranged accuracy and +1 Ranged strength only while firing compatible arrows/bolts. Self-ammunition bows, blowpipes, atlatls and bolt racks receive no charged passive. Verify sunfire charges.".into());
    }
    if set.get(3) == Some(&Some(28988)) {
        notes.push(format!("Blue Moon spear: elemental, binding, Ice and Grasp spells, plus melee. Free melee: {} (requires the in-game automatic melee option and melee range).",settings.blue_moon_melee));
    }
    if rules::equipped_effect(set, "bulwark") {
        notes.push(format!("Dinh's Block: {}. Active Block prevents attacks. Only settled ordinary unprotected incoming hits are supported; activation delay and offensive defence scaling are excluded.", if settings.bulwark_block { "active" } else { "inactive" }));
    }
    if crate::rotation::lightbearer(set) {
        notes.push("Lightbearer: 10% energy every 25 active ticks in the timed rotation model. It pauses at full energy; normal gear scores and full-fight survival simulations still exclude special rotations.".into());
    }
    if set
        .get(13)
        .copied()
        .flatten()
        .is_some_and(|id| rules::ammunition(id) == rules::AmmoEffect::Onyx)
    {
        notes.push("Onyx: direct proc damage only; healing and sustain are not simulated.".into());
    }
    if set
        .get(13)
        .copied()
        .flatten()
        .is_some_and(|id| rules::ammunition(id) == rules::AmmoEffect::Seeking)
    {
        notes.push("Seeking arrows: successful damage rolls below three are raised to three before NPC reductions. Misses remain misses; no damage reroll or uniform minimum-hit distribution is implied.".into());
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
