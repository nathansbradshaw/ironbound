//! Pinned Wiki raid scaling. A comparison window records its complete scale;
//! no party size or other players' skills are inferred from a bank import.
use crate::planner::Monster;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Cox,
    Tob,
    TobEntry,
    Toa,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    pub kind: Kind,
    pub party_size: u32,
    #[serde(default)]
    pub challenge: bool,
    #[serde(default = "combat")]
    pub highest_combat: u32,
    #[serde(default = "ninety_nine")]
    pub highest_hp: u32,
    #[serde(default = "ninety_nine")]
    pub average_mining: u32,
    #[serde(default)]
    pub invocation: u32,
    #[serde(default)]
    pub path_level: u32,
}
fn combat() -> u32 {
    126
}
fn ninety_nine() -> u32 {
    99
}
impl Scale {
    pub fn valid(&self) -> bool {
        (1..=match self.kind {
            Kind::Cox => 100,
            Kind::Toa => 8,
            _ => 5,
        })
            .contains(&self.party_size)
            && (3..=126).contains(&self.highest_combat)
            && (1..=99).contains(&self.highest_hp)
            && (1..=99).contains(&self.average_mining)
            && (!self.challenge || matches!(self.kind, Kind::Cox))
            && self.invocation <= 600
            && self.path_level <= 6
            && (matches!(self.kind, Kind::Toa) || self.invocation == 0 && self.path_level == 0)
    }
}
#[derive(Deserialize)]
struct Rules {
    magic_defensive: Vec<i32>,
    singles: Vec<i32>,
    guardians: Vec<i32>,
    tekton: Vec<i32>,
    crystals: Vec<i32>,
    olm_head: Vec<i32>,
    olm_melee: Vec<i32>,
    olm_magic: Vec<i32>,
}
fn rules() -> &'static Rules {
    static DATA: std::sync::OnceLock<Rules> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/raid-scaling.json"))
            .expect("raid scaling metadata")
    })
}
fn sqrt(n: u32) -> u32 {
    let mut root = 0;
    while (root + 1) * (root + 1) <= n {
        root += 1;
    }
    root
}
fn percent(n: i64, p: i64) -> i64 {
    n + n * p / 100
}
pub fn apply(m: &Monster, source: i32, s: &Scale) -> Result<Monster, String> {
    if !s.valid() {
        return Err("Invalid recorded raid scale.".into());
    }
    let mut result = m.clone();
    if matches!(s.kind, Kind::Toa) {
        if [
            11781, 11709, 11712, 11711, 11714, 11715, 11718, 11710, 11713, 11717, 11716,
        ]
        .contains(&source)
        {
            return Ok(result);
        }
        let core = [11755, 11758].contains(&source);
        let mut hp = if core {
            4500
        } else {
            i64::from(m.skills["hp"])
        };
        hp += hp * i64::from((if core { 1 } else { 4 }) * s.invocation / 10) / 100;
        if s.path_level > 0
            && ((11789..=11799).contains(&source)
                || [
                    11778, 11779, 11780, 11719, 11721, 11724, 11725, 11726, 11730, 11732, 11733,
                ]
                .contains(&source))
        {
            hp = hp * i64::from(103 + 5 * s.path_level) / 100;
        }
        if s.party_size >= 2 {
            let mut factor = 9 * if s.party_size >= 3 { 2 } else { 1 };
            if s.party_size >= 4 {
                factor += 6 * (s.party_size - 3);
            }
            hp = hp * i64::from(10 + factor) / 10;
        }
        if hp > 100 {
            let round = if hp > 300 { 10 } else { 5 };
            hp = (hp + round / 2) / round * round;
        }
        result.skills.insert("hp".into(), hp as i32);
        return Ok(result);
    }
    if !matches!(s.kind, Kind::Cox) {
        let (n, d) = match s.kind {
            Kind::TobEntry => ([10, 19, 27, 34, 40][s.party_size as usize - 1], 40),
            _ => (s.party_size.max(3) + 3, 8),
        };
        result.skills.insert(
            "hp".into(),
            (i64::from(m.skills["hp"]) * i64::from(n) / i64::from(d)) as i32,
        );
        return Ok(result);
    }
    if !m.attributes.iter().any(|a| a == "xerician") {
        return Err("Chambers scaling requires a reviewed Xerician target.".into());
    }
    let r = rules();
    let magic_def = r.magic_defensive.contains(&source);
    let offensive: Vec<_> = ["atk", "str", "ranged", "magic"]
        .into_iter()
        .filter(|k| (*k != "magic" || !magic_def) && m.skills.get(*k).is_some_and(|v| *v != 1))
        .collect();
    let defensive: Vec<_> = ["def", "magic"]
        .into_iter()
        .filter(|k| (*k != "magic" || magic_def) && m.skills.get(*k).is_some_and(|v| *v != 1))
        .collect();
    let base_o = i64::from(offensive.iter().map(|k| m.skills[*k]).max().unwrap_or(1));
    let base_d = i64::from(defensive.iter().map(|k| m.skills[*k]).max().unwrap_or(1));
    let base_hp = if r.guardians.contains(&source) {
        151 + i64::from(s.average_mining)
    } else {
        i64::from(m.skills["hp"])
    };
    let highest = i64::from(s.highest_combat.clamp(60, 126));
    let singles = r.singles.contains(&source);
    let (mut o, mut d, mut hp);
    if singles {
        let mut stat = i64::from(s.highest_hp.clamp(55, 99));
        let mut health = highest;
        if s.challenge {
            stat = percent(stat, 50);
            health = percent(health, 50);
        }
        o = (base_o * stat / 99).max(1);
        d = (base_d * stat / 99).max(1);
        hp = (base_hp * health / 126).max(5);
    } else {
        let n = s.party_size - 1;
        let level = i64::from((55 + 44 * s.highest_hp / 99).clamp(55, 99));
        o = base_o * level / 99;
        d = base_d * level / 99;
        hp = base_hp * highest / 126;
        o = o * i64::from(100 + sqrt(n) * 7 + n) / 100;
        d = d * i64::from(100 + sqrt(n) + n * 7 / 10) / 100;
        hp += hp * i64::from(s.party_size / 2);
        if s.challenge {
            o = percent(o, 50);
            if !r.crystals.contains(&source) {
                hp = percent(hp, 50);
            }
            if r.tekton.contains(&source) {
                d = percent(d, if s.party_size < 4 { 20 } else { 35 });
            } else if !r.crystals.contains(&source) {
                d = percent(d, 50);
            }
        }
        o = o.clamp(50, 5000);
        d = d.clamp(50, 20000);
        hp = hp.clamp(50, 30000);
    }
    for k in offensive {
        result.skills.insert(k.into(), o as i32);
    }
    for k in defensive {
        result.skills.insert(k.into(), d as i32);
    }
    let hand = r.olm_melee.contains(&source) || r.olm_magic.contains(&source);
    if hand || r.olm_head.contains(&source) {
        let factor =
            i64::from((s.party_size - 1).min(50)) - 3 * i64::from(s.party_size.min(50) / 8);
        hp = if hand {
            600 + 300 * factor
        } else {
            800 + 400 * factor
        };
        if r.olm_magic.contains(&source) {
            result.skills.entry("magic".into()).and_modify(|v| *v /= 2);
        }
    }
    result.skills.insert("hp".into(), hp as i32);
    Ok(result)
}
