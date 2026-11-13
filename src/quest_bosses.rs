//! Quest-body outgoing rules and explicitly scoped gear screening.
//! Factual rule sources and readiness policy are recorded in docs/quest-bosses.md.
use crate::{combat, encounters, planner, rules};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub quest: String,
    pub ids: Vec<i32>,
    pub level: u32,
    pub styles: Vec<String>,
    #[serde(default)]
    pub weapons: Vec<i32>,
    pub note: String,
}
pub fn profiles() -> &'static [Profile] {
    static DATA: OnceLock<Vec<Profile>> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/quest-bosses.json")).expect("quest boss rules")
    })
}
pub fn profile(name: &str) -> Option<&'static Profile> {
    profiles().iter().find(|p| p.name == name)
}
pub fn target(id: i32) -> Option<&'static Profile> {
    let id = encounters::source_npc_id(id);
    profiles().iter().find(|p| p.ids.contains(&id))
}
pub fn visible(entry: &encounters::Entry, hide_quests: bool) -> bool {
    !hide_quests || profile(&entry.name).is_none()
}
/// Explain a required quest weapon independently of ownership or DPS support.
pub fn weapon_requirement(id: i32, set: &[Option<i32>], style: &str) -> Option<String> {
    let p = target(id)?;
    if p.weapons.is_empty()
        || rules::base_style(style) == "Melee"
            && set.get(3).copied().flatten()
                .is_some_and(|w| p.weapons.contains(&w))
    {
        return None;
    }
    let weapons = p.weapons.iter()
        .map(|id| planner::item_name(*id))
        .collect::<Vec<_>>()
        .join(" or ");
    let mut message = format!("{} requires {} for this quest fight.", p.name, weapons);
    if p.ids.contains(&16204) {
        message.push_str(" Create the Hallowed flail during The Blood Moon Rises before this fight. The Sunspear is awarded after defeating Drakan.");
    }
    Some(message)
}
pub fn check_loadout(id: i32, set: &[Option<i32>], style: &str, spell: &str) -> Result<(), String> {
    let Some(_) = target(id) else {
        return Ok(());
    };
    let weapon = set.get(3).copied().flatten().unwrap_or(0);
    let style = rules::base_style(style);
    if let Some(message) = weapon_requirement(id, set, style) {
        return Err(message);
    }
    let element = rules::spells()
        .iter()
        .find(|s| s.name == spell)
        .and_then(|s| s.element.as_deref());
    match encounters::source_npc_id(id) {
        13482 if style != "Magic" || rules::weapon(weapon).is_none_or(|r| r.family.formula == rules::Formula::Powered) =>
            Err("Surok requires a cast combat spell; powered staves, melee and ranged cannot damage him.".into()),
        3456 if set.get(9).copied().flatten() != Some(1580) =>
            Err("Wear owned ice gloves to prevent Fareed from disarming this weapon.".into()),
        3456 if style == "Magic" && element != Some("water") =>
            Err("Fareed can only take Magic damage from Water spells.".into()),
        3458 if style == "Magic" && element != Some("fire") =>
            Err("Kamil can only take Magic damage from Fire spells.".into()),
        12215 if style == "Melee" => Err("The quest Leviathan cannot be reached with melee.".into()),
        8097 | 8096 if style == "Melee" => Err("Galvek is airborne in this phase; use Ranged or Magic.".into()),
        _ => Ok(()),
    }
}
pub fn vampyre_accuracy(id: i32, weapon: i32, ring: Option<i32>, attack: i64) -> i64 {
    if !matches!(encounters::source_npc_id(id), 16204 | 8242 | 9567 | 3459) {
        return attack;
    }
    let (n, d) = match weapon {
        33718 => (5, 4),
        24699 => (21, 20),
        _ => (1, 1),
    };
    let attack = attack * n / d;
    if ring == Some(21140) && [33718, 24699, 22398].contains(&weapon) {
        attack * 23 / 20
    } else {
        attack
    }
}
pub fn damage_transform(
    id: i32,
    weapon: i32,
    ring: Option<i32>,
) -> Option<crate::monster_rules::Transform> {
    if !matches!(encounters::source_npc_id(id), 16204 | 8242 | 9567 | 3459) {
        return None;
    }
    let (numerator, denominator) = match weapon {
        22398 => (6, 5),
        24699 | 33718 => (5, 4),
        _ => return None,
    };
    Some(crate::monster_rules::Transform::QuestVampyre {
        numerator,
        denominator,
        efaritay: ring == Some(21140),
        drakan_cap: encounters::source_npc_id(id) == 16204,
    })
}

/// Candidate repair is deliberately limited to quest-required gear. It never
/// invents stock or overrides a user's pinned slot in the main planner.
pub fn candidate_sets<'a>(
    p: &Profile,
    sets: &[(&'a str, Vec<Option<i32>>)],
    member: &crate::account::Member,
    owned: &BTreeMap<i32, u32>,
    shots: u32,
) -> Vec<(&'a str, Vec<Option<i32>>)> {
    let mut out = Vec::new();
    for (style, set) in sets {
        let base = rules::base_style(style);
        if !p.styles.iter().any(|s| s == base) {
            continue;
        }
        let mut candidates = vec![set.clone()];
        // Required flails are often weaker in generic scoring than an owned
        // whip/axe; Surok must also consider elemental and Iban staves.
        if !p.weapons.is_empty()
            || p.ids.contains(&13482)
            || p.ids.contains(&3456)
            || p.ids.contains(&3458)
        {
            for id in owned.keys().filter(|id| {
                owned.get(id).is_some_and(|q| *q > 0)
                    && planner::can_use(**id, member)
                    && rules::weapon(**id).is_some_and(|r| r.permits(style))
                    && (p.weapons.is_empty() || p.weapons.contains(id))
            }) {
                let mut next = set.clone();
                next[3] = Some(*id);
                if planner::gear(*id).is_some_and(|g| g.two_handed) {
                    next[5] = None;
                }
                candidates.push(next);
            }
        }
        for mut next in candidates {
            if p.ids.contains(&3456) {
                if !owned.get(&1580).is_some_and(|q| *q > 0) {
                    continue;
                }
                next[9] = Some(1580);
            }
            if next[3].is_some_and(|w| rules::weapon(w).is_some_and(|r| r.family.style == "Ranged"))
            {
                next[13] = next[3].and_then(|w| {
                    rules::ammo_ids(w)
                        .iter()
                        .find(|a| {
                            owned.get(a).is_some_and(|q| *q >= shots)
                                && planner::can_use(**a, member)
                        })
                        .copied()
                });
            }
            if !out.iter().any(|(s, e)| s == style && e == &next) {
                out.push((*style, next));
            }
        }
    }
    out
}

pub fn best_set(
    p: &Profile,
    style: &str,
    sets: Vec<(&str, Vec<Option<i32>>)>,
    member: &crate::account::Member,
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
) -> Option<Vec<Option<i32>>> {
    sets.into_iter()
        .filter(|(s, _)| *s == style)
        .filter_map(|(_, set)| {
            let mut lowest = f64::INFINITY;
            for id in &p.ids {
                let m = planner::monster(*id)?;
                let spell = rules::spell_name_for_target(member, &set, owned, settings, m, 0);
                if rules::base_style(style) == "Magic"
                    && rules::spell_requirement(member, &set, owned, settings, spell).is_some()
                {
                    return None;
                }
                let result =
                    combat::calculate_configured(member, &set, style, m, spell, 0, settings)
                        .ok()?;
                lowest = lowest.min(result.dps);
            }
            (lowest > 0.0 && lowest.is_finite()).then_some((lowest, set))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, set)| set)
}
