//! Bounded owned-only boat search. Analytic screening precedes seeded trip
//! comparisons; the result is a scenario recommendation, not a global BIS proof.
use crate::{
    account::Snapshot,
    planner::{self, Options, Plan},
    sailing::{Assignment, Boat, Settings},
    sailing_combat::{self, Model, Report},
    sailing_owned::{self, BALL_IDS, KIT_IDS},
};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Recommendation {
    pub boat: Settings,
    pub style: String,
    pub equipment: Vec<Option<i32>>,
    pub report: Report,
    pub compared: usize,
    pub screened: usize,
}
struct Outfit {
    style: String,
    set: Vec<Option<i32>>,
    plan: Plan,
    dps: f64,
    player_cannon: bool,
}
#[derive(Clone)]
struct Mount {
    tier: usize,
    ball: usize,
    person: Option<usize>,
    dps: f64,
}
struct Candidate {
    boat: Settings,
    outfit: usize,
    score: f64,
    dps: f64,
}
fn base_plan(snapshot: &Snapshot, o: &Options, style: &str) -> Plan {
    let mut t = o.clone();
    t.targets[0].style = style.into();
    t.gear_overrides.clear();
    t.inventory_overrides.clear();
    t.inventory_quantities.clear();
    t.recommended = Some(false);
    planner::plan(snapshot, &t)
}
fn cannon_set(
    snapshot: &Snapshot,
    o: &Options,
    target: &planner::Monster,
    mut set: Vec<Option<i32>>,
) -> Vec<Option<i32>> {
    let Some(member) = snapshot.selected() else {
        return set;
    };
    let stock = sailing_owned::stock(snapshot, o);
    let value = |set: &[Option<i32>]| {
        sailing_combat::cannon(member, set, target, 0, 0, None, false, &o.combat)
            .map(|r| r.dps)
            .unwrap_or(0.0)
    };
    for _ in 0..2 {
        for slot in [0, 1, 2, 4, 7, 9, 10, 12] {
            let mut best = value(&set);
            let mut pick = set[slot];
            for id in stock.keys() {
                if o.item_excluded(*id)
                    || !planner::can_use(*id, member)
                    || !planner::gear(*id).is_some_and(|g| g.slot == planner::SLOT_KEYS[slot])
                {
                    continue;
                }
                let mut trial = set.clone();
                trial[slot] = Some(*id);
                let score = value(&trial);
                if score > best + 1e-10 {
                    best = score;
                    pick = Some(*id);
                }
            }
            set[slot] = pick;
        }
    }
    for pieces in [[11664, 8839, 8840, 8842], [11664, 13072, 13073, 8842]] {
        if o.exclude_void
            || pieces.iter().any(|id| {
                !stock.contains_key(id) || o.item_excluded(*id) || !planner::can_use(*id, member)
            })
        {
            continue;
        }
        let mut trial = set.clone();
        for (slot, id) in [0, 4, 7, 9].into_iter().zip(pieces) {
            trial[slot] = Some(id);
        }
        if value(&trial) > value(&set) {
            set = trial;
        }
    }
    set
}
pub fn recommend(snapshot: &Snapshot, o: &Options) -> Result<Recommendation, String> {
    if !crate::sailing::active(o) || o.targets.len() != 1 {
        return Err("Choose one Sailing creature for a trip recommendation.".into());
    }
    let member = snapshot.selected().ok_or("Select your account")?;
    let fleet = &member.sailing_owned;
    if !fleet.valid() || !fleet.boats.iter().any(|n| *n) {
        return Err("Confirm your boat, parts and recruited crew first.".into());
    }
    let target = planner::monster(o.targets[0].id).ok_or("Sea creature missing")?;
    let base = member.level("Sailing").ok_or("Enter your Sailing level")?;
    let stock = sailing_owned::stock(snapshot, o);
    let mut outfits = Vec::new();
    for style in ["Ranged", "Magic"] {
        let plan = base_plan(snapshot, o, style);
        let set = plan
            .phases
            .first()
            .map(|p| p.equipment.clone())
            .unwrap_or_else(|| plan.equipment.clone());
        let mut trial = o.clone();
        trial.targets[0].style = style.into();
        // Evaluate the weapon independently of an unfinished manual boat layout.
        trial.sailing = Settings {
            boat: Boat::Raft,
            hull: 0,
            keel: 0,
            cargo: Some(0),
            cannon: None,
            cannon_count: 1,
            player_role: 0,
            crew: Default::default(),
            perildance: false,
            attractor: false,
            repair_workbench: false,
            repair_kits: 0,
            cannonballs: 0,
            cargo_reserved: 0,
            ..o.sailing.clone()
        };
        if let Ok(model) = Model::build(snapshot, &trial, &plan, &set, style, target) {
            if model.player_dps <= 0.0 {
                continue;
            }
            outfits.push(Outfit {
                style: style.into(),
                set,
                plan,
                dps: model
                    .streams
                    .iter()
                    .filter(|s| s.player && s.funded(&model.stock, trial.sailing.powered_charges))
                    .map(|s| s.hit.dps)
                    .sum(),
                player_cannon: false,
            });
        }
    }
    let plan = base_plan(snapshot, o, "Ranged");
    let set = cannon_set(snapshot, o, target, plan.equipment.clone());
    outfits.push(Outfit {
        style: "Ranged".into(),
        set,
        plan,
        dps: 0.0,
        player_cannon: true,
    });
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut screened = 0usize;
    for boat in Boat::ALL {
        let bi = boat.index();
        if !fleet.boats[bi] || base < boat.level() {
            continue;
        }
        let Some(hull) = fleet.hulls[bi].iter().rposition(|n| *n) else {
            continue;
        };
        let keel = if bi == 0 {
            0
        } else {
            let Some(k) = fleet.keels[bi].iter().rposition(|n| *n) else {
                continue;
            };
            k
        };
        let mut b = o.sailing.clone();
        b.boat = boat;
        b.hull = hull;
        b.keel = keel;
        b.cargo = fleet.cargo[bi].iter().rposition(|n| *n);
        b.attractor = fleet.attractor && bi > 0;
        b.perildance = fleet.perildance && bi > 0;
        b.repair_workbench = fleet.workbench && bi > 0;
        b.crew = Default::default();
        b.cannon = None;
        b.cannon_count = 1;
        b.second_cannon = None;
        b.second_ball = None;
        b.repair_kits = 0;
        b.cannonballs = 0;
        let incoming = sailing_combat::incoming(&b, target)?.dps * f64::from(b.aggressors);
        let slots = b.crew_limit(base);
        let visible = b.visible_sailing.unwrap_or(base);
        for (outfit_index, outfit) in outfits.iter().enumerate() {
            let mut mounts: Vec<Mount> = Vec::new();
            for tier in 0..7 {
                if fleet.cannons[tier] == 0 || visible < crate::sailing::CANNON_LEVELS[tier] {
                    continue;
                }
                for ball in 0..=tier {
                    if stock.get(&BALL_IDS[ball]).copied().unwrap_or(0) == 0 {
                        continue;
                    }
                    if outfit.player_cannon {
                        if let Ok(hit) = sailing_combat::cannon(
                            member,
                            &outfit.set,
                            target,
                            tier,
                            ball,
                            None,
                            b.perildance,
                            &o.combat,
                        ) {
                            mounts.push(Mount {
                                tier,
                                ball,
                                person: None,
                                dps: hit.dps,
                            });
                        }
                    } else {
                        for (person, available) in fleet.crew.iter().enumerate() {
                            if !*available || slots == 0 {
                                continue;
                            }
                            if let Ok(hit) = sailing_combat::cannon(
                                member,
                                &outfit.set,
                                target,
                                tier,
                                ball,
                                Some(person),
                                b.perildance,
                                &o.combat,
                            ) {
                                mounts.push(Mount {
                                    tier,
                                    ball,
                                    person: Some(person),
                                    dps: hit.dps,
                                });
                            }
                        }
                    }
                }
            }
            // No-cannon weapon plans remain valid, especially on a raft.
            let first: Vec<Option<&Mount>> = if outfit.player_cannon {
                mounts.iter().map(Some).collect()
            } else {
                std::iter::once(None)
                    .chain(mounts.iter().map(Some))
                    .collect()
            };
            for first in first {
                let mut second: Vec<Option<Mount>> = vec![None];
                if boat == Boat::Sloop && first.is_some() {
                    for tier in 0..7 {
                        let used = usize::from(first.is_some_and(|m| m.tier == tier));
                        if usize::from(fleet.cannons[tier]) <= used
                            || visible < crate::sailing::CANNON_LEVELS[tier]
                        {
                            continue;
                        }
                        for ball in 0..=tier {
                            if first.is_some_and(|m| m.ball != ball) {
                                continue;
                            }
                            if stock.get(&BALL_IDS[ball]).copied().unwrap_or(0) == 0 {
                                continue;
                            }
                            for (person, available) in fleet.crew.iter().enumerate() {
                                if !*available || first.is_some_and(|m| m.person == Some(person)) {
                                    continue;
                                }
                                if let Ok(hit) = sailing_combat::cannon(
                                    member,
                                    &outfit.set,
                                    target,
                                    tier,
                                    ball,
                                    Some(person),
                                    b.perildance,
                                    &o.combat,
                                ) {
                                    second.push(Some(Mount {
                                        tier,
                                        ball,
                                        person: Some(person),
                                        dps: hit.dps,
                                    }));
                                }
                            }
                        }
                    }
                }
                for second in second {
                    screened += 1;
                    let needed = usize::from(first.is_some_and(|m| m.person.is_some()))
                        + usize::from(second.is_some());
                    if needed > slots {
                        continue;
                    }
                    let mut trial = b.clone();
                    trial.player_role = usize::from(outfit.player_cannon);
                    let mut seat = 0;
                    if let Some(m) = first {
                        trial.cannon = Some(m.tier);
                        trial.cannonball = m.ball;
                        if let Some(person) = m.person {
                            trial.crew[seat] = Assignment {
                                person: Some(person),
                                role: 3,
                            };
                            seat += 1;
                        }
                    }
                    if let Some(m) = &second {
                        trial.cannon_count = 2;
                        trial.second_cannon = Some(m.tier);
                        trial.second_ball = Some(m.ball);
                        trial.crew[seat] = Assignment {
                            person: m.person,
                            role: 4,
                        };
                        seat += 1;
                    }
                    if trial.repair_workbench && seat < slots {
                        if let Some(person) = fleet
                            .crew
                            .iter()
                            .enumerate()
                            .find(|(p, available)| {
                                **available && !trial.crew.iter().any(|c| c.person == Some(*p))
                            })
                            .map(|(p, _)| p)
                        {
                            trial.crew[seat] = Assignment {
                                person: Some(person),
                                role: 2,
                            };
                            seat += 1;
                        }
                    }
                    if seat < slots {
                        if let Some(person) = fleet
                            .crew
                            .iter()
                            .enumerate()
                            .find(|(p, available)| {
                                **available && !trial.crew.iter().any(|c| c.person == Some(*p))
                            })
                            .map(|(p, _)| p)
                        {
                            trial.crew[seat] = Assignment {
                                person: Some(person),
                                role: 1,
                            };
                        }
                    }
                    if boat == Boat::Raft && first.is_some() {
                        trial.cargo = None;
                        trial.attractor = false;
                        trial.perildance = false;
                        trial.repair_workbench = false;
                    }
                    let ammo_slots = if first.is_none() {
                        0
                    } else {
                        1 + u32::from(
                            second
                                .as_ref()
                                .is_some_and(|s| s.ball != first.unwrap().ball),
                        )
                    };
                    let capacity = trial
                        .cargo_capacity()
                        .saturating_sub(trial.cargo_reserved + ammo_slots);
                    let kit = KIT_IDS
                        .iter()
                        .enumerate()
                        .filter(|(i, id)| {
                            stock.get(id).copied().unwrap_or(0) > 0
                                && sailing_combat::KIT_HEAL[*i] < trial.hp()
                        })
                        .max_by_key(|(i, id)| {
                            stock.get(id).copied().unwrap_or(0).min(capacity)
                                * sailing_combat::KIT_HEAL[*i]
                        });
                    let can_repair = trial.manual_repairs
                        || trial.repair_workbench
                            && trial.crew.iter().any(|c| c.person.is_some() && c.role == 2);
                    if let Some((tier, id)) = kit.filter(|_| can_repair) {
                        trial.repair_tier = tier;
                        trial.repair_kits = stock.get(id).copied().unwrap_or(0).min(capacity);
                    }
                    if !fleet.permits(&trial) || !trial.conflicts().is_empty() {
                        continue;
                    }
                    let dps = (outfit.dps
                        + first.map(|m| m.dps).unwrap_or(0.0)
                        + second.as_ref().map(|m| m.dps).unwrap_or(0.0))
                        * f64::from(trial.uptime)
                        / 100.0;
                    if dps <= 0.0 {
                        continue;
                    }
                    let time = f64::from(target.skills["hp"]) / dps;
                    let desired = time * f64::from(trial.trip_kills);
                    let heal = f64::from(trial.hp()) * 0.8
                        + f64::from(
                            trial.repair_kits * sailing_combat::KIT_HEAL[trial.repair_tier],
                        );
                    let damage = incoming * desired;
                    let survive = if damage > 0.0 {
                        (heal / damage).min(1.0)
                    } else {
                        1.0
                    };
                    let mut feasible =
                        survive.min((f64::from(trial.minutes) * 60.0 / desired).min(1.0));
                    let mut demand = std::collections::BTreeMap::<i32, f64>::new();
                    for m in first.into_iter().chain(second.as_ref()) {
                        *demand.entry(BALL_IDS[m.ball]).or_default() +=
                            desired / 4.2 * f64::from(trial.uptime) / 100.0
                                * if trial.attractor { 0.4 } else { 1.0 };
                    }
                    for (id, n) in demand {
                        if n > 0.0 {
                            feasible =
                                feasible.min(f64::from(stock.get(&id).copied().unwrap_or(0)) / n);
                        }
                    }
                    let score = feasible.min(1.0) * 1e6 + dps;
                    // Preserve outfit diversity so scarce fast ammo cannot crowd out
                    // every weapon/cannon alternative before finite-stock simulation.
                    let same: Vec<_> = candidates
                        .iter()
                        .filter(|c| c.outfit == outfit_index)
                        .collect();
                    if same.len() < 16 || same.last().is_some_and(|c| score > c.score) {
                        candidates.push(Candidate {
                            boat: trial,
                            outfit: outfit_index,
                            score,
                            dps,
                        });
                        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
                        let mut counts = vec![0usize; outfits.len()];
                        candidates.retain(|c| {
                            counts[c.outfit] += 1;
                            counts[c.outfit] <= 16
                        });
                    }
                }
            }
        }
    }
    let mut best: Option<Recommendation> = None;
    let mut compared = 0;
    for c in candidates {
        let outfit = &outfits[c.outfit];
        let mut options = o.clone();
        options.sailing = c.boat;
        options.targets[0].style = outfit.style.clone();
        let Ok(model) = Model::build(
            snapshot,
            &options,
            &outfit.plan,
            &outfit.set,
            &outfit.style,
            target,
        ) else {
            continue;
        };
        compared += 1;
        let report = model.run(64);
        let better = best.as_ref().is_none_or(|old| {
            let old_rate = f64::from(old.report.completed) / f64::from(old.report.trials);
            let rate = f64::from(report.completed) / f64::from(report.trials);
            rate > old_rate
                || rate == old_rate
                    && (report.mean_kills > old.report.mean_kills
                        || report.mean_kills == old.report.mean_kills
                            && c.dps > old.report.outgoing_dps)
        });
        if better {
            best = Some(Recommendation {
                boat: options.sailing,
                style: outfit.style.clone(),
                equipment: outfit.set.clone(),
                report,
                compared: 0,
                screened,
            });
        }
    }
    let mut best=best.ok_or("No usable owned setup was found. Confirm fitted parts and recruited crew, import ammunition/repair stock, and check levels and loaded charges.")?;
    let mut final_o = o.clone();
    final_o.sailing = best.boat.clone();
    final_o.targets[0].style = best.style.clone();
    let plan = base_plan(snapshot, &final_o, &best.style);
    best.report = Model::build(
        snapshot,
        &final_o,
        &plan,
        &best.equipment,
        &best.style,
        target,
    )?
    .run(512);
    best.compared = compared;
    Ok(best)
}

/// Current manual setup; confirmations are not required for a hypothetical forecast.
pub fn forecast(snapshot: &Snapshot, o: &Options) -> Result<Report, String> {
    if !crate::sailing::active(o) || o.targets.len() != 1 {
        return Err("Choose one Sailing creature.".into());
    }
    let target = planner::monster(o.targets[0].id).ok_or("Sea creature missing")?;
    let plan = planner::plan(snapshot, o);
    let style = &o.targets[0].style;
    let set = plan
        .phases
        .iter()
        .find(|p| p.style == *style)
        .map(|p| p.equipment.as_slice())
        .unwrap_or(&plan.equipment);
    Ok(Model::build(snapshot, o, &plan, set, style, target)?.run(256))
}
/// Revalidate against live stock before applying an ephemeral recommendation.
pub fn apply(snapshot: &Snapshot, o: &Options, r: &Recommendation) -> Result<Options, String> {
    let member = snapshot.selected().ok_or("Select your account")?;
    if !member.sailing_owned.permits(&r.boat) {
        return Err("Your available boat parts or crew changed. Recommend again.".into());
    }
    let mut next = o.clone();
    next.sailing = r.boat.clone();
    next.targets[0].style = r.style.clone();
    next.recommended = Some(false);
    next.gear_overrides.clear();
    next.inventory_overrides.clear();
    next.inventory_quantities.clear();
    for (slot, id) in r.equipment.iter().enumerate() {
        next.gear_overrides
            .insert(format!("{}:{slot}", r.style), id.unwrap_or(0));
    }
    let plan = planner::plan(snapshot, &next);
    let set = plan
        .phases
        .iter()
        .find(|p| p.style == r.style)
        .map(|p| p.equipment.as_slice())
        .unwrap_or(&plan.equipment);
    if set != r.equipment {
        return Err(
            "This equipment could not be packed from your current stock. Recommend again.".into(),
        );
    }
    let target = planner::monster(next.targets[0].id).ok_or("Target changed")?;
    Model::build(snapshot, &next, &plan, set, &r.style, target)?;
    Ok(next)
}
