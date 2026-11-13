//! Boat attack distributions and bounded, seeded trip scenarios. Formula inputs
//! are pinned to current Wiki raw source. Repair cadence is an execution input.
use crate::{
    account::{Member, Snapshot},
    combat, defence, modifiers,
    planner::{self, Monster, Options, Plan},
    prayer, rules,
    sailing::{self, CREW, Settings},
    sailing_owned::{self, BALL_IDS, KIT_IDS},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};
pub const KIT_HEAL: [u32; 7] = [5, 10, 20, 30, 40, 45, 50];
const C_STR: [i64; 7] = [8, 10, 12, 15, 23, 26, 32];
const C_ACC: [i64; 7] = [30, 42, 60, 78, 98, 163, 194];
const P_ACC: [i64; 7] = [60, 82, 120, 148, 194, 273, 344];
const B_STR: [i64; 7] = [43, 56, 74, 105, 157, 201, 270];
const P_STR: [i64; 7] = [58, 86, 114, 155, 237, 291, 370];
const B_ACC: [i64; 7] = [12, 16, 20, 30, 50, 64, 80];
const RANGE_REQ: [u32; 7] = [1, 1, 5, 20, 30, 40, 60];
pub const PRIVATEERING: [u32; 7] = [1, 2, 2, 3, 3, 4, 4];
#[derive(Deserialize)]
struct Data {
    hull_defence: [[i64; 7]; 3],
    hull_bonuses: [[[i64; 7]; 7]; 3],
}
fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/sailing-combat.json"))
            .expect("boat defence data")
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hit {
    pub accuracy: f64,
    pub max_hit: u32,
    pub raw_max: u32,
    pub attack_roll: i64,
    pub defence_roll: i64,
    pub distribution: Vec<(u32, f64)>,
    pub ticks: f64,
    pub dps: f64,
}
fn hit(attack: i64, defence: i64, max: u32, armour: u32, ticks: f64) -> Hit {
    let accuracy = combat::accuracy(attack, defence);
    let mut weights = BTreeMap::<u32, f64>::new();
    for roll in 0..=max {
        *weights.entry(roll.saturating_sub(armour)).or_default() += accuracy / f64::from(max + 1);
    }
    *weights.entry(0).or_default() += 1.0 - accuracy;
    let distribution: Vec<_> = weights.into_iter().collect();
    let mean = distribution
        .iter()
        .map(|(n, p)| f64::from(*n) * p)
        .sum::<f64>();
    Hit {
        accuracy,
        max_hit: max.saturating_sub(armour),
        raw_max: max,
        attack_roll: attack,
        defence_roll: defence,
        distribution,
        ticks,
        dps: mean / (ticks * 0.6),
    }
}
pub fn cannon(
    member: &Member,
    set: &[Option<i32>],
    target: &Monster,
    tier: usize,
    ball: usize,
    person: Option<usize>,
    perildance: bool,
    settings: &defence::Settings,
) -> Result<Hit, String> {
    if tier >= 7 || ball > tier || !sailing::is_sea(target.id) || set.len() != 14 {
        return Err("Unsupported cannon configuration or sea target.".into());
    }
    let (effective_attack, effective_strength, accuracy, strength) = if let Some(person) = person {
        let crew = CREW.get(person).ok_or("Unknown crew")?;
        if crew.privateering < PRIVATEERING[tier] {
            return Err("Crew cannot operate this cannon.".into());
        }
        let level = i64::from(crew.privateering) * 12 + 8;
        (level, level, C_ACC[tier], B_STR[ball])
    } else {
        if !member.level("Ranged").is_some_and(|n| n >= RANGE_REQ[tier]) {
            return Err("Ranged requirement for this cannon is unmet or unknown.".into());
        }
        prayer::eligible_settings(member, settings)?;
        let level = i64::from(
            modifiers::level(member, "Ranged", &settings.boosts).ok_or("Ranged level unknown")?,
        );
        let p = prayer::selected(settings);
        let ranged = p.style == "Ranged";
        let mut a = level * (100 + if ranged { p.attack } else { 0 }) / 100 + 8;
        let mut s = level * (100 + if ranged { p.strength } else { 0 }) / 100 + 8;
        if rules::set_effect(set, "void_ranged") {
            a = a * 11 / 10;
            s = if rules::set_effect(set, "elite_void_ranged") {
                s * 9 / 8
            } else {
                s * 11 / 10
            };
        }
        (a, s, P_ACC[tier], P_STR[ball])
    };
    let mut worn = set.to_vec();
    for slot in [3, 5, 13] {
        worn[slot] = None;
    }
    let bonuses = if person.is_none() {
        combat::bonuses(&worn)
    } else {
        Default::default()
    };
    let attack = effective_attack
        * (accuracy + B_ACC[ball] + i64::from(*bonuses.offensive.get("ranged").unwrap_or(&0)) + 64)
            .max(0);
    let max = ((effective_strength
        * (C_STR[tier]
            + strength
            + i64::from(*bonuses.other.get("ranged_str").unwrap_or(&0))
            + 64)
            .max(0)
        + 320)
        / 640)
        .max(0) as u32
        + u32::from(perildance);
    let def = (i64::from(*target.skills.get("def").ok_or("Target Defence missing")?) + 9)
        * (i64::from(
            *target
                .defensive
                .get("heavy")
                .ok_or("Heavy Ranged defence missing")?,
        ) + 64);
    let armour = (*target.defensive.get("flat_armour").unwrap_or(&0)).max(0) as u32;
    Ok(hit(attack, def, max, armour, 7.0))
}
pub fn incoming(b: &Settings, target: &Monster) -> Result<Hit, String> {
    if !b.valid() || !sailing::is_sea(target.id) || target.attack_styles.len() != 1 {
        return Err("A reviewed single-style sea attack is required.".into());
    }
    let (skill, offence, index) = match target.attack_styles[0].as_str() {
        "Stab" => ("atk", "atk", 0),
        "Slash" => ("atk", "atk", 1),
        "Crush" => ("atk", "atk", 2),
        "Ranged" => ("ranged", "ranged", 4),
        "Magic" => ("magic", "magic", 6),
        _ => return Err("Sea attack style unverified.".into()),
    };
    let a = (i64::from(*target.skills.get(skill).ok_or("NPC attack level missing")?) + 9)
        * (i64::from(
            *target
                .offensive
                .get(offence)
                .ok_or("NPC offensive bonus missing")?,
        ) + 64);
    let d = (data().hull_defence[b.boat.index()][b.hull] + 9)
        * (data().hull_bonuses[b.boat.index()][b.hull][index] + 64);
    let max = target
        .max_hit
        .parse::<u32>()
        .map_err(|_| "NPC max hit unverified")?;
    if target.speed <= 0 {
        return Err("NPC attack interval unverified.".into());
    }
    Ok(hit(a, d, max, b.armour(), f64::from(target.speed)))
}
#[derive(Clone, Debug)]
pub struct Stream {
    pub name: String,
    pub hit: Hit,
    pub personal: Option<combat::ResultStats>,
    pub ammo: Option<i32>,
    pub ammo_cost: u32,
    pub runes: Vec<(i32, u32)>,
    pub charge: bool,
    pub player: bool,
    pub unprayed: Option<Box<Stream>>,
}
impl Stream {
    pub fn funded(&self, stock: &BTreeMap<i32, u32>, charges: u32) -> bool {
        self.ammo
            .is_none_or(|id| stock.get(&id).copied().unwrap_or(0) >= self.ammo_cost)
            && self
                .runes
                .iter()
                .all(|(id, n)| stock.get(id).copied().unwrap_or(0) >= *n)
            && (!self.charge || charges > 0)
    }
}
#[derive(Clone, Debug)]
pub struct Model {
    pub boat: Settings,
    pub target_hp: u32,
    pub incoming: Hit,
    pub streams: Vec<Stream>,
    pub stock: BTreeMap<i32, u32>,
    pub kit_id: i32,
    pub kit_heal: u32,
    pub kit_count: u32,
    pub crew_repairs: bool,
    pub prayer_points: f64,
    pub prayer_drain: f64,
    pub player_hp: u32,
    pub crew_dps: f64,
    pub player_dps: f64,
    pub notes: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub trials: u32,
    pub completed: u32,
    pub sunk: u32,
    pub depleted: u32,
    pub timed_out: u32,
    pub mean_kills: f64,
    pub mean_seconds: f64,
    pub mean_kits: f64,
    pub p95_kits: u32,
    pub mean_cannonballs: f64,
    pub outgoing_dps: f64,
    pub player_dps: f64,
    pub crew_dps: f64,
    pub incoming_dps: f64,
    pub hit_chance: f64,
    pub max_taken: u32,
    pub boat_hp: u32,
    pub ammo: Vec<(i32, u32)>,
    pub notes: Vec<String>,
}
impl Model {
    pub fn build(
        snapshot: &Snapshot,
        o: &Options,
        plan: &Plan,
        set: &[Option<i32>],
        style: &str,
        target: &Monster,
    ) -> Result<Self, String> {
        let b = &o.sailing;
        if !b.valid() {
            return Err("Invalid Sailing settings.".into());
        }
        let conflicts = b.conflicts();
        if !conflicts.is_empty() {
            return Err(conflicts.join(" "));
        }
        let member = snapshot.selected().ok_or("Select an account")?;
        let base = member.level("Sailing").ok_or("Enter your Sailing level")?;
        if base < b.boat.level() {
            return Err("Boat requires a higher unboosted Sailing level.".into());
        }
        if b.crew.iter().filter(|c| c.person.is_some()).count() > b.crew_limit(base) {
            return Err("Too many active crew for your boat or base Sailing level.".into());
        }
        if set.len() != 14 {
            return Err("Equipment must have fourteen slots.".into());
        }
        let owned = sailing_owned::stock(snapshot, o);
        let mut needed = BTreeMap::<i32, u32>::new();
        for id in set.iter().flatten() {
            *needed.entry(*id).or_default() += 1;
            if owned.get(id).copied().unwrap_or(0) < needed[id] || o.item_excluded(*id) {
                return Err(
                    "Planned equipment is absent from your available imported items.".into(),
                );
            }
            if !planner::can_use(*id, member) {
                return Err("Planned equipment has unmet skill requirements.".into());
            }
        }
        let mut stock = sailing_owned::stock(snapshot, o);
        // Reserve every item in the actual personal packing list before loading cargo.
        for item in &plan.inventory {
            if let Some(q) = stock.get_mut(&item.id) {
                *q = q.saturating_sub(item.quantity);
            }
        }
        let member = planner::member_with_stock(member, &sailing_owned::stock(snapshot, o));
        let member = &member;
        let mut streams = Vec::new();
        let mut notes = Vec::new();
        for mount in 0..b.cannon.map(|_| b.cannon_count).unwrap_or(0) {
            let tier = if mount == 0 {
                b.cannon.unwrap()
            } else {
                b.second_cannon.unwrap_or(b.cannon.unwrap())
            };
            let ball = if mount == 0 {
                b.cannonball
            } else {
                b.second_ball.unwrap_or(b.cannonball)
            };
            let visible = b.visible_sailing.unwrap_or(base);
            if visible < sailing::CANNON_LEVELS[tier] {
                return Err("Current Sailing level cannot operate the selected cannon.".into());
            }
            let operator = if mount == 0 && b.player_role == 1 {
                Some(None)
            } else {
                b.crew
                    .iter()
                    .find(|c| c.person.is_some() && c.role == mount + 3)
                    .map(|c| c.person)
            };
            if let Some(person) = operator {
                let r = cannon(
                    member,
                    set,
                    target,
                    tier,
                    ball,
                    person,
                    b.perildance,
                    &o.combat,
                )?;
                let plain = if person.is_none() && o.combat.offensive != "None" {
                    let mut settings = o.combat.clone();
                    settings.offensive = "None".into();
                    Some(Box::new(Stream {
                        name: "Player cannon (no prayer)".into(),
                        hit: cannon(
                            member,
                            set,
                            target,
                            tier,
                            ball,
                            None,
                            b.perildance,
                            &settings,
                        )?,
                        personal: None,
                        ammo: Some(BALL_IDS[ball]),
                        ammo_cost: 1,
                        runes: vec![],
                        charge: false,
                        player: true,
                        unprayed: None,
                    }))
                } else {
                    None
                };
                streams.push(Stream {
                    name: person
                        .map(|p| CREW[p].name.into())
                        .unwrap_or_else(|| "Player cannon".into()),
                    hit: r,
                    personal: None,
                    ammo: Some(BALL_IDS[ball]),
                    ammo_cost: 1,
                    runes: vec![],
                    charge: false,
                    player: person.is_none(),
                    unprayed: plain,
                });
            }
        }
        if b.player_role == 0 {
            let spell = rules::spell_name_for_target(
                member,
                set,
                &sailing_owned::stock(snapshot, o),
                &o.combat,
                target,
                o.raid_level,
            );
            let r = combat::calculate_configured(
                member,
                set,
                style,
                target,
                spell,
                o.raid_level,
                &o.combat,
            )?;
            if r.burn.is_some() {
                return Err("Delayed weapon burn needs a separate Sailing event model. Choose a direct-damage weapon.".into());
            }
            let weapon = set[3].ok_or("Choose a weapon")?;
            let rule = rules::weapon_for_style(weapon, style).ok_or("Weapon style unavailable")?;
            let ammo = if rules::base_style(style) == "Ranged" {
                if planner::is_stackable(weapon) {
                    Some(weapon)
                } else if rule.family.internal_ammo.is_empty() {
                    set[13].filter(|id| rules::ammo_ids(weapon).contains(id))
                } else {
                    rules::internal_ammo(member, weapon, &o.combat)
                }
            } else {
                None
            };
            let runes = if rules::base_style(style) == "Magic" {
                rules::rune_supplies_for_set(spell, 1, set)
            } else {
                vec![]
            };
            let charge = crate::simulation::charged_weapon(weapon, style, ammo)
                || !rule.family.internal_ammo.is_empty();
            // Personal ammunition/runes are already reserved by the packing list.
            for item in &plan.inventory {
                if ammo == Some(item.id) || runes.iter().any(|(id, _)| *id == item.id) {
                    stock
                        .entry(item.id)
                        .and_modify(|q| *q = q.saturating_add(item.quantity))
                        .or_insert(item.quantity);
                }
            }
            if ammo.is_some()
                && o.combat.loaded_ammo_confirmed
                && !o.item_excluded(ammo.unwrap())
                && ammo == o.combat.loaded_dart
                && !rule.family.internal_ammo.is_empty()
            {
                stock
                    .entry(ammo.unwrap())
                    .and_modify(|q| *q = q.saturating_add(o.combat.ammo_shots))
                    .or_insert(o.combat.ammo_shots);
            }
            let make = |r: combat::ResultStats| Stream {
                name: format!("Player {style}"),
                hit: Hit {
                    accuracy: r.accuracy,
                    max_hit: r.max_hit as u32,
                    raw_max: r.max_hit as u32,
                    attack_roll: r.attack_roll,
                    defence_roll: r.defence_roll,
                    distribution: r
                        .hit_distribution
                        .iter()
                        .map(|(d, p)| ((*d).max(0) as u32, *p))
                        .collect(),
                    ticks: r.expected_attack_ticks,
                    dps: r.dps,
                },
                personal: Some(r),
                ammo,
                ammo_cost: if rule.has(rules::Effect::TwinHits) {
                    2
                } else {
                    1
                },
                runes: runes.clone(),
                charge,
                player: true,
                unprayed: None,
            };
            let mut stream = make(r);
            if o.combat.offensive != "None" {
                let mut settings = o.combat.clone();
                settings.offensive = "None".into();
                stream.unprayed = Some(Box::new(make(combat::calculate_configured(
                    member,
                    set,
                    style,
                    target,
                    spell,
                    o.raid_level,
                    &settings,
                )?)));
            }
            streams.push(stream);
            if charge && b.powered_charges == 0 {
                return Err(
                    "Confirm the loaded weapon charges or blowpipe scales for this trip, or choose another weapon."
                        .into(),
                );
            }
        }
        if streams.is_empty() || streams.iter().all(|s| s.hit.dps <= 0.0) {
            return Err(
                "Assign a weapon user or cannon gunner that can damage this creature.".into(),
            );
        }
        let mut ammo = BTreeMap::<i32, u32>::new();
        for stream in &streams {
            if let Some(id) = stream.ammo.filter(|id| BALL_IDS.contains(id)) {
                let q = sailing_owned::stock(snapshot, o)
                    .get(&id)
                    .copied()
                    .unwrap_or(0);
                let requested = if b.cannonballs == 0 {
                    q
                } else {
                    b.cannonballs.min(q)
                };
                ammo.entry(id).or_insert(requested);
            }
        }
        for (id, q) in &ammo {
            let free = stock.get(id).copied().unwrap_or(0);
            stock.insert(*id, (*q).min(free));
        }
        let kit_id = KIT_IDS[b.repair_tier];
        let kit_count = b.repair_kits.min(stock.get(&kit_id).copied().unwrap_or(0));
        stock.insert(kit_id, kit_count);
        let bag_ammo = b.cargo.is_none() && b.player_role == 1 && ammo.len() == 1;
        if bag_ammo && plan.inventory.len() >= 28 {
            return Err("Leave one inventory slot for your player-operated cannonballs.".into());
        }
        let cargo_slots =
            if bag_ammo { 0 } else { ammo.len() as u32 } + kit_count + b.cargo_reserved;
        if cargo_slots > b.cargo_capacity() {
            return Err(format!(
                "Cargo needs {cargo_slots} slots; hold has {}. Reduce kits or reserved space.",
                b.cargo_capacity()
            ));
        }
        if !ammo.is_empty()
            && ammo
                .iter()
                .any(|(id, _)| stock.get(id).copied().unwrap_or(0) == 0)
        {
            return Err(
                "Selected cannonballs are absent from your imported stock and confirmed cargo."
                    .into(),
            );
        }
        if b.cargo.is_none() && kit_count > 0 {
            return Err("Repair kits need cargo storage for this model.".into());
        }
        let crew_repairs =
            b.repair_workbench && b.crew.iter().any(|c| c.person.is_some() && c.role == 2);
        if kit_count > 0 && !crew_repairs && !b.manual_repairs {
            notes.push(
                "No repairs are credited: assign a repairer or enable manual repairs.".into(),
            );
        }
        notes.push("Targets are reacquired immediately; no respawn or looting delay. Personal ammunition is conservatively consumed without recovery.".into());
        notes.push(format!("Stationary {}-creature scenario; {}% outgoing uptime; {} ticks between repair opportunities. Crew healing uses the kit's base heal, with no uncertain Deckhandiness bonus. Crew repairs when missing at least a kit heal; manual repairs wait until half hull HP.",b.aggressors,b.uptime,b.repair_ticks));
        notes.push("Uses standard NPC hull defence rolls and cannon 0–max rolls, including misses and per-hit armour. Repair cadence is an editable scenario assumption; no gameplay calibration or movement is claimed.".into());
        let mut prayer_settings = o.combat.clone();
        prayer_settings.protection = "None".into();
        prayer_settings.preserve = false;
        let prayer_points = member.level("Prayer").unwrap_or(1)
            + prayer::restore_points(member.level("Prayer").unwrap_or(1), &plan.inventory);
        let player_dps = streams
            .iter()
            .filter(|s| s.player && s.funded(&stock, b.powered_charges))
            .map(|s| s.hit.dps)
            .sum::<f64>()
            * f64::from(b.uptime)
            / 100.0;
        let crew_dps = streams
            .iter()
            .filter(|s| !s.player && s.funded(&stock, b.powered_charges))
            .map(|s| s.hit.dps)
            .sum::<f64>()
            * f64::from(b.uptime)
            / 100.0;
        Ok(Self {
            boat: b.clone(),
            target_hp: combat::target_hitpoints(target, 0).ok_or("Target HP unavailable")? as u32,
            incoming: incoming(b, target)?,
            streams,
            stock,
            kit_id,
            kit_heal: KIT_HEAL[b.repair_tier],
            kit_count,
            crew_repairs,
            prayer_points: f64::from(prayer_points),
            prayer_drain: prayer::drain_per_minute(prayer::bonus(set), &prayer_settings) / 100.0,
            player_hp: member.level("Hitpoints").unwrap_or(10),
            crew_dps,
            player_dps,
            notes,
        })
    }
    pub fn run(&self, trials: u32) -> Report {
        let trials = trials.clamp(32, 1024);
        let mut completed = 0;
        let mut sunk = 0;
        let mut depleted = 0;
        let mut timed_out = 0;
        let mut kills_sum = 0.0;
        let mut seconds = 0.0;
        let mut kits = Vec::new();
        let mut balls_sum = 0.0;
        let mut ammo = BTreeMap::<i32, u32>::new();
        for trial in 0..trials {
            let mut rng = Rng(0x9e3779b97f4a7c15u64.wrapping_mul(u64::from(trial) + 1));
            let mut stocks = self.stock.clone();
            let mut hull = self.boat.hp();
            let mut hp = self.target_hp;
            let mut kills = 0;
            let mut next = vec![0.0_f64; self.streams.len()];
            let mut repair_at = 0u32;
            let mut used_kits = 0;
            let mut used_balls = 0;
            let mut charges = self.boat.powered_charges;
            let mut prayer = self.prayer_points;
            let mut player_hp = self.player_hp;
            let mut tick = 0;
            let limit = self.boat.minutes * 100;
            let mut exhausted = false;
            while tick < limit && kills < self.boat.trip_kills && hull > 0 {
                // Resolve enemy damage before a repair at the same tick (conservative).
                if tick > 0 && tick % (self.incoming.ticks as u32) == 0 {
                    for _ in 0..self.boat.aggressors {
                        hull = hull.saturating_sub(sample(&self.incoming.distribution, rng.unit()));
                        if hull == 0 {
                            break;
                        }
                    }
                }
                if hull == 0 {
                    break;
                }
                if tick >= repair_at
                    && (self.crew_repairs || hull <= self.boat.hp() / 2)
                    && self.boat.hp() - hull >= self.kit_heal
                    && stocks.get(&self.kit_id).copied().unwrap_or(0) > 0
                    && (self.crew_repairs || self.boat.manual_repairs)
                {
                    *stocks.get_mut(&self.kit_id).unwrap() -= 1;
                    hull = (hull + self.kit_heal).min(self.boat.hp());
                    used_kits += 1;
                    repair_at = tick + self.boat.repair_ticks;
                    if !self.crew_repairs {
                        for (i, s) in self.streams.iter().enumerate() {
                            if s.player {
                                next[i] = next[i].max(f64::from(tick)) + 3.0;
                            }
                        }
                    }
                }
                let mut active = 0;
                for (i, original) in self.streams.iter().enumerate() {
                    let s = if prayer <= 0.0 {
                        original.unprayed.as_deref().unwrap_or(original)
                    } else {
                        original
                    };
                    let enough = s.funded(&stocks, charges);
                    if !enough {
                        continue;
                    }
                    active += 1;
                    if f64::from(tick) < next[i] || self.boat.uptime == 0 {
                        continue;
                    }
                    next[i] = f64::from(tick) + s.hit.ticks * 100.0 / f64::from(self.boat.uptime);
                    if let Some(id) = s.ammo {
                        for _ in 0..s.ammo_cost {
                            let loss = if BALL_IDS.contains(&id) {
                                used_balls += 1;
                                *ammo.entry(id).or_default() += 1;
                                !self.boat.attractor || rng.unit() >= 0.6
                            } else {
                                true
                            };
                            if loss {
                                *stocks.get_mut(&id).unwrap() -= 1;
                            }
                        }
                    }
                    for (id, n) in &s.runes {
                        *stocks.get_mut(id).unwrap() -= *n;
                    }
                    if s.charge {
                        charges -= 1;
                    }
                    let (damage, ruby) = if let Some(r) = &s.personal {
                        roll_personal(r, &mut rng, hp, player_hp)
                    } else {
                        (sample(&s.hit.distribution, rng.unit()), false)
                    };
                    if ruby {
                        player_hp = player_hp.saturating_sub(player_hp / 10);
                    }
                    hp = hp.saturating_sub(damage);
                    if hp == 0 {
                        kills += 1;
                        hp = self.target_hp;
                        if kills == self.boat.trip_kills {
                            break;
                        }
                    }
                }
                if active == 0 {
                    exhausted = true;
                    break;
                }
                if self.streams.iter().any(|s| s.player) {
                    prayer = (prayer - self.prayer_drain).max(0.0);
                }
                tick += 1;
            }
            if kills == self.boat.trip_kills {
                completed += 1;
            } else if hull == 0 {
                sunk += 1;
            } else if exhausted {
                depleted += 1;
            } else {
                timed_out += 1;
            }
            kills_sum += f64::from(kills);
            seconds += f64::from(tick) * 0.6;
            kits.push(used_kits);
            balls_sum += f64::from(used_balls);
        }
        kits.sort_unstable();
        Report {
            trials,
            completed,
            sunk,
            depleted,
            timed_out,
            mean_kills: kills_sum / f64::from(trials),
            mean_seconds: seconds / f64::from(trials),
            mean_kits: f64::from(kits.iter().sum::<u32>()) / f64::from(trials),
            p95_kits: kits[(kits.len() * 95 / 100).min(kits.len() - 1)],
            mean_cannonballs: balls_sum / f64::from(trials),
            outgoing_dps: self.player_dps + self.crew_dps,
            player_dps: self.player_dps,
            crew_dps: self.crew_dps,
            incoming_dps: self.incoming.dps * f64::from(self.boat.aggressors),
            hit_chance: self.incoming.accuracy,
            max_taken: self.incoming.max_hit,
            boat_hp: self.boat.hp(),
            ammo: ammo
                .into_iter()
                .map(|(id, q)| (id, q.div_ceil(trials)))
                .collect(),
            notes: self.notes.clone(),
        }
    }
}
struct Rng(u64);
impl Rng {
    fn unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}
fn sample(hits: &[(u32, f64)], mut pick: f64) -> u32 {
    for (d, p) in hits {
        if pick < *p {
            return *d;
        }
        pick -= p;
    }
    0
}
fn roll_personal(r: &combat::ResultStats, rng: &mut Rng, hp: u32, player_hp: u32) -> (u32, bool) {
    let mut pick = rng.unit();
    let proc = r.hp_dependent;
    let no_ruby = proc.is_some() && player_hp < 10;
    let count = r.hit_distribution.len() - proc.map(|p| p.distribution_entries).unwrap_or(0);
    for (d, p) in &r.hit_distribution[..count] {
        let p = if no_ruby {
            *p / (1.0 - proc.unwrap().chance)
        } else {
            *p
        };
        if pick < p {
            return ((*d).max(0) as u32, false);
        }
        pick -= p;
    }
    if let Some(proc) = proc.filter(|_| !no_ruby) {
        for (d, p) in proc.outcomes(hp) {
            if pick < p {
                return (d.max(0) as u32, true);
            }
            pick -= p;
        }
    }
    (0, false)
}
