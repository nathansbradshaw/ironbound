//! Owned, complete-set what-ifs for the displayed phase. Never a global BIS claim.
use crate::{
    account::{Member, Snapshot},
    optimization,
    planner::{self, Boss, Options, Packed, Plan},
    recommendation, rules,
    tradeoffs::{self, Metrics},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Alternative {
    pub id: i32,
    /// Exact equipment context used for this what-if, including carried switches.
    pub style: String,
    pub slot: usize,
    pub dps_delta: Option<f64>,
    /// Alternative minus current continuous drain; negative means less drain.
    pub prayer_delta: f64,
    pub defence_delta: BTreeMap<String, i32>,
    pub max_changes: usize,
    pub fits_budget: bool,
    pub note: Option<String>,
}
struct Context<'a> {
    set: &'a [Option<i32>],
    style: String,
    target: i32,
}
fn style(set: &[Option<i32>], label: &str) -> String {
    if rules::STYLES.contains(&label) {
        return label.into();
    }
    set.get(3)
        .copied()
        .flatten()
        .and_then(rules::weapon)
        .map(|r| r.family.style.clone())
        .unwrap_or_else(|| label.into())
}
fn context<'a>(o: &Options, p: &'a Plan, phase: Option<usize>) -> Option<Context<'a>> {
    let (set, label, name) = match phase {
        Some(i) => {
            let phase = p.phases.get(i)?;
            (
                phase.equipment.as_slice(),
                phase.style.as_str(),
                Some(phase.name.as_str()),
            )
        }
        None => (p.equipment.as_slice(), planner::primary_style(o), None),
    };
    if set.len() != 14 {
        return None;
    }
    let style = style(set, label);
    let target = if o.boss == Boss::Custom {
        phase
            .and_then(|i| o.targets.get(i))
            .filter(|t| t.style == style)
            .or_else(|| o.targets.iter().find(|t| t.style == style))
            .map(|t| t.id)
    } else {
        let available: Vec<_> = recommendation::targets(o)
            .into_iter()
            .filter(|(_, s)| rules::base_style(s) == rules::base_style(&style))
            .collect();
        match name {
            None => available.first().map(|(id, _)| *id),
            Some(name) => available
                .iter()
                .find(|(id, _)| {
                    planner::monster(*id).is_some_and(|m| {
                        name.split('·').any(|label| {
                            m.name
                                .to_lowercase()
                                .starts_with(&label.trim().to_lowercase())
                        })
                    })
                })
                .map(|(id, _)| *id),
        }
    }
    .unwrap_or(0);
    Some(Context { set, style, target })
}
fn equipped(set: &[Option<i32>], shots: u32) -> BTreeMap<i32, u32> {
    let mut quantities = BTreeMap::new();
    for (slot, id) in set.iter().enumerate() {
        if let Some(id) = id {
            *quantities.entry(*id).or_default() += planner::equipped_quantity(slot, *id, shots);
        }
    }
    quantities
}
fn maximum_gear(sets: &[Vec<Option<i32>>], shots: u32) -> BTreeMap<i32, u32> {
    let mut need = BTreeMap::new();
    for set in sets {
        for (id, q) in equipped(set, shots) {
            need.entry(id)
                .and_modify(|n: &mut u32| *n = (*n).max(q))
                .or_insert(q);
        }
    }
    need
}
fn all_sets(p: &Plan) -> Vec<Vec<Option<i32>>> {
    std::iter::once(p.equipment.clone())
        .chain(p.phases.iter().map(|p| p.equipment.clone()))
        .collect()
}
fn compatible(
    set: &[Option<i32>],
    style: &str,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
) -> bool {
    let Some(weapon) = set.get(3).copied().flatten() else {
        return false;
    };
    let Some(rule) = rules::weapon(weapon) else {
        return false;
    };
    if !rule.permits(style) {
        return false;
    }
    if rule.family.style == "Ranged" {
        if !rule.family.ammo.is_empty()
            && !set[13].is_some_and(|id| {
                rule.family.ammo.contains(&id)
                    && owned.get(&id).copied().unwrap_or(0) >= o.combat.ammo_shots
            })
        {
            return false;
        }
        if !rule.family.internal_ammo.is_empty() {
            let Some(dart) = rules::internal_ammo(m, weapon, &o.combat) else {
                return false;
            };
            if !planner::can_use(dart, m)
                || !(o.combat.loaded_ammo_confirmed && o.combat.loaded_dart == Some(dart))
                    && owned.get(&dart).copied().unwrap_or(0) < o.combat.ammo_shots
            {
                return false;
            }
        }
    }
    rules::casting_supported_configured(m, set, owned, o.combat.spell_casts, &o.combat)
}
fn spell_supplies(
    set: &[Option<i32>],
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
    target: i32,
) -> Vec<(i32, u32)> {
    if !set
        .get(3)
        .copied()
        .flatten()
        .and_then(rules::weapon)
        .is_some_and(|r| r.family.style == "Magic")
    {
        return vec![];
    }
    let spell = planner::monster(target)
        .map(|t| rules::spell_name_for_target(m, set, owned, &o.combat, t, o.raid_level))
        .unwrap_or_else(|| rules::spell_name_configured(m, set, owned, &o.combat));
    rules::rune_supplies_for_set(spell, o.combat.spell_casts, set)
}
fn budget(
    p: &Plan,
    o: &Options,
    c: &Context<'_>,
    slot: usize,
    trial: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    m: &Member,
) -> (usize, bool, Vec<String>) {
    let before = all_sets(p);
    let mut after = before.clone();
    let mut notes = vec![];
    let affected: Vec<_> = before
        .iter()
        .enumerate()
        .filter(|(i, set)| {
            let label = if *i == 0 {
                planner::primary_style(o)
            } else {
                p.phases[*i - 1].style.as_str()
            };
            style(set, label) == c.style && set.get(slot) == c.set.get(slot)
        })
        .map(|(i, _)| i)
        .collect();
    for index in affected {
        for changed in 0..14 {
            if c.set[changed] != trial[changed] {
                after[index][changed] = trial[changed];
            }
        }
        if after[index][3]
            .and_then(planner::gear)
            .is_some_and(|g| g.two_handed)
        {
            after[index][5] = None;
        }
    }
    let max_changes = after
        .iter()
        .flat_map(|a| after.iter().map(move |b| optimization::changes(a, b)))
        .max()
        .unwrap_or(0);
    if o.max_switches.is_some_and(|cap| max_changes > cap) {
        notes.push(format!("Needs {max_changes} changed slots per switch."));
    }
    let extra: BTreeSet<_> = after
        .iter()
        .skip(1)
        .flat_map(|s| {
            s.iter()
                .enumerate()
                .filter_map(|(slot, id)| id.filter(|id| after[0][slot] != Some(*id)))
        })
        .collect();
    if (o.max_switches.is_some() || p.optimization.is_some()) && extra.len() > o.switch_slots {
        notes.push(format!("Needs {} carried gear slots.", extra.len()));
    }
    let old_gear = maximum_gear(&before, o.combat.ammo_shots);
    let new_gear = maximum_gear(&after, o.combat.ammo_shots);
    let mut pool = equipped(&p.equipment, p.ammo_shots);
    for item in p.inventory.iter().filter(|p| p.id > 0 && p.quantity > 0) {
        *pool.entry(item.id).or_default() += item.quantity;
    }
    // Keep every supply and spare item; remove only the old gear requirement
    // superseded by this complete style replacement, then reuse carried items.
    for (id, q) in old_gear {
        let remove = q.saturating_sub(new_gear.get(&id).copied().unwrap_or(0));
        let n = pool.entry(id).or_default();
        *n = n.saturating_sub(remove);
    }
    for (id, q) in new_gear {
        let n = pool.entry(id).or_default();
        *n = (*n).max(q);
    }
    let stock_ok = pool
        .iter()
        .all(|(id, q)| *q <= owned.get(id).copied().unwrap_or(0));
    if !stock_ok {
        notes.push("Carried stock needs a full rebuild.".into());
    }
    let mut maximum = 0;
    for set in &after {
        let mut bag = pool.clone();
        for (id, q) in equipped(set, o.combat.ammo_shots) {
            let Some(n) = bag.get_mut(&id) else { continue };
            *n = n.saturating_sub(q);
        }
        let count: usize = bag
            .iter()
            .map(|(id, q)| {
                if planner::is_stackable(*id) {
                    usize::from(*q > 0)
                } else {
                    *q as usize
                }
            })
            .sum();
        maximum = maximum.max(count);
    }
    if maximum > 28 {
        notes.push(format!("Needs {maximum}/28 bag slots; supplies were kept."));
    }
    if spell_supplies(c.set, m, owned, o, c.target) != spell_supplies(trial, m, owned, o, c.target)
    {
        notes.push("Spell supplies change; rebuild to check the bag.".into());
    }
    let loaded_weapons: BTreeSet<_> = after
        .iter()
        .filter_map(|s| s[3])
        .filter(|id| rules::weapon(*id).is_some_and(|r| !r.family.internal_ammo.is_empty()))
        .collect();
    let mut load = BTreeMap::<i32, u32>::new();
    for weapon in &loaded_weapons {
        if let Some(dart) = rules::internal_ammo(m, *weapon, &o.combat) {
            if !(o.combat.loaded_ammo_confirmed && o.combat.loaded_dart == Some(dart)) {
                *load.entry(dart).or_default() += o.combat.ammo_shots;
            }
        }
    }
    if load.iter().any(|(id, q)| {
        pool.get(id).copied().unwrap_or(0).saturating_add(*q) > owned.get(id).copied().unwrap_or(0)
    }) {
        notes.push("Dart stock is also needed inside the weapon.".into());
    }
    if loaded_weapons.len() > 1 && o.combat.loaded_ammo_confirmed {
        notes.push("Rebuild to verify each loaded weapon's contents.".into());
    }
    (max_changes, notes.is_empty(), notes)
}
fn add_note(notes: &mut Vec<String>, note: String) {
    if !notes.contains(&note) {
        notes.push(note);
    }
}
fn delta(current: &Metrics, trial: &Metrics) -> (Option<f64>, f64, BTreeMap<String, i32>) {
    let dps = current
        .dps
        .zip(trial.dps)
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .map(|(a, b)| b - a);
    let defence = current
        .defence
        .iter()
        .filter_map(|(key, value)| trial.defence.get(key).map(|v| (key.clone(), v - value)))
        .collect();
    (
        dps,
        trial.prayer_per_minute - current.prayer_per_minute,
        defence,
    )
}
fn defensive_gain(a: &Alternative) -> i32 {
    a.defence_delta.values().copied().max().unwrap_or(0).max(0)
}
fn dominates(a: &Alternative, b: &Alternative) -> bool {
    if a.fits_budget != b.fits_budget {
        return false;
    }
    let (Some(ad), Some(bd)) = (a.dps_delta, b.dps_delta) else {
        return false;
    };
    if ad + 1e-8 < bd
        || a.prayer_delta > b.prayer_delta + 1e-8
        || a.defence_delta.keys().collect::<Vec<_>>() != b.defence_delta.keys().collect::<Vec<_>>()
    {
        return false;
    }
    if a.defence_delta.iter().any(|(k, v)| *v < b.defence_delta[k]) {
        return false;
    }
    ad > bd + 1e-8
        || a.prayer_delta < b.prayer_delta - 1e-8
        || a.defence_delta.iter().any(|(k, v)| *v > b.defence_delta[k])
}
fn equivalent(a: &Alternative, b: &Alternative) -> bool {
    a.fits_budget == b.fits_budget
        && a.max_changes == b.max_changes
        && a.defence_delta == b.defence_delta
        && (a.prayer_delta - b.prayer_delta).abs() < 1e-8
        && match (a.dps_delta, b.dps_delta) {
            (Some(a), Some(b)) => (a - b).abs() < 1e-8,
            (None, None) => a.note == b.note,
            _ => false,
        }
}
fn order(a: &Alternative, b: &Alternative) -> std::cmp::Ordering {
    b.fits_budget
        .cmp(&a.fits_budget)
        .then_with(|| {
            b.dps_delta
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&a.dps_delta.unwrap_or(f64::NEG_INFINITY))
        })
        .then_with(|| a.prayer_delta.total_cmp(&b.prayer_delta))
        .then_with(|| defensive_gain(b).cmp(&defensive_gain(a)))
        .then(a.id.cmp(&b.id))
}
fn choose(mut rows: Vec<Alternative>, limit: usize) -> Vec<Alternative> {
    rows.sort_by(order);
    let mut unique: Vec<Alternative> = vec![];
    for row in rows {
        if !unique.iter().any(|a| a.id == row.id || equivalent(a, &row)) {
            unique.push(row);
        }
    }
    let mut rows: Vec<_> = unique
        .iter()
        .filter(|b| !unique.iter().any(|a| a.id != b.id && dominates(a, b)))
        .cloned()
        .collect();
    rows.sort_by(order);
    let mut selected = vec![];
    while !rows.is_empty() && selected.len() < limit.min(4) {
        let index = if selected.is_empty() {
            0
        } else {
            let fit = rows[0].fits_budget;
            let already_prayer = selected
                .iter()
                .any(|a: &Alternative| a.prayer_delta < -0.001);
            let already_defence = selected.iter().any(|a| defensive_gain(a) > 0);
            rows.iter()
                .enumerate()
                .filter(|(_, a)| a.fits_budget == fit)
                .filter(|(_, a)| {
                    !already_prayer && a.prayer_delta < -0.001
                        || !already_defence && defensive_gain(a) > 0
                })
                .min_by(|(_, a), (_, b)| {
                    a.prayer_delta
                        .total_cmp(&b.prayer_delta)
                        .then_with(|| defensive_gain(b).cmp(&defensive_gain(a)))
                        .then_with(|| order(a, b))
                })
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        selected.push(rows.remove(index));
    }
    selected
}
/// Compare owned, usable one-slot alternatives in the actual displayed phase.
/// Weapon trials may also remove a two-handed shield and choose compatible ammo;
/// these changes are included in the reported complete-plan budget.
pub fn equipment(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    phase: Option<usize>,
    slot: usize,
    limit: usize,
) -> Vec<Alternative> {
    if limit == 0
        || slot >= 14
        || planner::SLOT_KEYS[slot].is_empty()
        || p.label.starts_with("Shared setup")
    {
        return vec![];
    }
    let Some(c) = context(o, p, phase) else {
        return vec![];
    };
    let Some(original) = snapshot.selected() else {
        return vec![];
    };
    if slot == 5
        && c.set[3]
            .and_then(planner::gear)
            .is_some_and(|g| g.two_handed)
    {
        return vec![];
    }
    let mut owned = snapshot.available(o.shared);
    owned.retain(|id, _| !o.item_excluded(*id));
    let member = planner::member_with_stock(original, &owned);
    let baseline = tradeoffs::measure(snapshot, o, c.set, &c.style, c.target);
    // Missing equipment stats also make prayer drain unknown. The metric's
    // scalar drain is not an estimate we can honestly subtract in that case.
    if baseline.defence.is_empty() {
        return vec![];
    }
    let mut rows = vec![];
    let mut measured = 0;
    for gear in planner::equipment()
        .iter()
        .filter(|g| {
            g.slot == planner::SLOT_KEYS[slot]
                && owned.get(&g.id).copied().unwrap_or(0) > 0
                && planner::can_use(g.id, &member)
                && c.set[slot] != Some(g.id)
        })
        .take(128)
    {
        let mut trial = c.set.to_vec();
        trial[slot] = Some(gear.id);
        let mut adjustments = vec![];
        if slot == 3 {
            let Some(rule) = rules::weapon(gear.id).filter(|r| r.permits(&c.style)) else {
                continue;
            };
            if gear.two_handed && trial[5].take().is_some() {
                adjustments.push("Two-handed: shield removed.".to_string());
            }
            if rule.family.style == "Ranged"
                && rule.family.ammo.is_empty()
                && trial[13].is_some_and(|id| rules::all_ammo_ids().contains(&id))
            {
                trial[13] = None;
                adjustments.push("Does not use worn ammunition.".into());
            }
            if !rule.family.internal_ammo.is_empty() {
                adjustments.push("Verify the configured loaded darts and charges.".into());
            }
        }
        let ammo: Vec<_> = if slot == 3 && rules::base_style(&c.style) == "Ranged" {
            let allowed = rules::ammo_ids(gear.id);
            if allowed.is_empty() {
                vec![trial[13]]
            } else if trial[13].is_some_and(|id| {
                allowed.contains(&id) && owned.get(&id).copied().unwrap_or(0) >= o.combat.ammo_shots
            }) {
                vec![trial[13]]
            } else {
                allowed
                    .iter()
                    .copied()
                    .filter(|id| {
                        owned.get(id).copied().unwrap_or(0) >= o.combat.ammo_shots
                            && planner::can_use(*id, &member)
                    })
                    .take(24)
                    .map(Some)
                    .collect()
            }
        } else {
            vec![trial[13]]
        };
        for ammunition in ammo {
            if measured >= 256 {
                break;
            }
            trial[13] = ammunition;
            if slot == 13
                && (!rules::ammo_ids(trial[3].unwrap_or(0)).contains(&gear.id)
                    || owned.get(&gear.id).copied().unwrap_or(0) < o.combat.ammo_shots)
            {
                continue;
            }
            if !compatible(&trial, &c.style, &member, &owned, o) {
                continue;
            }
            measured += 1;
            let metric = tradeoffs::measure(snapshot, o, &trial, &c.style, c.target);
            if metric.defence.is_empty() {
                continue;
            }
            let (dps_delta, prayer_delta, defence_delta) = delta(&baseline, &metric);
            let (max_changes, fits_budget, mut notes) =
                budget(p, o, &c, slot, &trial, &owned, &member);
            for note in &adjustments {
                add_note(&mut notes, note.clone());
            }
            if slot == 3 && trial[13] != c.set[13] {
                add_note(
                    &mut notes,
                    "Compatible worn ammunition also changes.".into(),
                );
            }
            if dps_delta.is_none() {
                let reason = metric
                    .dps_reason
                    .as_ref()
                    .or(baseline.dps_reason.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "Combat coverage is unavailable.".into());
                add_note(&mut notes, format!("DPS delta unavailable: {reason}"));
            }
            if phase.is_none() {
                if let Some(target) = planner::monster(c.target) {
                    add_note(
                        &mut notes,
                        format!("DPS vs {} only; not a whole-encounter score.", target.name),
                    );
                }
            }
            let alternative = Alternative {
                id: gear.id,
                style: c.style.clone(),
                slot,
                dps_delta,
                prayer_delta,
                defence_delta,
                max_changes,
                fits_budget,
                note: (!notes.is_empty()).then(|| notes.join(" ")),
            };
            // An owned backup can be useful even when it loses damage. Rank
            // alternatives against each other, not against a pinned BIS item.
            rows.push(alternative);
        }
        if measured >= 256 {
            break;
        }
    }
    choose(rows, limit)
}
/// Packed equipment is compared where it is actually equipped, not against an
/// invented DPS role for food, supplies, spare specs or loaded ammunition.
pub fn packed(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    item: &Packed,
    limit: usize,
) -> Vec<Alternative> {
    if item.id <= 0 || item.quantity == 0 || p.label.starts_with("Shared setup") {
        return vec![];
    }
    let Some(gear) = planner::gear(item.id) else {
        return vec![];
    };
    let Some(slot) = planner::SLOT_KEYS.iter().position(|key| *key == gear.slot) else {
        return vec![];
    };
    if let Some(phase) = p
        .phases
        .iter()
        .position(|phase| phase.equipment.get(slot) == Some(&Some(item.id)))
    {
        return equipment(snapshot, o, p, Some(phase), slot, limit);
    }
    if p.equipment.get(slot) == Some(&Some(item.id)) {
        return equipment(snapshot, o, p, None, slot, limit);
    }
    vec![]
}
