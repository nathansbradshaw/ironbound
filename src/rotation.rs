//! Tick-based natural recovery and fixed-equipment, finite-window rotations.
//! Damage is sampled from the reviewed exact distributions. No encounter/incoming
//! simulation, external restores, ring hopping or automatic gear ranking.
use crate::{
    account::Member,
    combat::{self, AttackMode},
    defence,
    planner::Monster,
    rules::{self, SpecialKind},
    simulation::Rng,
    special_attacks,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    #[default]
    SpecialsWhenReady,
    NormalOnly,
}
impl Policy {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpecialsWhenReady => "Specials when ready",
            Self::NormalOnly => "Normal attacks only",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub enabled: bool,
    pub window_ticks: u32,
    /// Zero declares a fresh recovery interval; otherwise an explicit remaining
    /// timer for the currently worn ring. Full energy always pauses this timer.
    pub next_pulse: u32,
    pub trials: u32,
    pub seed: u64,
    pub policy: Policy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            window_ticks: 100,
            next_pulse: 0,
            trials: 512,
            seed: 0x524f544154494f4e,
            policy: Default::default(),
        }
    }
}
impl Settings {
    pub fn valid(&self) -> bool {
        (1..=500).contains(&self.window_ticks)
            && self.next_pulse <= 50
            && (1..=4096).contains(&self.trials)
    }
}

pub fn lightbearer(set: &[Option<i32>]) -> bool {
    rules::equipped_effect(set, "lightbearer")
}
pub fn interval(lightbearer: bool) -> u32 {
    if lightbearer { 25 } else { 50 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyClock {
    pub energy: u32,
    pub next_pulse: u32,
    pub lightbearer: bool,
}
impl EnergyClock {
    pub fn new(energy: u32, next_pulse: u32, lightbearer: bool) -> Result<Self, String> {
        let period = interval(lightbearer);
        if energy > 100 || next_pulse > 50 || (energy < 100 && next_pulse > period) {
            return Err(format!(
                "Starting recovery pulse must be 1–{period} ticks for this ring, or fresh interval."
            ));
        }
        Ok(Self {
            energy,
            next_pulse: if energy == 100 || next_pulse == 0 {
                period
            } else {
                next_pulse
            },
            lightbearer,
        })
    }
    /// Return actual energy restored, preserving partial time across spends.
    /// Full energy pauses recovery; it cannot bank a nearly completed pulse.
    pub fn advance(&mut self, mut ticks: u32) -> u32 {
        let mut restored = 0;
        while ticks > 0 && self.energy < 100 {
            if ticks < self.next_pulse {
                self.next_pulse -= ticks;
                return restored;
            }
            ticks -= self.next_pulse;
            let amount = 10.min(100 - self.energy);
            self.energy += amount;
            restored += amount;
            self.next_pulse = interval(self.lightbearer);
        }
        if self.energy == 100 {
            self.next_pulse = interval(self.lightbearer);
        }
        restored
    }
    pub fn spend(&mut self, cost: u32) -> bool {
        if cost > self.energy {
            return false;
        }
        self.energy -= cost;
        true
    }
    /// Current (post-April-2024) equip rule. Merely owning a ring does nothing.
    pub fn equip_lightbearer(&mut self, worn: bool) {
        if worn == self.lightbearer {
            return;
        }
        self.next_pulse = if worn { self.next_pulse.min(25) } else { 50 };
        self.lightbearer = worn;
        if self.energy == 100 {
            self.next_pulse = interval(worn);
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub tick: u32,
    pub attack: String,
    pub energy_before: u32,
    pub energy_after: u32,
    pub damage: u32,
    pub remaining_hp: u32,
    pub empowered_hits: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub trials: u32,
    pub window_ticks: u32,
    pub recovery_interval: u32,
    pub initial_pulse: u32,
    pub effective_damage: f64,
    pub damage: f64,
    pub damage_standard_error: f64,
    pub kill_chance: f64,
    pub kill_interval_95: (f64, f64),
    pub normal_attacks: f64,
    pub special_activations: f64,
    pub empowered_attacks: f64,
    pub energy_spent: f64,
    pub energy_restored: f64,
    pub energy_remaining: f64,
    pub empowered_remaining: f64,
    pub remaining_hp: f64,
    pub trace: Vec<Trace>,
}
struct Attack {
    ticks: u32,
    cdf: Vec<(f64, u32, bool)>,
}
impl Attack {
    fn sample(&self, rng: &mut Rng) -> (u32, bool) {
        let p = rng.unit();
        let index = self
            .cdf
            .partition_point(|(q, _, _)| *q <= p)
            .min(self.cdf.len() - 1);
        (self.cdf[index].1, self.cdf[index].2)
    }
}
fn attack(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &defence::Settings,
    mode: AttackMode,
    zorya: bool,
) -> Result<Attack, String> {
    let r = combat::calculate_mode(member, set, style, monster, "", invocation, settings, mode)?;
    let mut outcomes = Vec::new();
    if zorya && mode == AttackMode::Special {
        outcomes.push((0, false, 1.0 - r.accuracy));
        outcomes.extend(
            special_attacks::successful_outcomes(&r)?
                .into_iter()
                .map(|(d, p)| (d as u32, true, p)),
        );
    } else {
        outcomes.extend(
            r.hit_distribution
                .iter()
                .map(|(d, p)| (*d as u32, false, *p)),
        );
    }
    let mut sum = 0.0;
    let mut cdf = Vec::new();
    for (damage, success, p) in outcomes {
        if p > 0.0 {
            sum += p;
            cdf.push((sum, damage, success));
        }
    }
    if cdf.is_empty() || !sum.is_finite() || (sum - 1.0).abs() > 1e-8 {
        return Err("Invalid rotation attack distribution.".into());
    }
    cdf.last_mut().unwrap().0 = 1.0;
    Ok(Attack {
        ticks: r.attack_ticks as u32,
        cdf,
    })
}

pub fn evaluate(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &defence::Settings,
) -> Result<Report, String> {
    let (special, start_hp) =
        special_attacks::validate_setup(member, set, style, monster, invocation, settings)?;
    let cfg = &settings.rotation;
    let initial = EnergyClock::new(settings.special_energy, cfg.next_pulse, lightbearer(set))?;
    let zorya = special.kind == SpecialKind::Zorya;
    let dynamic = crate::encounters::window(monster.id).is_some_and(|w| w.hp_scaling.is_some());
    let mut cache = BTreeMap::new();
    let mut report = Report {
        trials: cfg.trials,
        window_ticks: cfg.window_ticks,
        recovery_interval: interval(initial.lightbearer),
        initial_pulse: initial.next_pulse,
        effective_damage: 0.0,
        damage: 0.0,
        damage_standard_error: 0.0,
        kill_chance: 0.0,
        kill_interval_95: (0.0, 0.0),
        normal_attacks: 0.0,
        special_activations: 0.0,
        empowered_attacks: 0.0,
        energy_spent: 0.0,
        energy_restored: 0.0,
        energy_remaining: 0.0,
        empowered_remaining: 0.0,
        remaining_hp: 0.0,
        trace: vec![],
    };
    let mut kills = 0;
    let mut sum_squared = 0.0;
    for trial in 0..cfg.trials {
        let mut rng = Rng(cfg.seed ^ (u64::from(trial).wrapping_mul(0x9e3779b97f4a7c15)));
        let mut clock = initial;
        let mut hp = start_hp;
        let mut tick = 0;
        let mut elapsed = 0;
        let mut pending = if zorya {
            settings.zorya_empowered_hits
        } else {
            0
        };
        while tick < cfg.window_ticks && hp > 0 {
            report.energy_restored += f64::from(clock.advance(tick - elapsed));
            elapsed = tick;
            let mode = if pending > 0 {
                AttackMode::Empowered
            } else if cfg.policy == Policy::SpecialsWhenReady && clock.energy >= special.energy {
                AttackMode::Special
            } else {
                AttackMode::Normal
            };
            let key = (if dynamic { hp } else { 0 }, mode);
            if !cache.contains_key(&key) {
                let mut at_hp = settings.clone();
                at_hp.target_hp.insert(monster.id, hp);
                cache.insert(
                    key,
                    attack(member, set, style, monster, invocation, &at_hp, mode, zorya)?,
                );
            }
            let attack = &cache[&key];
            let before = clock.energy;
            match mode {
                AttackMode::Special => {
                    if !clock.spend(special.energy) {
                        return Err("Rotation attempted an unaffordable special.".into());
                    }
                    report.energy_spent += f64::from(special.energy);
                    report.special_activations += 1.0;
                }
                AttackMode::Empowered => {
                    pending -= 1;
                    report.empowered_attacks += 1.0;
                }
                AttackMode::Normal => report.normal_attacks += 1.0,
            }
            let (damage, success) = attack.sample(&mut rng);
            if zorya && mode == AttackMode::Special && success {
                pending = 3;
            }
            report.damage += f64::from(damage);
            hp = hp.saturating_sub(damage);
            if trial == 0 {
                report.trace.push(Trace {
                    tick,
                    attack: match mode {
                        AttackMode::Normal => "Normal",
                        AttackMode::Special => "Special",
                        AttackMode::Empowered => "Empowered",
                    }
                    .into(),
                    energy_before: before,
                    energy_after: clock.energy,
                    damage,
                    remaining_hp: hp,
                    empowered_hits: pending,
                });
            }
            tick += attack.ticks;
        }
        if hp > 0 {
            report.energy_restored += f64::from(clock.advance(cfg.window_ticks - elapsed));
        }
        let damage = f64::from(start_hp - hp);
        report.effective_damage += damage;
        sum_squared += damage * damage;
        kills += u32::from(hp == 0);
        report.energy_remaining += f64::from(clock.energy);
        report.empowered_remaining += f64::from(pending);
        report.remaining_hp += f64::from(hp);
    }
    let n = f64::from(cfg.trials);
    for value in [
        &mut report.effective_damage,
        &mut report.damage,
        &mut report.normal_attacks,
        &mut report.special_activations,
        &mut report.empowered_attacks,
        &mut report.energy_spent,
        &mut report.energy_restored,
        &mut report.energy_remaining,
        &mut report.empowered_remaining,
        &mut report.remaining_hp,
    ] {
        *value /= n;
    }
    if cfg.trials > 1 {
        report.damage_standard_error =
            ((sum_squared - n * report.effective_damage.powi(2)).max(0.0) / (n - 1.0) / n).sqrt();
    }
    report.kill_chance = f64::from(kills) / n;
    let z = 1.959963984540054;
    let denominator = 1.0 + z * z / n;
    let centre = (report.kill_chance + z * z / (2.0 * n)) / denominator;
    let radius = z
        * (report.kill_chance * (1.0 - report.kill_chance) / n + z * z / (4.0 * n * n)).sqrt()
        / denominator;
    report.kill_interval_95 = ((centre - radius).max(0.0), (centre + radius).min(1.0));
    Ok(report)
}

/// Ring is equipped at the start, preserving a nearly completed natural pulse.
/// Callers must supply an actually available, usable ring; combat uses its stats.
pub fn equip_at_start(
    settings: &defence::Settings,
    set: &[Option<i32>],
    ring: i32,
) -> Result<(Vec<Option<i32>>, defence::Settings), String> {
    if set.len() != 14 || !rules::equipment_effect(ring, "lightbearer") {
        return Err("Choose a reviewed Lightbearer variant.".into());
    }
    let mut clock = EnergyClock::new(
        settings.special_energy,
        settings.rotation.next_pulse,
        lightbearer(set),
    )?;
    clock.equip_lightbearer(true);
    let mut next = settings.clone();
    next.rotation.next_pulse = clock.next_pulse;
    let mut gear = set.to_vec();
    gear[12] = Some(ring);
    Ok((gear, next))
}
