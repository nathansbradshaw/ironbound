//! Common-workload encounter comparisons, measured from the actual planned sets.
//! HP / net DPS is an attack-time proxy, never a wall-clock kill-time prediction.
use crate::{combat, defence, encounters, planner, rules, tradeoffs, yama};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Totals {
    pub seconds: f64,
    pub effective_dps: f64,
    pub prayer_points: f64,
    pub prayer_per_minute: f64,
    /// Time-weighted equipment bonuses, not a damage reduction percentage.
    pub defence: BTreeMap<String, f64>,
    /// Bounds over recorded normal attack scenarios; no attack mix is invented.
    pub incoming_damage: Option<(f64, f64)>,
}
#[derive(Clone, Debug)]
pub struct Workload {
    pub name: String,
    pub hp: f64,
    pub left_seconds: f64,
    pub right_seconds: f64,
}
#[derive(Clone, Debug)]
pub struct Overview {
    pub left: Option<Totals>,
    pub right: Option<Totals>,
    pub workloads: Vec<Workload>,
    pub covered_windows: usize,
    pub total_windows: usize,
    pub omissions: Vec<String>,
    pub assumptions: Vec<String>,
}
impl Overview {
    pub fn complete(&self) -> bool {
        self.left.is_some() && self.omissions.is_empty()
    }
}
type Key = (i32, String);
fn key(window: &tradeoffs::Window) -> Key {
    (window.target_id, window.style.clone())
}
fn workload_group(window: &tradeoffs::Window) -> String {
    encounters::window(window.target_id)
        .and_then(|w| w.progress_group.clone())
        .unwrap_or_else(|| window.target_id.to_string())
}
fn hp(window: &tradeoffs::Window, invocation: u32) -> Option<f64> {
    encounters::progress_hp(window.target_id, &window.style).or_else(|| {
        combat::target_hitpoints(planner::monster(window.target_id)?, invocation).map(f64::from)
    })
}
fn seconds(
    window: &tradeoffs::Window,
    hp: f64,
    split: f64,
    finite_burn: bool,
) -> Result<f64, String> {
    let out = window.metrics.attack.as_ref().ok_or_else(|| {
        window
            .metrics
            .dps_reason
            .clone()
            .unwrap_or_else(|| "Outgoing damage is unverified.".into())
    })?;
    // Healing changes progress, rather than becoming a decorative defence stat.
    let healing = if defence::profile(window.target_id).is_some_and(|p| {
        p.healing_modelled
            && (p.hits.iter().any(|h| h.heal_multiplier > 0)
                || p.attacks.iter().any(|a| a.healing_fraction > 0.0))
    }) {
        let scenarios = &window.metrics.incoming;
        if scenarios.len() != 1 {
            return Err("Healing needs a verified single normal-attack model.".into());
        }
        scenarios[0]
            .healing_per_second
            .ok_or("Healing is unverified.")?
    } else {
        0.0
    };
    let net = out.dps - healing;
    if !net.is_finite() || net <= 0.0 {
        return Err("Damage does not overcome modelled healing.".into());
    }
    if encounters::window(window.target_id).is_some_and(|w| w.hp_scaling.is_some()) {
        if healing > 0.0 {
            return Err("Dynamic HP timing with NPC healing needs a full-fight scenario.".into());
        }
        return window
            .metrics
            .dynamic_budget
            .as_ref()
            .ok_or_else(|| "Dynamic HP attack budget is unavailable.".to_string())?
            .as_ref()
            .map(|b| b.seconds / split)
            .map_err(Clone::clone);
    }
    if finite_burn {
        if healing > 0.0 {
            return Err("Finite burn timing with NPC healing needs a full-fight scenario.".into());
        }
        let entities = encounters::window(window.target_id)
            .map(|w| w.entity_count)
            .unwrap_or(1);
        let each = (hp / f64::from(entities)).ceil() as u32;
        return crate::attack_budget::for_hp(out, each, 0).map(|t| t.seconds * f64::from(entities));
    }
    let value = yama::window_attack_time(window.target_id, out)
        .map(|time| time / split)
        .unwrap_or(hp / net);
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("Attack-time estimate is unavailable.".into())
    }
}
#[derive(Default)]
struct Accumulator {
    seconds: f64,
    prayer: f64,
    defence: BTreeMap<String, f64>,
    damage_min: f64,
    damage_max: f64,
    incoming_unknown: bool,
}
impl Accumulator {
    fn add(&mut self, window: &tradeoffs::Window, seconds: f64) {
        self.seconds += seconds;
        self.prayer += window.metrics.prayer_per_minute * seconds / 60.0;
        for (kind, bonus) in &window.metrics.defence {
            *self.defence.entry(kind.clone()).or_default() += *bonus as f64 * seconds;
        }
        let scenarios = &window.metrics.incoming;
        if scenarios.is_empty() || scenarios.iter().any(|s| s.dps.is_none()) {
            self.incoming_unknown = true;
        } else {
            self.damage_min += scenarios
                .iter()
                .filter_map(|s| s.dps)
                .fold(f64::INFINITY, f64::min)
                * seconds;
            self.damage_max += scenarios.iter().filter_map(|s| s.dps).fold(0.0, f64::max) * seconds;
        }
    }
    fn finish(self, hp: f64) -> Option<Totals> {
        (self.seconds.is_finite() && self.seconds > 0.0).then(|| Totals {
            seconds: self.seconds,
            effective_dps: hp / self.seconds,
            prayer_points: self.prayer,
            prayer_per_minute: self.prayer * 60.0 / self.seconds,
            defence: self
                .defence
                .into_iter()
                .map(|(k, v)| (k, v / self.seconds))
                .collect(),
            incoming_damage: (!self.incoming_unknown).then_some((self.damage_min, self.damage_max)),
        })
    }
}
/// Both sides use exactly the same covered workload. Missing damage on either
/// side excludes that window from both totals and produces a visible omission.
pub fn compare(comparison: &tradeoffs::Comparison) -> Overview {
    budgets(&comparison.left, &comparison.right)
}
pub fn budgets(left: &tradeoffs::Budget, right: &tradeoffs::Budget) -> Overview {
    let a: BTreeMap<_, _> = left.windows.iter().map(|w| (key(w), w)).collect();
    let b: BTreeMap<_, _> = right.windows.iter().map(|w| (key(w), w)).collect();
    let keys: BTreeSet<_> = a.keys().chain(b.keys()).cloned().collect();
    let mut result = Overview {
        left: None, right: None, workloads: vec![], covered_windows: 0,
        total_windows: keys.len(), omissions: vec![], assumptions: vec![
            "Attack-time estimate: allocated HP / net DPS, or the finite attack budget for burns and HP-dependent NPC stats, summed across the same windows on both sides. Dynamic windows share their complete workload across allocated styles. Effective DPS is total HP / total attack time, not an arithmetic average.".into(),
            "Uses current prayers and boosts continuously. Excludes movement, transitions, specials, downtime, eating and encounter-imposed prayer drain; this is not a kill-time or survival prediction.".into(),
            "Phase priorities affect gear selection, not enemy HP. Multiple styles or alternative states share their target's HP equally unless an explicit paired-cycle model applies.".into(),
            "Incoming bounds use the lowest and highest recorded normal-attack scenarios in each window. Unknown scenarios make the total unverified; specials are excluded.".into(),
        ],
    };
    for name in left
        .unmodelled_phases
        .iter()
        .chain(&right.unmodelled_phases)
        .collect::<BTreeSet<_>>()
    {
        result
            .omissions
            .push(format!("{name}: no registered comparison window."));
    }
    if !left.feasible || !right.feasible {
        result.omissions.push(
            "Both switch limits need feasible setups before an encounter comparison is available."
                .into(),
        );
        return result;
    }
    let mut count = BTreeMap::<String, usize>::new();
    for window in a.values() {
        *count.entry(workload_group(window)).or_default() += 1;
    }
    let mut done_cycles = BTreeSet::new();
    let (mut av, mut bv) = (Accumulator::default(), Accumulator::default());
    let mut total_hp = 0.0;
    for k in &keys {
        let Some((wa, wb)) = a.get(k).zip(b.get(k)) else {
            result.omissions.push(format!(
                "Target {} · {}: window missing from one setup.",
                k.0, k.1
            ));
            continue;
        };
        if let Some(cycle_id) = encounters::window(k.0).and_then(|w| w.cycle.as_deref()) {
            if !done_cycles.insert(cycle_id.to_string()) {
                continue;
            }
            let pair =
                |windows: &BTreeMap<Key, &tradeoffs::Window>| -> Result<(_, _, _, _), String> {
                    if cycle_id != "judge_alternation" {
                        return Err("This paired attack cycle is not modelled.".into());
                    }
                    let matching: Vec<_> = windows
                        .values()
                        .copied()
                        .filter(|w| {
                            encounters::window(w.target_id).and_then(|w| w.cycle.as_deref())
                                == Some(cycle_id)
                        })
                        .collect();
                    if matching.len() != 2 {
                        return Err("Judge requires exactly one Magic and one melee window.".into());
                    }
                    let magic = *matching
                        .iter()
                        .find(|w| rules::base_style(&w.style) == "Magic")
                        .ok_or("Judge Magic window missing.")?;
                    let melee = *matching
                        .iter()
                        .find(|w| rules::base_style(&w.style) == "Melee")
                        .ok_or("Judge melee window missing.")?;
                    let ma = magic
                        .metrics
                        .attack
                        .as_ref()
                        .ok_or("Judge Magic damage is unverified.")?;
                    let me = melee
                        .metrics
                        .attack
                        .as_ref()
                        .ok_or("Judge melee damage is unverified.")?;
                    let data =
                        encounters::alternating_cycle(cycle_id).ok_or("Cycle metadata missing.")?;
                    if data.first_style != "Magic" {
                        return Err("Cycle opening style is unsupported.".into());
                    }
                    let cycle = yama::simulate_judge_distributions(
                        ma.guaranteed_hits
                            .as_deref()
                            .ok_or("Judge Magic hit distribution missing.")?,
                        ma.attack_ticks,
                        me.guaranteed_hits
                            .as_deref()
                            .ok_or("Judge melee hit distribution missing.")?,
                        me.attack_ticks,
                        data.hp,
                        data.occurrences,
                    )?;
                    let magic_time = cycle.magic_hits * ma.attack_ticks as f64 * 0.6;
                    let melee_time = cycle.melee_hits * me.attack_ticks as f64 * 0.6;
                    Ok((key(magic), key(melee), magic_time, melee_time))
                };
            match pair(&a).and_then(|pa| pair(&b).map(|pb| (pa, pb))) {
                Ok(((am, ae, amt, aet), (bm, be, bmt, bet))) => {
                    av.add(a[&am], amt);
                    av.add(a[&ae], aet);
                    bv.add(b[&bm], bmt);
                    bv.add(b[&be], bet);
                    let cycle = encounters::alternating_cycle(cycle_id).unwrap();
                    let hp = f64::from(cycle.hp) * f64::from(cycle.occurrences);
                    total_hp += hp;
                    result.covered_windows += 2;
                    result.workloads.push(Workload {
                        name: format!("Judge · {} alternating intermissions", cycle.occurrences),
                        hp,
                        left_seconds: amt + aet,
                        right_seconds: bmt + bet,
                    });
                    result.assumptions.push("Judge uses the shared alternating-hit model, including full weapon cooldowns and finite HP. Prayer and defence assume each set stays worn for its attack commitment; early swaps during cooldown are not simulated.".into());
                }
                Err(why) => result.omissions.push(format!("Judge cycle: {why}")),
            }
            continue;
        }
        let split = count.get(&workload_group(wa)).copied().unwrap_or(1) as f64;
        let Some(hp) = hp(wa, left.options.raid_level)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| v / split)
        else {
            result
                .omissions
                .push(format!("{}: HP allocation unavailable.", wa.name));
            continue;
        };
        let finite_burn = wa.metrics.attack.as_ref().is_some_and(|a| a.burn.is_some())
            || wb.metrics.attack.as_ref().is_some_and(|a| a.burn.is_some());
        match seconds(wa, hp, split, finite_burn)
            .and_then(|at| seconds(wb, hp, split, finite_burn).map(|bt| (at, bt)))
        {
            Ok((at, bt)) => {
                av.add(wa, at);
                bv.add(wb, bt);
                total_hp += hp;
                result.covered_windows += 1;
                result.workloads.push(Workload {
                    name: format!("{} · {}", wa.name, wa.style),
                    hp,
                    left_seconds: at,
                    right_seconds: bt,
                });
                if encounters::window(k.0).is_some_and(|w| w.comparison_probe) {
                    result.assumptions.push(format!(
                        "{}: one declared probe workload, not a prediction of spawn count.",
                        wa.name
                    ));
                }
            }
            Err(why) => result
                .omissions
                .push(format!("{} · {}: {why}", wa.name, wa.style)),
        }
    }
    result.left = av.finish(total_hp);
    result.right = bv.finish(total_hp);
    result
}
