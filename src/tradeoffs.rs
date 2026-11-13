//! Compare actual repacked plans under two switch caps. Unknown combat coverage
//! remains unknown; these are attack-window comparisons, not trip simulations.
use crate::{
    account::Snapshot,
    combat, defence, optimization,
    planner::{self, Options, Plan},
    prayer, recommendation, rules,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug)]
pub struct Incoming {
    pub attack: String,
    pub dps: Option<f64>,
    pub reason: Option<String>,
    pub healing_per_second: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Metrics {
    pub dps: Option<f64>,
    pub dps_reason: Option<String>,
    pub defence: BTreeMap<String, i32>,
    pub prayer_per_minute: f64,
    pub incoming: Vec<Incoming>,
    pub attack: Option<combat::ResultStats>,
}
#[derive(Clone, Debug)]
pub struct Window {
    pub target_id: i32,
    pub name: String,
    pub style: String,
    pub equipment: Vec<Option<i32>>,
    pub metrics: Metrics,
}
#[derive(Clone, Debug)]
pub struct Budget {
    pub cap: usize,
    pub options: Options,
    pub plan: Plan,
    /// Whether the optimizer actually produced sets satisfying this budget.
    /// Preparation blockers are reported separately and may still prevent export.
    pub feasible: bool,
    pub max_pair: usize,
    pub carried_gear: usize,
    pub inventory_space: usize,
    pub max_inventory_space: usize,
    pub prep_blockers: Vec<String>,
    pub windows: Vec<Window>,
    pub notes: Vec<String>,
    pub unmodelled_phases: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Comparison {
    pub left: Budget,
    pub right: Budget,
    pub added_gear: Vec<i32>,
    pub removed_gear: Vec<i32>,
    pub notes: Vec<String>,
}
fn stock(snapshot: &Snapshot, o: &Options) -> BTreeMap<i32, u32> {
    let mut owned = snapshot.available(o.shared);
    owned.retain(|id, _| !o.excluded.contains(id));
    owned
}
/// Measure exactly the supplied set/style/target. No item is substituted, and
/// base wear requirements remain distinct from boosted combat levels.
pub fn measure(
    snapshot: &Snapshot,
    o: &Options,
    set: &[Option<i32>],
    style: &str,
    target_id: i32,
) -> Metrics {
    let bonuses = combat::bonuses(set);
    let stats_known =
        set.len() == 14 && set.iter().flatten().all(|id| combat::stats(*id).is_some());
    let equipment_defence = ["stab", "slash", "crush", "magic", "ranged"]
        .into_iter()
        .filter(|_| stats_known)
        .map(|key| (key.into(), bonuses.defensive.get(key).copied().unwrap_or(0)))
        .collect();
    let mut result = Metrics {
        dps: None,
        dps_reason: None,
        defence: equipment_defence,
        prayer_per_minute: prayer::drain_per_minute(prayer::bonus(set), &o.combat),
        incoming: vec![],
        attack: None,
    };
    let Some(target) = planner::monster(target_id) else {
        let why = "This target's combat model is unavailable.".to_string();
        result.dps_reason = Some(why.clone());
        result.incoming.push(Incoming {
            attack: "Unverified attacks".into(),
            dps: None,
            reason: Some(why),
            healing_per_second: None,
        });
        return result;
    };
    let Some(selected) = snapshot.selected() else {
        let why = "Select an account with known combat levels.".to_string();
        result.dps_reason = Some(why.clone());
        let attacks = defence::attacks(target);
        for attack in if attacks.is_empty() {
            vec!["Unverified attacks".into()]
        } else {
            attacks
        } {
            result.incoming.push(Incoming {
                attack,
                dps: None,
                reason: Some(why.clone()),
                healing_per_second: None,
            });
        }
        return result;
    };
    let owned = stock(snapshot, o);
    let member = planner::member_with_stock(selected, &owned);
    let equipment_usable = stats_known
        && set
            .iter()
            .flatten()
            .all(|id| planner::can_use(*id, &member));
    if !equipment_usable {
        result.defence.clear();
    }
    let outgoing = if !stats_known {
        Err("Equipment combat stats are unavailable or the equipment layout is invalid.".into())
    } else if set.len() != 14 {
        Err("Equipment needs 14 slots.".into())
    } else if let Some(id) = set
        .iter()
        .flatten()
        .find(|id| !planner::can_use(**id, &member))
    {
        Err(format!(
            "Base equipment requirements are unmet for {}.",
            planner::item_name(*id)
        ))
    } else if !rules::casting_supported_for_style(&member, set, &owned, &o.combat, style) {
        Err("This set needs a usable casting weapon and sufficient spell runes.".into())
    } else {
        combat::calculate_configured(
            &member,
            set,
            style,
            target,
            rules::spell_name_for_target(&member, set, &owned, &o.combat, target, o.raid_level),
            o.raid_level,
            &o.combat,
        )
    };
    match outgoing {
        Ok(attack) => {
            result.dps = Some(attack.dps);
            result.attack = Some(attack);
        }
        Err(reason) => result.dps_reason = Some(reason),
    }
    let attacks = defence::attacks(target);
    if attacks.is_empty() {
        result.incoming.push(Incoming {
            attack: "Unverified attacks".into(),
            dps: None,
            reason: Some(
                "No verified incoming attack scenarios are available for this target.".into(),
            ),
            healing_per_second: None,
        });
    }
    for attack in attacks {
        let measured = if equipment_usable {
            defence::estimate(&member, set, target, &attack, &o.combat)
        } else {
            Err("Equipment combat stats or base wear requirements are unavailable or unmet.".into())
        };
        let (dps, healing_per_second, reason) = match measured {
            Ok(r) => (Some(r.damage_per_second), Some(r.healing_per_second), None),
            Err(reason) => (None, None, Some(reason)),
        };
        result.incoming.push(Incoming {
            attack,
            dps,
            reason,
            healing_per_second,
        });
    }
    result
}
fn permits(set: &[Option<i32>], style: &str) -> bool {
    set.get(3)
        .copied()
        .flatten()
        .and_then(rules::weapon)
        .is_some_and(|r| r.permits(style))
}
/// Resolve composite phase labels through the actual weapon's capabilities.
/// For Akkha's other styles, use the corresponding actual carried phase set.
pub fn set_for_window<'a>(
    p: &'a Plan,
    o: &Options,
    target_id: i32,
    style: &str,
) -> Option<&'a [Option<i32>]> {
    if o.boss == planner::Boss::Custom {
        if let Some((index, target)) = o
            .targets
            .iter()
            .enumerate()
            .find(|(_, t)| t.id == target_id && t.style == style)
        {
            if let Some(phase) = p
                .phases
                .get(index)
                .filter(|phase| phase.name == target.name() && permits(&phase.equipment, style))
            {
                return Some(&phase.equipment);
            }
        }
    }
    let name = planner::monster(target_id)?.name.to_lowercase();
    if let Some(phase) = p.phases.iter().find(|phase| {
        let matches = phase
            .name
            .split('·')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .any(|label| name.starts_with(&label.to_lowercase()));
        matches
            && permits(&phase.equipment, style)
            && (phase.style == style || !rules::STYLES.contains(&phase.style.as_str()))
    }) {
        return Some(&phase.equipment);
    }
    p.phases
        .iter()
        .find(|phase| phase.style == style && permits(&phase.equipment, style))
        .map(|phase| phase.equipment.as_slice())
        .or_else(|| {
            (rules::base_style(planner::primary_style(o)) == rules::base_style(style)
                && permits(&p.equipment, style))
            .then_some(p.equipment.as_slice())
        })
}
fn gear_ids(p: &Plan) -> BTreeSet<i32> {
    p.equipment
        .iter()
        .flatten()
        .copied()
        .chain(
            p.phases
                .iter()
                .flat_map(|phase| phase.equipment.iter().flatten().copied()),
        )
        .collect()
}
pub fn budget(snapshot: &Snapshot, o: &Options, requested_cap: usize) -> Budget {
    let cap = requested_cap.min(11);
    let mut options = o.clone();
    options.max_switches = Some(cap);
    // Gear budgets repack supplies, so stale positional edits cannot bias one side.
    options.inventory_overrides.clear();
    options.inventory_quantities.clear();
    let plan = planner::plan(snapshot, &options);
    let sets = std::iter::once(&plan.equipment)
        .chain(plan.phases.iter().map(|phase| &phase.equipment))
        .collect::<Vec<_>>();
    let max_pair = sets
        .iter()
        .flat_map(|a| sets.iter().map(move |b| optimization::changes(a, b)))
        .max()
        .unwrap_or(0);
    let carried_gear = plan
        .phases
        .iter()
        .flat_map(|phase| {
            phase.equipment.iter().enumerate().filter_map(|(slot, id)| {
                id.filter(|id| plan.equipment.get(slot) != Some(&Some(*id)))
            })
        })
        .collect::<BTreeSet<_>>()
        .len();
    let inventory_space = plan.inventory_slots();
    let max_inventory_space = std::iter::once(inventory_space)
        .chain((0..plan.phases.len()).map(|i| {
            plan.preview_inventory(Some(i))
                .iter()
                .filter(|item| item.id > 0 && item.quantity > 0)
                .count()
        }))
        .max()
        .unwrap_or(inventory_space);
    let feasible = plan.optimization.is_some()
        && sets.iter().all(|set| set.len() == 14 && set[3].is_some())
        && max_pair <= cap
        && carried_gear <= options.switch_slots.min(20)
        && max_inventory_space <= 28;
    let mut prep_blockers = plan.missing.clone();
    let mut notes = vec![];
    if requested_cap > 11 {
        notes.push(format!(
            "Requested {requested_cap} changes; the supported maximum is 11."
        ));
    }
    if !feasible {
        let why = "No verified plan satisfies this switch and carried-gear budget; unconstrained fallback metrics are withheld.";
        notes.push(why.into());
        if !prep_blockers.iter().any(|s| s.contains("No setup found")) {
            prep_blockers.push(why.into());
        }
    }
    let windows:Vec<Window>=recommendation::targets(&options).into_iter().map(|(target_id,style)| {
        let set=set_for_window(&plan,&options,target_id,style);
        let equipment=set.map(<[_]>::to_vec).unwrap_or_else(||vec![None;14]);
        let mut metrics=measure(snapshot,&options,&equipment,style,target_id);
        if !feasible||set.is_none() {
            let why=if feasible {"No actual planned equipment set supports this attack window."}else{"No verified setup satisfies this budget; fallback damage is not a comparison result."};
            metrics.dps=None;metrics.attack=None;metrics.dps_reason=Some(why.into());metrics.defence.clear();for incoming in &mut metrics.incoming {incoming.dps=None;incoming.healing_per_second=None;incoming.reason=Some(why.into());}
        }
        Window {target_id,name:planner::monster(target_id).map(|m|m.name.clone()).unwrap_or_else(||"Unknown target".into()),style:style.into(),equipment,metrics}
    }).collect();
    let mut unmodelled_phases = Vec::new();
    for phase in &plan.phases {
        if !windows.iter().any(|window| {
            phase
                .name
                .split('·')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .any(|label| {
                    window
                        .name
                        .to_lowercase()
                        .starts_with(&label.to_lowercase())
                })
        }) {
            unmodelled_phases.push(phase.name.clone());
            notes.push(format!("{}: no comparison target is registered; this phase's damage and incoming mechanics are not estimated.",phase.name));
        }
    }
    Budget {
        cap,
        options,
        plan,
        feasible,
        max_pair,
        carried_gear,
        inventory_space,
        max_inventory_space,
        prep_blockers,
        windows,
        notes,
        unmodelled_phases,
    }
}
pub fn compare(snapshot: &Snapshot, o: &Options, left_cap: usize, right_cap: usize) -> Comparison {
    let left = budget(snapshot, o, left_cap);
    let right = budget(snapshot, o, right_cap);
    let before = if left.feasible && right.feasible {
        gear_ids(&left.plan)
    } else {
        BTreeSet::new()
    };
    let after = if left.feasible && right.feasible {
        gear_ids(&right.plan)
    } else {
        BTreeSet::new()
    };
    Comparison {added_gear:after.difference(&before).copied().collect(),removed_gear:before.difference(&after).copied().collect(),left,right,notes:vec![
        "Switch limits are maxima; a larger limit need not use every allowed change.".into(),
        "Gear deltas require two feasible budget plans; infeasible fallback equipment is not a valid comparison.".into(),
        "Account, equipment choices, prayers, boosts, reductions, phase priorities, bag limit and supply counts are held fixed. Positional inventory edits are cleared on both sides for repacking.".into(),
        "Damage and prayer rates describe supported attack windows. Every incoming scenario is shown; unknown damage is never treated as zero. Movement, encounter drains and full-trip survival remain unmodelled.".into(),
    ]}
}
