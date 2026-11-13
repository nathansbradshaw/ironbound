//! User confirmations remain distinct from verified account/encounter data.
use crate::{
    account::Snapshot,
    planner::{Options, Plan},
};

pub const CONFIRMATIONS: [(&str, &str); 4] = [
    ("access", "I checked encounter access and quests"),
    ("unlocks", "I checked spellbook, spell and prayer unlocks"),
    ("charges", "I checked equipment, tome and teleport charges"),
    (
        "supplies",
        "I checked ammunition, casts, food and potions for this trip",
    ),
];

/// An invalidation token, not a security hash. Include bank/levels and every
/// option and the resulting packing list so a changed recommendation after an
/// app/data update also requires a new check.
pub fn token(snapshot: &Snapshot, options: &Options) -> String {
    let mut options = options.clone();
    options.confirmations.clear();
    // An unapplied recommendation preference does not change trip preparation.
    options.switch_tolerance = crate::efficient::DEFAULT_TOLERANCE_PERCENT;
    options.damage_tolerance = 0;
    options.prayer_tolerance = 0;
    let plan = crate::planner::plan(snapshot, &options);
    let phases: Vec<_> = plan
        .phases
        .iter()
        .map(|phase| (&phase.name, &phase.style, &phase.equipment))
        .collect();
    let bytes = serde_json::to_vec(&(
        snapshot,
        options,
        &plan.equipment,
        &plan.inventory,
        phases,
        &plan.loaded_ammo,
        &plan.spell,
    ))
    .expect("serializable account/options");
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

pub fn confirmed(snapshot: &Snapshot, options: &Options, id: &str) -> bool {
    options
        .confirmations
        .get(id)
        .is_some_and(|saved| *saved == token(snapshot, options))
}

pub fn pending(snapshot: &Snapshot, options: &Options) -> usize {
    let token = token(snapshot, options);
    CONFIRMATIONS
        .iter()
        .filter(|(id, _)| options.confirmations.get(*id) != Some(&token))
        .count()
}

pub fn summary(snapshot: &Snapshot, options: &Options, plan: &Plan) -> &'static str {
    if !plan.missing.is_empty() || plan.checks.iter().any(|c| c.state == Some(false)) {
        "Preparation needed"
    } else if plan.checks.iter().any(|c| c.state.is_none()) {
        "Confirm your levels"
    } else if pending(snapshot, options) > 0 {
        "Manual checks needed"
    } else {
        "Manual checks confirmed · mechanics may be partial"
    }
}
