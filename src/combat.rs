//! Locally computed standard sustained attacks. Unsupported effects return an explanation.
use crate::{account::Member, planner::Monster};
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
    if set.len() != 14 {
        return Err("Equipment needs 14 slots.".into());
    }
    let weapon = set[3].ok_or("Choose a weapon.")?;
    let stats = stats(weapon).ok_or("Weapon stats unavailable.")?;
    if set
        .iter()
        .flatten()
        .any(|id| matches!(*id, 25264 | 26770 | 11665 | 11664 | 11663))
    {
        return Err("Special rings and Void effects are not implemented yet.".into());
    }
    if monster.skills.is_empty() {
        return Err("Monster combat stats unavailable.".into());
    }
    if monster.defensive.get("flat_armour").copied().unwrap_or(0) != 0 {
        return Err("Flat armour is not implemented yet.".into());
    }
    if monster.name.contains("Warden") {
        return Err("Warden phases need separate accuracy and damage models.".into());
    }
    let toa = [
        11778, 11779, 11780, 11719, 11721, 11730, 11732, 11733, 11789, 11790, 11791, 11792, 11793,
        11794, 11795, 11796,
    ]
    .contains(&monster.id);
    if monster
        .attributes
        .iter()
        .any(|a| a.starts_with("vampyre") || a == "leafy" || a == "flying")
        || [
            "Zulrah",
            "Kraken",
            "Cave kraken",
            "Fragment of Seren",
            "Tormented Demon",
            "Slagilith",
            "Ice demon",
            "Tekton",
            "Corporeal Beast",
        ]
        .iter()
        .any(|n| monster.name.starts_with(n))
    {
        return Err("This monster needs a special damage or immunity model.".into());
    }
    let mut b = bonuses(set);
    if style == "Magic" && weapon == 27275 {
        let factor = if toa { 4 } else { 3 };
        *b.offensive.entry("magic".into()).or_default() *= factor;
        let magic = b.other.entry("magic_str".into()).or_default();
        *magic = (*magic * factor).min(1000);
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
            if ![
                4151, 12006, 26219, 27246, 11889, 21009, 25979, 25981, 27291, 27287, 1215, 4587,
            ]
            .contains(&weapon)
            {
                return Err("This melee weapon is not implemented.".into());
            }
            let kind = if [4151, 12006, 4587].contains(&weapon) {
                "slash"
            } else {
                "stab"
            };
            let a = (level("Attack")? + 11) * (get(&b.offensive, kind) as i64 + 64);
            let max = ((level("Strength")? + 8) * (get(&b.other, "str") as i64 + 64) + 320) / 640;
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
            let bow = [861, 12788].contains(&weapon);
            if !(if bow {
                [890, 892].contains(&ammo)
            } else {
                [9185, 26486, 21902, 11785, 26374].contains(&weapon)
                    && [9142, 9143, 9144, 9242, 9243].contains(&ammo)
            }) {
                return Err("Weapon/ammunition combination not supported.".into());
            }
            let effective = level("Ranged")? + 8;
            let a = effective * (get(&b.offensive, "ranged") as i64 + 64);
            let max = (effective * (get(&b.other, "ranged_str") as i64 + 64) + 320) / 640;
            (
                "ranged",
                a,
                max.max(0) as i32,
                (stats.speed - 1).max(1),
                get(&monster.skills, "def"),
                get(&monster.defensive, if bow { "standard" } else { "heavy" }),
            )
        }
        "Magic" => {
            let mage = level("Magic")?;
            let base = match weapon {
                1381 | 1405 => match spell {
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
                27275 => mage / 3 + 1,
                22323 | 25731 => (mage / 3).max(1),
                11905 | 11907 | 22288 => (mage / 3 - 5).max(1),
                12899 | 22292 => (mage / 3 - 2).max(1),
                28585 => (8 * mage + 96) / 37,
                12658 | 1409 => 25,
                _ => return Err("Spell or powered staff not supported.".into()),
            };
            let mut max = base + base * get(&b.other, "magic_str") as i64 / 1000;
            let mut a = (mage
                + if matches!(weapon, 1381 | 1405 | 12658 | 1409) {
                    9
                } else {
                    11
                })
                * (get(&b.offensive, "magic") as i64 + 64);
            if matches!(weapon, 1381 | 1405) {
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
                if matches!(weapon, 1381 | 1405 | 12658 | 1409 | 27275) {
                    5
                } else {
                    4
                },
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
    let keris = style == "Melee"
        && [25979, 25981, 27291, 27287].contains(&weapon)
        && monster.attributes.iter().any(|a| a == "kalphite");
    if keris {
        max_hit = max_hit * 133 / 100;
        if weapon == 25981 {
            attack = attack * 133 / 100;
        }
    }
    let mut hit = accuracy(attack, defence);
    if style == "Magic" && set[12] == Some(22975) {
        hit = 0.75 * hit + 0.25 * accuracy(attack, defence * 9 / 10);
    }
    let fang = style == "Melee" && [26219, 27246].contains(&weapon);
    if fang {
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
    let mean = if max_hit > 0 {
        (min_hit + max_hit) as f64 / 2.0
            + if min_hit == 0 {
                1.0 / (max_hit as f64 + 1.0)
            } else {
                0.0
            }
    } else {
        0.0
    };
    let mut expected = hit * mean;
    if keris {
        expected = hit * (max_hit as f64 / 2.0 * 53.0 / 51.0 + 1.0 / (max_hit as f64 + 1.0));
        max_hit *= 3;
    }
    if style == "Ranged" {
        let zcb = weapon == 26374;
        match set[13] {
            Some(9242) => {
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
            Some(9243) => {
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
