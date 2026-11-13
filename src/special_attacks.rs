//! One paid activation or explicitly held Zorya follow-ups. Never used by normal
//! gear ranking or full-fight simulation; energy recovery and rotations are absent.
use crate::{
    account::Member,
    combat::{self, AttackMode, ResultStats},
    defence::Settings,
    planner::{self, Monster},
    rules::{self, SpecialKind},
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Outcome {
    pub remaining_hp: u32,
    pub empowered_hits: u32,
    pub probability: f64,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Report {
    pub name: String,
    pub energy_spent: u32,
    pub start_hp: u32,
    pub opener: Option<ResultStats>,
    /// Preview at the starting HP; the burst recalculates at each remaining HP.
    pub empowered: Option<ResultStats>,
    pub buff_chance: Option<f64>,
    pub expected_attacks: f64,
    pub cooldown_ticks: f64,
    pub damage: f64,
    pub effective_damage: f64,
    pub kill_chance: f64,
    pub remaining_empowered_hits: f64,
    pub outcomes: Vec<Outcome>,
}

pub fn supported(weapon: i32) -> bool {
    rules::weapon(weapon).is_some_and(|r| r.weapon.special.is_some())
}

pub(crate) fn validate_setup(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &Settings,
) -> Result<(&'static rules::SpecialAttack, u32), String> {
    if set.len() != 14 || !settings.valid() {
        return Err("Invalid special attack setup.".into());
    }
    let weapon = set[3].ok_or("Choose a special-attack weapon.")?;
    let rule = rules::weapon_for_style(weapon, style)
        .ok_or("This weapon cannot use the selected style.")?;
    let special = rule
        .weapon
        .special
        .as_ref()
        .ok_or("This weapon's special attack is not verified.".to_string())?;
    if planner::gear(weapon).is_some_and(|g| g.two_handed) && set[5].is_some() {
        return Err("This two-handed weapon cannot be combined with a shield.".into());
    }
    if set
        .iter()
        .flatten()
        .any(|id| !planner::can_use(*id, member))
    {
        return Err("Known equipment or weapon requirements are unmet or unknown.".into());
    }
    let window = crate::encounters::window(monster.id);
    if window.is_some_and(|w| {
        w.invulnerable
            || w.counter != crate::encounters::Counter::None
            || w.guaranteed_accuracy
            || w.min_hp.is_some_and(|hp| hp > 1)
            || w.resource != crate::encounters::Resource::Hitpoints
            || ![
                crate::encounters::DamageModel::Standard,
                crate::encounters::DamageModel::VerifiedStandard,
                crate::encounters::DamageModel::HalfDamage,
            ]
            .contains(&w.damage_model)
    }) || crate::monster_rules::profile(monster.id)
        .is_some_and(|p| p.kind == crate::monster_rules::Kind::Respiratory)
        || crate::monster_rules::instant_defeat(monster.id, weapon).is_some()
    {
        return Err(
            "This special attack's encounter bounds or resource interactions are not verified."
                .into(),
        );
    }
    let hp = combat::current_target_hp(monster, invocation, settings)?;
    if !(1..=10_000).contains(&hp) {
        return Err("Special comparison needs 1–10,000 target HP.".into());
    }
    Ok((special, hp))
}

pub fn evaluate(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &Settings,
) -> Result<Report, String> {
    let (special, hp) = validate_setup(member, set, style, monster, invocation, settings)?;
    let pending = if special.kind == SpecialKind::Zorya {
        settings.zorya_empowered_hits
    } else {
        0
    };
    let energy_spent = if pending > 0 { 0 } else { special.energy };
    if settings.special_energy < energy_spent {
        return Err(format!(
            "One activation needs {}% special energy; {}% is available.",
            special.energy, settings.special_energy
        ));
    }
    let calculate =
        |mode| combat::calculate_mode(member, set, style, monster, "", invocation, settings, mode);
    let empowered = if special.kind == SpecialKind::Zorya {
        Some(calculate(AttackMode::Empowered)?)
    } else {
        None
    };
    let opener = if pending == 0 {
        Some(calculate(AttackMode::Special)?)
    } else {
        None
    };
    let mut report = Report {
        name: if special.kind == SpecialKind::Zorya {
            "Zorya's tome"
        } else {
            "Brutal Swing"
        }
        .into(),
        energy_spent,
        start_hp: hp,
        opener,
        empowered,
        buff_chance: None,
        expected_attacks: 0.0,
        cooldown_ticks: 0.0,
        damage: 0.0,
        effective_damage: 0.0,
        kill_chance: 0.0,
        remaining_empowered_hits: 0.0,
        outcomes: vec![],
    };
    let mut states: BTreeMap<(u32, u32), f64> = BTreeMap::new();
    if let Some(opener) = &report.opener {
        report.expected_attacks = 1.0;
        report.cooldown_ticks = f64::from(opener.attack_ticks);
        report.damage = mean(&opener.hit_distribution);
        if special.kind == SpecialKind::Zorya {
            report.buff_chance = Some(opener.accuracy);
            // Buff application depends on accuracy, including accurate damage
            // reduced to zero. Do not confuse it with a positive hitsplat.
            let successes = successful_outcomes(opener)?;
            *states.entry((hp, 0)).or_default() += 1.0 - opener.accuracy;
            for (damage, p) in successes {
                *states
                    .entry((hp.saturating_sub(damage as u32), 3))
                    .or_default() += p;
            }
        } else {
            for (damage, p) in &opener.hit_distribution {
                *states
                    .entry((hp.saturating_sub(*damage as u32), 0))
                    .or_default() += p;
            }
        }
    } else {
        states.insert((hp, pending), 1.0);
    }
    let mut cache = BTreeMap::new();
    for _ in 0..3 {
        let mut next = BTreeMap::new();
        for ((hp, left), probability) in states {
            if probability <= 0.0 {
                continue;
            }
            if hp == 0 || left == 0 {
                *next.entry((hp, left)).or_default() += probability;
                continue;
            }
            if !cache.contains_key(&hp) {
                let mut at_hp = settings.clone();
                at_hp.target_hp.insert(monster.id, hp);
                let attack = combat::calculate_mode(
                    member,
                    set,
                    style,
                    monster,
                    "",
                    invocation,
                    &at_hp,
                    AttackMode::Empowered,
                )?;
                cache.insert(hp, attack);
            }
            let attack = &cache[&hp];
            report.expected_attacks += probability;
            report.cooldown_ticks += probability * f64::from(attack.attack_ticks);
            report.damage += probability * mean(&attack.hit_distribution);
            for (damage, p) in &attack.hit_distribution {
                *next
                    .entry((hp.saturating_sub(*damage as u32), left - 1))
                    .or_default() += probability * p;
            }
        }
        states = next;
    }
    for ((remaining_hp, empowered_hits), probability) in states {
        if probability <= 0.0 {
            continue;
        }
        report.effective_damage += f64::from(report.start_hp - remaining_hp) * probability;
        report.kill_chance += if remaining_hp == 0 { probability } else { 0.0 };
        report.remaining_empowered_hits += f64::from(empowered_hits) * probability;
        report.outcomes.push(Outcome {
            remaining_hp,
            empowered_hits,
            probability,
        });
    }
    Ok(report)
}
fn mean(outcomes: &[(i32, f64)]) -> f64 {
    outcomes.iter().map(|(d, p)| f64::from(*d) * p).sum()
}
pub(crate) fn successful_outcomes(result: &ResultStats) -> Result<Vec<(i32, f64)>, String> {
    let mut hits: BTreeMap<i32, f64> = BTreeMap::new();
    for (damage, p) in &result.hit_distribution {
        *hits.entry(*damage).or_default() += p;
    }
    let zero = hits.entry(0).or_default();
    *zero -= 1.0 - result.accuracy;
    if *zero < -1e-9 {
        return Err("This NPC's accuracy-conditioned follow-up trigger is not verified.".into());
    }
    *zero = zero.max(0.0);
    Ok(hits.into_iter().filter(|(_, p)| *p > 0.0).collect())
}
