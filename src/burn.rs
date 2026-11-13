//! Target-local Eclipse stacks and bounded, seeded finite-HP timing.
use serde::{Deserialize, Serialize};

/// One four-tick pulse clock; five separate ten-pulse stacks. Adding a stack
/// neither moves the pulse nor refreshes any old stack. Full stacks discard procs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Burns {
    remaining: [u8; 5],
    until_pulse: u8,
}
impl Burns {
    pub fn stacks(&self) -> u32 {
        self.remaining.iter().filter(|n| **n > 0).count() as u32
    }
    pub fn apply(&mut self) -> bool {
        let empty = self.stacks() == 0;
        let Some(slot) = self.remaining.iter_mut().find(|n| **n == 0) else {
            return false;
        };
        *slot = 10;
        if empty {
            self.until_pulse = 4;
        }
        true
    }
    /// Pulses and expiry precede attacks scheduled on the same tick.
    pub fn tick(&mut self) -> u32 {
        if self.until_pulse == 0 {
            return 0;
        }
        self.until_pulse -= 1;
        if self.until_pulse > 0 {
            return 0;
        }
        let damage = self.stacks();
        for left in &mut self.remaining {
            *left = left.saturating_sub(1);
        }
        if self.stacks() > 0 {
            self.until_pulse = 4;
        }
        damage
    }
    pub fn clear(&mut self) -> u32 {
        let cleared = self.stacks();
        *self = Self::default();
        cleared
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BurnHit {
    pub damage: i32,
    pub probability: f64,
    pub applies_burn: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PulseRule {
    pub transform: crate::monster_rules::Transform,
    pub divisor: u32,
}
impl Default for PulseRule {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            divisor: 1,
        }
    }
}
impl PulseRule {
    pub fn damage(self, stacks: u32) -> u32 {
        self.transform.outcomes(stacks as i32, 1.0)[0].0.max(0) as u32 / self.divisor.max(1)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BurnAttack {
    /// Retains accuracy-zero hits: a miss cannot apply burn, an accurate zero can.
    pub hits: Vec<BurnHit>,
    pub pulse: PulseRule,
}
impl BurnAttack {
    pub fn valid(&self) -> bool {
        !self.hits.is_empty()
            && self.hits.len() <= 2048
            && (1..=2).contains(&self.pulse.divisor)
            && self.hits.iter().all(|h| {
                (0..=100_000).contains(&h.damage)
                    && h.probability.is_finite()
                    && (0.0..=1.0).contains(&h.probability)
            })
            && (self.hits.iter().map(|h| h.probability).sum::<f64>() - 1.0).abs() < 1e-8
    }
    pub fn matches(&self, marginal: &[(i32, f64)]) -> bool {
        if !self.valid() {
            return false;
        }
        let mut joint = std::collections::BTreeMap::<i32, f64>::new();
        let mut expected = joint.clone();
        for h in &self.hits {
            *joint.entry(h.damage).or_default() += h.probability;
        }
        for (d, p) in marginal {
            *expected.entry(*d).or_default() += p;
        }
        joint.keys().chain(expected.keys()).all(|d| {
            (joint.get(d).copied().unwrap_or(0.0) - expected.get(d).copied().unwrap_or(0.0)).abs()
                < 1e-8
        })
    }
    pub fn sample(&self, mut pick: f64) -> BurnHit {
        for hit in &self.hits {
            if pick < hit.probability {
                return *hit;
            }
            pick -= hit.probability;
        }
        *self.hits.last().expect("validated burn distribution")
    }
}
/// Common per-trial seeds make loadout comparisons reproducible.
pub struct Random(u64);
impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 11) as f64 / ((1u64 << 53) as f64)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Phase {
    pub stop_hp: u32,
    pub downtime: u32,
    pub clear_burns: bool,
}
#[derive(Clone, Debug)]
pub struct Estimate {
    pub trials: u32,
    pub attacks: f64,
    pub attacks_error: f64,
    pub cooldown_ticks: f64,
    pub cooldown_error: f64,
    pub death_ticks: f64,
    pub death_error: f64,
    pub burn_damage: f64,
}
#[derive(Default)]
struct Mean {
    n: f64,
    mean: f64,
    m2: f64,
}
impl Mean {
    fn add(&mut self, value: f64) {
        self.n += 1.0;
        let delta = value - self.mean;
        self.mean += delta / self.n;
        self.m2 += delta * (value - self.mean);
    }
    fn error(&self) -> f64 {
        if self.n < 2.0 {
            0.0
        } else {
            (self.m2 / (self.n - 1.0) / self.n).sqrt()
        }
    }
}
/// Starts empty; keeps the real attack cooldown, burn buildup/expiry, HP caps
/// and the last attack's committed cooldown. A burn kill never fires another shot.
/// Downtime ages stacks without damage (an invulnerable interval). Reset
/// boundaries discard stacks. Separate entities each start a fresh estimate.
pub fn estimate_phases(
    attack: &BurnAttack,
    ticks: u32,
    start: u32,
    phases: &[Phase],
    trials: u32,
    seed: u64,
) -> Result<Estimate, String> {
    if !attack.valid() {
        return Err("Invalid finite-HP burn outcomes.".into());
    }
    estimate_timeline(|_| attack, ticks, start, phases, trials, seed)
}

/// One profile for each HP in stop+1..=start, in ascending HP order. Both
/// direct-hit/proc probabilities and pulse reductions follow the current HP.
pub fn estimate_dynamic(
    attacks: &[BurnAttack],
    ticks: u32,
    start: u32,
    stop: u32,
    trials: u32,
    seed: u64,
) -> Result<Estimate, String> {
    if stop >= start
        || attacks.len() != (start - stop) as usize
        || attacks.iter().any(|a| !a.valid())
    {
        return Err("Invalid HP-dependent burn outcomes.".into());
    }
    estimate_timeline(
        |hp| &attacks[(hp - stop - 1) as usize],
        ticks,
        start,
        &[Phase {
            stop_hp: stop,
            downtime: 0,
            clear_burns: true,
        }],
        trials,
        seed,
    )
}

fn estimate_timeline<'a>(
    attack_at: impl Fn(u32) -> &'a BurnAttack,
    ticks: u32,
    start: u32,
    phases: &[Phase],
    trials: u32,
    seed: u64,
) -> Result<Estimate, String> {
    if !(1..=20).contains(&ticks)
        || !(1..=10_000).contains(&start)
        || !(1..=16_384).contains(&trials)
        || phases.is_empty()
        || phases.len() > 64
        || phases[0].stop_hp >= start
        || phases.iter().any(|p| p.downtime > 10_000)
        || phases.windows(2).any(|p| p[1].stop_hp >= p[0].stop_hp)
    {
        return Err("Invalid finite-HP burn timeline.".into());
    }
    let mut attacks = Mean::default();
    let mut cooldown = Mean::default();
    let mut death = Mean::default();
    let mut burn_damage = Mean::default();
    let mut work = 0u64;
    for trial in 0..trials {
        let mut random = Random::new(seed ^ u64::from(trial).wrapping_mul(0xd1342543de82ef95));
        let mut burns = Burns::default();
        let mut hp = start;
        let mut now = 0;
        let mut ready = 0;
        let mut shots = 0;
        let mut damage = 0;
        for (index, phase) in phases.iter().enumerate() {
            while hp > phase.stop_hp {
                work += 1;
                if now > 100_000 || work > 32_000_000 {
                    return Err("Burn timing exceeds the bounded simulation budget; no partial-completion estimate is returned.".into());
                }
                if now >= ready {
                    let hit = attack_at(hp).sample(random.unit());
                    shots += 1;
                    ready = now + ticks;
                    hp = hp.saturating_sub(hit.damage as u32);
                    if hp > 0 && hit.applies_burn {
                        burns.apply();
                    }
                    if hp <= phase.stop_hp {
                        break;
                    }
                }
                now += 1;
                let remaining = if index + 1 == phases.len() {
                    hp.saturating_sub(phase.stop_hp)
                } else {
                    hp
                };
                let n = attack_at(hp).pulse.damage(burns.tick()).min(remaining);
                hp -= n;
                damage += n;
            }
            if index + 1 == phases.len() || hp == 0 {
                break;
            }
            if phase.clear_burns {
                burns.clear();
            }
            for _ in 0..phase.downtime {
                now += 1;
                burns.tick();
            }
        }
        attacks.add(shots as f64);
        cooldown.add(now.max(ready) as f64);
        death.add(now as f64);
        burn_damage.add(damage as f64);
    }
    Ok(Estimate {
        trials,
        attacks: attacks.mean,
        attacks_error: attacks.error(),
        cooldown_ticks: cooldown.mean,
        cooldown_error: cooldown.error(),
        death_ticks: death.mean,
        death_error: death.error(),
        burn_damage: burn_damage.mean,
    })
}
pub fn estimate(
    attack: &BurnAttack,
    ticks: u32,
    start: u32,
    stop: u32,
) -> Result<Estimate, String> {
    let mut key = vec![
        u64::from(ticks),
        u64::from(start),
        u64::from(stop),
        u64::from(attack.pulse.divisor),
    ];
    let transform = serde_json::to_vec(&attack.pulse.transform).expect("serializable pulse rule");
    key.push(transform.len() as u64);
    key.extend(transform.into_iter().map(u64::from));
    for hit in &attack.hits {
        key.extend([
            hit.damage as u64,
            hit.probability.to_bits(),
            u64::from(hit.applies_burn),
        ]);
    }
    thread_local! {static CACHE:std::cell::RefCell<std::collections::VecDeque<(Vec<u64>,Estimate)>>=const{std::cell::RefCell::new(std::collections::VecDeque::new())};}
    if let Some(value) = CACHE.with(|c| {
        c.borrow()
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    }) {
        return Ok(value);
    }
    let value = estimate_phases(
        attack,
        ticks,
        start,
        &[Phase {
            stop_hp: stop,
            downtime: 0,
            clear_burns: true,
        }],
        4096,
        0xecc11e5e,
    )?;
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= 64 {
            c.pop_front();
        }
        c.push_back((key, value.clone()));
    });
    Ok(value)
}
