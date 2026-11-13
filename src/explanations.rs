//! Small, factual reasons for the actual displayed loadout. These describe the
//! selected setup, rather than claiming a globally optimal item or full fight.
use crate::{
    account::Snapshot,
    combat, guidance,
    planner::{self, Options, Packed, Plan},
    prayer, recommendation, rules,
};
use std::collections::BTreeSet;
fn short(text: &str, max: usize) -> String {
    let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max {
        return clean;
    }
    let prefix = clean
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    format!(
        "{}…",
        prefix.rsplit_once(' ').map(|(s, _)| s).unwrap_or(&prefix)
    )
}
fn shared(plan: &Plan) -> bool {
    plan.label.starts_with("Shared setup")
}
fn context<'a>(
    o: &'a Options,
    p: &'a Plan,
    phase: Option<usize>,
) -> Option<(&'a [Option<i32>], &'a str, String)> {
    match phase {
        Some(i) => p
            .phases
            .get(i)
            .map(|p| (p.equipment.as_slice(), p.style.as_str(), short(&p.name, 65))),
        None => Some((
            &p.equipment,
            planner::primary_style(o),
            "the starting setup".into(),
        )),
    }
}
fn targets(o: &Options, p: &Plan, phase: Option<usize>, style: &str) -> Vec<i32> {
    let available = recommendation::targets(o)
        .into_iter()
        .filter(|(_, s)| *s == style)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let Some(i) = phase else { return available };
    let Some(part) = p.phases.get(i) else {
        return vec![];
    };
    if o.boss == planner::Boss::Custom {
        return o
            .targets
            .get(i)
            .filter(|t| t.style == style && t.name() == part.name)
            .map(|t| vec![t.id])
            .unwrap_or_default();
    }
    let labels = part
        .name
        .split('·')
        .map(|s| s.trim().to_lowercase())
        .collect::<Vec<_>>();
    available
        .into_iter()
        .filter(|id| {
            planner::monster(*id).is_some_and(|m| {
                labels
                    .iter()
                    .any(|label| m.name.to_lowercase().starts_with(label))
            })
        })
        .collect()
}
fn damage(
    snapshot: &Snapshot,
    o: &Options,
    set: &[Option<i32>],
    style: &str,
    ids: &[i32],
) -> Option<Vec<f64>> {
    if ids.is_empty() {
        return None;
    }
    let mut owned = snapshot.available(o.shared);
    owned.retain(|id, _| !o.excluded.contains(id));
    let member = planner::member_with_stock(snapshot.selected()?, &owned);
    if set.len() != 14
        || !rules::casting_supported_configured(
            &member,
            set,
            &owned,
            o.combat.spell_casts,
            &o.combat,
        )
    {
        return None;
    }
    ids.iter()
        .map(|id| {
            let target = planner::monster(*id)?;
            combat::calculate_configured(
                &member,
                set,
                style,
                target,
                rules::spell_name_for_target(&member, set, &owned, &o.combat, target, o.raid_level),
                o.raid_level,
                &o.combat,
            )
            .ok()
            .map(|r| r.dps)
        })
        .collect()
}
fn range(values: &[f64]) -> String {
    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (high - low).abs() < 0.005 {
        format!("{low:.2}")
    } else {
        format!("{low:.2}–{high:.2}")
    }
}
fn marginal(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    phase: Option<usize>,
    slot: usize,
    set: &[Option<i32>],
    style: &str,
    label: &str,
) -> Option<String> {
    if phase.is_none() || set.get(slot) == p.equipment.get(slot) {
        return None;
    }
    let mut alternative = set.to_vec();
    alternative[slot] = *p.equipment.get(slot)?;
    if alternative
        .get(3)
        .copied()
        .flatten()
        .and_then(planner::gear)
        .is_some_and(|g| g.two_handed)
    {
        alternative[5] = None;
    }
    let ids = targets(o, p, phase, style);
    let current = damage(snapshot, o, set, style, &ids)?;
    let prior = damage(snapshot, o, &alternative, style, &ids)?;
    let delta = current
        .iter()
        .zip(&prior)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();
    let baseline = p.equipment[slot]
        .map(planner::item_name)
        .map(|s| short(&s, 55))
        .unwrap_or_else(|| "an empty slot".into());
    if delta.iter().all(|d| *d >= 0.005) {
        return Some(format!(
            "For {label}: +{} modeled DPS versus {baseline}.",
            range(&delta)
        ));
    }
    let saved = prayer::drain_per_minute(prayer::bonus(&alternative), &o.combat)
        - prayer::drain_per_minute(prayer::bonus(set), &o.combat);
    if delta.iter().all(|d| d.abs() < 1e-8) && saved >= 0.05 {
        return Some(format!(
            "Same modeled DPS for {label}; saves {saved:.1} prayer/min versus {baseline}."
        ));
    }
    None
}
fn stat_fact(id: i32, style: &str) -> Option<String> {
    let s = combat::stats(id)?;
    let bonus = |key: &str| s.bonuses.get(key).copied().unwrap_or(0);
    let prayer = bonus("prayer");
    let damage = match rules::base_style(style) {
        "Magic" if bonus("magic_str") > 0 => Some(format!(
            "+{:.1}% Magic damage",
            f64::from(bonus("magic_str")) / 10.0
        )),
        "Ranged" if bonus("ranged_str") > 0 => {
            Some(format!("+{} ranged strength", bonus("ranged_str")))
        }
        "Melee" if bonus("str") > 0 => Some(format!("+{} melee strength", bonus("str"))),
        _ => None,
    };
    match (damage, prayer > 0) {
        (Some(d), true) => Some(format!("Adds {d} and +{prayer} prayer bonus.")),
        (Some(d), false) => Some(format!("Adds {d}.")),
        (None, true) => Some(format!("Adds +{prayer} prayer bonus.")),
        _ => None,
    }
}
/// Reason for an actual worn slot; callers provide the name and slot label.
fn equipment_reason(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    phase: Option<usize>,
    slot: usize,
) -> String {
    let Some((set, style, label)) = context(o, p, phase) else {
        return "Setup details unavailable.".into();
    };
    let Some(id) = set.get(slot).copied().flatten() else {
        let weapon = set.get(3).copied().flatten();
        if slot == 5 && weapon.and_then(planner::gear).is_some_and(|g| g.two_handed) {
            return "Left empty for this two-handed weapon.".into();
        }
        if slot == 13 {
            if let Some(rule) = weapon.and_then(rules::weapon) {
                if !rule.family.internal_ammo.is_empty() {
                    return "Darts are stored inside the weapon; no worn ammunition is needed."
                        .into();
                }
                if rule.family.style == "Ranged" && rule.family.ammo.is_empty() {
                    return "This weapon supplies its own ammunition; the worn ammo slot is optional.".into();
                }
            }
        }
        return "No item assigned to this slot.".into();
    };
    if shared(p) {
        return "Shared loadout item; ownership, levels and benefits are unverified.".into();
    }
    if o.gear_overrides.get(&format!("{style}:{slot}")) == Some(&id) {
        return format!("You chose this item for {style}.");
    }
    if let Some(reason) = marginal(snapshot, o, p, phase, slot, set, style, &label) {
        return reason;
    }
    let styles = p
        .phases
        .iter()
        .filter(|part| part.equipment.get(slot) == Some(&Some(id)))
        .map(|part| rules::base_style(&part.style))
        .collect::<BTreeSet<_>>();
    if styles.len() > 1 {
        return format!(
            "Worn across {} styles, avoiding an extra {} switch.",
            styles.len(),
            planner::SLOT_KEYS.get(slot).unwrap_or(&"gear")
        );
    }
    if slot == 3 {
        if rules::weapon(id).is_some_and(|r| r.permits(style)) {
            if let Some(dps) = damage(snapshot, o, set, style, &targets(o, p, phase, style)) {
                return format!(
                    "Provides {style} attacks for {label}: {} modeled DPS.",
                    range(&dps)
                );
            }
            return format!(
                "Provides {style} attacks for {label}; damage benefits are unverified."
            );
        }
        return format!("Equipped for {label}; this weapon/style combination is unverified.");
    }
    if guidance::item_tier(style, slot, id, o, set).is_some() {
        return format!(
            "A reviewed {style} option for this encounter. {}",
            stat_fact(id, style)
                .unwrap_or_else(|| "Its complete-set benefit depends on the attack window.".into())
        );
    }
    if let Some(fact) = stat_fact(id, style) {
        return format!("For {label}. {fact}");
    }
    format!("Part of the {style} setup for {label}; an individual damage benefit is unverified.")
}
pub fn equipment(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    phase: Option<usize>,
    slot: usize,
) -> String {
    short(&equipment_reason(snapshot, o, p, phase, slot), 260)
}
/// Carried switches explain their actual phase; supplies retain their packer role.
pub fn packed(snapshot: &Snapshot, o: &Options, p: &Plan, item: &Packed) -> String {
    if shared(p) {
        return "Shared inventory item; ownership, levels and benefits are unverified.".into();
    }
    if o.inventory_overrides.values().any(|id| *id == item.id) {
        return "You selected this inventory item.".into();
    }
    for (index, phase) in p.phases.iter().enumerate() {
        if let Some(slot) = phase.equipment.iter().enumerate().find_map(|(slot, id)| {
            (*id == Some(item.id) && p.equipment.get(slot) != Some(id)).then_some(slot)
        }) {
            return equipment(snapshot, o, p, Some(index), slot);
        }
    }
    if !item.purpose.trim().is_empty() {
        return short(&item.purpose, 180);
    }
    "Carried supply; its specific encounter role is unverified.".into()
}
