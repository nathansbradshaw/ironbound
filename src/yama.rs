//! Paired normal-Judge attack commitments; not a full-fight survival simulator.
use crate::{
    account::Member,
    combat, encounters,
    optimization::StyleSet,
    planner::{self, Boss, Options},
    rules,
};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct JudgeCycle {
    pub ticks: f64,
    pub seconds: f64,
    pub magic_hits: f64,
    pub melee_hits: f64,
    pub magic_max: i32,
    pub melee_max: i32,
    pub intermissions: u32,
    pub hp_per_judge: u32,
    pub note: &'static str,
}
pub fn is_judge_window(id: i32) -> bool {
    encounters::window(id).is_some_and(|w| w.cycle.as_deref() == Some("judge_alternation"))
}
fn paired_windows(o: &Options) -> Option<(&planner::Target, &planner::Target)> {
    if o.boss != Boss::Custom {
        return None;
    }
    let magic = o
        .targets
        .iter()
        .find(|t| is_judge_window(t.id) && rules::base_style(&t.style) == "Magic")?;
    let melee = o
        .targets
        .iter()
        .find(|t| is_judge_window(t.id) && rules::base_style(&t.style) == "Melee")?;
    Some((magic, melee))
}
pub fn has_judge_cycle(o: &Options) -> bool {
    paired_windows(o).is_some()
}
#[derive(Clone, Copy, Default)]
struct State {
    ticks: f64,
    magic: f64,
    melee: f64,
}
fn valid_hits(hits: &[(i32, f64)]) -> bool {
    !hits.is_empty()
        && hits.len() <= 64
        && hits
            .iter()
            .all(|(damage, p)| *damage > 0 && p.is_finite() && *p >= 0.0)
        && (hits.iter().map(|(_, p)| p).sum::<f64>() - 1.0).abs() < 1e-8
}
/// Exact finite-HP expectation for alternating hit distributions. Each attack
/// commits its full weapon cooldown; the preceding weapon's delay remains
/// binding across a switch. This includes the finishing cooldown, so the result
/// is an attack-cycle budget rather than the final projectile's landing time.
pub fn simulate_judge_distributions(
    magic: &[(i32, f64)],
    magic_ticks: i32,
    melee: &[(i32, f64)],
    melee_ticks: i32,
    hp: u32,
    intermissions: u32,
) -> Result<JudgeCycle, String> {
    if !valid_hits(magic)
        || !valid_hits(melee)
        || magic_ticks <= 0
        || melee_ticks <= 0
        || hp == 0
        || intermissions == 0
        || hp > 100_000
        || intermissions > 100
    {
        return Err(
            "Judge cycle requires positive hit distributions, cooldowns and encounter counts."
                .into(),
        );
    }
    let mut magic_next = vec![State::default(); hp as usize + 1];
    let mut melee_next = magic_next.clone();
    for remaining in 1..=hp as usize {
        let mut a = State {
            ticks: f64::from(magic_ticks),
            magic: 1.0,
            melee: 0.0,
        };
        for (damage, p) in magic {
            let after = melee_next[remaining.saturating_sub(*damage as usize)];
            a.ticks += p * after.ticks;
            a.magic += p * after.magic;
            a.melee += p * after.melee;
        }
        let mut b = State {
            ticks: f64::from(melee_ticks),
            magic: 0.0,
            melee: 1.0,
        };
        for (damage, p) in melee {
            let after = magic_next[remaining.saturating_sub(*damage as usize)];
            b.ticks += p * after.ticks;
            b.magic += p * after.magic;
            b.melee += p * after.melee;
        }
        magic_next[remaining] = a;
        melee_next[remaining] = b;
    }
    let result = magic_next[hp as usize];
    let count = f64::from(intermissions);
    let ticks = result.ticks * count;
    Ok(JudgeCycle {
        ticks,
        seconds: ticks * 0.6,
        magic_hits: result.magic * count,
        melee_hits: result.melee * count,
        magic_max: magic.iter().map(|(damage, _)| *damage).max().unwrap(),
        melee_max: melee.iter().map(|(damage, _)| *damage).max().unwrap(),
        intermissions,
        hp_per_judge: hp,
        note: "Expected alternating guaranteed-max attacks, Magic first, with complete weapon cooldown commitments. Finite HP caps finishing damage; supported damage procs remain probabilistic. Excludes travel, projectile flight, dodging and environmental Judge damage.",
    })
}
pub fn simulate_judges(
    magic_max: i32,
    magic_ticks: i32,
    melee_max: i32,
    melee_ticks: i32,
    hp: u32,
    intermissions: u32,
) -> Result<JudgeCycle, String> {
    simulate_judge_distributions(
        &[(magic_max, 1.0)],
        magic_ticks,
        &[(melee_max, 1.0)],
        melee_ticks,
        hp,
        intermissions,
    )
}
fn calculated(
    member: &Member,
    set: &[Option<i32>],
    target: &planner::Target,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Result<combat::ResultStats, String> {
    if !rules::casting_supported(member, set, owned, rules::DEFAULT_CASTS) {
        return Err("Judge set needs a usable weapon and sufficient spell runes.".into());
    }
    combat::calculate_configured(
        member,
        set,
        &target.style,
        planner::monster(target.id).ok_or("Judge target unavailable")?,
        rules::spell_name(member, set, owned),
        o.raid_level,
        &o.combat,
    )
}
pub fn judge_cycle(
    member: &Member,
    melee: &[Option<i32>],
    magic: &[Option<i32>],
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Result<JudgeCycle, String> {
    let (magic_target, melee_target) =
        paired_windows(o).ok_or("Select both Judge attack windows for a paired estimate.")?;
    let cycle = encounters::alternating_cycle("judge_alternation")
        .ok_or("Judge cycle metadata unavailable.")?;
    if cycle.first_style != "Magic" {
        return Err("Unsupported alternating-cycle opening style.".into());
    }
    let mage = calculated(member, magic, magic_target, o, owned)?;
    let melee = calculated(member, melee, melee_target, o, owned)?;
    let magic_hits = mage
        .guaranteed_hits
        .as_deref()
        .ok_or("Judge Magic distribution unavailable")?;
    let melee_hits = melee
        .guaranteed_hits
        .as_deref()
        .ok_or("Judge melee distribution unavailable")?;
    simulate_judge_distributions(
        magic_hits,
        mage.attack_ticks,
        melee_hits,
        melee.attack_ticks,
        cycle.hp,
        cycle.occurrences,
    )
}
pub fn judge_cycle_weight(o: &Options) -> f64 {
    paired_windows(o)
        .map(|(a, b)| {
            f64::from(
                o.phase_priorities.get(&a.id).copied().unwrap_or(1).min(10)
                    + o.phase_priorities.get(&b.id).copied().unwrap_or(1).min(10),
            ) / 2.0
        })
        .unwrap_or(1.0)
}
/// Replace independent style proxies at the complete-loadout leaf. Proxy costs
/// still guide bounded per-style candidate pruning; they are not counted twice.
pub fn paired_adjustment(
    member: &Member,
    sets: &[StyleSet],
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Option<f64> {
    if !crate::recommendation::enabled(o) || !has_judge_cycle(o) {
        return Some(0.0);
    }
    let (magic_target, melee_target) = paired_windows(o)?;
    let magic = &sets
        .iter()
        .find(|set| set.style == magic_target.style)?
        .equipment;
    let melee = &sets
        .iter()
        .find(|set| set.style == melee_target.style)?
        .equipment;
    let cycle = judge_cycle(member, melee, magic, o, owned).ok()?;
    let mut proxy = 0.0;
    for (target, set) in [(magic_target, magic), (melee_target, melee)] {
        let hit = calculated(member, set, target, o, owned).ok()?;
        if hit.dps <= 0.0 {
            return None;
        }
        let hp = encounters::progress_hp(target.id, &target.style).unwrap_or_else(|| {
            f64::from(
                planner::monster(target.id)
                    .map(|m| m.skills.get("hp").copied().unwrap_or(400))
                    .unwrap_or(400),
            )
        });
        proxy += hp / hit.dps
            * f64::from(
                o.phase_priorities
                    .get(&target.id)
                    .copied()
                    .unwrap_or(1)
                    .min(10),
            );
    }
    Some(cycle.seconds * judge_cycle_weight(o) - proxy)
}
/// Discrete expected casts for the explicitly labelled flare comparison probe.
/// One pair is a declared comparison input, never a predicted spawn count.
pub fn window_attack_time(id: i32, result: &combat::ResultStats) -> Option<f64> {
    let window = encounters::window(id)?;
    if window.cycle.is_some()
        || !window.hit_rule.as_ref().is_some_and(|rule| rule.always_max)
        || result.max_hit <= 0
        || result.attack_ticks <= 0
    {
        return None;
    }
    let hp = planner::monster(id)?.skills.get("hp").copied()?;
    if hp <= 0 {
        return None;
    }
    let fallback = [(result.max_hit, 1.0)];
    let hits = result.guaranteed_hits.as_deref().unwrap_or(&fallback);
    if !valid_hits(hits) {
        return None;
    }
    let mut attacks = vec![0.0; hp as usize + 1];
    for remaining in 1..=hp as usize {
        attacks[remaining] = 1.0
            + hits
                .iter()
                .map(|(damage, p)| p * attacks[remaining.saturating_sub(*damage as usize)])
                .sum::<f64>();
    }
    Some(
        attacks[hp as usize]
            * f64::from(result.attack_ticks)
            * 0.6
            * f64::from(window.entity_count),
    )
}
