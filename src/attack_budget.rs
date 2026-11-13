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
    let start = o
        .combat
        .target_hp
        .get(&target.id)
        .copied()
        .unwrap_or(stage_start);
    if start > stage_start {
        return Err("Current HP exceeds the starting HP of this attack window.".into());
    }
    let entities = window.map(|w| w.entity_count).unwrap_or(1);
    let attacks = expected_attacks(&result.hit_distribution, start, stop, result.hp_dependent)?
        * f64::from(entities);
    Ok(Budget {
        attacks,
        seconds: attacks * result.expected_attack_ticks * 0.6,
        start_hp: start,
        stop_hp: stop,
        entities,
    })
}
