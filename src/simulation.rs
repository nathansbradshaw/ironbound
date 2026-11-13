//! Seeded, tick-based encounter scenarios. Game rules and execution assumptions
//! are deliberately separate. Never use simulated demand as an export override.
use crate::{
    account::Snapshot,
    combat::{self, ResultStats},
    planner::{self, Boss, Options, Plan},
    prayer, rules,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bump when fight timeline or consumption semantics change. A fitted worksheet
/// must not silently validate another version of the simulation engine.
pub const MODEL_REVISION: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub trials: u32,
    pub seed: u32,
    pub trip_limit: u32,
    pub uptime: u32,
    pub incoming: u32,
    pub mistakes: u32,
    pub eat_at: u32,
    pub ammo_loss: u32,
    pub charges: u32,
    pub start_hp: u32,
    pub start_prayer: u32,
    pub transition: u32,
    pub between: u32,
    pub acid_ticks: u32,
    pub freeze_ticks: u32,
    pub acid_uptime: u32,
    pub judge_travel: u32,
    pub flare_interval: u32,
    pub flare_charge: u32,
    pub flare_heal: u32,
    pub flare_regen: u32,
    pub glyph_hp: u32,
    pub glyph_prayer: u32,
    pub melee_autos: u32,
    pub warden: u32,
    pub prayer_rotation: u32,
    pub skull_ticks: u32,
    pub skull_damage: u32,
    pub burn_phase_resets: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            trials: 128,
            seed: 1,
            trip_limit: 5,
            uptime: 90,
            incoming: 60,
            mistakes: 5,
            eat_at: 50,
            ammo_loss: 20,
            charges: 2500,
            start_hp: 100,
            start_prayer: 100,
            transition: 8,
            between: 10,
            acid_ticks: 30,
            freeze_ticks: 12,
            acid_uptime: 0,
            judge_travel: 20,
            flare_interval: 80,
            flare_charge: 25,
            flare_heal: 5,
            flare_regen: 10,
            glyph_hp: 2,
            glyph_prayer: 2,
            melee_autos: 0,
            warden: 0,
            prayer_rotation: 21,
            skull_ticks: 16,
            skull_damage: 5,
            burn_phase_resets: 1,
        }
    }
}
impl Settings {
    /// Public metadata for the same bounded controls used by validation and UI.
    pub fn fields(&self) -> Vec<(&'static str, &'static str, u32, u32, u32)> {
        vec![
            (
                "burn_phase_resets",
                "Clear burns at unverified boundaries, excluding Vorkath acid (0 no, 1 yes)",
                self.burn_phase_resets,
                0,
                1,
            ),
            ("trials", "Trials", self.trials, 16, 256),
            ("seed", "Random seed", self.seed, 1, 1_000_000),
            ("trip_limit", "Trip fight cap", self.trip_limit, 1, 20),
            (
                "uptime",
                "Attack opportunities used (%)",
                self.uptime,
                10,
                100,
            ),
            (
                "incoming",
                "Residual HP / minute in exposed phases",
                self.incoming,
                0,
                600,
            ),
            (
                "mistakes",
                "Mechanic failure chance (%)",
                self.mistakes,
                0,
                100,
            ),
            ("eat_at", "Eat below HP (%)", self.eat_at, 10, 90),
            (
                "ammo_loss",
                "Projectile loss per shot (%)",
                self.ammo_loss,
                0,
                100,
            ),
            (
                "charges",
                "Starting charges per weapon",
                self.charges,
                0,
                20_000,
            ),
            ("start_hp", "Starting HP (%)", self.start_hp, 10, 100),
            (
                "start_prayer",
                "Starting Prayer (%)",
                self.start_prayer,
                0,
                100,
            ),
            (
                "transition",
                "Transition delay (ticks)",
                self.transition,
                0,
                50,
            ),
            ("between", "Between fights (ticks)", self.between, 0, 100),
            (
                "acid_ticks",
                "Vorkath acid duration (ticks)",
                self.acid_ticks,
                1,
                100,
            ),
            (
                "freeze_ticks",
                "Vorkath freeze / spawn duration (ticks)",
                self.freeze_ticks,
                1,
                100,
            ),
            (
                "acid_uptime",
                "Vorkath acid attack opportunities used (%)",
                self.acid_uptime,
                0,
                100,
            ),
            (
                "judge_travel",
                "Travel per Judge (ticks)",
                self.judge_travel,
                0,
                100,
            ),
            (
                "flare_interval",
                "Yama flare pair interval (ticks)",
                self.flare_interval,
                20,
                300,
            ),
            (
                "flare_charge",
                "Yama flare charge time (ticks)",
                self.flare_charge,
                10,
                100,
            ),
            (
                "flare_heal",
                "Full flare healing (% boss maximum)",
                self.flare_heal,
                0,
                20,
            ),
            (
                "flare_regen",
                "Flare HP regeneration interval (ticks)",
                self.flare_regen,
                1,
                100,
            ),
            ("glyph_hp", "HP lost per glyph visit", self.glyph_hp, 0, 12),
            (
                "glyph_prayer",
                "Prayer lost per glyph visit",
                self.glyph_prayer,
                0,
                12,
            ),
            (
                "melee_autos",
                "Yama final autos in melee range (%)",
                self.melee_autos,
                0,
                100,
            ),
            (
                "warden",
                "P2 Warden: 0 Elidinis, 1 Tumeken",
                self.warden,
                0,
                1,
            ),
            (
                "prayer_rotation",
                "P2 open-style rotation (ticks)",
                self.prayer_rotation,
                1,
                100,
            ),
            (
                "skull_ticks",
                "Time per skull set (ticks)",
                self.skull_ticks,
                1,
                100,
            ),
            (
                "skull_damage",
                "Successful skull damage (% maximum HP)",
                self.skull_damage,
                0,
                20,
            ),
        ]
    }
    pub fn valid(&self) -> bool {
        self.fields()
            .iter()
            .all(|(_, _, n, min, max)| (min..=max).contains(&n))
    }
    pub fn set(&mut self, key: &str, n: u32) -> bool {
        let Some((_, _, _, min, max)) = self.fields().into_iter().find(|(k, _, _, _, _)| *k == key)
        else {
            return false;
        };
        let n = n.clamp(min, max);
        match key {
            "burn_phase_resets" => self.burn_phase_resets = n,
            "trials" => self.trials = n,
            "seed" => self.seed = n,
            "trip_limit" => self.trip_limit = n,
            "uptime" => self.uptime = n,
            "incoming" => self.incoming = n,
            "mistakes" => self.mistakes = n,
            "eat_at" => self.eat_at = n,
            "ammo_loss" => self.ammo_loss = n,
            "charges" => self.charges = n,
            "start_hp" => self.start_hp = n,
            "start_prayer" => self.start_prayer = n,
            "transition" => self.transition = n,
            "between" => self.between = n,
            "acid_ticks" => self.acid_ticks = n,
            "freeze_ticks" => self.freeze_ticks = n,
            "acid_uptime" => self.acid_uptime = n,
            "judge_travel" => self.judge_travel = n,
            "flare_interval" => self.flare_interval = n,
            "flare_charge" => self.flare_charge = n,
            "flare_heal" => self.flare_heal = n,
            "flare_regen" => self.flare_regen = n,
            "glyph_hp" => self.glyph_hp = n,
            "glyph_prayer" => self.glyph_prayer = n,
            "melee_autos" => self.melee_autos = n,
            "warden" => self.warden = n,
            "prayer_rotation" => self.prayer_rotation = n,
            "skull_ticks" => self.skull_ticks = n,
            "skull_damage" => self.skull_damage = n,
            _ => return false,
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encounter {
    Vorkath,
    Yama,
    Wardens,
}
impl Encounter {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vorkath => "Vorkath",
            Self::Yama => "Normal Yama",
            Self::Wardens => "Wardens",
        }
    }
    pub fn selected(o: &Options) -> Option<Self> {
        if o.boss == Boss::Toa {
            return Some(Self::Wardens);
        }
        if o.boss != Boss::Custom || o.targets.is_empty() {
            return None;
        }
        if o.targets.iter().all(|t| [8059, 100020].contains(&t.id)) {
            return Some(Self::Vorkath);
        }
        if o.targets.iter().all(|t| {
            [
                14176, 14179, 14180, 100010, 100011, 100012, 100013, 100014, 100030,
            ]
            .contains(&t.id)
        }) {
            return Some(Self::Yama);
        }
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attack {
    pub hits: ResultStats,
    pub weapon: i32,
    pub runes: Vec<(i32, u32)>,
    pub projectile: Option<i32>,
    #[serde(default)]
    pub projectile_loss: Option<u32>,
    pub charge: bool,
    #[serde(default)]
    pub charge_on_damage: bool,
    pub scales: bool,
    pub prayer: f64,
}
impl Attack {
    fn roll(&self, rng: &mut Rng, hp: u32, player_hp: f64) -> (i32, bool, bool) {
        let mut pick = rng.unit();
        if let Some(burn) = &self.hits.burn {
            let hit = burn.sample(pick);
            return (hit.damage, false, hit.applies_burn);
        }
        let proc = self.hits.hp_dependent;
        let no_ruby = proc.is_some() && player_hp < 10.0;
        let count =
            self.hits.hit_distribution.len() - proc.map(|p| p.distribution_entries).unwrap_or(0);
        for (damage, p) in &self.hits.hit_distribution[..count] {
            let p = if no_ruby {
                *p / (1.0 - proc.unwrap().chance)
            } else {
                *p
            };
            if pick < p {
                return (*damage, false, false);
            }
            pick -= p;
        }
        if let Some(proc) = proc.filter(|_| !no_ruby) {
            for (damage, p) in proc.outcomes(hp) {
                if pick < p {
                    return (damage, true, false);
                }
                pick -= p;
            }
        }
        (0, false, false)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub encounter: Encounter,
    pub settings: Settings,
    pub attacks: Vec<Attack>,
    pub max_hp: u32,
    pub max_prayer: u32,
    pub hp: Vec<u32>,
    pub available: BTreeMap<i32, u32>,
    pub foods: Vec<(i32, u32, u32)>,
    pub potions: Vec<(u32, u32)>,
    pub notes: Vec<String>,
    pub mark_ticks: u32,
}
#[derive(Deserialize)]
struct ConsumptionRules {
    nonrecoverable_projectiles: Vec<i32>,
    charge_free_weapons: Vec<i32>,
    charge_on_damage: Vec<i32>,
}
fn consumption_rules() -> &'static ConsumptionRules {
    static DATA: std::sync::OnceLock<ConsumptionRules> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/consumption-rules.json"))
            .expect("reviewed consumption rules")
    })
}
/// Shared conservative resource classification for other trip simulators.
pub fn charged_weapon(weapon: i32, style: &str, projectile: Option<i32>) -> bool {
    let Some(rule) = rules::weapon(weapon) else {
        return false;
    };
    !consumption_rules().charge_free_weapons.contains(&weapon)
        && (rule.family.formula == rules::Formula::Powered
            || consumption_rules().charge_on_damage.contains(&weapon)
            || (style == "Ranged" && projectile.is_none() && rule.family.ammo.is_empty()))
}
fn attack(
    snapshot: &Snapshot,
    o: &Options,
    p: &Plan,
    id: i32,
    style: &str,
    enrage: bool,
) -> Result<Attack, String> {
    let set = crate::tradeoffs::set_for_window(p, o, id, style)
        .or_else(|| {
            std::iter::once(p.equipment.as_slice())
                .chain(p.phases.iter().map(|v| v.equipment.as_slice()))
                .find(|set| {
                    set.get(3)
                        .copied()
                        .flatten()
                        .and_then(rules::weapon)
                        .is_some_and(|r| r.permits(style))
                })
        })
        .ok_or_else(|| {
            format!(
                "Pack a complete {style} switch for {}.",
                planner::monster(id)
                    .map(|m| m.name.as_str())
                    .unwrap_or("this phase")
            )
        })?;
    let member = snapshot.selected().ok_or("Select an account.")?;
    if rules::equipped_effect(set, "confliction_accuracy") {
        return Err("Full-fight Confliction miss-state transitions are not verified. Choose other gloves for this simulation.".into());
    }
    if rules::set_effect(set, "dharok") {
        return Err("Dharok's changing player-HP damage is not yet verified in the fight simulator. Use another packed set for this scenario.".into());
    }
    let mut target = planner::monster(id)
        .ok_or("Missing simulation target")?
        .clone();
    let mut settings = o.combat.clone();
    settings.loaded_dart = p
        .loaded_ammo
        .as_ref()
        .map(|a| a.id)
        .or(settings.loaded_dart);
    settings.target_hp.clear();
    if enrage {
        target.skills.insert("def".into(), 180);
        settings.reductions.remove(&id);
    }
    let mut stock = snapshot.available(o.shared);
    stock.retain(|id, _| !o.excluded.contains(id));
    let spell = rules::spell_name_for_target(member, set, &stock, &settings, &target, o.raid_level);
    let hits =
        combat::calculate_configured(member, set, style, &target, spell, o.raid_level, &settings)?;
    if (hits.expected_attack_ticks - hits.attack_ticks as f64).abs() > 1e-8 {
        return Err("Conditional attack-speed procs need a verified per-attack timing model. Use another packed set for this simulation.".into());
    }
    if !hits.dps.is_finite() || hits.dps <= 0.0 {
        return Err("This phase cannot make damage progress.".into());
    }
    let weapon = set[3].ok_or("Weapon missing")?;
    let rule = rules::weapon(weapon).ok_or("Unverified weapon")?;
    if rule.has(rules::Effect::Soulreaper) {
        return Err("Soulreaper stack buildup, HP costs and recovery are not verified in this fight simulator. Use another packed weapon.".into());
    }
    let internal = !rule.family.internal_ammo.is_empty();
    let projectile = if style == "Ranged" && planner::is_stackable(weapon) {
        Some(weapon)
    } else if internal {
        p.loaded_ammo
            .as_ref()
            .map(|a| a.id)
            .or(settings.loaded_dart)
    } else {
        (!rule.family.ammo.is_empty()).then_some(set[13]).flatten()
    };
    let runes = rules::rune_supplies_for_set(spell, 1, set);
    let charge = charged_weapon(weapon, style, projectile);
    Ok(Attack {
        hits,
        weapon,
        runes,
        projectile,
        projectile_loss: projectile
            .filter(|id| consumption_rules().nonrecoverable_projectiles.contains(id))
            .map(|_| 100),
        charge,
        charge_on_damage: consumption_rules().charge_on_damage.contains(&weapon),
        scales: internal,
        prayer: prayer::drain_per_minute(prayer::bonus(set), &settings) / 100.0,
    })
}
impl Model {
    pub fn from_plan(snapshot: &Snapshot, o: &Options, p: &Plan) -> Result<Self, String> {
        if !p.exportable() {
            return Err("Resolve missing carried items and preparation blockers before simulating this packed setup.".into());
        }
        if !o.simulation.valid() {
            return Err("Invalid simulation assumptions.".into());
        }
        let encounter =
            Encounter::selected(o).ok_or("Select Vorkath, normal Yama or Tombs of Amascut.")?;
        let member = snapshot.selected().ok_or("Select an account")?;
        let mut attacks = Vec::new();
        let mut hp = Vec::new();
        let mut add = |id, style, enrage| -> Result<(), String> {
            attacks.push(attack(snapshot, o, p, id, style, enrage)?);
            hp.push(
                combat::target_hitpoints(
                    planner::monster(id).ok_or("Missing target")?,
                    o.raid_level,
                )
                .ok_or("Missing target HP")? as u32,
            );
            Ok(())
        };
        match encounter {
            Encounter::Vorkath => {
                let style = planner::primary_style(o);
                add(8059, style, false)?;
                add(100020, style, false)?;
            }
            Encounter::Yama => {
                let style = planner::primary_style(o);
                for (id, style) in [
                    (14176, style),
                    (100011, style),
                    (100012, "Magic"),
                    (100013, "Melee"),
                    (14179, "Magic"),
                    (100030, "Magic"),
                ] {
                    add(id, style, false)?;
                }
            }
            Encounter::Wardens => {
                let (p2, core, p3) = if o.simulation.warden == 0 {
                    (11753, 11755, 11762)
                } else {
                    (11756, 11758, 11761)
                };
                add(11751, "Ranged", false)?;
                add(p2, "Magic", false)?;
                add(p2, "Ranged", false)?;
                add(core, "Melee", false)?;
                add(p3, "Ranged", false)?;
                add(p3, "Ranged", true)?;
            }
        }
        let mut available = BTreeMap::<i32, u32>::new();
        for item in &p.inventory {
            *available.entry(item.id).or_default() += item.quantity;
        }
        for (slot, id) in p.equipment.iter().enumerate() {
            if let Some(id) = id {
                *available.entry(*id).or_default() +=
                    planner::equipped_quantity(slot, *id, p.ammo_shots);
            }
        }
        if let Some(ammo) = &p.loaded_ammo {
            *available.entry(ammo.id).or_default() += ammo.quantity;
        }
        let mut foods = Vec::new();
        let mut potions = Vec::new();
        let mut notes = Vec::new();
        for item in &p.inventory {
            if let Some((_, instant, delayed, bites)) =
                crate::supplies::food_healing(item.id, member.level("Hitpoints"))
            {
                if delayed == 0 && instant > 0 {
                    foods.push((item.id, instant, item.quantity * bites));
                } else {
                    notes.push(format!(
                        "{} has delayed or unverified healing; it is not credited.",
                        planner::item_name(item.id)
                    ));
                }
            }
            let points = prayer::restore_points(
                member.level("Prayer").unwrap_or(1),
                std::slice::from_ref(item),
            );
            if points > 0 {
                let doses = planner::item_name(item.id)
                    .rsplit_once('(')
                    .and_then(|(_, s)| s.trim_end_matches(')').parse::<u32>().ok())
                    .unwrap_or(0)
                    * item.quantity;
                if doses > 0 {
                    potions.push((points / doses, doses));
                }
            }
        }
        foods.sort_by_key(|(_, heal, _)| std::cmp::Reverse(*heal));
        notes.push("Incoming HP/min applies during exposed attack phases after protection, excluding separate failed mechanics. It is not inferred from unverified attack mixtures. Yama prioritizes killing all flares rather than glyph-skipping them; melee-range autos can spawn extra final flares.".into());
        notes.push("Timing, uptime, missed mechanics, flare healing/regeneration, skull damage and recovery are editable scenario inputs. Defaults are illustrative, not measured player performance.".into());
        notes.push(format!("Eclipse burns use target-local timed stacks, not steady-state DPS. Unverified phase-boundary clearing: {}. This is an explicit scenario assumption; confirm it for the encounter. Vorkath acid entry/exit retain stacks independently of this setting, following inspected 2024 Eclipse gameplay; current-patch confirmation and other boundaries remain open. Invulnerable downtime ages retained stacks without damage, new targets/fights start empty, and dead targets receive no more pulses or attacks.",if o.simulation.burn_phase_resets==1 {"on"} else {"off"}));
        notes.push("Every fight starts with full boss HP. The comparison panel's current-target-HP and fixed-player-HP assumptions are not fight starting conditions. Player HP/Prayer and supplies carry between fights.".into());
        notes.push("No special attacks, passive healing, boost decay, combo eating, projectile travel, armour degradation, tome pages, poison/venom or potion protection expiry. Keep protection/boosts maintained manually. Weapon charges and blowpipe scales are reported separately.".into());
        if encounter == Encounter::Wardens {
            notes.push("Wardens segment only: solo, raid level 0–150, no mechanic-changing invocations. Supplies/HP/Prayer are those assumed on entering this room; preceding raid rooms are excluded. Enrage resets Defence to 180 without reapplying earlier reductions.".into());
        }
        for a in &attacks {
            for set in std::iter::once(&p.equipment)
                .chain(p.phases.iter().map(|v| &v.equipment))
                .filter(|s| s[3] == Some(a.weapon))
            {
                for (slot, id) in set.iter().enumerate() {
                    if let Some(id) = id {
                        let need = planner::equipped_quantity(slot, *id, p.ammo_shots);
                        if available.get(id).copied().unwrap_or(0) < need {
                            return Err("A simulation switch uses equipment not present in the starting setup.".into());
                        }
                    }
                }
            }
        }
        Ok(Self {
            mark_ticks: if encounter == Encounter::Yama && o.combat.mark_darkness {
                member.level("Magic").unwrap_or(1)
                    * 3
                    * if attacks.iter().any(|a| {
                        rules::weapon(a.weapon)
                            .is_some_and(|r| r.weapon.demonbane_spell_factor == Some(2))
                    }) {
                        5
                    } else {
                        1
                    }
            } else {
                0
            },
            encounter,
            settings: o.simulation.clone(),
            attacks,
            max_hp: member.level("Hitpoints").ok_or("Add Hitpoints level")?,
            max_prayer: member.level("Prayer").ok_or("Add Prayer level")?,
            hp,
            available,
            foods,
            potions,
            notes,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Usage {
    pub ticks: u32,
    pub attacks: u32,
    pub casts: u32,
    pub shots: u32,
    pub food_bites: u32,
    pub extra_food: u32,
    pub prayer_points: f64,
    pub prayer_doses: u32,
    pub extra_doses: u32,
    pub damage_taken: f64,
    pub ruby_damage: f64,
    pub burn_damage: u32,
    pub burns_applied: u32,
    pub burns_discarded: u32,
    pub burns_cleared: u32,
    pub resources: BTreeMap<i32, u32>,
    pub charges: BTreeMap<i32, u32>,
    pub scales: u32,
    pub crumble: u32,
    pub mark: u32,
    pub ammo_shortage: bool,
    pub acid: u32,
    pub freeze: u32,
    pub judges: u32,
    pub flares: u32,
    pub exploded: u32,
    pub core_downs: u32,
    pub skulls: u32,
    pub enrage: u32,
    pub healing: u32,
    pub phase_ticks: BTreeMap<String, u32>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Run {
    pub completed: bool,
    pub reason: String,
    pub packed: bool,
    pub usage: Usage,
    pub trip_kills: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub version: u32,
    pub inputs: Model,
    pub encounter: Encounter,
    pub settings: Settings,
    pub runs: Vec<Run>,
    pub notes: Vec<String>,
}
impl Report {
    pub fn completed(&self) -> usize {
        self.runs.iter().filter(|r| r.completed).count()
    }
    pub fn covered(&self) -> usize {
        self.runs.iter().filter(|r| r.completed && r.packed).count()
    }
    pub fn quantile(&self, f: impl Fn(&Usage) -> f64, p: f64) -> Option<f64> {
        let mut values: Vec<_> = self
            .runs
            .iter()
            .filter(|r| r.completed)
            .map(|r| f(&r.usage))
            .collect();
        if values.is_empty() {
            return None;
        }
        values.sort_by(f64::total_cmp);
        Some(values[((values.len() - 1) as f64 * p.clamp(0.0, 1.0)).ceil() as usize])
    }
}
pub(crate) struct Rng(pub(crate) u64);
impl Rng {
    pub(crate) fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn chance(&mut self, p: u32) -> bool {
        self.unit() * 100.0 < (p as f64)
    }
    fn damage(&mut self, max: u32) -> u32 {
        (self.unit() * (max + 1) as f64) as u32
    }
}
struct TargetBurn {
    stacks: crate::burn::Burns,
    pulse: u32,
    rule: crate::burn::PulseRule,
}
struct Runner<'a> {
    m: &'a Model,
    rng: Rng,
    hp: f64,
    prayer: f64,
    ready: u32,
    eat_ready: u32,
    drink_ready: u32,
    foods: Vec<(i32, u32, u32)>,
    potions: Vec<(u32, u32)>,
    usage: Usage,
    alive: bool,
    deadline: u32,
    mark_ready: u32,
    burns: BTreeMap<u32, TargetBurn>,
}
impl<'a> Runner<'a> {
    fn new(m: &'a Model, seed: u64) -> Self {
        Self {
            m,
            rng: Rng(seed),
            hp: f64::from(m.max_hp * m.settings.start_hp) / 100.0,
            prayer: f64::from(m.max_prayer * m.settings.start_prayer) / 100.0,
            ready: 0,
            eat_ready: 0,
            drink_ready: 0,
            foods: m.foods.clone(),
            potions: m.potions.clone(),
            usage: Usage::default(),
            alive: true,
            deadline: 6000,
            mark_ready: 0,
            burns: BTreeMap::new(),
        }
    }
    fn spend(&mut self, id: i32, n: u32) {
        *self.usage.resources.entry(id).or_default() += n;
    }
    fn hurt(&mut self, n: f64) {
        self.hp -= n;
        self.usage.damage_taken += n;
        if self.hp <= 0.0 {
            self.alive = false;
        }
    }
    fn fail(&mut self, max: u32) {
        if self.rng.chance(self.m.settings.mistakes) {
            let n = self.rng.damage(max);
            self.hurt(n as f64);
        }
    }
    fn covered(&self) -> bool {
        self.usage.extra_food == 0
            && self.usage.extra_doses == 0
            && !self.usage.ammo_shortage
            && self
                .usage
                .resources
                .iter()
                .all(|(id, n)| *n <= self.m.available.get(id).copied().unwrap_or(0))
            && self
                .usage
                .charges
                .values()
                .all(|n| *n <= self.m.settings.charges)
            && self.usage.scales <= self.m.settings.charges
    }
    fn tick(&mut self, phase: &str, attack: usize, incoming: bool) -> bool {
        self.usage.ticks += 1;
        if let Some(ticks) = self.usage.phase_ticks.get_mut(phase) {
            *ticks += 1;
        } else {
            self.usage.phase_ticks.insert(phase.into(), 1);
        }
        if self.usage.ticks > self.deadline || !self.alive {
            return false;
        }
        let now = self.usage.ticks;
        for burn in self.burns.values_mut() {
            burn.pulse = burn.stacks.tick();
        }
        self.burns
            .retain(|_, b| b.stacks.stacks() > 0 || b.pulse > 0);
        if self.m.mark_ticks > 0 && now >= self.mark_ready {
            for (id, n) in rules::mark_darkness_supplies() {
                self.spend(id, n);
            }
            self.usage.mark += 1;
            self.mark_ready = now + self.m.mark_ticks;
        }
        let drain = if phase == "Between fights" {
            0.0
        } else {
            self.m.attacks[attack].prayer
        };
        self.prayer -= drain;
        self.usage.prayer_points += drain;
        if incoming && now.is_multiple_of(5) {
            // A calibrated stochastic damage budget, NOT a fabricated NPC mix.
            let mean = f64::from(self.m.settings.incoming) / 20.0;
            let damage = self.rng.unit() * 2.0 * mean;
            self.hurt(damage);
        }
        if !self.alive {
            return false;
        }
        if self.hp < f64::from(self.m.max_hp * self.m.settings.eat_at) / 100.0
            && now >= self.eat_ready
        {
            let (heal, extra) =
                if let Some(food) = self.foods.iter_mut().find(|(_, _, left)| *left > 0) {
                    food.2 -= 1;
                    (food.1, false)
                } else {
                    (20, true)
                };
            self.hp = (self.hp + f64::from(heal)).min(self.m.max_hp as f64);
            self.usage.food_bites += 1;
            self.usage.extra_food += u32::from(extra);
            self.ready = self.ready.max(now) + 3;
            self.eat_ready = now + 3;
        }
        if self.prayer < 0.0 && now >= self.drink_ready {
            let (restore, extra) =
                if let Some(p) = self.potions.iter_mut().find(|(_, left)| *left > 0) {
                    p.1 -= 1;
                    (p.0, false)
                } else {
                    (self.m.max_prayer / 4 + 7, true)
                };
            self.prayer = (self.prayer + f64::from(restore)).min(self.m.max_prayer as f64);
            self.usage.prayer_doses += 1;
            self.usage.extra_doses += u32::from(extra);
            self.drink_ready = now + 3;
        }
        true
    }
    fn wait(&mut self, n: u32, phase: &str, attack: usize, incoming: bool) -> bool {
        for _ in 0..n {
            if !self.tick(phase, attack, incoming) {
                return false;
            }
        }
        true
    }
    fn hit(&mut self, index: usize, hp: u32, uptime: u32, target: u32) -> Option<u32> {
        self.hit_scaled(index, hp, uptime, target, 1, 1)
    }
    fn hit_scaled(
        &mut self,
        index: usize,
        hp: u32,
        uptime: u32,
        target: u32,
        divisor: u32,
        multiplier: u32,
    ) -> Option<u32> {
        let now = self.usage.ticks;
        if now < self.ready || !self.alive || hp == 0 {
            return None;
        }
        if !self.rng.chance(uptime) {
            if uptime > 0 {
                self.ready = now + self.m.attacks[index].hits.attack_ticks.max(1) as u32;
            }
            return None;
        }
        let a = &self.m.attacks[index];
        let (n, ruby, proc_burn) = a.roll(&mut self.rng, hp, self.hp);
        let n = (n.max(0) as u32 / divisor.max(1)).saturating_mul(multiplier);
        if n >= hp {
            self.burns.remove(&target);
        } else if proc_burn {
            let rule = a.hits.burn.as_ref().expect("burn proc profile").pulse;
            let pending = self.burns.entry(target).or_insert_with(|| TargetBurn {
                stacks: Default::default(),
                pulse: 0,
                rule,
            });
            pending.rule = rule;
            if pending.stacks.apply() {
                self.usage.burns_applied += 1;
            } else {
                self.usage.burns_discarded += 1;
            }
        }
        self.ready = now + a.hits.attack_ticks.max(1) as u32;
        self.usage.attacks += 1;
        let ruby_cost = if ruby { (self.hp / 10.0).floor() } else { 0.0 };
        if !a.runes.is_empty() {
            self.usage.casts += 1;
        }
        for (id, q) in &a.runes {
            *self.usage.resources.entry(*id).or_default() += q;
        }
        if let Some(id) = a.projectile {
            self.usage.shots += 1;
            let used = self.usage.resources.entry(id).or_default();
            if *used >= self.m.available.get(&id).copied().unwrap_or(0) {
                self.usage.ammo_shortage = true;
            }
            if self
                .rng
                .chance(a.projectile_loss.unwrap_or(self.m.settings.ammo_loss))
            {
                *used += 1;
            }
        }
        if a.charge && (!a.charge_on_damage || n > 0) {
            *self.usage.charges.entry(a.weapon).or_default() += 1;
        }
        if a.scales && self.rng.unit() < 2.0 / 3.0 {
            self.usage.scales += 1;
        }
        if ruby_cost > 0.0 {
            self.usage.ruby_damage += ruby_cost;
            self.hurt(ruby_cost);
        }
        Some(n)
    }
    fn burn_damage(&mut self, target: u32, hp: &mut u32, index: usize, divisor: Option<u32>) {
        if let Some(pending) = self.burns.get_mut(&target) {
            let mut rule = self.m.attacks[index]
                .hits
                .burn
                .as_ref()
                .map(|b| b.pulse)
                .unwrap_or(pending.rule);
            if let Some(divisor) = divisor {
                rule.divisor = divisor;
            }
            let damage = rule.damage(pending.pulse).min(*hp);
            pending.pulse = 0;
            *hp -= damage;
            self.usage.burn_damage += damage;
        }
        if *hp == 0 {
            self.burns.remove(&target);
        }
    }
    fn clear_burns(&mut self) {
        self.usage.burns_cleared += self.burns.values().map(|b| b.stacks.stacks()).sum::<u32>();
        self.burns.clear();
    }
    fn phase_reset(&mut self) {
        if self.m.settings.burn_phase_resets == 1 {
            self.clear_burns();
        }
    }
    fn vorkath_boundary(&mut self, acid: bool) {
        // Eclipse burns persist into and out of acid in the inspected footage.
        // Freeze/spawn reset semantics remain an explicit scenario assumption.
        // See docs/eclipse-burn-gameplay.md for observations and version limits.
        if !acid {
            self.phase_reset();
        }
    }
    fn attack_until(&mut self, hp: &mut u32, stop: u32, index: usize, phase: &str) -> bool {
        while *hp > stop {
            if !self.tick(phase, index, true) {
                return false;
            }
            self.burn_damage(0, hp, index, None);
            if *hp <= stop {
                break;
            }
            if let Some(n) = self.hit(index, *hp, self.m.settings.uptime, 0) {
                *hp = hp.saturating_sub(n);
            }
        }
        true
    }
    fn vorkath(&mut self) -> bool {
        let mut hp = self.m.hp[0];
        let mut acid = self.rng.chance(50);
        while hp > 0 {
            for _ in 0..30 {
                if !self.tick("Normal attacks", 0, true) {
                    return false;
                }
                self.burn_damage(0, &mut hp, 0, None);
                if hp == 0 {
                    return true;
                }
                if let Some(n) = self.hit(0, hp, self.m.settings.uptime, 0) {
                    hp = hp.saturating_sub(n);
                }
                if hp == 0 {
                    return true;
                }
            }
            self.vorkath_boundary(acid);
            if acid {
                self.usage.acid += 1;
                for _ in 0..self.m.settings.acid_ticks {
                    if !self.tick("Acid", 1, false) {
                        return false;
                    }
                    if self.rng.chance(self.m.settings.mistakes) {
                        let heal = self.rng.damage(10);
                        hp = (hp + heal).min(self.m.hp[0]);
                        self.usage.healing += heal;
                        let hit = self.rng.damage(16) + 25;
                        self.hurt(hit as f64 + heal as f64);
                    }
                    self.burn_damage(0, &mut hp, 1, Some(2));
                    if hp == 0 {
                        return self.alive;
                    }
                    if let Some(n) = self.hit(1, hp, self.m.settings.acid_uptime, 0) {
                        hp = hp.saturating_sub(n);
                    }
                    if hp == 0 {
                        return self.alive;
                    }
                }
            } else {
                self.usage.freeze += 1;
                self.usage.crumble += 1;
                // Crumble Undead: two air, two earth, one chaos per spawn.
                self.spend(556, 2);
                self.spend(557, 2);
                self.spend(562, 1);
                self.ready = self.ready.max(self.usage.ticks + 5);
                if !self.wait(self.m.settings.freeze_ticks, "Freeze / spawn", 0, false) {
                    return false;
                }
                self.fail(60);
            }
            self.vorkath_boundary(acid);
            acid = !acid;
            if !self.alive {
                return false;
            }
        }
        true
    }
    fn yama(&mut self) -> bool {
        let maximum = self.m.hp[0];
        let mut hp = maximum;
        for stage in 0..3 {
            let stop = match stage {
                0 => maximum * 2 / 3,
                1 => maximum / 3,
                _ => 0,
            };
            let attack = usize::from(stage == 2);
            let mut elapsed = 0u32;
            let mut flare_total = 0;
            let mut flares: Vec<(u32, u32, u32, u32)> = Vec::new();
            while hp > stop {
                elapsed += 1;
                if !self.tick(
                    if stage == 2 {
                        "Yama final"
                    } else {
                        "Yama P1/P2"
                    },
                    attack,
                    true,
                ) {
                    return false;
                }
                self.burn_damage(0, &mut hp, attack, None);
                if hp == 0 {
                    return self.alive;
                }
                for flare in &mut flares {
                    self.burn_damage(flare.3, &mut flare.0, if stage == 2 { 5 } else { 4 }, None);
                }
                flares.retain(|f| f.0 > 0);
                if hp <= stop {
                    break;
                }
                let auto = if stage == 2 { 7 } else { 8 };
                if stage == 2
                    && elapsed.is_multiple_of(7)
                    && self.rng.chance(self.m.settings.melee_autos)
                    && flares.len() < 4
                {
                    flares.push((
                        71,
                        71,
                        elapsed + self.m.settings.flare_charge,
                        flare_total + 1,
                    ));
                    flare_total += 1;
                    self.usage.flares += 1;
                }
                if elapsed == auto || elapsed > auto && (elapsed - auto).is_multiple_of(auto * 3) {
                    if stage < 2 {
                        self.hurt(self.m.settings.glyph_hp as f64);
                        self.prayer -= self.m.settings.glyph_prayer as f64;
                        self.usage.prayer_points += self.m.settings.glyph_prayer as f64;
                    }
                    self.fail(60);
                }
                if elapsed.is_multiple_of(self.m.settings.flare_interval) {
                    let count = if stage == 2 && flare_total >= 8 { 3 } else { 2 };
                    for _ in 0..count {
                        if flares.len() < 4 {
                            let maximum = if stage == 2 { 71 } else { 140 };
                            flares.push((
                                maximum,
                                maximum,
                                elapsed + self.m.settings.flare_charge,
                                flare_total + 1,
                            ));
                            flare_total += 1;
                            self.usage.flares += 1;
                        }
                    }
                }
                let mut i = 0;
                while i < flares.len() {
                    if elapsed >= flares[i].2 {
                        let fraction = flares[i].0 as f64 / flares[i].1 as f64;
                        let heal = (maximum as f64 * self.m.settings.flare_heal as f64 / 100.0
                            * fraction)
                            .floor() as u32;
                        hp = (hp + heal).min(maximum);
                        self.usage.healing += heal;
                        self.usage.exploded += 1;
                        self.hurt((if stage == 2 { 24.0 } else { 43.0 }) * fraction);
                        self.burns.remove(&flares[i].3);
                        flares.remove(i);
                    } else {
                        if elapsed.is_multiple_of(self.m.settings.flare_regen) {
                            flares[i].0 = (flares[i].0 + 1).min(flares[i].1)
                        }
                        i += 1;
                    }
                }
                if let Some(flare) = flares.first_mut() {
                    if let Some(n) = self.hit(
                        if stage == 2 { 5 } else { 4 },
                        flare.0,
                        self.m.settings.uptime,
                        flare.3,
                    ) {
                        flare.0 = flare.0.saturating_sub(n);
                        if flare.0 == 0 {
                            flares.remove(0);
                        }
                    }
                } else if let Some(n) = self.hit(attack, hp, self.m.settings.uptime, 0) {
                    hp = hp.saturating_sub(n);
                }
            }
            if hp == 0 {
                return self.alive;
            }
            if stage < 2 {
                for flare in &flares {
                    self.burns.remove(&flare.3);
                }
                self.phase_reset();
                // Thresholds are soft: attacks already overkill into the next
                // stage. Transition attacks continue at half damage.
                for _ in 0..self.m.settings.transition {
                    if !self.tick("Yama transition", attack, false) {
                        return false;
                    }
                    self.burn_damage(0, &mut hp, attack, Some(2));
                    if hp == 0 {
                        return self.alive;
                    }
                    if let Some(n) = self.hit_scaled(attack, hp, self.m.settings.uptime, 0, 2, 1) {
                        hp = hp.saturating_sub(n);
                    }
                }
                if !self.wait(self.m.settings.judge_travel, "Judge travel", 2, false) {
                    return false;
                }
                let mut judge = self.m.hp[2];
                let mut magic = true;
                self.usage.judges += 1;
                let judge_key = 10_000 + stage;
                while judge > 0 {
                    let index = if magic { 2 } else { 3 };
                    if !self.tick("Judge alternation", index, true) {
                        return false;
                    }
                    self.burn_damage(judge_key, &mut judge, index, None);
                    if judge == 0 {
                        break;
                    }
                    if let Some(n) = self.hit(index, judge, self.m.settings.uptime, judge_key) {
                        judge = judge.saturating_sub(n);
                        magic = !magic;
                    }
                }
                self.burns.remove(&judge_key);
                self.phase_reset();
                if !self.wait(
                    self.m.settings.transition,
                    "Return from Judge",
                    attack,
                    false,
                ) {
                    return false;
                }
            }
        }
        self.alive
    }
    fn wardens(&mut self) -> bool {
        let mut obelisk = self.m.hp[0];
        if !self.attack_until(&mut obelisk, 0, 0, "Obelisk") {
            return false;
        }
        if !self.wait(self.m.settings.transition, "P1 to P2", 0, false) {
            return false;
        }
        let mut body = self.m.hp[3];
        let max_body = body;
        let first_down = self.usage.core_downs;
        let mut elapsed = 0u32;
        while body > 0 {
            if self.usage.core_downs - first_down >= 100 {
                return false;
            }
            let mut shield = self.m.hp[1];
            while shield > 0 {
                // Elidinis opens to Magic; Tumeken opens to Ranged.
                let ranged =
                    ((elapsed / self.m.settings.prayer_rotation + self.m.settings.warden) % 2) == 1;
                let index = if ranged { 2 } else { 1 };
                elapsed += 1;
                if !self.tick("P2 shield", index, true) {
                    return false;
                }
                self.burn_damage(1, &mut shield, index, None);
                if shield == 0 {
                    break;
                }
                if let Some(n) = self.hit(index, shield, self.m.settings.uptime, 1) {
                    shield = shield.saturating_sub(n);
                }
            }
            self.usage.core_downs += 1;
            self.phase_reset();
            if !self.wait(self.m.settings.transition, "Core ejection", 3, false) {
                return false;
            }
            let exposure = core_exposure(body, max_body);
            for _ in 0..exposure {
                if !self.tick("Core exposure", 3, false) {
                    return false;
                }
                self.burn_damage(2, &mut body, 3, None);
                if body == 0 {
                    break;
                }
                if let Some(n) = self.hit_scaled(3, body, self.m.settings.uptime, 2, 1, 5) {
                    body = body.saturating_sub(n);
                }
                if body == 0 {
                    break;
                }
            }
            if body > 0 && !self.wait(self.m.settings.transition, "Core return", 3, false) {
                return false;
            }
            self.phase_reset();
        }
        if !self.wait(self.m.settings.transition, "P2 to P3", 4, false) {
            return false;
        }
        let maximum = self.m.hp[4];
        self.phase_reset();
        let mut hp = maximum;
        let mut skull = 80;
        let mut enraged = false;
        let p3_start = self.usage.ticks;
        while hp > 0 || !enraged {
            if skull > 0 && hp * 100 <= maximum * skull {
                self.phase_reset();
                self.usage.skulls += 1;
                if !self.wait(self.m.settings.skull_ticks, "Skulls", 3, false) {
                    return false;
                }
                if self.rng.chance(self.m.settings.mistakes) {
                    self.fail_skulls();
                } else {
                    hp = hp.saturating_sub(maximum * self.m.settings.skull_damage / 100);
                }
                skull = skull.saturating_sub(20);
                continue;
            }
            if !enraged && hp * 100 <= maximum * 5 {
                self.phase_reset();
                hp = hp.saturating_add(maximum / 5);
                self.usage.healing += maximum / 5;
                self.usage.enrage += 1;
                enraged = true;
                if !self.wait(self.m.settings.transition, "Enrage transition", 5, false) {
                    return false;
                }
            }
            let index = if enraged { 5 } else { 4 };
            if !self.tick(if enraged { "Enrage" } else { "P3" }, index, true) {
                return false;
            }
            if (self.usage.ticks - p3_start).is_multiple_of(21) {
                self.fail(40);
            }
            self.burn_damage(3, &mut hp, index, None);
            if hp == 0 && enraged {
                return self.alive;
            }
            if skull > 0 && hp * 100 <= maximum * skull || !enraged && hp * 100 <= maximum * 5 {
                continue;
            }
            if let Some(n) = self.hit(index, hp, self.m.settings.uptime, 3) {
                hp = hp.saturating_sub(n);
            }
        }
        self.alive
    }
    fn fail_skulls(&mut self) {
        let n = self.rng.damage(40);
        self.hurt(n as f64);
    }
    fn fight(&mut self) -> bool {
        self.clear_burns();
        let start = self.usage.ticks;
        self.deadline = start + 6000;
        // Do not let an earlier fight's long-running clock consume this one's
        // timeout allowance. A single kill is bounded to one hour.
        let completed = match self.m.encounter {
            Encounter::Vorkath => self.vorkath(),
            Encounter::Yama => self.yama(),
            Encounter::Wardens => self.wardens(),
        };
        self.clear_burns();
        completed && self.alive && self.usage.ticks - start <= 6000
    }
}
pub fn core_exposure(remaining: u32, maximum: u32) -> u32 {
    if u64::from(remaining) * 100 >= u64::from(maximum) * 80 {
        21
    } else if u64::from(remaining) * 100 >= u64::from(maximum) * 60 {
        29
    } else {
        37
    }
}
pub fn simulate(model: &Model) -> Result<Report, String> {
    if model
        .attacks
        .iter()
        .any(|a| a.hits.burn_dps > 0.0 && a.hits.burn.is_none())
    {
        return Err("This saved scenario lacks the joint Eclipse burn outcomes. Rebuild it from the current gear before replaying.".into());
    }
    let count = match model.encounter {
        Encounter::Vorkath => 2,
        _ => 6,
    };
    if !model.settings.valid()
        || model.attacks.len() != count
        || model.hp.len() != count
        || model.hp.iter().any(|hp| *hp == 0 || *hp > 100_000)
        || model.available.len() > 256
        || model.foods.len() > 28
        || model.potions.len() > 28
        || model.mark_ticks > 10_000
        || model.attacks.iter().any(|a| {
            !(1..=20).contains(&a.hits.attack_ticks)
                || a.hits.burn.as_ref().is_some_and(|b| {
                    !b.matches(&a.hits.hit_distribution) || a.hits.hp_dependent.is_some()
                })
                || a.projectile_loss.is_some_and(|n| n > 100)
                || !(0.0..=100.0).contains(&a.prayer)
                || a.runes.len() > 32
                || a.runes
                    .iter()
                    .any(|(id, q)| *id <= 0 || *q == 0 || *q > 100)
                || a.hits.hit_distribution.len() > 2048
                || a.hits.hp_dependent.is_some_and(|p| {
                    !(0.0..1.0).contains(&p.chance)
                        || !(1..=100).contains(&p.percent)
                        || !(1..=10_000).contains(&p.cap)
                        || !(1..=2).contains(&p.divisor)
                        || a.hits
                            .hit_distribution
                            .get(
                                a.hits
                                    .hit_distribution
                                    .len()
                                    .saturating_sub(p.distribution_entries)..,
                            )
                            .is_none_or(|v| {
                                p.distribution_entries == 0
                                    || p.distribution_entries > a.hits.hit_distribution.len()
                                    || (v.iter().map(|v| v.1).sum::<f64>() - p.chance).abs() > 1e-8
                            })
                })
                || a.hits.hit_distribution.is_empty()
                || a.hits
                    .hit_distribution
                    .iter()
                    .any(|(d, p)| *d < 0 || !p.is_finite() || *p < 0.0)
                || (a.hits.hit_distribution.iter().map(|(_, p)| *p).sum::<f64>() - 1.0).abs() > 1e-8
        })
        || !(1..=255).contains(&model.max_hp)
        || !(1..=255).contains(&model.max_prayer)
    {
        return Err("Invalid simulation model.".into());
    }
    let mut runs = Vec::new();
    for trial in 0..model.settings.trials {
        let mut r = Runner::new(
            model,
            u64::from(model.settings.seed) ^ u64::from(trial).wrapping_mul(0x9e3779b97f4a7c15),
        );
        let completed = r.fight();
        let packed = r.covered();
        let usage = r.usage.clone();
        let mut kills = u32::from(completed && packed);
        if completed && packed {
            for _ in 1..model.settings.trip_limit {
                r.deadline = r.usage.ticks + model.settings.between + 1;
                if !r.wait(model.settings.between, "Between fights", 0, false)
                    || !r.fight()
                    || !r.covered()
                {
                    break;
                }
                kills += 1;
            }
        }
        runs.push(Run {
            completed,
            packed,
            usage,
            trip_kills: kills,
            reason: if completed {
                "Completed"
            } else if !r.alive {
                "HP depleted"
            } else {
                "Simulation time limit"
            }
            .into(),
        });
    }
    Ok(Report {
        version: 1,
        inputs: model.clone(),
        encounter: model.encounter,
        settings: model.settings.clone(),
        runs,
        notes: model.notes.clone(),
    })
}

#[cfg(test)]
mod consumption_tests {
    use super::*;
    fn eclipse_model() -> Model {
        let mut member = crate::account::Member::default();
        for skill in crate::account::SKILLS {
            member.level_overrides.insert(skill.into(), 99);
        }
        let mut set = vec![None; 14];
        for (slot, id) in [(0, 29010), (4, 29004), (7, 29007), (3, 29000), (13, 28991)] {
            set[slot] = Some(id);
        }
        let mut hits =
            combat::calculate(&member, &set, "Ranged", planner::monster(26).unwrap(), "").unwrap();
        hits.hit_distribution = vec![(0, 1.0)];
        hits.attack_ticks = 3;
        hits.expected_attack_ticks = 3.0;
        hits.burn = Some(crate::burn::BurnAttack {
            hits: vec![crate::burn::BurnHit {
                damage: 0,
                probability: 1.0,
                applies_burn: true,
            }],
            pulse: Default::default(),
        });
        let attack = Attack {
            hits,
            weapon: 29000,
            runes: vec![],
            projectile: None,
            projectile_loss: None,
            charge: false,
            charge_on_damage: false,
            scales: false,
            prayer: 0.0,
        };
        Model {
            encounter: Encounter::Vorkath,
            settings: Settings {
                incoming: 0,
                mistakes: 0,
                ..Default::default()
            },
            attacks: vec![attack; 2],
            max_hp: 99,
            max_prayer: 99,
            hp: vec![100; 2],
            available: BTreeMap::new(),
            foods: vec![],
            potions: vec![],
            notes: vec![],
            mark_ticks: 0,
        }
    }
    #[test]
    fn burns_stay_on_their_target_survive_weapon_switches_and_expire_in_downtime() {
        let mut model = eclipse_model();
        model.attacks[1].hits.burn = None;
        let mut r = Runner::new(&model, 1);
        assert!(r.tick("Attack", 0, false));
        assert_eq!(r.hit(0, 100, 100, 7), Some(0));
        assert!(r.wait(4, "Invulnerable downtime", 0, false));
        let mut other = 100;
        r.burn_damage(8, &mut other, 1, None);
        assert_eq!(other, 100);
        let mut first = 100;
        r.burn_damage(7, &mut first, 1, None);
        assert_eq!(first, 99);
        assert!(r.wait(50, "Invulnerable downtime", 0, false));
        r.burn_damage(7, &mut first, 1, None);
        assert_eq!(first, 99, "expired pulses are not queued for later");
        assert!(r.burns.is_empty());
    }
    #[test]
    fn vorkath_acid_preserves_burn_clock_and_stacks_at_both_boundaries() {
        let model = eclipse_model();
        let mut r = Runner::new(&model, 1);
        let mut stacks = crate::burn::Burns::default();
        for _ in 0..5 {
            assert!(stacks.apply());
        }
        r.burns.insert(
            0,
            TargetBurn {
                stacks,
                pulse: 0,
                rule: Default::default(),
            },
        );
        r.ready = 20;
        r.vorkath_boundary(true);
        let mut hp = 100;
        for _ in 0..8 {
            assert!(r.tick("Acid", 1, false));
            r.burn_damage(0, &mut hp, 1, Some(2));
        }
        assert_eq!(
            hp, 96,
            "two combined five-stack acid pulses, each floored to two"
        );
        r.vorkath_boundary(true);
        for _ in 0..4 {
            assert!(r.tick("Normal", 0, false));
            r.burn_damage(0, &mut hp, 0, None);
        }
        assert_eq!(hp, 91, "the existing clock and stacks survive acid exit");
        assert_eq!(
            (r.ready, r.usage.attacks, r.usage.burns_cleared),
            (20, 0, 0)
        );
        r.vorkath_boundary(false);
        assert!(r.burns.is_empty());
        assert_eq!(
            (r.ready, r.usage.burns_cleared),
            (20, 5),
            "freeze policy remains independent"
        );
    }
    #[test]
    fn vorkath_acid_carry_still_expires_and_stops_at_target_death() {
        let model = eclipse_model();
        for initial_hp in [1, 100] {
            let mut r = Runner::new(&model, 1);
            let mut stacks = crate::burn::Burns::default();
            for _ in 0..5 {
                stacks.apply();
            }
            r.burns.insert(
                0,
                TargetBurn {
                    stacks,
                    pulse: 0,
                    rule: Default::default(),
                },
            );
            let mut hp = initial_hp;
            r.vorkath_boundary(true);
            while hp > 0 && r.usage.ticks < 40 {
                assert!(r.tick("Acid", 1, false));
                r.burn_damage(0, &mut hp, 1, Some(2));
            }
            if initial_hp == 1 {
                assert_eq!((hp, r.usage.ticks, r.usage.burn_damage), (0, 4, 1));
            } else {
                assert_eq!((hp, r.usage.burn_damage), (80, 20));
                r.vorkath_boundary(true);
                assert!(r.tick("Normal", 0, false));
                r.burn_damage(0, &mut hp, 0, None);
                assert_eq!(hp, 80, "expired acid pulses cannot queue damage after exit");
            }
            assert!(r.burns.is_empty());
            assert_eq!(r.usage.attacks, 0);
        }
    }
    #[test]
    fn reset_discards_target_stacks_without_resetting_attack_cooldown() {
        let model = eclipse_model();
        let mut r = Runner::new(&model, 1);
        r.tick("Attack", 0, false);
        r.hit(0, 100, 100, 7);
        let ready = r.ready;
        r.phase_reset();
        assert!(r.burns.is_empty());
        assert_eq!(r.ready, ready);
        assert_eq!(r.usage.burns_cleared, 1);
        let mut hp = 100;
        r.wait(4, "Next phase", 0, false);
        r.burn_damage(7, &mut hp, 0, None);
        assert_eq!(hp, 100);
    }
    #[test]
    fn phase_scaled_hits_apply_burn_and_death_checks_using_actual_damage() {
        let mut model = eclipse_model();
        model.attacks[0].hits.hit_distribution = vec![(4, 1.0)];
        model.attacks[0].hits.burn.as_mut().unwrap().hits[0].damage = 4;
        let mut r = Runner::new(&model, 1);
        r.tick("Transition", 0, false);
        assert_eq!(r.hit_scaled(0, 3, 100, 7, 2, 1), Some(2));
        assert_eq!(
            r.burns[&7].stacks.stacks(),
            1,
            "halved hit did not kill the target"
        );
        let mut r = Runner::new(&model, 1);
        r.tick("Core", 0, false);
        assert_eq!(r.hit_scaled(0, 9, 100, 7, 1, 5), Some(20));
        assert!(!r.burns.contains_key(&7));
        assert_eq!(r.usage.burns_applied, 0);
    }
    #[test]
    fn real_weapon_adapters_distinguish_racks_charged_and_corrupted_crystal_bows() {
        let mut snapshot = crate::account::demo();
        for skill in crate::account::SKILLS {
            snapshot.members[0].level_overrides.insert(skill.into(), 99);
        }
        let o = Options {
            boss: Boss::Custom,
            targets: vec![planner::Target {
                id: 8059,
                style: "Ranged".into(),
            }],
            ..Default::default()
        };
        let mut plan = planner::plan(&snapshot, &Options::default());
        plan.phases.clear();
        plan.equipment = vec![None; 14];
        for (weapon, ammo, charged, loss) in [
            (4734, Some(4740), false, Some(100)),
            (25865, None, true, None),
            (25867, None, false, None),
        ] {
            plan.equipment[3] = Some(weapon);
            plan.equipment[13] = ammo;
            let a = attack(&snapshot, &o, &plan, 8059, "Ranged", false).unwrap();
            assert_eq!((a.charge, a.projectile_loss), (charged, loss));
        }
    }
}
