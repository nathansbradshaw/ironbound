//! User confirmations remain distinct from verified account/encounter data.
use crate::{
    account::Snapshot,
    planner::{Options, Plan},
};

pub const CONFIRMATIONS: [(&str, &str); 4] = [
    (
        "access",
        "I have the quests and access needed for this encounter",
    ),
    (
        "unlocks",
        "My spellbook, spells and selected prayers are unlocked",
    ),
    (
        "charges",
        "My weapons, armour, tomes and teleports have enough charges/pages",
    ),
    (
        "supplies",
        "I checked ammunition, casts, food and potions for my intended trip",
    ),
];

/// An invalidation token, not a security hash. Include bank/levels and every
/// option so a changed account, supply budget or encounter requires a new check.
pub fn token(snapshot: &Snapshot, options: &Options) -> String {
    let mut options = options.clone();
    options.confirmations.clear();
    let bytes = serde_json::to_vec(&(snapshot, options)).expect("serializable account/options");
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
