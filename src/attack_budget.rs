//! Finite-HP, independent-attack expectations. Includes the finishing cooldown;
//! excludes movement, specials, regeneration and player damage.
use crate::{
    combat::{self, HpDependentHit, ResultStats},
    planner::{Monster, Options},
};

#[derive(Debug)]
pub struct Budget {
    pub attacks: f64,
    pub seconds: f64,
    pub start_hp: u32,
    pub stop_hp: u32,
    pub entities: u32,
    pub burn: Option<crate::burn::Estimate>,
}
pub struct Timing {
    pub attacks: f64,
    pub seconds: f64,
    pub burn: Option<crate::burn::Estimate>,
}
pub fn for_hp(result: &ResultStats, start: u32, stop: u32) -> Result<Timing, String> {
    if let Some(burn) = &result.burn {
        if !burn.matches(&result.hit_distribution) {
            return Err("Burn and direct-hit distributions disagree.".into());
        }
        let estimate = crate::burn::estimate(burn, result.attack_ticks as u32, start, stop)?;
        return Ok(Timing {
            attacks: estimate.attacks,
            seconds: estimate.cooldown_ticks * 0.6,
            burn: Some(estimate),
        });
    }
    let attacks = expected_attacks(&result.hit_distribution, start, stop, result.hp_dependent)?;
    Ok(Timing {
        attacks,
        seconds: attacks * result.expected_attack_ticks * 0.6,
        burn: None,
    })
}
/// Use the same finite workload on both sides whenever a usable Eclipse set is
/// in the comparison pool, rather than comparing burn timing to stationary DPS.
pub fn eclipse_available(
    member: &crate::account::Member,
    owned: &std::collections::BTreeMap<i32, u32>,
    o: &Options,
) -> bool {
    crate::rules::registry().equipment_sets.iter().any(|s| {
        s.effect == "eclipse_moon"
            && s.pieces.iter().all(|piece| {
                piece.ids.iter().any(|id| {
                    owned.get(id).copied().unwrap_or(0) > 0
                        && !o.item_excluded(*id)
                        && crate::planner::can_use(*id, member)
                })
            })
            && s.pieces.iter().find(|p| p.slot == 3).is_some_and(|p| {
                p.ids.iter().any(|id| {
                    crate::rules::ammo_ids(*id)
                        .iter()
                        .any(|ammo| owned.get(ammo).copied().unwrap_or(0) >= o.combat.ammo_shots)
                })
            })
    })
}

/// E[h] = (1 + sum(p[d] E[max(stop,h-d)])) / (1-p[0]).
/// A ruby proc contributes the final entries and is recomputed at each HP,
/// including NPC reductions/rerolls.
pub fn expected_attacks(
    hits: &[(i32, f64)],
    start: u32,
    stop: u32,
    proc: Option<HpDependentHit>,
) -> Result<f64, String> {
    if start == 0
        || start > 10_000
        || stop >= start
        || hits.is_empty()
        || hits
            .iter()
            .any(|(d, p)| *d < 0 || !p.is_finite() || *p < 0.0)
        || (hits.iter().map(|(_, p)| p).sum::<f64>() - 1.0).abs() > 1e-8
        || proc.is_some_and(|p| {
            !(0.0..1.0).contains(&p.chance)
                || p.percent <= 0
                || p.cap <= 0
                || p.divisor <= 0
                || p.distribution_entries == 0
                || p.distribution_entries > hits.len()
                || (hits
                    .iter()
                    .rev()
                    .take(p.distribution_entries)
                    .map(|(_, q)| q)
                    .sum::<f64>()
                    - p.chance)
                    .abs()
                    > 1e-8
        })
    {
        return Err(
            "Attack budgets require a verified distribution and 1–10,000 HP within one window."
                .into(),
        );
    }
    let mut expected = vec![0.0; start as usize + 1];
    for hp in stop + 1..=start {
        let mut weighted = 1.0;
        let mut zero = 0.0;
        let mut add = |damage: i32, probability: f64| {
            if damage == 0 {
                zero += probability;
            } else {
                weighted +=
                    probability * expected[hp.saturating_sub(damage as u32).max(stop) as usize];
            }
        };
        let count = hits.len() - proc.map(|p| p.distribution_entries).unwrap_or(0);
        for (damage, probability) in &hits[..count] {
            add(*damage, *probability);
        }
        if let Some(proc) = proc {
            for (damage, probability) in proc.outcomes(hp) {
                add(damage, probability);
            }
        }
        if zero >= 1.0 {
            return Err("This window cannot make damage progress.".into());
        }
        expected[hp as usize] = weighted / (1.0 - zero);
        if !expected[hp as usize].is_finite() {
            return Err("Attack expectation is unbounded.".into());
        }
    }
    Ok(expected[start as usize])
}

pub fn for_window(
    target: &Monster,
    result: &ResultStats,
    o: &Options,
    set: &[Option<i32>],
) -> Result<Budget, String> {
    if crate::encounters::window(target.id).is_some_and(|w| w.hp_scaling.is_some()) {
        return Err("HP-dependent accuracy needs the configured attack budget; a fixed snapshot cannot be used for the whole window.".into());
    }
    if crate::yama::is_judge_window(target.id) {
        return Err("Use the paired Judge cycle; alternating attacks cannot be counted as independent styles.".into());
    }
    if crate::rules::equipped_effect(set, "confliction_accuracy") {
        return Err("Finite-HP miss-state timing for Confliction is unverified; sustained DPS remains available.".into());
    }
    let window = crate::encounters::window(target.id);
    if window
        .is_some_and(|w| w.comparison_probe && crate::encounters::source_npc_id(target.id) != 14179)
    {
        return Err(
            "This probe has no complete encounter workload; use its outgoing attack values.".into(),
        );
    }
    let full =
        combat::target_hitpoints(target, o.raid_level).ok_or("Target HP unavailable.")? as u32;
    let stage_start = window.and_then(|w| w.start_hp).unwrap_or(full);
    let style = if result.attack_type == "magic" {
        "Magic"
    } else if ["light", "standard", "heavy"].contains(&result.attack_type.as_str()) {
        "Ranged"
    } else {
        "Melee"
    };
    let progress =
        crate::encounters::progress_hp(target.id, style).unwrap_or(stage_start as f64) as u32;
    let stop = stage_start.saturating_sub(progress);
    let start = combat::current_target_hp(target, o.raid_level, &o.combat)?;
    let entities = window.map(|w| w.entity_count).unwrap_or(1);
    let timing = for_hp(result, start, stop)?;
    Ok(Budget {
        attacks: timing.attacks * f64::from(entities),
        seconds: timing.seconds * f64::from(entities),
        start_hp: start,
        stop_hp: stop,
        entities,
        burn: timing.burn,
    })
}

/// Re-evaluate all outgoing rolls/effects at each remaining HP. This recurrence
/// tracks both attack commitments and time, so HP-dependent Blood Moon timing
/// is not mistaken for a constant cooldown. It does not simulate the NPC's turns.
pub fn for_dynamic_window(
    member: &crate::account::Member,
    set: &[Option<i32>],
    style: &str,
    target: &Monster,
    spell: &str,
    o: &Options,
) -> Result<Budget, String> {
    let window = crate::encounters::window(target.id).ok_or("Dynamic window unavailable.")?;
    if window.hp_scaling.is_none() || window.comparison_probe || window.cycle.is_some() {
        return Err("This window has no reviewed dynamic HP workload.".into());
    }
    if crate::rules::equipped_effect(set, "confliction_accuracy") {
        return Err("Finite-HP miss-state timing for Confliction is unverified; sustained DPS remains available.".into());
    }
    let start = combat::current_target_hp(target, o.raid_level, &o.combat)?;
    if start > 10_000 {
        return Err("Dynamic attack budgets are limited to 10,000 HP.".into());
    }
    let stage = window.start_hp.unwrap_or(
        combat::target_hitpoints(target, o.raid_level).ok_or("Target HP unavailable.")? as u32,
    );
    let stop = stage.saturating_sub(
        crate::encounters::progress_hp(target.id, style).unwrap_or(stage as f64) as u32,
    );
    if start <= stop {
        return Err("Current HP is outside the dynamic workload.".into());
    }
    let mut attacks = vec![0.0; start as usize + 1];
    let mut seconds = attacks.clone();
    let mut settings = o.combat.clone();
    for hp in stop + 1..=start {
        settings.target_hp.insert(target.id, hp);
        let r = combat::calculate_configured(
            member,
            set,
            style,
            target,
            spell,
            o.raid_level,
            &settings,
        )?;
        if r.burn_dps > 0.0 {
            return Err("Finite Eclipse burn timing with HP-dependent NPC accuracy is unverified; a fixed snapshot cannot replace that changing accuracy.".into());
        }
        let mut a = 1.0;
        let mut time = r.expected_attack_ticks * 0.6;
        let mut zero = 0.0;
        if r.hit_distribution
            .iter()
            .any(|(d, p)| *d < 0 || !p.is_finite() || *p < 0.0)
            || (r.hit_distribution.iter().map(|(_, p)| p).sum::<f64>() - 1.0).abs() > 1e-8
        {
            return Err("Dynamic damage distribution is invalid.".into());
        }
        for (damage, p) in r.hit_distribution {
            if damage == 0 {
                zero += p;
            } else {
                let next = hp.saturating_sub(damage as u32).max(stop) as usize;
                a += p * attacks[next];
                time += p * seconds[next];
            }
        }
        if zero >= 1.0 {
            return Err("This dynamic window cannot make damage progress.".into());
        }
        attacks[hp as usize] = a / (1.0 - zero);
        seconds[hp as usize] = time / (1.0 - zero);
        if !attacks[hp as usize].is_finite() || !seconds[hp as usize].is_finite() {
            return Err("Dynamic attack expectation is unbounded.".into());
        }
    }
    Ok(Budget {
        attacks: attacks[start as usize] * f64::from(window.entity_count),
        seconds: seconds[start as usize] * f64::from(window.entity_count),
        start_hp: start,
        stop_hp: stop,
        entities: window.entity_count,
        burn: None,
    })
}
