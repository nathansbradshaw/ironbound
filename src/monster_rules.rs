//! Reviewed outgoing NPC rules, applied to rolled hits after equipment/bolt effects.
//! Sources: SOURCES.md (pinned Wiki calculator and encounter mechanics pages).
use crate::{combat, encounters, planner, rules};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Standard,
    Zulrah,
    Kraken,
    Corporeal,
    Respiratory,
    Skotizo,
    Altar,
    Flower,
    Totem,
    PhosaniMinion,
    HueyTail,
    TitanElemental,
    Tormented,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    #[default]
    Any,
    Polearm,
    Flying,
    None,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub ids: Vec<i32>,
    pub kind: Kind,
    #[serde(default)]
    pub melee: Reach,
    pub allowed_styles: Vec<String>,
    #[serde(default)]
    pub light_weapons: Vec<i32>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub profiles: Vec<Profile>,
}
impl Registry {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for p in &self.profiles {
            if p.ids.is_empty()
                || p.ids
                    .iter()
                    .any(|id| !ids.insert(*id) || planner::monster(*id).is_none())
                || p.allowed_styles.is_empty()
                || p.allowed_styles
                    .iter()
                    .any(|s| !["Melee", "Ranged", "Magic"].contains(&s.as_str()))
                || matches!(p.kind, Kind::Skotizo | Kind::Altar) && p.light_weapons.is_empty()
                || p.light_weapons
                    .iter()
                    .any(|id| planner::gear(*id).is_none())
            {
                return Err("Invalid or duplicate outgoing NPC profile.".into());
            }
        }
        Ok(())
    }
}
pub fn registry() -> &'static Registry {
    static DATA: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let data: Registry = serde_json::from_str(include_str!("../assets/monster-rules.json"))
            .expect("outgoing NPC rules");
        data.validate().expect("valid outgoing NPC rules");
        data
    })
}
pub fn profile(id: i32) -> Option<&'static Profile> {
    // Internal windows only inherit a reviewed source rule when explicitly enabled.
    let source = encounters::source_npc_id(id);
    registry().profiles.iter().find(|p| p.ids.contains(&source))
}
pub fn hp_scaled(target: &planner::Monster, hp: u32) -> Result<Option<planner::Monster>, String> {
    let Some(curve) = encounters::window(target.id).and_then(|w| w.hp_scaling.as_ref()) else {
        return Ok(None);
    };
    let mut result = target.clone();
    result.skills.extend(curve.levels(hp)?);
    Ok(Some(result))
}
pub fn instant_defeat(id: i32, weapon: i32) -> Option<i32> {
    let p = profile(id)?;
    (p.kind == Kind::Flower || p.kind == Kind::Altar && p.light_weapons.contains(&weapon))
        .then(|| planner::monster(id).and_then(|m| m.skills.get("hp").copied()))
        .flatten()
}
pub fn guaranteed_accuracy(id: i32, attack_type: &str) -> bool {
    profile(id).is_some_and(|p| p.kind == Kind::Totem && attack_type == "magic")
        || encounters::window(id).is_some_and(|w| w.tormented_defenceless)
}
/// Elemental accuracy bypasses normal rolls, weakness accuracy and reroll gear.
pub fn elemental_accuracy(id: i32, attack_type: &str, magic_bonus: i32, void: bool) -> Option<f64> {
    profile(id)
        .filter(|p| p.kind == Kind::TitanElemental && attack_type == "magic")
        .map(|_| {
            let base = (f64::from(magic_bonus.max(0)) / 100.0 + 0.3).min(1.0);
            (base * if void { 1.45 } else { 1.0 }).min(1.0)
        })
}
pub fn guaranteed_max(id: i32, attack_type: &str) -> bool {
    profile(id).is_some_and(|p| p.kind == Kind::PhosaniMinion && attack_type == "crush")
}
pub fn style_limitation(id: i32, style: &str) -> Option<&'static str> {
    let style = rules::base_style(style);
    if encounters::hit_rule(id).is_some_and(|r| !r.allowed_styles.iter().any(|s| s == style)) {
        return Some(
            "Outgoing mechanics are not verified for this style in the selected attack window.",
        );
    }
    let p = profile(id)?;
    if !p.allowed_styles.iter().any(|s| s == style) {
        Some("This target is immune to the selected attack style.")
    } else if style == "Melee" && p.melee == Reach::None {
        Some("This target cannot be reached with melee attacks.")
    } else {
        None
    }
}
pub fn check_weapon(id: i32, style: &str, weapon: i32) -> Result<(), String> {
    if let Some(reason) = style_limitation(id, style) {
        return Err(reason.into());
    }
    if rules::base_style(style) != "Melee" {
        return Ok(());
    }
    if let Some(p) = profile(id) {
        let category = combat::stats(weapon)
            .map(|s| s.category.as_str())
            .unwrap_or("");
        let reachable = match p.melee {
            Reach::Any => true,
            Reach::Polearm => category == "Polearm",
            Reach::Flying => ["Polearm", "Salamander"].contains(&category),
            Reach::None => false,
        };
        if !reachable {
            return Err("This target is out of reach for this melee weapon; use a supported ranged/magic weapon or a reviewed reach weapon.".into());
        }
    }
    Ok(())
}

/// HP-dependent ruby damage uses the same transform as ordinary rolled hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Transform {
    #[default]
    Identity,
    Zulrah,
    Kraken,
    Corporeal,
    TormentedShield,
    Immune,
    Scaled {
        numerator: u32,
        denominator: u32,
        cap: Option<i32>,
    },
    HueyTail {
        cap: i32,
        crush_minimum: bool,
        pillar: bool,
    },
    Skotizo {
        altars: u8,
        light: bool,
    },
}
impl Transform {
    pub fn outcomes(self, damage: i32, probability: f64) -> Vec<(i32, f64)> {
        match self {
            Self::Zulrah if damage > 50 => (45..=50).map(|d| (d, probability / 6.0)).collect(),
            // A positive hit stays at least one; an accuracy miss remains zero.
            Self::Kraken if damage > 0 => vec![((damage / 7).max(1), probability)],
            Self::Corporeal => vec![(damage / 2, probability)],
            Self::TormentedShield if damage > 0 => vec![((damage * 4 / 5).max(1), probability)],
            Self::Immune => vec![(0, probability)],
            Self::Scaled {
                numerator,
                denominator,
                cap,
            } => {
                let scaled = (i64::from(damage) * i64::from(numerator)
                    / i64::from(denominator.max(1))) as i32;
                vec![(scaled.min(cap.unwrap_or(i32::MAX)), probability)]
            }
            Self::HueyTail {
                cap,
                crush_minimum,
                pillar,
            } => {
                let damage = damage.min(cap).max(i32::from(crush_minimum));
                vec![(if pillar { damage * 13 / 10 } else { damage }, probability)]
            }
            Self::Skotizo { altars, light } => {
                let percent = 100 - i32::from(altars.min(4)) * if light { 15 } else { 25 };
                vec![(
                    (i64::from(damage) * i64::from(percent) / 100) as i32,
                    probability,
                )]
            }
            _ => vec![(damage, probability)],
        }
    }
}
pub fn transform_for(
    id: i32,
    weapon: i32,
    attack_type: &str,
    bonuses: &combat::Bonuses,
    spell: &str,
) -> Transform {
    if encounters::window(id).is_some_and(|w| w.invulnerable) {
        return Transform::Immune;
    }
    let pillar = encounters::window(id).is_some_and(|w| w.huey_pillar);
    if profile(id).is_some_and(|p| p.kind == Kind::HueyTail) {
        let bonus = |style: &str| bonuses.offensive.get(style).copied().unwrap_or(0);
        let crush = attack_type == "crush"
            && bonus("crush") > bonus("stab")
            && bonus("crush") > bonus("slash");
        let earth = attack_type == "magic"
            && rules::spells()
                .iter()
                .any(|r| r.name == spell && r.element.as_deref() == Some("earth"));
        return Transform::HueyTail {
            cap: if crush || earth { 9 } else { 4 },
            crush_minimum: crush,
            pillar,
        };
    }
    if pillar {
        return Transform::Scaled {
            numerator: 13,
            denominator: 10,
            cap: None,
        };
    }
    if let Some(damage) = encounters::window(id).and_then(|w| w.npc_damage.as_ref()) {
        return Transform::Scaled {
            numerator: damage.factor[0],
            denominator: damage.factor[1],
            cap: damage.cap,
        };
    }
    match profile(id).map(|p| p.kind) {
        Some(Kind::Zulrah) => Transform::Zulrah,
        Some(Kind::Kraken) if ["light", "standard", "heavy"].contains(&attack_type) => {
            Transform::Kraken
        }
        Some(Kind::Corporeal) if !corpbane(weapon, attack_type) => Transform::Corporeal,
        Some(Kind::Tormented) if !tormented_shield_exempt(weapon, attack_type, spell) => {
            Transform::TormentedShield
        }
        Some(Kind::Skotizo) => Transform::Skotizo {
            altars: encounters::window(id)
                .and_then(|w| w.active_altars)
                .unwrap_or(4),
            light: profile(id).is_some_and(|p| p.light_weapons.contains(&weapon)),
        },
        Some(Kind::Totem) if attack_type == "magic" => Transform::Scaled {
            numerator: 2,
            denominator: 1,
            cap: None,
        },
        _ => Transform::Identity,
    }
}
/// Retain each hitsplat until NPC transformations finish; cap(60)+cap(30)
/// differs from cap(90), just as floor(3/2)+floor(3/2) differs from floor(6/2).
pub fn transform_splats(splats: &[i32], probability: f64, transform: Transform) -> Vec<(i32, f64)> {
    if transform == Transform::Identity {
        return vec![(splats.iter().sum(), probability)];
    }
    let mut totals = vec![(0, probability)];
    for damage in splats {
        let outcomes = transform.outcomes(*damage, 1.0);
        totals = compact(
            totals
                .into_iter()
                .flat_map(|(sum, p)| outcomes.iter().map(move |(d, q)| (sum + d, p * q))),
        );
    }
    totals
}
fn compact(hits: impl IntoIterator<Item = (i32, f64)>) -> Vec<(i32, f64)> {
    let mut result = BTreeMap::<i32, f64>::new();
    for (damage, probability) in hits {
        if probability > 0.0 {
            *result.entry(damage).or_default() += probability;
        }
    }
    result.into_iter().collect()
}
/// Fire-shield exceptions are style-specific; holding a demonbane melee weapon
/// while casting an ordinary spell does not exempt the spell.
pub fn tormented_shield_exempt(weapon: i32, attack_type: &str, spell: &str) -> bool {
    let name = planner::gear(weapon).map(|g| g.name.as_str()).unwrap_or("");
    match attack_type {
        "magic" => rules::spells()
            .iter()
            .any(|s| s.name == spell && s.name.contains("Demonbane")),
        "light" | "standard" | "heavy" => name == "Scorching bow",
        "stab" | "slash" | "crush" => [
            "Silverlight",
            "Darklight",
            "Arclight",
            "Emberlight",
            "Bone claws",
            "Burning claws",
            "Abyssal bludgeon",
            "Abyssal dagger",
            "Abyssal whip",
            "Abyssal tentacle",
            "Abyssal whip (or)",
            "Abyssal tentacle (or)",
        ]
        .contains(&name),
        _ => false,
    }
}
pub fn corpbane(weapon: i32, attack_type: &str) -> bool {
    if attack_type == "magic" {
        return true;
    }
    if attack_type != "stab" {
        return false;
    }
    let fang = rules::weapon(weapon).is_some_and(|w| w.has(rules::Effect::Fang));
    fang || planner::gear(weapon).is_some_and(|g| {
        g.name.ends_with("halberd")
            || g.name.contains("spear")
                && !g
                    .name
                    .split(" (")
                    .next()
                    .unwrap_or("")
                    .eq_ignore_ascii_case("Blue moon spear")
    })
}
/// Corp applies after other bolt effects but before ruby damage. Other NPC
/// transforms apply to every hit including ruby; its suffix stays identifiable
/// so finite-HP budgets can recompute it rather than freezing a full-health proc.
pub fn apply(
    id: i32,
    npc_transform: Transform,
    distribution: &mut Vec<(i32, f64)>,
    ruby: &mut Option<combat::HpDependentHit>,
    normal_already_transformed: bool,
) {
    let Some(p) = profile(id) else { return };
    // Corp exempts ruby procs; all other NPC transforms also apply to ruby.
    let transform = if p.kind == Kind::Corporeal {
        Transform::Identity
    } else {
        npc_transform
    };
    let suffix = usize::from(ruby.is_some());
    let split = distribution.len() - suffix;
    let corp = npc_transform == Transform::Corporeal;
    let normal = distribution[..split].iter().flat_map(|(d, probability)| {
        if normal_already_transformed {
            vec![(*d, *probability)]
        } else {
            transform.outcomes(if corp { d / 2 } else { *d }, *probability)
        }
    });
    let mut result = compact(normal);
    if let Some(ruby) = ruby {
        ruby.transform = transform;
        let (damage, probability) = distribution[split];
        let outcomes = transform.outcomes(damage, probability);
        ruby.distribution_entries = outcomes.len();
        result.extend(outcomes);
    }
    *distribution = result;
}
