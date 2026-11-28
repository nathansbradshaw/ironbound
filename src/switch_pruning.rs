//! Search for a smaller packed setup within the encounter's combat allowances.
use crate::{
    account::Snapshot,
    efficient::{Candidate, Suggestion, Tolerances},
    planner::{self, Options},
    tradeoffs,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Trial {
    pub item: i32,
    pub label: String,
    pub replacement: String,
    pub changes: Option<usize>,
    pub keep: i32,
    pub round: usize,
    pub dps_loss: Option<f64>,
    pub within_limits: bool,
}
#[derive(Clone)]
pub(crate) struct Removal {
    pub item: i32,
    pub pins: BTreeMap<String, i32>,
    pub remove: BTreeSet<i32>,
    pub add: BTreeSet<i32>,
    pub label: String,
    pub replacement: String,
}
pub struct Search {
    baseline: Candidate,
    current: Candidate,
    original_pins: BTreeSet<i32>,
    limits: Tolerances,
    pending: Vec<Removal>,
    phase_candidates: Option<Vec<Candidate>>,
    best: Option<Candidate>,
    done: bool,
    round: usize,
    pub trials: Vec<Trial>,
}
fn pool(plan: &planner::Plan) -> BTreeSet<i32> {
    std::iter::once(&plan.equipment)
        .chain(plan.phases.iter().map(|p| &p.equipment))
        .flat_map(|s| s.iter().flatten().copied())
        .chain(
            plan.inventory
                .iter()
                .filter(|p| p.quantity > 0 && planner::gear(p.id).is_some())
                .map(|p| p.id),
        )
        .chain(plan.loaded_ammo.iter().map(|ammo| ammo.id))
        .collect()
}
fn gear_changes(full: &planner::Plan, reduced: &planner::Plan) -> (Vec<i32>, Vec<i32>) {
    let full = pool(full);
    let reduced = pool(reduced);
    (
        full.difference(&reduced).copied().collect(),
        reduced.difference(&full).copied().collect(),
    )
}
fn complete_phase_plan(candidate: &Candidate) -> bool {
    candidate.covered == candidate.budget.options.targets.len() && candidate.omissions.is_empty()
}
fn removable(c: &Candidate, pins: &BTreeSet<i32>) -> Vec<Removal> {
    let supplies: BTreeSet<_> = c
        .budget
        .plan
        .inventory
        .iter()
        .filter(|p| !p.purpose.to_lowercase().contains("switch"))
        .map(|p| p.id)
        .collect();
    let mut removals = vec![];
    for slot in crate::equipment_view::POSITIONS.map(|(slot, _)| slot) {
        let mut styles = BTreeMap::<String, BTreeSet<Option<i32>>>::new();
        styles
            .entry(planner::primary_style(&c.budget.options).into())
            .or_default()
            .insert(c.budget.plan.equipment[slot]);
        for window in &c.budget.windows {
            styles
                .entry(window.style.clone())
                .or_default()
                .insert(window.equipment[slot]);
        }
        // Do not flatten phase-specific equipment into a style-wide pin.
        if styles.values().any(|values| values.len() != 1) {
            continue;
        }
        let choices: BTreeSet<_> = styles.values().flat_map(|v| v.iter().copied()).collect();
        if choices.len() < 2 {
            continue;
        }
        for item in choices.iter().flatten().copied() {
            if pins.contains(&item) || supplies.contains(&item) {
                continue;
            }
            for replacement in choices.iter().copied().filter(|id| *id != Some(item)) {
                let pins = styles
                    .iter()
                    .filter(|(_, items)| items.contains(&Some(item)))
                    .map(|(style, _)| (format!("{style}:{slot}"), replacement.unwrap_or(0)))
                    .collect();
                removals.push(Removal {
                    item,
                    pins,
                    remove: [item].into(),
                    add: BTreeSet::new(),
                    label: planner::item_name(item),
                    replacement: replacement
                        .map(planner::item_name)
                        .unwrap_or_else(|| "Empty slot".into()),
                });
            }
        }
    }
    removals
}
fn better(a: &Candidate, b: &Candidate) -> bool {
    a.totals
        .as_ref()
        .unwrap()
        .seconds
        .total_cmp(&b.totals.as_ref().unwrap().seconds)
        .then_with(|| {
            (
                a.budget.max_pair,
                a.budget.carried_gear,
                a.budget.max_inventory_space,
            )
                .cmp(&(
                    b.budget.max_pair,
                    b.budget.carried_gear,
                    b.budget.max_inventory_space,
                ))
        })
        .is_lt()
}
impl Search {
    pub fn new(snapshot: &Snapshot, options: &Options, limits: Tolerances) -> Self {
        let mut full = options.clone();
        full.switch_pool = None;
        full.switch_pins.clear();
        full.max_switches = None;
        if let Some(member) = snapshot.selected() {
            let owned = planner::planning_stock(snapshot, &full);
            let observed = snapshot.available(full.shared);
            full = planner::effective_options(&full, member, &owned, &observed);
        }
        let baseline = Candidate::new(tradeoffs::budget(snapshot, &full, 11));
        let phase_candidates = crate::huey::split_gear_supported(&full).then(Vec::new);
        let mut current = baseline.clone();
        if phase_candidates.is_none() {
            current.budget.options.switch_pool = Some(pool(&baseline.budget.plan));
        }
        let original_pins = baseline
            .budget
            .options
            .gear_overrides
            .values()
            .copied()
            .collect();
        let done = phase_candidates.is_none() && !baseline.usable();
        let pending = if done || phase_candidates.is_some() {
            vec![]
        } else {
            removable(&current, &original_pins)
        };
        let mut result = Self {
            baseline,
            current,
            original_pins,
            limits,
            pending,
            phase_candidates,
            best: None,
            done,
            round: 0,
            trials: vec![],
        };
        if !result.done && result.phase_candidates.is_none() {
            result.pending.extend(crate::shared_armour::candidates(
                snapshot,
                &result.current,
                &result.baseline,
                limits,
            ));
        }
        result
    }
    /// Evaluate one packed plan, allowing the browser to yield between trials.
    pub fn step(&mut self, snapshot: &Snapshot) -> bool {
        if self.done {
            return false;
        }
        if let Some(candidates) = &mut self.phase_candidates {
            // A switch ceiling rebuilds both roles from owned gear. It can
            // keep a shared amulet while changing the weapon and off-hand.
            let cap = candidates.len();
            if cap >= self.baseline.budget.cap
                || (self.baseline.usable()
                    && complete_phase_plan(&self.baseline)
                    && cap > self.baseline.budget.max_pair)
            {
                self.done = true;
                return false;
            }
            candidates.push(Candidate::new(tradeoffs::budget(
                snapshot,
                &self.baseline.budget.options,
                cap,
            )));
            return true;
        }
        if self.pending.is_empty() {
            if let Some(best) = self.best.take() {
                self.current = best;
                self.round += 1;
                self.pending = removable(&self.current, &self.original_pins);
                self.pending.extend(crate::shared_armour::candidates(
                    snapshot,
                    &self.current,
                    &self.baseline,
                    self.limits,
                ));
            } else {
                self.done = true;
                return false;
            }
        }
        let Some(removal) = self.pending.pop() else {
            self.done = true;
            return false;
        };
        let mut options = self.current.budget.options.clone();
        for id in &removal.remove {
            options.switch_pool.as_mut().unwrap().remove(id);
        }
        options.switch_pool.as_mut().unwrap().extend(&removal.add);
        let keep = removal.pins.values().next().copied().unwrap_or(0);
        options.switch_pins.extend(removal.pins);
        let trial = tradeoffs::budget(snapshot, &options, 11);
        // Measure the exact cap that applying this candidate will reproduce.
        let trial = Candidate::new(tradeoffs::budget(
            snapshot,
            &options,
            trial.max_pair.min(11),
        ));
        // A failed attack window can shorten the apparent workload. Do not
        // present that missing damage as a DPS gain in the comparison table.
        let comparable = trial.usable() && trial.signature == self.baseline.signature;
        let loss = comparable
            .then(|| trial.totals.as_ref().zip(self.baseline.totals.as_ref()))
            .flatten()
            .map(|(a, b)| (1.0 - b.seconds / a.seconds) * 100.0);
        let improves = trial.budget.carried_gear < self.current.budget.carried_gear
            && trial.budget.max_pair <= self.current.budget.max_pair;
        let within_limits = crate::efficient::qualifies(&trial, &self.baseline, self.limits);
        let qualifies = improves && within_limits;
        self.trials.push(Trial {
            item: removal.item,
            label: removal.label,
            replacement: removal.replacement,
            changes: comparable.then_some(trial.budget.max_pair),
            keep,
            round: self.round,
            dps_loss: loss,
            within_limits,
        });
        if qualifies && self.best.as_ref().is_none_or(|b| better(&trial, b)) {
            self.best = Some(trial);
        }
        true
    }
    pub fn finish(mut self) -> Suggestion {
        if let Some(mut candidates) = self.phase_candidates.take() {
            candidates.push(self.baseline);
            // A Huey recommendation must cover the body, head, and tail.
            candidates.retain(complete_phase_plan);
            let mut result = crate::efficient::choose(candidates, self.limits);
            if let Some((best, compact)) = result.best.zip(result.compact) {
                (result.removed, result.added) = gear_changes(
                    &result.candidates[best].budget.plan,
                    &result.candidates[compact].budget.plan,
                );
            }
            return result;
        }
        if !self.baseline.usable() {
            return Suggestion{candidates:vec![self.baseline],best:None,compact:None,reason:Some("The full setup is not prepared or has no comparable attack windows. Review Preparation.".into()),removed:vec![],added:vec![],trials:self.trials};
        }
        let (removed, added) = gear_changes(&self.baseline.budget.plan, &self.current.budget.plan);
        if removed.is_empty()
            && self.current.budget.carried_gear == self.baseline.budget.carried_gear
        {
            self.current = self.baseline.clone();
        }
        Suggestion {
            candidates: vec![self.baseline, self.current],
            best: Some(0),
            compact: Some(1),
            reason: None,
            removed,
            added,
            trials: self.trials,
        }
    }
}
pub fn search(snapshot: &Snapshot, options: &Options, limits: Tolerances) -> Suggestion {
    let mut search = Search::new(snapshot, options, limits);
    while search.step(snapshot) {}
    search.finish()
}
