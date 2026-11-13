//! Tick-based finite rotations with a declared spec swap and external restores.
//! No encounter/incoming simulation, ring hopping or automatic gear ranking.
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
    pub swap_weapon: Option<i32>,
    pub swap_style: String,
    /// Additional execution time at each setup change, after the previous cooldown.
    pub swap_delay: u32,
    pub surge_tick: Option<u32>,
    pub surge_ready: u32,
    pub transfer_tick: Option<u32>,
    /// Remaining lifetime of one already-cast Death Charge, in ticks.
    pub death_charge: Option<u32>,
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
            swap_weapon: None,
            swap_style: String::new(),
            swap_delay: 0,
            surge_tick: None,
            surge_ready: 0,
            transfer_tick: None,
            death_charge: None,
        }
    }
}
impl Settings {
    pub fn update_time(&mut self, key: &str, value: &str) -> bool {
        let Ok(value) = value.parse::<u32>() else {
            return false;
        };
        if value > 500 {
            return false;
        }
        let field = match key {
            "surge-tick" if self.surge_tick.is_some() => &mut self.surge_tick,
            "transfer-tick" if self.transfer_tick.is_some() => &mut self.transfer_tick,
            "death-expiry" if self.death_charge.is_some() && (1..=100).contains(&value) => {
                &mut self.death_charge
            }
            "surge-ready" if self.surge_tick.is_some() => {
                if self.surge_ready == value {
                    return false;
                }
                self.surge_ready = value;
                return true;
            }
            _ => return false,
        };
        if *field == Some(value) {
            return false;
        }
        *field = Some(value);
        true
    }
    pub fn valid(&self) -> bool {
        (1..=500).contains(&self.window_ticks)
            && self.next_pulse <= 50
            && (1..=4096).contains(&self.trials)
            && self.swap_weapon.is_none_or(special_attacks::supported)
            && ["", "Stab", "Slash", "Crush", "Ranged", "Magic"].contains(&self.swap_style.as_str())
            && self.swap_delay <= 10
            && self.surge_tick.is_none_or(|t| t <= 500)
            && self.surge_ready <= 500
            && self.transfer_tick.is_none_or(|t| t <= 500)
            && self.death_charge.is_none_or(|t| (1..=100).contains(&t))
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
    /// External recovery preserves progress unless the bar reaches full.
    pub fn restore(&mut self, amount: u32) -> u32 {
        let restored = amount.min(100 - self.energy);
        self.energy += restored;
        if self.energy == 100 {
            self.next_pulse = interval(self.lightbearer);
        }
        restored
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
    pub weapon: Option<i32>,
    pub style: String,
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
    pub external_energy: f64,
    pub switches: f64,
    pub surge_doses: f64,
    pub transfer_events: f64,
    pub death_charge_triggers: f64,
    pub surge_cooldown_remaining: f64,
    pub energy_remaining: f64,
    pub empowered_remaining: f64,
    pub remaining_hp: f64,
    pub trace: Vec<Trace>,
}
struct Attack {
    ticks: u32,
    hp_dependent: bool,
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
    spell: &str,
) -> Result<Attack, String> {
    let r = combat::calculate_mode(
        member, set, style, monster, spell, invocation, settings, mode,
    )?;
    if r.attack_ticks < 1
        || r.burn.is_some()
        || r.expected_attack_ticks != f64::from(r.attack_ticks)
        || rules::weapon(set[3].unwrap_or(0))
            .is_some_and(|r| r.weapon.effects.contains(&rules::Effect::Soulreaper))
        || rules::set_effect(set, "blue_moon") && settings.blue_moon_melee != "Off"
    {
        return Err("This filler's delayed damage, variable cooldown or held-stack transitions are not verified for swaps.".into());
    }
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
        hp_dependent: r.hp_dependent.is_some(),
        cdf,
    })
}

/// Only the weapon/stance changes. A two-handed swap temporarily removes the
/// offhand; returning to the filler restores its original offhand and stats.
pub fn special_loadout(
    set: &[Option<i32>],
    style: &str,
    cfg: &Settings,
) -> Result<(Vec<Option<i32>>, String), String> {
    if set.len() != 14 {
        return Err("Invalid rotation equipment.".into());
    }
    let mut gear = set.to_vec();
    let mut attack_style = style.to_string();
    if let Some(id) = cfg.swap_weapon {
        let rule = rules::weapon(id)
            .filter(|r| r.weapon.special.is_some())
            .ok_or("Choose a reviewed special swap.")?;
        gear[3] = Some(id);
        if crate::planner::gear(id).is_some_and(|g| g.two_handed) {
            gear[5] = None;
        }
        attack_style = if cfg.swap_style.is_empty() {
            if rule.family.style == "Melee" {
                match rule.family.attack_types[0].as_str() {
                    "stab" => "Stab",
                    "slash" => "Slash",
                    "crush" => "Crush",
                    _ => return Err("This special's default melee style is unreviewed.".into()),
                }
                .into()
            } else {
                rule.family.style.clone()
            }
        } else {
            cfg.swap_style.clone()
        };
    }
    Ok((gear, attack_style))
}

/// Availability check for the UI's selected (possibly shared) stock. These are
/// comparison resources, not items silently inserted in the exported bag.
pub fn validate_available(
    stock: &BTreeMap<i32, u32>,
    options: &crate::planner::Options,
) -> Result<(), String> {
    let cfg = &options.combat.rotation;
    if cfg
        .swap_weapon
        .is_some_and(|id| stock.get(&id).copied().unwrap_or(0) == 0 || options.item_excluded(id))
    {
        return Err("The configured special swap is unavailable or excluded.".into());
    }
    if cfg.surge_tick.is_some()
        && ![30875, 30878, 30881, 30884]
            .into_iter()
            .any(|id| stock.get(&id).copied().unwrap_or(0) > 0 && !options.item_excluded(id))
    {
        return Err(
            "A scheduled Surge dose needs an owned, non-excluded unnoted Surge potion.".into(),
        );
    }
    Ok(())
}

pub fn death_charge_eligible(monster: &Monster) -> bool {
    let id = crate::encounters::source_npc_id(monster.id);
    let name = monster.name.to_ascii_lowercase();
    !crate::sailing::is_sea(id)
        && !matches!(id, 13011 | 13012 | 13013 | 7550 | 7552)
        && ![
            "totem",
            "unstable ice",
            "boulder",
            "energy siphon",
            "respiratory",
            "glowing crystal",
        ]
        .iter()
        .any(|s| name.contains(s))
}

#[derive(PartialEq, Eq)]
struct Loadout {
    gear: Vec<Option<i32>>,
    style: String,
}
struct Trial {
    clock: EnergyClock,
    hp: u32,
    pending: u32,
    elapsed: u32,
    current: usize,
    processed: [bool; 2],
    surge_ready: u32,
}
impl Trial {
    fn record(
        &self,
        report: &mut Report,
        first: bool,
        tick: u32,
        action: &str,
        before: u32,
        damage: u32,
        loadout: &Loadout,
    ) {
        if first {
            report.trace.push(Trace {
                tick,
                attack: action.into(),
                energy_before: before,
                energy_after: self.clock.energy,
                damage,
                remaining_hp: self.hp,
                empowered_hits: self.pending,
                weapon: loadout.gear[3],
                style: loadout.style.clone(),
            });
        }
    }
    /// Natural pulse, scheduled Surge, then confirmed Energy Transfer. Restore
    /// events also run during attack cooldowns and declared switch overhead.
    fn advance(
        &mut self,
        to: u32,
        cfg: &Settings,
        report: &mut Report,
        first: bool,
        loadouts: &[Loadout; 2],
    ) {
        let mut events = [(cfg.surge_tick, 0), (cfg.transfer_tick, 1)];
        events.sort_by_key(|(tick, kind)| (tick.unwrap_or(u32::MAX), *kind));
        for (tick, kind) in events {
            let Some(tick) =
                tick.filter(|t| *t <= to && *t >= self.elapsed && !self.processed[kind])
            else {
                continue;
            };
            report.energy_restored += f64::from(self.clock.advance(tick - self.elapsed));
            self.elapsed = tick;
            self.processed[kind] = true;
            let before = self.clock.energy;
            let label = if kind == 0 {
                if self.clock.energy == 100 {
                    "Surge skipped · full energy"
                } else {
                    report.external_energy += f64::from(self.clock.restore(25));
                    report.surge_doses += 1.0;
                    self.surge_ready = tick + 500;
                    "Surge dose"
                }
            } else {
                report.external_energy += f64::from(self.clock.restore(100));
                report.transfer_events += 1.0;
                "Energy Transfer arrival"
            };
            self.record(
                report,
                first,
                tick,
                label,
                before,
                0,
                &loadouts[self.current],
            );
        }
        report.energy_restored += f64::from(self.clock.advance(to - self.elapsed));
        self.elapsed = to;
    }
}

pub fn evaluate(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &defence::Settings,
) -> Result<Report, String> {
    evaluate_with_spell(member, set, style, monster, invocation, settings, "")
}

pub fn evaluate_with_spell(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    invocation: u32,
    settings: &defence::Settings,
    filler_spell: &str,
) -> Result<Report, String> {
    let cfg = &settings.rotation;
    let (spec_gear, spec_style) = special_loadout(set, style, cfg)?;
    let (special, start_hp) = special_attacks::validate_setup(
        member,
        &spec_gear,
        &spec_style,
        monster,
        invocation,
        settings,
    )?;
    if set
        .iter()
        .flatten()
        .any(|id| !crate::planner::can_use(*id, member))
        || set[3].is_some_and(|id| crate::planner::gear(id).is_some_and(|g| g.two_handed))
            && set[5].is_some()
    {
        return Err("Filler equipment requirements or two-handed restrictions are unmet.".into());
    }
    if cfg.surge_tick.is_some_and(|t| t < cfg.surge_ready) {
        return Err("The Surge dose is scheduled before its five-minute cooldown is ready.".into());
    }
    if cfg.death_charge.is_some()
        && (!death_charge_eligible(monster)
            || crate::modifiers::level(member, "Magic", &settings.boosts).unwrap_or(0) < 80)
    {
        return Err("Precast Death Charge needs 80 Magic and an eligible target; sea creatures and excluded counters cannot trigger it.".into());
    }
    let initial = EnergyClock::new(settings.special_energy, cfg.next_pulse, lightbearer(set))?;
    let zorya = special.kind == SpecialKind::Zorya;
    let loadouts = [
        Loadout {
            gear: set.to_vec(),
            style: style.into(),
        },
        Loadout {
            gear: spec_gear,
            style: spec_style,
        },
    ];
    let empowered_loadout = if zorya {
        Some(1)
    } else if set[3] == Some(34725) {
        Some(0)
    } else {
        None
    };
    let mut at_hp = settings.clone();
    at_hp.target_hp.insert(monster.id, start_hp);
    let filler = attack(
        member,
        set,
        style,
        monster,
        invocation,
        &at_hp,
        AttackMode::Normal,
        false,
        filler_spell,
    )?;
    let dynamic = filler.hp_dependent
        || crate::encounters::window(monster.id).is_some_and(|w| w.hp_scaling.is_some());
    let mut cache = BTreeMap::new();
    cache.insert(
        (0, if dynamic { start_hp } else { 0 }, AttackMode::Normal),
        filler,
    );
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
        external_energy: 0.0,
        switches: 0.0,
        surge_doses: 0.0,
        transfer_events: 0.0,
        death_charge_triggers: 0.0,
        surge_cooldown_remaining: 0.0,
        energy_remaining: 0.0,
        empowered_remaining: 0.0,
        remaining_hp: 0.0,
        trace: vec![],
    };
    let mut kills = 0;
    let mut sum_squared = 0.0;
    for trial in 0..cfg.trials {
        let mut rng = Rng(cfg.seed ^ (u64::from(trial).wrapping_mul(0x9e3779b97f4a7c15)));
        let mut state = Trial {
            clock: initial,
            hp: start_hp,
            pending: if empowered_loadout.is_some() {
                settings.zorya_empowered_hits
            } else {
                0
            },
            elapsed: 0,
            current: 0,
            processed: [false; 2],
            surge_ready: cfg.surge_ready,
        };
        let mut tick = 0;
        while tick < cfg.window_ticks && state.hp > 0 {
            state.advance(tick, cfg, &mut report, trial == 0, &loadouts);
            let mode = if state.pending > 0 {
                AttackMode::Empowered
            } else if cfg.policy == Policy::SpecialsWhenReady
                && state.clock.energy >= special.energy
            {
                AttackMode::Special
            } else {
                AttackMode::Normal
            };
            let desired = match mode {
                AttackMode::Normal => 0,
                AttackMode::Special => 1,
                AttackMode::Empowered => empowered_loadout.unwrap(),
            };
            if loadouts[state.current] != loadouts[desired] {
                state.current = desired;
                report.switches += 1.0;
                state.record(
                    &mut report,
                    trial == 0,
                    tick,
                    "Swap",
                    state.clock.energy,
                    0,
                    &loadouts[desired],
                );
                tick = tick.saturating_add(cfg.swap_delay);
                state.advance(
                    tick.min(cfg.window_ticks),
                    cfg,
                    &mut report,
                    trial == 0,
                    &loadouts,
                );
                if tick >= cfg.window_ticks {
                    break;
                }
            } else {
                state.current = desired;
            }
            let key = (desired, if dynamic { state.hp } else { 0 }, mode);
            if !cache.contains_key(&key) {
                let mut at_hp = settings.clone();
                at_hp.target_hp.insert(monster.id, state.hp);
                cache.insert(
                    key,
                    attack(
                        member,
                        &loadouts[desired].gear,
                        &loadouts[desired].style,
                        monster,
                        invocation,
                        &at_hp,
                        mode,
                        zorya,
                        if desired == 0 && mode == AttackMode::Normal {
                            filler_spell
                        } else {
                            ""
                        },
                    )?,
                );
            }
            let attack = &cache[&key];
            let before = state.clock.energy;
            match mode {
                AttackMode::Special => {
                    if !state.clock.spend(special.energy) {
                        return Err("Rotation attempted an unaffordable special.".into());
                    }
                    report.energy_spent += f64::from(special.energy);
                    report.special_activations += 1.0;
                }
                AttackMode::Empowered => {
                    state.pending -= 1;
                    report.empowered_attacks += 1.0;
                }
                AttackMode::Normal => report.normal_attacks += 1.0,
            }
            let (damage, success) = attack.sample(&mut rng);
            if zorya && mode == AttackMode::Special && success {
                state.pending = 3;
            }
            report.damage += f64::from(damage);
            state.hp = state.hp.saturating_sub(damage);
            state.record(
                &mut report,
                trial == 0,
                tick,
                match mode {
                    AttackMode::Normal => "Normal",
                    AttackMode::Special => "Special",
                    AttackMode::Empowered => "Empowered",
                },
                before,
                damage,
                &loadouts[desired],
            );
            if state.hp == 0 && cfg.death_charge.is_some_and(|expiry| tick < expiry) {
                let before = state.clock.energy;
                report.external_energy += f64::from(state.clock.restore(15));
                report.death_charge_triggers += 1.0;
                state.record(
                    &mut report,
                    trial == 0,
                    tick,
                    "Death Charge",
                    before,
                    0,
                    &loadouts[desired],
                );
            }
            tick += attack.ticks;
        }
        if state.hp > 0 {
            state.advance(cfg.window_ticks, cfg, &mut report, trial == 0, &loadouts);
        }
        let damage = f64::from(start_hp - state.hp);
        report.effective_damage += damage;
        sum_squared += damage * damage;
        kills += u32::from(state.hp == 0);
        report.energy_remaining += f64::from(state.clock.energy);
        report.empowered_remaining += f64::from(state.pending);
        report.remaining_hp += f64::from(state.hp);
        report.surge_cooldown_remaining +=
            f64::from(state.surge_ready.saturating_sub(state.elapsed));
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
        &mut report.external_energy,
        &mut report.switches,
        &mut report.surge_doses,
        &mut report.transfer_events,
        &mut report.death_charge_triggers,
        &mut report.surge_cooldown_remaining,
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
