//! Prefer compact actual plans near the fastest tested encounter workload.
//! This is a bounded recommendation over existing plans, not an exact BIS proof.
use crate::{
    encounter_summary::{self, Totals},
    tradeoffs::Budget,
};

pub const DEFAULT_TOLERANCE_PERCENT: u32 = 3;
pub const MAX_TOLERANCE_PERCENT: u32 = 10;
pub fn default_tolerance() -> u32 {
    DEFAULT_TOLERANCE_PERCENT
}
pub fn is_default_tolerance(value: &u32) -> bool {
    *value == DEFAULT_TOLERANCE_PERCENT
}
pub fn is_zero_tolerance(value: &u32) -> bool {
    *value == 0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tolerances {
    pub dps_loss: u32,
    pub damage_taken: u32,
    pub prayer_drain: u32,
}
impl Default for Tolerances {
    fn default() -> Self {
        Self {
            dps_loss: DEFAULT_TOLERANCE_PERCENT,
            damage_taken: 0,
            prayer_drain: 0,
        }
    }
}
impl Tolerances {
    pub fn from_options(o: &crate::planner::Options) -> Self {
        Self {
            dps_loss: o.switch_tolerance,
            damage_taken: o.damage_tolerance,
            prayer_drain: o.prayer_tolerance,
        }
    }
}
impl std::fmt::Display for Tolerances {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}% DPS loss, {}% extra damage and {}% extra prayer drain",
            self.dps_loss, self.damage_taken, self.prayer_drain
        )
    }
}
pub fn cache_key(snapshot: &crate::account::Snapshot, options: &crate::planner::Options) -> String {
    let mut options = options.clone();
    // Trials set their own ceiling and repack both bags. Applying a lower cap
    // must not move the benchmark. Threshold changes restart the pruning path.
    options.max_switches = None;
    options.switch_pool = None;
    options.switch_pins.clear();
    options.simulation = Default::default();
    options.inventory_overrides.clear();
    options.inventory_quantities.clear();
    serde_json::to_string(&(snapshot, options)).unwrap_or_default()
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub budget: Budget,
    pub totals: Option<Totals>,
    pub signature: Vec<(String, u64)>,
    pub covered: usize,
    pub total: usize,
    pub omissions: Vec<String>,
}
impl Candidate {
    pub fn new(budget: Budget) -> Self {
        let overview = encounter_summary::budgets(&budget, &budget);
        let signature = overview
            .workloads
            .iter()
            .map(|w| (w.name.clone(), w.hp.to_bits()))
            .collect();
        Self {
            budget,
            totals: overview.left,
            signature,
            covered: overview.covered_windows,
            total: overview.total_windows,
            omissions: overview.omissions,
        }
    }
    pub(crate) fn usable(&self) -> bool {
        self.budget.feasible
            && self.budget.prep_blockers.is_empty()
            && self.covered > 0
            && self.totals.as_ref().is_some_and(|m| {
                m.seconds.is_finite() && m.seconds > 0.0 && m.prayer_points.is_finite()
            })
    }
}
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub candidates: Vec<Candidate>,
    pub best: Option<usize>,
    pub compact: Option<usize>,
    pub reason: Option<String>,
    pub removed: Vec<i32>,
    pub added: Vec<i32>,
    pub trials: Vec<crate::switch_pruning::Trial>,
}
fn within(value: f64, reference: f64, tolerance: u32) -> bool {
    value.is_finite()
        && reference.is_finite()
        && value <= reference * (1.0 + tolerance.min(MAX_TOLERANCE_PERCENT) as f64 / 100.0) + 1e-8
}
/// Do not buy fewer switches by losing protection in a known attack scenario.
/// Even when another unknown phase prevents an incoming encounter total, retain
/// every known per-window attack guard. Unknowns remain explicitly unverified.
fn incoming_guard(candidate: &Candidate, best: &Candidate, tolerance: u32) -> bool {
    if let Some((_, reference)) = best.totals.as_ref().and_then(|m| m.incoming_damage) {
        let Some((_, value)) = candidate.totals.as_ref().and_then(|m| m.incoming_damage) else {
            return false;
        };
        if !within(value, reference, tolerance) {
            return false;
        }
    }
    for reference in &best.budget.windows {
        let Some(window) = candidate
            .budget
            .windows
            .iter()
            .find(|w| w.target_id == reference.target_id && w.style == reference.style)
        else {
            return false;
        };
        for attack in reference
            .metrics
            .incoming
            .iter()
            .filter(|a| a.dps.is_some())
        {
            if !window
                .metrics
                .incoming
                .iter()
                .find(|a| a.attack == attack.attack)
                .and_then(|a| a.dps)
                .is_some_and(|value| within(value, attack.dps.unwrap(), tolerance))
            {
                return false;
            }
        }
    }
    true
}
pub fn qualifies(candidate: &Candidate, best: &Candidate, limits: Tolerances) -> bool {
    if !candidate.usable() || candidate.signature != best.signature {
        return false;
    }
    let (Some(a), Some(b)) = (&candidate.totals, &best.totals) else {
        return false;
    };
    // Workloads are identical, so effective DPS ratio is inverse attack time.
    let retained = 1.0 - limits.dps_loss.min(MAX_TOLERANCE_PERCENT) as f64 / 100.0;
    b.seconds / a.seconds + 1e-10 >= retained
        // Check each phase's drain directly. A slower phase can shift the
        // encounter's time-weighted average without changing prayer bonuses.
        && best.budget.windows.iter().all(|reference| {
            if reference.metrics.defence.is_empty() {
                return true;
            }
            candidate
                .budget
                .windows
                .iter()
                .find(|w| w.target_id == reference.target_id && w.style == reference.style)
                .is_some_and(|w| {
                    !w.metrics.defence.is_empty()
                        && within(
                            w.metrics.prayer_per_minute,
                            reference.metrics.prayer_per_minute,
                            limits.prayer_drain,
                        )
                })
        })
        && incoming_guard(candidate, best, limits.damage_taken)
}
pub fn choose(candidates: Vec<Candidate>, limits: Tolerances) -> Suggestion {
    let mut result = Suggestion {
        candidates,
        best: None,
        compact: None,
        reason: None,
        removed: vec![],
        added: vec![],
        trials: vec![],
    };
    // Establish coverage before speed. A fast plan cannot win by dropping a
    // difficult window. On equal coverage, use the largest tested cap's scope.
    let basis = result
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.usable())
        .max_by_key(|(_, c)| (c.covered, c.budget.cap))
        .map(|(i, _)| i);
    let Some(basis) = basis else {
        result.reason = Some(
            "No supported, prepared setup fits your bank and packing limits. Review Preparation."
                .into(),
        );
        return result;
    };
    let signature = &result.candidates[basis].signature;
    result.best = result
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.usable() && &c.signature == signature)
        .min_by(|(_, a), (_, b)| {
            let (a, b) = (a.totals.as_ref().unwrap(), b.totals.as_ref().unwrap());
            a.seconds
                .total_cmp(&b.seconds)
                .then_with(|| a.prayer_points.total_cmp(&b.prayer_points))
        })
        .map(|(i, _)| i);
    let best = &result.candidates[result.best.unwrap()];
    // Also preserve known defensive gains in the most flexible comparable plan;
    // a fast but fragile benchmark must not erase the value of that equipment.
    result.compact = result
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            qualifies(c, best, limits)
                && incoming_guard(c, &result.candidates[basis], limits.damage_taken)
        })
        .min_by(|(_, a), (_, b)| {
            (
                a.budget.max_pair,
                a.budget.carried_gear,
                a.budget.max_inventory_space,
                a.budget.cap,
            )
                .cmp(&(
                    b.budget.max_pair,
                    b.budget.carried_gear,
                    b.budget.max_inventory_space,
                    b.budget.cap,
                ))
                .then_with(|| {
                    a.totals
                        .as_ref()
                        .unwrap()
                        .prayer_points
                        .total_cmp(&b.totals.as_ref().unwrap().prayer_points)
                })
        })
        .map(|(i, _)| i);
    if result.compact.is_none() {
        result.reason = Some(format!(
            "No tested setup meets {limits}. Adjust the thresholds or compare the tradeoffs."
        ));
    }
    result
}
