//! Bounded encounter search with pairwise switches and one shared carried-item budget.
use crate::{
    account::Member,
    combat,
    planner::{self, Boss, Options},
};
use std::collections::{BTreeMap, BTreeSet};
pub type Set = Vec<Option<i32>>;
#[derive(Clone, Debug)]
pub struct StyleSet {
    pub style: String,
    pub equipment: Set,
}
#[derive(Clone, Debug)]
pub struct Comparison {
    pub name: String,
    pub style: String,
    pub before: Option<f64>,
    pub after: Option<f64>,
    pub incoming_before: Option<f64>,
    pub incoming_after: Option<f64>,
    pub prayer_before: f64,
    pub prayer_after: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub max_changes: usize,
    pub gear_slots: usize,
    pub comparisons: Vec<Comparison>,
    pub note: String,
}
#[derive(Clone)]
struct Candidate {
    set: Set,
    cost: f64,
    prayer: f64,
}
type SearchResult = (Vec<StyleSet>, f64, f64);
fn prayer_usage(
    m: &Member,
    set: &Set,
    style: &str,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> f64 {
    let drain = crate::prayer::drain_per_minute(crate::prayer::bonus(set), &o.combat);
    if drain <= 0.0 {
        return 0.0;
    }
    // Secondary estimate over supported attack windows only. It excludes
    // travel, mechanics and encounter drains, and is not a full-trip forecast.
    crate::recommendation::assess(m, set, style, o, owned)
        .map(|r| r.time * drain / 60.0)
        .unwrap_or(0.0)
}
fn better(candidate: &SearchResult, incumbent: &SearchResult, o: &Options) -> bool {
    candidate
        .1
        .total_cmp(&incumbent.1)
        .then_with(|| candidate.2.total_cmp(&incumbent.2))
        .then_with(|| {
            carried(&candidate.0, planner::primary_style(o))
                .cmp(&carried(&incumbent.0, planner::primary_style(o)))
        })
        .is_lt()
}
pub fn changes(a: &Set, b: &Set) -> usize {
    a.iter().zip(b).filter(|(a, b)| a != b).count()
}
pub fn carried(sets: &[StyleSet], starting: &str) -> usize {
    let Some(base) = sets.iter().find(|s| s.style == starting) else {
        return usize::MAX;
    };
    sets.iter()
        .flat_map(|s| {
            s.equipment
                .iter()
                .enumerate()
                .filter_map(|(slot, id)| id.filter(|id| base.equipment[slot] != Some(*id)))
        })
        .collect::<BTreeSet<_>>()
        .len()
}
fn targets(o: &Options) -> Vec<(i32, &str)> {
    match o.boss {
        Boss::Barrows => vec![
            (1673, "Magic"),
            (1674, "Magic"),
            (1676, "Magic"),
            (1677, "Magic"),
            (1675, "Magic"),
            (1672, "Ranged"),
        ],
        Boss::Toa if crate::recommendation::enabled(o) => crate::recommendation::targets(o),
        Boss::Toa => vec![
            (11778, "Melee"),
            (11719, "Melee"),
            (11730, "Ranged"),
            (11789, "Magic"),
        ],
        Boss::Custom => o.targets.iter().map(|t| (t.id, t.style.as_str())).collect(),
    }
}
fn spell(
    m: &Member,
    set: &Set,
    owned: &BTreeMap<i32, u32>,
    o: &Options,
    target: &planner::Monster,
) -> &'static str {
    crate::rules::spell_name_for_target(m, set, owned, &o.combat, target, o.raid_level)
}
fn value(
    m: &Member,
    set: &Set,
    reference: &Set,
    style: &str,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Option<f64> {
    if crate::recommendation::enabled(o) {
        return crate::recommendation::cost(m, set, reference, style, o, owned);
    }
    let relevant: Vec<_> = targets(o)
        .into_iter()
        .filter(|(_, s)| *s == style)
        .collect();
    if relevant.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for (id, _) in relevant {
        let target = planner::monster(id)?;
        let r = combat::calculate_configured(
            m,
            set,
            style,
            target,
            spell(m, set, owned, o, target),
            o.raid_level,
            &o.combat,
        )
        .ok()?;
        if r.dps <= 0.0 {
            return None;
        }
        let reference_dps = combat::calculate_configured(
            m,
            reference,
            style,
            target,
            spell(m, reference, owned, o, target),
            o.raid_level,
            &o.combat,
        )
        .ok()
        .map(|r| r.dps);
        let utility = crate::ranking::relative_utility(
            crate::ranking::equipment_metrics(set, Some(r.dps), o.ranking.defence),
            crate::ranking::equipment_metrics(reference, reference_dps, o.ranking.defence),
            &o.ranking.weights,
        )?;
        total += o.phase_priorities.get(&id).copied().unwrap_or(1).min(10) as f64
            * crate::encounters::progress_hp(id, style)
                .unwrap_or_else(|| target.skills.get("hp").copied().unwrap_or(100) as f64)
            / (utility * reference_dps.unwrap_or(1.0).max(0.000001));
    }
    Some(total)
}
fn normalize(mut set: Set) -> Set {
    if set[3].is_some_and(|id| planner::gear(id).is_some_and(|g| g.two_handed)) {
        set[5] = None;
    }
    set
}
fn candidates(
    m: &Member,
    base: &Set,
    ideal: &StyleSet,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
    limit: usize,
) -> Vec<Candidate> {
    let diff: Vec<_> = base
        .iter()
        .zip(&ideal.equipment)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    let supported = value(
        m,
        &ideal.equipment,
        &ideal.equipment,
        &ideal.style,
        o,
        owned,
    )
    .is_some();
    let mut buckets = BTreeMap::<usize, Vec<Candidate>>::new();
    for mask in 0usize..1usize << diff.len() {
        let mut set = base.clone();
        for (i, slot) in diff.iter().enumerate() {
            if mask & (1 << i) != 0 {
                set[*slot] = ideal.equipment[*slot];
            }
        }
        set = normalize(set);
        if o.gear_overrides.iter().any(|(key, id)| {
            key.strip_prefix(&format!("{}:", ideal.style))
                .and_then(|s| s.parse::<usize>().ok())
                .is_some_and(|slot| {
                    slot < 14 && set[slot] != if *id == 0 { None } else { Some(*id) }
                })
        }) {
            continue;
        }
        if set[3] != ideal.equipment[3]
            || (ideal.style == "Ranged" && set[13] != ideal.equipment[13])
            || changes(base, &set) > limit
        {
            continue;
        }
        let cost = if supported {
            let Some(v) = value(m, &set, &ideal.equipment, &ideal.style, o, owned) else {
                continue;
            };
            v
        } else {
            (14 - set
                .iter()
                .zip(&ideal.equipment)
                .filter(|(a, b)| a == b)
                .count()) as f64
                * 1000.0
        };
        let slots = set
            .iter()
            .enumerate()
            .filter_map(|(slot, id)| id.filter(|id| base[slot] != Some(*id)))
            .collect::<BTreeSet<_>>()
            .len();
        let list = buckets.entry(slots).or_default();
        let prayer = prayer_usage(m, &set, &ideal.style, o, owned);
        list.push(Candidate { set, cost, prayer });
        list.sort_by(|a, b| {
            a.cost
                .total_cmp(&b.cost)
                .then_with(|| a.prayer.total_cmp(&b.prayer))
        });
        list.truncate(4);
    }
    buckets.into_values().flatten().collect()
}
fn evaluate(
    m: &Member,
    base: &Set,
    anchor: usize,
    ideals: &[StyleSet],
    o: &Options,
    owned: &BTreeMap<i32, u32>,
    limit: usize,
    bag: usize,
) -> Option<SearchResult> {
    let mut choices = vec![];
    for (i, ideal) in ideals.iter().enumerate() {
        if i == anchor {
            choices.push(vec![Candidate {
                set: base.clone(),
                prayer: prayer_usage(m, base, &ideal.style, o, owned),
                cost: match value(m, base, &ideal.equipment, &ideal.style, o, owned) {
                    Some(cost) => cost,
                    None if value(
                        m,
                        &ideal.equipment,
                        &ideal.equipment,
                        &ideal.style,
                        o,
                        owned,
                    )
                    .is_none() =>
                    {
                        0.0
                    }
                    None => return None,
                },
            }]);
        } else {
            choices.push(candidates(m, base, ideal, o, owned, limit));
        }
    }
    struct Walk<'a> {
        choices: &'a [Vec<Candidate>],
        ideals: &'a [StyleSet],
        o: &'a Options,
        member: &'a Member,
        owned: &'a BTreeMap<i32, u32>,
        limit: usize,
        bag: usize,
        best: Option<SearchResult>,
        visited: usize,
    }
    impl Walk<'_> {
        fn step(&mut self, index: usize, sets: &mut Vec<StyleSet>, cost: f64, prayer: f64) {
            self.visited += 1;
            if self.visited > 50_000 {
                return;
            }
            if index == self.choices.len() {
                let Some(adjustment) =
                    crate::yama::paired_adjustment(self.member, sets, self.o, self.owned)
                else {
                    return;
                };
                let cost = cost + adjustment;
                let slots = carried(sets, planner::primary_style(self.o));
                let candidate = (sets.clone(), cost, prayer);
                if slots <= self.bag
                    && self
                        .best
                        .as_ref()
                        .is_none_or(|old| better(&candidate, old, self.o))
                {
                    self.best = Some(candidate);
                }
                return;
            }
            for candidate in &self.choices[index] {
                if sets
                    .iter()
                    .any(|s| changes(&s.equipment, &candidate.set) > self.limit)
                {
                    continue;
                }
                sets.push(StyleSet {
                    style: self.ideals[index].style.clone(),
                    equipment: candidate.set.clone(),
                });
                self.step(
                    index + 1,
                    sets,
                    cost + candidate.cost,
                    prayer + candidate.prayer,
                );
                sets.pop();
            }
        }
    }
    let mut walk = Walk {
        choices: &choices,
        ideals,
        o,
        member: m,
        owned,
        limit,
        bag,
        best: None,
        visited: 0,
    };
    walk.step(0, &mut vec![], 0.0, 0.0);
    walk.best
}
pub fn solve(
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    ideals: Vec<StyleSet>,
    o: &Options,
) -> Result<(Vec<StyleSet>, Report), String> {
    let effective = planner::effective_options(o, m, owned);
    let o = &effective;
    let limit = o.max_switches.unwrap_or(11).min(11);
    let bag = o.switch_slots.min(20);
    let anchor = ideals
        .iter()
        .position(|s| s.style == o.focus_style)
        .or_else(|| {
            ideals
                .iter()
                .position(|s| s.style == planner::primary_style(o))
        })
        .unwrap_or(0);
    let mut base = ideals[anchor].equipment.clone();
    let mut seed = evaluate(m, &base, anchor, &ideals, o, owned, limit, bag);
    if o.focus_style == "Balanced" {
        for slot in [0, 1, 2, 4, 5, 7, 9, 10, 12, 13] {
            if o.gear_overrides
                .contains_key(&format!("{}:{slot}", ideals[anchor].style))
            {
                continue;
            }
            for other in &ideals {
                let mut trial = base.clone();
                trial[slot] = other.equipment[slot];
                trial = normalize(trial);
                if let Some(result) = evaluate(m, &trial, anchor, &ideals, o, owned, limit, bag) {
                    if seed.as_ref().is_none_or(|old| better(&result, old, o)) {
                        base = trial;
                        seed = Some(result);
                    }
                }
            }
        }
    }
    let mut best=seed.ok_or_else(||format!("No setup found within {limit} changed slots per switch and {bag} switch-gear slots. Increase a limit or change the encounter."))?;
    if o.focus_style == "Balanced" {
        for _ in 0..2 {
            let mut improved = false;
            for slot in [0, 1, 2, 4, 5, 7, 9, 10, 12, 13] {
                if o.gear_overrides
                    .contains_key(&format!("{}:{slot}", ideals[anchor].style))
                {
                    continue;
                }
                let ids: BTreeSet<_> = ideals.iter().map(|s| s.equipment[slot]).collect();
                for id in ids {
                    if base[slot] == id {
                        continue;
                    }
                    let mut trial = base.clone();
                    trial[slot] = id;
                    trial = normalize(trial);
                    if let Some(candidate) =
                        evaluate(m, &trial, anchor, &ideals, o, owned, limit, bag)
                    {
                        if better(&candidate, &best, o) {
                            base = trial;
                            best = candidate;
                            improved = true;
                        }
                    }
                }
            }
            if !improved {
                break;
            }
        }
    }
    let comparisons = targets(o)
        .into_iter()
        .filter_map(|(id, style)| {
            let target = planner::monster(id)?;
            let before = &ideals.iter().find(|s| s.style == style)?.equipment;
            let after = &best.0.iter().find(|s| s.style == style)?.equipment;
            let incoming = |set: &Set| {
                crate::defence::attacks(target)
                    .first()
                    .and_then(|attack| {
                        crate::defence::estimate(m, set, target, attack, &o.combat).ok()
                    })
                    .map(|r| r.damage_per_second)
            };
            Some(Comparison {
                name: target.name.clone(),
                style: style.into(),
                incoming_before: incoming(before),
                incoming_after: incoming(after),
                prayer_before: crate::prayer::drain_per_minute(
                    crate::prayer::bonus(before),
                    &o.combat,
                ),
                prayer_after: crate::prayer::drain_per_minute(
                    crate::prayer::bonus(after),
                    &o.combat,
                ),
                before: combat::calculate_configured(
                    m,
                    before,
                    style,
                    target,
                    spell(m, before, owned, o, target),
                    o.raid_level,
                    &o.combat,
                )
                .ok()
                .map(|r| r.dps),
                after: combat::calculate_configured(
                    m,
                    after,
                    style,
                    target,
                    spell(m, after, owned, o, target),
                    o.raid_level,
                    &o.combat,
                )
                .ok()
                .map(|r| r.dps),
            })
        })
        .collect::<Vec<_>>();
    let max_changes = best
        .0
        .iter()
        .flat_map(|a| {
            best.0
                .iter()
                .map(move |b| changes(&a.equipment, &b.equipment))
        })
        .max()
        .unwrap_or(0);
    let gear_slots = carried(&best.0, planner::primary_style(o));
    let partial = comparisons
        .iter()
        .any(|c| c.before.is_none() || c.after.is_none());
    Ok((
        best.0,
        Report {
            max_changes,
            gear_slots,
            comparisons,
            note: format!(
                "Best found in a bounded search over owned sets and hybrid anchor pieces. Equal primary scores prefer lower prayer use across supported attack windows, then fewer carried items. Prayer timing excludes travel, mechanics and encounter drains; it is not a full-trip estimate. {} {} {}",
                if crate::recommendation::enabled(o) {
                    "Recommended uses reviewed encounter gear roles and supported attack windows. Moons includes normal-attack damage and Blood healing, with a 15% time tolerance; other encounters compare supported damage only."
                } else if o.focus_style == "Balanced" {
                    "Balanced mode uses your DPS, defence-bonus, prayer and equipment-mass priorities. Full loadouts are compared relative to each style’s ideal set; this is a utility score, not a kill-time simulation."
                } else {
                    "The focused style’s starting set is held fixed; other styles absorb the trade-off."
                },
                if partial {
                    "Some targets lack DPS support; those use gear-priority fallback."
                } else if o.boss == Boss::Toa {
                    "Wardens are excluded from the DPS objective; packing still includes their styles."
                } else {
                    "Movement and supply consumption are excluded."
                },
                if crate::yama::has_judge_cycle(o) && crate::recommendation::enabled(o) {
                    "The paired Judge attack-cycle estimate replaces the two independent style proxies. It includes two 400-HP intermissions, starts with Magic and alternates complete sets. The one-pair Void Flare probe is a comparison assumption, not a predicted spawn count. Movement, projectile flight, regeneration and environmental damage are excluded."
                } else {
                    ""
                }
            ),
        },
    ))
}
