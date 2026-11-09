//! Locally computed standard sustained attacks. Unsupported effects return an explanation.
use crate::{
    account::Member,
    planner::Monster,
    rules::{self, AmmoEffect, Effect, Formula},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EquipmentStats {
    pub id: i32,
    #[serde(default)]
    pub speed: i32,
    pub category: String,
    pub bonuses: BTreeMap<String, i32>,
    pub offensive: BTreeMap<String, i32>,
    pub defensive: BTreeMap<String, i32>,
}
pub fn equipment_stats() -> &'static [EquipmentStats] {
    static DATA: std::sync::OnceLock<Vec<EquipmentStats>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/combat-equipment.json"))
            .expect("combat equipment")
    })
}
pub fn stats(id: i32) -> Option<&'static EquipmentStats> {
    static INDEX: std::sync::OnceLock<BTreeMap<i32, usize>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let mut map = BTreeMap::new();
        for (i, s) in equipment_stats().iter().enumerate() {
            map.entry(s.id).or_insert(i);
        }
        map
    });
    index.get(&id).map(|i| &equipment_stats()[*i])
}
#[derive(Clone, Debug, Default)]
pub struct Bonuses {
    pub offensive: BTreeMap<String, i32>,
    pub defensive: BTreeMap<String, i32>,
    pub other: BTreeMap<String, i32>,
}
pub fn bonuses(set: &[Option<i32>]) -> Bonuses {
    bonuses_in_context(set, false)
}
fn bonuses_in_context(set: &[Option<i32>], toa: bool) -> Bonuses {
    let mut result = Bonuses::default();
    for id in set.iter().flatten() {
        if let Some(g) = stats(*id) {
            for (map, dest) in [
                (&g.offensive, &mut result.offensive),
                (&g.defensive, &mut result.defensive),
                (&g.bonuses, &mut result.other),
            ] {
                for (k, v) in map {
                    *dest.entry(k.clone()).or_default() += v;
                }
            }
            if let Some(rule) = rules::weapon(*id) {
                for adjustment in &rule.weapon.adjustments {
                    if (adjustment.context == "outside_toa" && !toa)
                        || (adjustment.context == "toa" && toa)
                    {
                        for (key, amount) in &adjustment.offensive {
                            *result.offensive.entry(key.clone()).or_default() += amount;
                        }
                        for (key, amount) in &adjustment.bonuses {
                            *result.other.entry(key.clone()).or_default() += amount;
                        }
                    }
                }
            }
        }
    }
    result
}
#[derive(Clone, Debug)]
pub struct ResultStats {
    pub dps: f64,
    pub accuracy: f64,
    pub max_hit: i32,
    pub attack_ticks: i32,
    pub attack_roll: i64,
    pub defence_roll: i64,
    pub attack_type: String,
}
/// Normal accuracy includes negative attack/defence rolls.
pub fn accuracy(mut a: i64, mut d: i64) -> f64 {
    if a < 0 {
        a = (a + 2).min(0);
    }
    if d < 0 {
        d = (d + 2).min(0);
    }
    let normal = |a: i64, d: i64| {
        if a > d {
            1.0 - (d + 2) as f64 / (2.0 * (a + 1) as f64)
        } else {
            a as f64 / (2.0 * (d + 1) as f64)
        }
    };
    match (a >= 0, d >= 0) {
        (true, true) => normal(a, d),
        (true, false) => 1.0 - 1.0 / (-d + 1) as f64 / (a + 1) as f64,
        (false, true) => 0.0,
        (false, false) => normal(-d, -a),
    }
    .clamp(0.0, 1.0)
}
pub fn calculate(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
) -> Result<ResultStats, String> {
    calculate_at_invocation(member, set, style, monster, spell, 0)
}
pub fn calculate_at_invocation(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
    invocation: u32,
) -> Result<ResultStats, String> {
    calculate_configured(
        member,
        set,
        style,
        monster,
        spell,
        invocation,
        &Default::default(),
    )
}
pub fn calculate_configured(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
    invocation: u32,
    settings: &crate::defence::Settings,
) -> Result<ResultStats, String> {
    if !settings.valid() {
        return Err("Invalid combat settings.".into());
    }
    let prayer = crate::prayer::selected(settings);
    crate::prayer::eligible_settings(member, settings)?;
    if set.len() != 14 {
        return Err("Equipment needs 14 slots.".into());
    }
    let weapon = set[3].ok_or("Choose a weapon.")?;
    let stats = stats(weapon).ok_or("Weapon stats unavailable.")?;
    let rule =
        rules::weapon(weapon).ok_or("This weapon's combat mechanics are not modelled yet.")?;
    if !rule.permits(style) {
        return Err("This weapon does not support the selected attack style.".into());
    }
    let requested_style = style;
    let style = rules::base_style(style);
    let prayer_attack = if prayer.style == style {
        prayer.attack
    } else {
        0
    };
    let prayer_strength = if prayer.style == style {
        prayer.strength
    } else {
        0
    };
    if let Some(reason) = set
        .iter()
        .flatten()
        .find_map(|id| rules::unsupported_equipment(*id))
    {
        return Err(reason.into());
    }
    if monster.skills.is_empty() {
        return Err("Monster combat stats unavailable.".into());
    }
    let flat_armour = monster
        .defensive
        .get(if style == "Magic" {
            "magic_resistance"
        } else {
            "flat_armour"
        })
        .copied()
        .unwrap_or(0);
    if flat_armour != 0 && style == "Ranged" {
        return Err("This combination of flat armour and weapon effects needs a separate hit-distribution model.".into());
    }
    if let Some(reason) = crate::encounters::limitation(monster) {
        return Err(reason.into());
    }
    let toa = crate::encounters::mechanics().toa_ids.contains(&monster.id);
    let mut b = bonuses_in_context(set, toa);
    if style == "Magic" && rule.has(Effect::Shadow) {
        let factor = if toa { 4 } else { 3 };
        *b.offensive.entry("magic".into()).or_default() *= factor;
        let magic = b.other.entry("magic_str".into()).or_default();
        *magic = (*magic * factor).min(1000);
    }
    if style == "Magic" {
        *b.other.entry("magic_str".into()).or_default() += if prayer.style == style {
            prayer.magic_damage as i32 * 10
        } else {
            0
        };
    }
    let get = |m: &BTreeMap<String, i32>, k: &str| m.get(k).copied().unwrap_or(0);
    let level = |s: &str| {
        member
            .level(s)
            .map(|v| v as i64)
            .ok_or_else(|| format!("Enter your {s} level."))
    };
    let (attack_type, attack, max_hit, ticks, def_level, def_bonus) = match style {
        "Melee" => {
            let kind = if requested_style == "Melee" {
                rule.family.attack_types[0].as_str()
            } else {
                rule.family
                    .attack_types
                    .iter()
                    .find(|t| t.eq_ignore_ascii_case(requested_style))
                    .unwrap()
                    .as_str()
            };
            let stance = rule.stance(kind);
            let a = (level("Attack")? * (100 + prayer_attack) / 100 + 8 + stance.attack_bonus())
                * (get(&b.offensive, kind) as i64 + 64);
            let max = ((level("Strength")? * (100 + prayer_strength) / 100
                + 8
                + stance.strength_bonus())
                * (get(&b.other, "str") as i64 + 64)
                + 320)
                / 640;
            (
                kind,
                a,
                max.max(0) as i32,
                stats.speed.max(1),
                get(&monster.skills, "def"),
                get(&monster.defensive, kind),
            )
        }
        "Ranged" => {
            let ammo = set[13].ok_or("Choose compatible ammunition.")?;
            if !rule.family.ammo.contains(&ammo) {
                return Err("Weapon/ammunition combination not supported.".into());
            }
            let effective = level("Ranged")? * (100 + prayer_attack) / 100 + 8;
            let a = effective * (get(&b.offensive, "ranged") as i64 + 64);
            let strength = level("Ranged")? * (100 + prayer_strength) / 100 + 8;
            let max = (strength * (get(&b.other, "ranged_str") as i64 + 64) + 320) / 640;
            (
                "ranged",
                a,
                max.max(0) as i32,
                (stats.speed - i32::from(rule.family.rapid)).max(1),
                get(&monster.skills, "def"),
                get(&monster.defensive, &rule.family.attack_types[0]),
            )
        }
        "Magic" => {
            let mage = level("Magic")?;
            let base = match rule.family.formula {
                Formula::Wind => match spell {
                    "Wind Wave" => {
                        if mage >= 75 {
                            20
                        } else if mage >= 70 {
                            19
                        } else if mage >= 65 {
                            18
                        } else if mage >= 62 {
                            17
                        } else {
                            return Err("Wind Wave requires 62 Magic.".into());
                        }
                    }
                    "Wind Blast" => {
                        if mage >= 59 {
                            16
                        } else if mage >= 53 {
                            15
                        } else if mage >= 47 {
                            14
                        } else if mage >= 41 {
                            13
                        } else {
                            return Err("Wind Blast requires 41 Magic.".into());
                        }
                    }
                    _ => return Err("Choose Wind Blast or Wind Wave.".into()),
                },
                Formula::Powered => if rule.family.offset_before_division {
                    (rule.family.numerator * mage + rule.family.offset) / rule.family.denominator
                } else {
                    rule.family.numerator * mage / rule.family.denominator + rule.family.offset
                }
                .max(1),
                Formula::Fixed => rule.family.offset,
                _ => return Err("Spell or powered staff not supported.".into()),
            };
            let mut max = base + base * get(&b.other, "magic_str") as i64 / 1000;
            let mut a = (mage * (100 + prayer_attack) / 100 + rule.family.accuracy_bonus)
                * (get(&b.offensive, "magic") as i64 + 64);
            if rule.family.formula == Formula::Wind {
                if let Some(w) = &monster.weakness {
                    if w.element == "air" {
                        a += a * w.severity as i64 / 100;
                        max += base * w.severity as i64 / 100;
                    }
                }
            }
            (
                "magic",
                a,
                max.max(0) as i32,
                rule.family.ticks.unwrap_or(stats.speed).max(1),
                get(&monster.skills, "magic"),
                get(&monster.defensive, "magic"),
            )
        }
        _ => return Err("Select one attack style.".into()),
    };
    let mut defence = (def_level as i64 + 9) * (def_bonus as i64 + 64);
    if toa {
        defence = defence * (250 + invocation.min(150) as i64) / 250;
    }
    let mut attack = attack;
    let mut max_hit = max_hit;
    let keris =
        rule.has(Effect::KalphiteDamage) && monster.attributes.iter().any(|a| a == "kalphite");
    if keris {
        max_hit = max_hit * rule.weapon.kalphite_damage_percent.unwrap_or(133) / 100;
        if rule.has(Effect::KalphiteAccuracy) {
            attack = attack * 133 / 100;
        }
    }
    for multiplier in &rule.weapon.multipliers {
        if monster.attributes.contains(&multiplier.attribute) {
            if multiplier.accuracy {
                attack = attack * multiplier.numerator / multiplier.denominator;
            }
            if multiplier.damage {
                max_hit = (max_hit as i64 * multiplier.numerator / multiplier.denominator) as i32;
            }
        }
    }
    let mut hit = accuracy(attack, defence);
    if style == "Magic"
        && set
            .iter()
            .flatten()
            .any(|id| rules::equipment_effect(*id, "brimstone_accuracy"))
    {
        hit = 0.75 * hit + 0.25 * accuracy(attack, defence * 9 / 10);
    }
    let fang = rule.has(Effect::Fang);
    if fang && attack_type == "stab" {
        hit = if toa {
            1.0 - (1.0 - hit).powi(2)
        } else {
            fang_accuracy(attack, defence)
        };
    }
    let min_hit = if fang { max_hit * 3 / 20 } else { 0 };
    if fang {
        max_hit -= min_hit;
    }
    let (mean, ceiling) =
        conditional_hit(min_hit, max_hit, flat_armour, &rule.family.procs, monster);
    let mut expected = hit * mean;
    max_hit = ceiling;
    if style == "Ranged" {
        let zcb = rule.has(Effect::Zaryte);
        match rules::ammunition(set[13].unwrap_or(0)) {
            AmmoEffect::Ruby => {
                let mut hp = monster.skills.get("hp").copied().unwrap_or(1);
                if toa {
                    hp += hp * (4 * invocation.min(150) as i32 / 10) / 100;
                    if hp > 100 {
                        let round = if hp > 300 { 10 } else { 5 };
                        hp = (hp + round / 2) / round * round;
                    }
                }
                let ruby = (hp * if zcb { 22 } else { 20 } / 100).min(if zcb { 110 } else { 100 });
                expected = expected * 0.94 + ruby as f64 * 0.06;
                max_hit = max_hit.max(ruby);
            }
            AmmoEffect::Diamond => {
                let diamond = max_hit * if zcb { 126 } else { 115 } / 100;
                expected = expected * 0.9 + diamond as f64 / 2.0 * 0.1;
                max_hit = max_hit.max(diamond);
            }
            _ => {}
        }
    }
    Ok(ResultStats {
        dps: expected / (ticks as f64 * 0.6),
        accuracy: hit,
        max_hit,
        attack_ticks: ticks,
        attack_roll: attack,
        defence_roll: defence,
        attack_type: attack_type.into(),
    })
}
/// Shared single-hit distribution: composable procs, accurate-zero handling, then NPC armour.
fn conditional_hit(
    min: i32,
    max: i32,
    armour: i32,
    procs: &[rules::HitProc],
    monster: &Monster,
) -> (f64, i32) {
    if max <= 0 {
        return (0.0, 0);
    }
    let mut hits: Vec<_> = (min..=max)
        .map(|damage| (damage, 1.0 / (max - min + 1) as f64))
        .collect();
    for before in [true, false] {
        if !before {
            for (damage, _) in &mut hits {
                *damage = (*damage).max(1);
            }
        }
        for proc in procs.iter().filter(|p| {
            p.before_accurate_zero == before
                && p.attribute
                    .as_ref()
                    .is_none_or(|a| monster.attributes.contains(a))
        }) {
            let chance = proc.numerator as f64 / proc.denominator as f64;
            hits = hits
                .into_iter()
                .flat_map(|(damage, probability)| {
                    let changed = match proc.operation {
                        rules::ProcOperation::Add => damage + proc.amount,
                        rules::ProcOperation::Multiply => damage * proc.amount,
                    };
                    [
                        (damage, probability * (1.0 - chance)),
                        (changed, probability * chance),
                    ]
                })
                .collect();
        }
    }
    let mut mean = 0.0;
    let mut ceiling = 0;
    for (damage, probability) in hits {
        let damage = (damage - armour).max(0);
        mean += damage as f64 * probability;
        if probability > 0.0 {
            ceiling = ceiling.max(damage);
        }
    }
    (mean, ceiling)
}
pub fn fang_accuracy(a: i64, d: i64) -> f64 {
    if a < 0 || d < 0 {
        return accuracy(a, d);
    }
    let (a, d) = (a as f64, d as f64);
    (if a > d {
        1.0 - (d + 2.0) * (2.0 * d + 3.0) / (6.0 * (a + 1.0) * (a + 1.0))
    } else {
        a * (4.0 * a + 5.0) / (6.0 * (a + 1.0) * (d + 1.0))
    })
    .clamp(0.0, 1.0)
}
