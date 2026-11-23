//! Owned common armour and joint Void/Justiciar alternatives for switch pruning.
use crate::{
    account::Snapshot,
    efficient::{self, Candidate, Tolerances},
    planner, rules,
    switch_pruning::Removal,
    tradeoffs,
};
use std::collections::{BTreeMap, BTreeSet};

fn styles(c: &Candidate) -> BTreeMap<String, Vec<Option<i32>>> {
    let mut sets = BTreeMap::from([(
        planner::primary_style(&c.budget.options).into(),
        c.budget.plan.equipment.clone(),
    )]);
    for w in &c.budget.windows {
        sets.entry(w.style.clone())
            .or_insert_with(|| w.equipment.clone());
    }
    sets
}
fn decision(
    current: &Candidate,
    pins: BTreeMap<String, i32>,
    label: String,
    replacement: String,
) -> Removal {
    let mut remove = BTreeSet::new();
    let mut add = BTreeSet::new();
    for (style, set) in styles(current) {
        for slot in 0..14 {
            if let Some(id) = pins.get(&format!("{style}:{slot}")) {
                if let Some(old) = set[slot] {
                    remove.insert(old);
                }
                if *id > 0 {
                    add.insert(*id);
                }
            }
        }
    }
    remove.retain(|id| !add.contains(id));
    let protected: BTreeSet<_> = current
        .budget
        .plan
        .inventory
        .iter()
        .filter(|p| !p.purpose.to_lowercase().contains("switch"))
        .map(|p| p.id)
        .collect();
    remove.retain(|id| !protected.contains(id));
    Removal {
        item: 0,
        pins,
        remove,
        add,
        label,
        replacement,
    }
}
fn preview(
    snapshot: &Snapshot,
    current: &Candidate,
    full: &Candidate,
    limits: Tolerances,
    trial: &Removal,
) -> Option<(bool, f64)> {
    let mut budget = current.budget.clone();
    for (key, id) in &trial.pins {
        if full
            .budget
            .options
            .gear_overrides
            .get(key)
            .is_some_and(|pin| pin != id)
        {
            return None;
        }
    }
    budget.options.switch_pool.as_mut()?.extend(&trial.add);
    for id in &trial.remove {
        budget.options.switch_pool.as_mut()?.remove(id);
    }
    budget.options.switch_pins.extend(trial.pins.clone());
    for w in &mut budget.windows {
        for slot in 0..14 {
            if let Some(id) = trial.pins.get(&format!("{}:{slot}", w.style)) {
                w.equipment[slot] = (*id > 0).then_some(*id);
            }
        }
        if w.equipment[3].is_some_and(|id| planner::gear(id).is_some_and(|g| g.two_handed))
            && w.equipment[5].is_some()
        {
            return None;
        }
        w.metrics = tradeoffs::measure(
            snapshot,
            &budget.options,
            &w.equipment,
            &w.style,
            w.target_id,
        );
    }
    let c = Candidate::new(budget);
    let time = c.totals.as_ref()?.seconds;
    Some((efficient::qualifies(&c, full, limits), time))
}
/// Screen all distinct owned variants; repack the best three qualifying choices
/// and the best nonqualifying comparison per slot. Joint sets are tried whole.
pub(crate) fn candidates(
    snapshot: &Snapshot,
    current: &Candidate,
    full: &Candidate,
    limits: Tolerances,
) -> Vec<Removal> {
    let sets = styles(current);
    if sets.len() < 2 {
        return vec![];
    }
    let Some(member) = snapshot.selected() else {
        return vec![];
    };
    let mut available = full.budget.options.clone();
    available.switch_pool = None;
    available.switch_pins.clear();
    let owned = planner::planning_stock(snapshot, &available);
    let first = sets.keys().next().unwrap();
    let mut result = vec![];
    for slot in [0, 1, 2, 4, 5, 7, 9, 10, 12] {
        let choices: BTreeSet<_> = sets.values().map(|set| set[slot]).collect();
        if choices.len() < 2 {
            continue;
        }
        let ids = crate::set_search::variants(
            owned.keys().copied(),
            slot,
            first,
            member,
            &owned,
            &available,
        );
        let mut scored = vec![];
        for id in ids {
            if choices.contains(&Some(id)) {
                continue;
            }
            let pins = sets
                .keys()
                .map(|style| (format!("{style}:{slot}"), id))
                .collect();
            let trial = decision(
                current,
                pins,
                format!("Shared {}", planner::SLOTS[slot]),
                planner::item_name(id),
            );
            if let Some((fits, seconds)) = preview(snapshot, current, full, limits, &trial) {
                scored.push((fits, seconds, trial));
            }
        }
        scored.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut good = 0;
        let mut comparison = false;
        for (fits, _, trial) in scored {
            if fits && good < 3 {
                good += 1;
                result.push(trial);
            } else if !fits && !comparison {
                comparison = true;
                result.push(trial);
            }
        }
    }
    // Helmets may differ by style, while the set's other pieces must be shared.
    type Key = (String, Vec<(usize, Option<i32>)>);
    let mut groups = BTreeMap::<Key, BTreeMap<String, BTreeMap<String, i32>>>::new();
    for (style, set) in &sets {
        for seed in crate::set_search::candidates(set, style, member, &owned, &available) {
            if seed.slots.contains(&3) || seed.slots.contains(&13) {
                continue;
            }
            let family = if rules::set_effect(
                &seed.equipment,
                &format!("void_{}", rules::base_style(style).to_lowercase()),
            ) {
                "Void armour"
            } else if rules::set_effect(&seed.equipment, "justiciar") {
                "Justiciar armour"
            } else {
                continue;
            };
            let key = (
                family.into(),
                seed.slots
                    .iter()
                    .filter(|slot| **slot != 0)
                    .map(|slot| (*slot, seed.equipment[*slot]))
                    .collect(),
            );
            let pins = seed
                .slots
                .iter()
                .map(|slot| {
                    (
                        format!("{style}:{slot}"),
                        seed.equipment[*slot].unwrap_or(0),
                    )
                })
                .collect();
            groups
                .entry(key)
                .or_default()
                .entry(style.clone())
                .or_insert(pins);
        }
    }
    for ((family, _), group) in groups {
        if sets.keys().any(|style| !group.contains_key(style)) {
            continue;
        }
        let pins = group.into_values().flatten().collect();
        result.push(decision(current, pins, family, "Complete owned set".into()));
    }
    let mut seen = BTreeSet::new();
    result.retain(|r| seen.insert(r.pins.clone()));
    result
}
