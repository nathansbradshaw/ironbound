//! Reviewed starting points, not a claim that a raid is completely simulated.
//! Item families and weapon-dependent tiers are data; eligibility is centralized.
use crate::{
    account::Member,
    combat,
    planner::{self, Boss, Options},
    rules,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub source_urls: Vec<String>,
    pub rationale: Vec<String>,
    pub incoming_model: String,
    pub roles: Vec<Role>,
    #[serde(default)]
    pub phase_weapons: Vec<PhaseWeapons>,
    #[serde(default)]
    pub target_ids: Vec<i32>,
    #[serde(default)]
    pub joint_sets: Vec<JointSet>,
    #[serde(default)]
    pub supply_roles: Vec<SupplyRole>,
    #[serde(default)]
    pub food_ids: Vec<i32>,
    #[serde(default)]
    pub skip_stamina: bool,
    #[serde(default)]
    pub recommended_mark_darkness: bool,
}
/// Ordered owned-item alternatives for a trip purpose; no encounter-specific
/// potion IDs belong in the packer. Multi-purpose bottles count only once.
#[derive(Debug, Deserialize)]
pub struct SupplyRole {
    pub id: String,
    pub label: String,
    pub ids: Vec<i32>,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default)]
    pub skipped_by: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub prayer_ids: Vec<i32>,
    #[serde(default)]
    pub requires_magic_benefit: bool,
}
#[derive(Debug, Deserialize)]
pub struct JointSet {
    pub id: String,
    pub style: String,
    pub slots: Vec<JointSlot>,
}
#[derive(Debug, Deserialize)]
pub struct JointSlot {
    pub slot: String,
    pub items: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct PhaseWeapons {
    pub target: i32,
    pub families: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct Role {
    pub style: String,
    pub slots: Vec<SlotGuide>,
}
#[derive(Debug, Deserialize)]
pub struct SlotGuide {
    pub slot: String,
    pub rationale: String,
    pub tiers: Vec<Tier>,
}
#[derive(Debug, Deserialize)]
pub struct Tier {
    pub items: Vec<String>,
    #[serde(default)]
    pub required_weapon_names: Vec<String>,
    #[serde(default)]
    pub required_set: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Guidance {
    #[serde(default)]
    aliases: BTreeMap<String, Vec<String>>,
    profiles: Vec<Profile>,
}
fn data() -> &'static Guidance {
    static DATA: OnceLock<Guidance> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/encounter-guidance.json"))
            .expect("reviewed encounter guidance")
    })
}

pub fn profile(options: &Options) -> Option<&'static Profile> {
    let id = match options.boss {
        Boss::Toa => "toa",
        Boss::Custom
            if !options.targets.is_empty()
                && options
                    .targets
                    .iter()
                    .all(|t| matches!(t.id, 13011 | 13012 | 13013)) =>
        {
            "moons"
        }
        Boss::Custom if !options.targets.is_empty() => {
            return data().profiles.iter().find(|p| {
                !p.target_ids.is_empty()
                    && options.targets.iter().all(|t| p.target_ids.contains(&t.id))
            });
        }
        _ => return None,
    };
    data().profiles.iter().find(|p| p.id == id)
}
fn named(actual: &str, reviewed: &str) -> bool {
    actual.eq_ignore_ascii_case(reviewed)
        || data().aliases.get(reviewed).is_some_and(|aliases| {
            aliases
                .iter()
                .any(|alias| actual.eq_ignore_ascii_case(alias))
        })
}
fn usable_variant(g: &planner::Gear) -> bool {
    !["0", "broken", "inactive", "uncharged"].contains(&g.version.to_ascii_lowercase().as_str())
}
fn tier_applies(tier: &Tier, set: &[Option<i32>]) -> bool {
    (tier.required_weapon_names.is_empty()
        || set
            .get(3)
            .copied()
            .flatten()
            .and_then(planner::gear)
            .is_some_and(|weapon| {
                tier.required_weapon_names
                    .iter()
                    .any(|name| named(&weapon.name, name))
            }))
        && tier.required_set.as_ref().is_none_or(|id| {
            data()
                .profiles
                .iter()
                .flat_map(|p| &p.joint_sets)
                .find(|joint| joint.id == *id)
                .is_some_and(|joint| {
                    joint.slots.iter().all(|slot| {
                        planner::SLOT_KEYS
                            .iter()
                            .position(|key| *key == slot.slot)
                            .and_then(|index| set.get(index).copied().flatten())
                            .and_then(planner::gear)
                            .is_some_and(|g| {
                                usable_variant(g) && slot.items.iter().any(|n| named(&g.name, n))
                            })
                    })
                })
        })
}

/// Joint equipment effects cannot be discovered reliably one slot at a time.
/// Return only complete owned, level-usable sets, leaving the original seed intact.
/// Callers apply manual pins and validate weapon/ammunition after this operation.
pub fn joint_sets(
    seed: &[Option<i32>],
    style: &str,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) -> Vec<Vec<Option<i32>>> {
    let Some(profile) = profile(options) else {
        return Vec::new();
    };
    profile
        .joint_sets
        .iter()
        .filter(|joint| joint.style == rules::base_style(style))
        .filter_map(|joint| {
            let mut set = seed.to_vec();
            for slot in &joint.slots {
                let index = planner::SLOT_KEYS
                    .iter()
                    .position(|key| *key == slot.slot)?;
                if ![0, 1, 2, 4, 5, 7, 9, 10, 12].contains(&index) || index >= set.len() {
                    return None;
                }
                let id = slot.items.iter().find_map(|name| {
                    planner::equipment()
                        .iter()
                        .filter(|g| {
                            g.slot == slot.slot
                                && named(&g.name, name)
                                && usable_variant(g)
                                && owned.get(&g.id).copied().unwrap_or(0) > 0
                                && !options.excluded.contains(&g.id)
                                && planner::can_use(g.id, member)
                        })
                        .min_by_key(|g| g.id)
                        .map(|g| g.id)
                })?;
                set[index] = Some(id);
            }
            Some(set)
        })
        .collect()
}
// These are within-tier tie breakers, never global armour weights. A reviewed
// tank tier remains ahead of low-defence strength armour at the Moons.
fn within_tier_quality(id: i32, style: &str, tank: bool) -> (i32, i32, i32, i32) {
    let Some(s) = combat::stats(id) else {
        return (i32::MIN, 0, 0, 0);
    };
    let get = |map: &BTreeMap<String, i32>, key: &str| map.get(key).copied().unwrap_or(0);
    let defence = get(&s.defensive, "slash");
    let (strength, accuracy) = match rules::base_style(style) {
        "Magic" => (get(&s.bonuses, "magic_str"), get(&s.offensive, "magic")),
        "Ranged" => (get(&s.bonuses, "ranged_str"), get(&s.offensive, "ranged")),
        _ => (
            get(&s.bonuses, "str"),
            match style {
                "Stab" => get(&s.offensive, "stab"),
                "Crush" => get(&s.offensive, "crush"),
                _ => get(&s.offensive, "slash"),
            },
        ),
    };
    if tank {
        (defence, strength, accuracy, get(&s.bonuses, "prayer"))
    } else {
        (strength, accuracy, defence, get(&s.bonuses, "prayer"))
    }
}

/// Seed only reviewed armour/accessory slots. Unknown roles and unowned tiers
/// preserve the caller's existing candidates; user overrides belong after this.
pub fn seed(
    set: &mut Vec<Option<i32>>,
    style: &str,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) {
    let Some(p) = profile(options) else { return };
    let Some(role) = p.roles.iter().find(|r| r.style == rules::base_style(style)) else {
        return;
    };
    for guide in &role.slots {
        let Some(index) = planner::SLOT_KEYS.iter().position(|key| *key == guide.slot) else {
            continue;
        };
        // Defence in depth: external data must never replace weapons or ammunition.
        if ![0, 1, 2, 4, 5, 7, 9, 10, 12].contains(&index) || index >= set.len() {
            continue;
        }
        // Do not introduce a shield beside a two-handed weapon.
        if index == 5
            && set
                .get(3)
                .copied()
                .flatten()
                .and_then(planner::gear)
                .is_some_and(|weapon| weapon.two_handed)
        {
            continue;
        }
        let chosen = guide
            .tiers
            .iter()
            .filter(|tier| tier_applies(tier, set))
            .find_map(|tier| {
                planner::equipment()
                    .iter()
                    .filter(|g| {
                        g.slot == guide.slot
                            && owned.get(&g.id).copied().unwrap_or(0) > 0
                            && !options.excluded.contains(&g.id)
                            && usable_variant(g)
                            && planner::can_use(g.id, member)
                            && combat::stats(g.id).is_some()
                            && tier.items.iter().any(|name| named(&g.name, name))
                    })
                    .max_by_key(|g| {
                        (
                            within_tier_quality(g.id, style, p.incoming_model == "normal_attacks"),
                            std::cmp::Reverse(g.id),
                        )
                    })
                    .map(|g| g.id)
            });
        if let Some(id) = chosen {
            set[index] = Some(id);
        }
    }
}

/// Reviewed tier for a concrete item in the current weapon context. Lower is
/// earlier progression priority. None means unreviewed, not unusable or bad.
pub fn item_tier(
    style: &str,
    slot: usize,
    id: i32,
    options: &Options,
    set: &[Option<i32>],
) -> Option<usize> {
    let key = *planner::SLOT_KEYS.get(slot)?;
    if ![0, 1, 2, 4, 5, 7, 9, 10, 12].contains(&slot) {
        return None;
    }
    let g = planner::gear(id)?;
    if g.slot != key || !usable_variant(g) {
        return None;
    }
    let role = profile(options)?
        .roles
        .iter()
        .find(|r| r.style == rules::base_style(style))?;
    role.slots
        .iter()
        .find(|guide| guide.slot == key)?
        .tiers
        .iter()
        .position(|tier| {
            tier_applies(tier, set) && tier.items.iter().any(|name| named(&g.name, name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn member() -> Member {
        Member {
            level_overrides: [
                "Attack",
                "Strength",
                "Defence",
                "Ranged",
                "Magic",
                "Prayer",
                "Hitpoints",
            ]
            .into_iter()
            .map(|name| (name.into(), 99))
            .collect(),
            ..Default::default()
        }
    }
    fn moons() -> Options {
        Options {
            boss: Boss::Custom,
            targets: [13011, 13012, 13013]
                .into_iter()
                .map(|id| planner::Target {
                    id,
                    style: "Slash".into(),
                })
                .collect(),
            ..Default::default()
        }
    }
    #[test]
    fn guidance_does_not_spill_into_other_groups() {
        let mut options = moons();
        assert_eq!(profile(&options).unwrap().id, "moons");
        options.targets.push(planner::Target {
            id: 11789,
            style: "Slash".into(),
        });
        assert!(profile(&options).is_none());
        options.targets.clear();
        assert!(profile(&options).is_none());
    }
    #[test]
    fn an_individual_moon_keeps_its_defensive_guidance() {
        for target in moons().targets {
            let options = Options {
                boss: Boss::Custom,
                targets: vec![target],
                ..Default::default()
            };
            assert_eq!(profile(&options).unwrap().id, "moons");
        }
    }
    #[test]
    fn tank_barrows_wins_over_torso_and_utility_at_moons() {
        let mut set = vec![None; 14];
        set[3] = Some(4587);
        set[4] = Some(10551);
        set[13] = Some(892);
        let bank = BTreeMap::from([(4749, 1), (10551, 1), (1204, 1)]);
        seed(&mut set, "Slash", &member(), &bank, &moons());
        assert_eq!(set[4], Some(4749));
        assert_eq!(set[3], Some(4587));
        assert_eq!(set[13], Some(892));
    }
    #[test]
    fn crystal_tiers_require_compatible_bow_and_accept_recolours() {
        let options = Options {
            boss: Boss::Toa,
            ..Default::default()
        };
        let mut set = vec![None; 14];
        set[3] = Some(9185);
        assert!(item_tier("Ranged", 4, 27745, &options, &set).is_none());
        set[3] = Some(25867);
        assert!(item_tier("Ranged", 4, 27745, &options, &set).is_some());
        assert!(item_tier("Ranged", 4, 27747, &options, &set).is_none());
    }
    #[test]
    fn reviewed_names_are_real_catalog_entries() {
        for (base, aliases) in &data().aliases {
            for name in std::iter::once(base).chain(aliases) {
                assert!(
                    planner::equipment().iter().any(|g| g.name == *name),
                    "missing {name}"
                );
            }
        }
        for p in &data().profiles {
            for role in &p.roles {
                for guide in &role.slots {
                    for tier in &guide.tiers {
                        for name in &tier.items {
                            assert!(
                                planner::equipment()
                                    .iter()
                                    .any(|g| g.name == *name && g.slot == guide.slot),
                                "bad slot/name {name}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// A guide-quality guard for whole-set refinement. Lower is better. Only slots
/// reviewed for this encounter/style contribute; unknown items in those slots
/// rank after the listed tiers, without declaring all unreviewed gear unusable.
pub fn quality(style: &str, set: &[Option<i32>], options: &Options) -> usize {
    let Some(p) = profile(options) else { return 0 };
    let Some(role) = p.roles.iter().find(|r| r.style == rules::base_style(style)) else {
        return 0;
    };
    role.slots
        .iter()
        .filter_map(|guide| {
            let index = planner::SLOT_KEYS
                .iter()
                .position(|key| *key == guide.slot)?;
            if index == 5
                && set
                    .get(3)
                    .copied()
                    .flatten()
                    .and_then(planner::gear)
                    .is_some_and(|g| g.two_handed)
            {
                return None;
            }
            Some(
                set.get(index)
                    .copied()
                    .flatten()
                    .and_then(|id| item_tier(style, index, id, options, set))
                    .unwrap_or(guide.tiers.len() + 1),
            )
        })
        .sum()
}

/// Data-defined specialist alternatives for one exact encounter phase. The
/// caller must still filter ownership, requirements and exclusions before use.
pub fn phase_weapon_ids(options: &Options, target: i32) -> Vec<i32> {
    let Some(p) = profile(options) else {
        return Vec::new();
    };
    p.phase_weapons
        .iter()
        .filter(|phase| phase.target == target)
        .flat_map(|phase| {
            phase
                .families
                .iter()
                .flat_map(|family| rules::family_ids(family))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod phase_weapon_tests {
    use super::*;
    #[test]
    fn only_kephri_gets_the_reviewed_partisan_family() {
        let options = Options {
            boss: Boss::Toa,
            ..Default::default()
        };
        let expected: BTreeSet<_> = rules::family_ids("partisan").into_iter().collect();
        assert!(!expected.is_empty());
        assert_eq!(
            phase_weapon_ids(&options, 11719)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(phase_weapon_ids(&options, 11721).is_empty());
        assert!(phase_weapon_ids(&Options::default(), 11719).is_empty());
    }
}
