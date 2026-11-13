//! Joint candidates for effects that a single-slot search cannot discover.
use crate::{
    account::Member,
    combat,
    planner::{self, Options},
    rules,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Seed {
    pub equipment: Vec<Option<i32>>,
    pub slots: BTreeSet<usize>,
}

fn usable(
    id: i32,
    slot: usize,
    style: &str,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
) -> bool {
    owned.get(&id).copied().unwrap_or(0) > 0
        && !o.item_excluded(id)
        && planner::can_use(id, m)
        && planner::gear(id).is_some_and(|g| {
            g.slot == planner::SLOT_KEYS[slot]
                && (!["0", "broken", "inactive", "uncharged"]
                    .contains(&g.version.to_ascii_lowercase().as_str())
                    || rules::equipment_effect(id, "wearable_broken"))
        })
        && o.gear_overrides
            .get(&format!("{style}:{slot}"))
            .is_none_or(|pin| *pin == id)
        && combat::stats(id).is_some()
}

/// Merge only truly equivalent variants: stats, weight and all passive
/// memberships matter. Manual pins are filtered before this collapse.
fn variants(
    ids: impl Iterator<Item = i32>,
    slot: usize,
    style: &str,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
) -> Vec<i32> {
    let mut seen = BTreeSet::new();
    ids.filter(|id| usable(*id, slot, style, m, owned, o))
        .filter(|id| {
            let stats = combat::stats(*id).unwrap();
            let membership: Vec<_> = rules::registry()
                .equipment_sets
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.pieces
                        .iter()
                        .any(|p| p.slot == slot && p.ids.contains(id))
                })
                .map(|(i, _)| i)
                .collect();
            let effects: Vec<_> = rules::registry()
                .equipment_effects
                .iter()
                .enumerate()
                .filter(|(_, e)| e.slot.is_none_or(|s| s == slot) && e.ids.contains(id))
                .map(|(i, _)| i)
                .collect();
            let weight = crate::ranking::item_weight(*id).map(f64::to_bits);
            seen.insert((
                stats.speed,
                stats.category.clone(),
                stats.offensive.clone(),
                stats.defensive.clone(),
                stats.bonuses.clone(),
                membership,
                effects,
                weight,
            ))
        })
        .collect()
}

pub fn candidates(
    base: &[Option<i32>],
    style: &str,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
) -> Vec<Seed> {
    let mut result = Vec::new();
    for rule in &rules::registry().equipment_sets {
        if !rule.search_style.as_ref().is_some_and(|s| {
            s == style
                || s == rules::base_style(style)
                || style == "Melee" && rules::base_style(s) == "Melee"
        }) {
            continue;
        }
        let mut rows = vec![(base.to_vec(), BTreeSet::new())];
        for piece in &rule.pieces {
            let ids = variants(piece.ids.iter().copied(), piece.slot, style, m, owned, o);
            let mut next = Vec::new();
            for (set, slots) in rows {
                if rule.partial {
                    next.push((set.clone(), slots.clone()));
                }
                for id in &ids {
                    let mut set = set.clone();
                    let mut slots = slots.clone();
                    set[piece.slot] = Some(*id);
                    slots.insert(piece.slot);
                    next.push((set, slots));
                }
            }
            rows = next;
        }
        if !rule.weapon_families.is_empty() {
            let ids = variants(
                owned.keys().copied().filter(|id| {
                    rules::weapon(*id).is_some_and(|r| {
                        rule.weapon_families.contains(&r.weapon.family) && r.permits(style)
                    })
                }),
                3,
                style,
                m,
                owned,
                o,
            );
            rows = rows
                .into_iter()
                .flat_map(|(set, slots)| {
                    ids.iter().map(move |id| {
                        let mut set = set.clone();
                        let mut slots = slots.clone();
                        set[3] = Some(*id);
                        slots.insert(3);
                        (set, slots)
                    })
                })
                .collect();
        }
        for (mut set, mut slots) in rows {
            if slots.is_empty() {
                continue;
            }
            let Some(weapon) = set[3].and_then(rules::weapon).filter(|w| w.permits(style)) else {
                continue;
            };
            if planner::gear(set[3].unwrap()).is_some_and(|g| g.two_handed) {
                if o.gear_overrides
                    .get(&format!("{style}:5"))
                    .is_some_and(|id| *id > 0)
                {
                    continue;
                }
                set[5] = None;
            }
            if rules::base_style(style) == "Magic"
                && !o.combat.spell_selection.permits_weapon(set[3].unwrap())
            {
                continue;
            }
            if rules::base_style(style) == "Ranged"
                && !o.combat.ranged_type.permits_weapon(set[3].unwrap())
            {
                continue;
            }
            let ammo: Vec<_> = if weapon.family.style == "Ranged" && slots.contains(&3) {
                slots.insert(13);
                if weapon.family.ammo.is_empty() {
                    // Self-ammunition bows can retain a manually worn blessing
                    // (or ignored ammunition); its stats are handled by combat.
                    vec![
                        o.gear_overrides
                            .get(&format!("{style}:13"))
                            .copied()
                            .filter(|id| *id > 0),
                    ]
                } else {
                    weapon
                        .family
                        .ammo
                        .iter()
                        .copied()
                        .filter(|id| {
                            usable(*id, 13, style, m, owned, o)
                                && owned.get(id).copied().unwrap_or(0) >= o.combat.ammo_shots
                                && rules::ammunition(*id) != rules::AmmoEffect::Unsupported
                        })
                        .map(Some)
                        .collect()
                }
            } else {
                vec![set[13]]
            };
            for ammo in ammo {
                let mut set = set.clone();
                set[13] = ammo;
                result.push(Seed {
                    equipment: set,
                    slots: slots.clone(),
                });
            }
        }
    }
    let mut seen = BTreeSet::new();
    result.retain(|s| seen.insert((s.equipment.clone(), s.slots.clone())));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn justiciar_is_jointly_considered_in_every_combat_style_and_respects_pins() {
        let mut m = Member::default();
        for skill in crate::account::SKILLS {
            m.level_overrides.insert(skill.into(), 99);
        }
        let owned = BTreeMap::from([
            (22326, 1),
            (22327, 1),
            (22328, 1),
            (4151, 1),
            (9185, 1),
            (9144, 1000),
            (11907, 1),
        ]);
        for (style, weapon) in [
            ("Melee", 4151),
            ("Slash", 4151),
            ("Magic", 11907),
            ("Ranged", 9185),
        ] {
            let mut base = vec![None; 14];
            base[3] = Some(weapon);
            base[13] = Some(9144);
            let mut o = Options::default();
            assert!(
                candidates(&base, style, &m, &owned, &o)
                    .iter()
                    .any(|s| rules::set_effect(&s.equipment, "justiciar"))
            );
            o.gear_overrides.insert(format!("{style}:0"), 0);
            assert!(
                !candidates(&base, style, &m, &owned, &o)
                    .iter()
                    .any(|s| rules::set_effect(&s.equipment, "justiciar"))
            );
        }
    }
}
