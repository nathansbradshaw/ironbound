//! Encounter policy: reviewed roles first; compare supported attack windows,
//! not normalized unrelated stats. Tolerances are planner policy, not Wiki claims.
use crate::{
    account::Member,
    planner::{self, Boss, Options},
};
use std::collections::BTreeMap;
pub fn enabled(o: &Options) -> bool {
    o.recommended
        .unwrap_or(o.ranking.weights == crate::ranking::Weights::default())
}
pub fn targets(o: &Options) -> Vec<(i32, &str)> {
    match o.boss {
        Boss::Toa => vec![
            (11778, "Melee"),
            (11719, "Melee"),
            (11730, "Ranged"),
            (11789, "Magic"),
            (11789, "Ranged"),
            (11789, "Melee"),
        ],
        Boss::Barrows => vec![
            (1673, "Magic"),
            (1674, "Magic"),
            (1676, "Magic"),
            (1677, "Magic"),
            (1675, "Magic"),
            (1672, "Ranged"),
        ],
        Boss::Custom => o.targets.iter().map(|t| (t.id, t.style.as_str())).collect(),
    }
}
#[derive(Clone, Copy)]
pub struct Assessment {
    pub time: f64,
    pub damage: Option<f64>,
    pub coverage: usize,
}
pub fn assess(
    m: &Member,
    set: &[Option<i32>],
    style: &str,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Option<Assessment> {
    let mut time = 0.0;
    let mut damage = 0.0;
    let mut coverage = 0;
    let mut complete = true;
    for (id, _) in targets(o).into_iter().filter(|(_, s)| *s == style) {
        let Some(target) = planner::monster(id) else {
            complete = false;
            continue;
        };
        let Ok(mut out) = crate::combat::calculate_configured(
            m,
            set,
            style,
            target,
            crate::rules::spell_name(m, set, owned),
            o.raid_level,
            &o.combat,
        ) else {
            complete = false;
            continue;
        };
        for weapon in crate::guidance::phase_weapon_ids(o, id) {
            if o.gear_overrides.contains_key(&format!("{style}:3"))
                || owned.get(&weapon).copied().unwrap_or(0) == 0
                || !planner::can_use(weapon, m)
            {
                continue;
            }
            let mut phase = set.to_vec();
            phase[3] = Some(weapon);
            if planner::gear(weapon).is_some_and(|g| g.two_handed) {
                phase[5] = None;
            }
            if let Ok(candidate) = crate::combat::calculate_configured(
                m,
                &phase,
                style,
                target,
                crate::rules::spell_name(m, &phase, owned),
                o.raid_level,
                &o.combat,
            ) {
                if candidate.dps > out.dps {
                    out = candidate;
                }
            }
        }
        let incoming = crate::defence::profile(id)
            .filter(|p| p.automatic)
            .and_then(|_| {
                crate::defence::attacks(target)
                    .first()
                    .and_then(|a| crate::defence::estimate(m, set, target, a, &o.combat).ok())
            });
        let net = out.dps
            - incoming
                .as_ref()
                .filter(|r| r.healing_modelled)
                .map(|r| r.healing_per_second)
                .unwrap_or(0.0);
        if net <= 0.0 {
            return None;
        }
        let weight = o.phase_priorities.get(&id).copied().unwrap_or(1).min(10) as f64;
        let progress = crate::encounters::progress_hp(id, style)
            .unwrap_or_else(|| target.skills.get("hp").copied().unwrap_or(100) as f64);
        let duration = progress / net * weight;
        time += duration;
        coverage += 1;
        if let Some(r) = incoming {
            damage += r.damage_per_second * duration;
        } else {
            complete = false;
        }
    }
    (coverage > 0).then_some(Assessment {
        time,
        damage: complete.then_some(damage),
        coverage,
    })
}
/// Pick one competitive set. Known normal-attack models prefer less damage
/// within15% of the fastest set; partial models allow only3% for gear-role quality
/// and free prayer/defence gains. Unknown DPS preserves the reviewed seed.
pub fn choose(
    trials: &[Vec<Option<i32>>],
    style: &str,
    m: &Member,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> usize {
    let rows: Vec<_> = trials
        .iter()
        .map(|s| assess(m, s, style, o, owned))
        .collect();
    let coverage = rows.iter().flatten().map(|r| r.coverage).max().unwrap_or(0);
    if coverage == 0 {
        return 0;
    }
    let fastest = rows
        .iter()
        .flatten()
        .filter(|r| r.coverage == coverage)
        .map(|r| r.time)
        .fold(f64::INFINITY, f64::min);
    let full = rows
        .iter()
        .flatten()
        .filter(|r| r.coverage == coverage)
        .all(|r| r.damage.is_some());
    let tolerance = if full {
        1.15
    } else if crate::guidance::profile(o).is_some() {
        1.03
    } else {
        1.0 + 1e-9
    };
    let mut best = None;
    for (i, row) in rows.iter().enumerate() {
        let Some(row) = row else { continue };
        if row.coverage != coverage || row.time > fastest * tolerance {
            continue;
        }
        let quality = crate::guidance::quality(style, &trials[i], o);
        let prayer = crate::prayer::bonus(&trials[i]);
        let defence = crate::ranking::equipment_metrics(&trials[i], None, o.ranking.defence)
            .defence
            .unwrap_or(0.0);
        let useful_prayer = crate::prayer::drain_per_minute(0, &o.combat) > 0.0;
        let prayer_tradeoff = useful_prayer && crate::guidance::profile(o).is_some();
        let key = (
            quality,
            if full {
                row.damage.unwrap_or(f64::INFINITY)
            } else {
                0.0
            },
            if prayer_tradeoff { -prayer } else { 0 },
            -defence,
            row.time,
            -prayer,
        );
        if best.as_ref().is_none_or(|(_, k)| key < *k) {
            best = Some((i, key));
        }
    }
    best.map(|(i, _)| i).unwrap_or(0)
}
pub fn cost(
    m: &Member,
    set: &[Option<i32>],
    reference: &[Option<i32>],
    style: &str,
    o: &Options,
    owned: &BTreeMap<i32, u32>,
) -> Option<f64> {
    let a = assess(m, set, style, o, owned)?;
    let b = assess(m, reference, style, o, owned)?;
    if a.coverage < b.coverage {
        return None;
    }
    if let (Some(d), Some(base)) = (a.damage, b.damage) {
        if a.time > b.time * 1.15 {
            return None;
        }
        Some(b.time * d / base.max(0.000001))
    } else {
        Some(a.time)
    }
}
pub fn summary(o: &Options) -> &'static str {
    if !enabled(o) {
        "Custom priorities"
    } else if crate::guidance::profile(o).is_some() {
        "Recommended · encounter gear roles"
    } else {
        "Recommended · combat baseline"
    }
}
