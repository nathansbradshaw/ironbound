//! Reviewed boat parts and sea-monster rules. Boat selections are manual
//! assumptions. Owned recommendations and trip scenarios use sailing_combat.
use crate::{
    account::Member,
    planner::{self, Monster, Options, Target},
};
use serde::{Deserialize, Serialize};

pub const SEA_IDS: [i32; 21] = [
    15194, 15196, 15198, 15200, 15202, 15204, 15206, 15208, 15210, 15212, 15214, 15216, 15218,
    15220, 15222, 15224, 15226, 15228, 15230, 15234, 15576,
];
pub fn is_sea(id: i32) -> bool {
    SEA_IDS.contains(&id)
}
pub fn active(o: &Options) -> bool {
    o.boss == planner::Boss::Custom && o.targets.iter().any(|t| is_sea(t.id))
}
pub fn entries() -> Vec<crate::encounters::Entry> {
    SEA_IDS.iter().map(|id| {
        let m = planner::monster(*id).expect("sea NPC in pinned catalog");
        crate::encounters::Entry { name: m.name.clone(), targets: vec![Target { id: *id, style: if [15206,15208,15210,15212].contains(id) {"Magic"} else {"Ranged"}.into() }], bank_equipment: true, note: "Sea creature: configure your boat and crew in Boat setup, confirm your available parts, then compare cannon DPS and modelled trip survival from your own stock.".into() }
    }).collect()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boat {
    Raft,
    #[default]
    Skiff,
    Sloop,
}
impl Boat {
    pub const ALL: [Self; 3] = [Self::Raft, Self::Skiff, Self::Sloop];
    pub fn name(self) -> &'static str {
        match self {
            Self::Raft => "Raft",
            Self::Skiff => "Skiff",
            Self::Sloop => "Sloop",
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Raft => 0,
            Self::Skiff => 1,
            Self::Sloop => 2,
        }
    }
    pub fn level(self) -> u32 {
        [1, 15, 50][self.index()]
    }
    pub fn crew(self) -> usize {
        [0, 2, 5][self.index()]
    }
    pub fn cannons(self) -> usize {
        if self == Self::Sloop { 2 } else { 1 }
    }
}
pub const WOODS: [&str; 7] = [
    "Wooden", "Oak", "Teak", "Mahogany", "Camphor", "Ironwood", "Rosewood",
];
pub const METALS: [&str; 7] = [
    "Bronze", "Iron", "Steel", "Mithril", "Adamant", "Rune", "Dragon",
];
const HULL_LEVELS: [u32; 7] = [1, 20, 31, 48, 67, 81, 93];
const HULL_HP: [[u32; 7]; 3] = [
    [20, 30, 40, 50, 80, 100, 120],
    [30, 45, 60, 75, 120, 150, 180],
    [40, 60, 80, 100, 160, 200, 240],
];
const KEEL_LEVELS: [u32; 7] = [1, 22, 39, 54, 66, 85, 97];
const KEEL_HP: [[u32; 7]; 2] = [
    [50, 50, 60, 60, 70, 80, 80],
    [70, 70, 90, 90, 100, 120, 120],
];
pub const FLAT_ARMOUR: [u32; 7] = [1, 2, 3, 4, 6, 8, 10];
const CARGO: [[u32; 7]; 3] = [
    [20, 30, 45, 60, 80, 105, 120],
    [30, 45, 60, 90, 120, 150, 180],
    [40, 60, 90, 120, 160, 210, 240],
];
const CARGO_LEVELS: [u32; 7] = [1, 18, 29, 46, 60, 80, 89];
pub const CANNON_LEVELS: [u32; 7] = [28, 35, 47, 57, 69, 80, 92];
const CANNON_PRIVATEERING: [u32; 7] = [1, 2, 2, 3, 3, 4, 4];
const CANNON_STRENGTH: [u32; 7] = [8, 10, 12, 15, 23, 26, 32];
pub const BALL_NAMES: [&str; 7] = METALS;
const BALL_TIER: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];
const BALL_STRENGTH: [u32; 7] = [43, 56, 74, 105, 157, 201, 270];

pub struct Crew {
    pub name: &'static str,
    pub level: u32,
    pub privateering: u32,
    pub access: &'static str,
}
pub const CREW: [Crew; 10] = [
    Crew {
        name: "Jobless Jim",
        level: 40,
        privateering: 1,
        access: "Port Sarim",
    },
    Crew {
        name: "Ex-Captain Siad",
        level: 50,
        privateering: 2,
        access: "The Tourist Trap",
    },
    Crew {
        name: "Adventurer Ada",
        level: 55,
        privateering: 2,
        access: "Port Roberts",
    },
    Crew {
        name: "Cabin Boy Jenkins",
        level: 60,
        privateering: 2,
        access: "Dragon Slayer I, The Restless Ghost and ghostspeak",
    },
    Crew {
        name: "Oarswoman Olga",
        level: 65,
        privateering: 4,
        access: "In Aid of the Myreque",
    },
    Crew {
        name: "Jittery Jim",
        level: 70,
        privateering: 4,
        access: "Port Khazard",
    },
    Crew {
        name: "Bosun Zarah",
        level: 80,
        privateering: 3,
        access: "Royal Trouble and Hard Fremennik Diary",
    },
    Crew {
        name: "Jolly Jim",
        level: 85,
        privateering: 1,
        access: "Brimhaven",
    },
    Crew {
        name: "Spotter Virginia",
        level: 90,
        privateering: 2,
        access: "100 Tempoross subdues",
    },
    Crew {
        name: "Sailor Jakob",
        level: 95,
        privateering: 1,
        access: "Catherby",
    },
];
pub const ROLES: [&str; 5] = ["Idle", "Helm", "Repair", "Cannon 1", "Cannon 2"];
pub const PLAYER_ROLES: [&str; 3] = ["Weapon", "Cannon 1", "Helm"];
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Assignment {
    pub person: Option<usize>,
    pub role: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub boat: Boat,
    pub second_cannon: Option<usize>,
    pub second_ball: Option<usize>,
    pub repair_tier: usize,
    pub trip_kills: u32,
    pub repair_ticks: u32,
    pub manual_repairs: bool,
    pub aggressors: u32,
    pub powered_charges: u32,
    pub hull: usize,
    pub keel: usize,
    pub cargo: Option<usize>,
    pub cannon: Option<usize>,
    pub cannon_count: usize,
    pub cannonball: usize,
    pub player_role: usize,
    pub crew: [Assignment; 5],
    pub visible_sailing: Option<u32>,
    pub perildance: bool,
    pub attractor: bool,
    pub repair_workbench: bool,
    pub cannonballs: u32,
    pub repair_kits: u32,
    pub cargo_reserved: u32,
    pub minutes: u32,
    pub uptime: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            boat: Boat::Skiff,
            second_cannon: None,
            second_ball: None,
            repair_tier: 0,
            trip_kills: 10,
            repair_ticks: 5,
            manual_repairs: true,
            aggressors: 1,
            powered_charges: 0,
            hull: 0,
            keel: 0,
            cargo: Some(0),
            cannon: None,
            cannon_count: 1,
            cannonball: 0,
            player_role: 0,
            crew: Default::default(),
            visible_sailing: None,
            perildance: false,
            attractor: false,
            repair_workbench: false,
            cannonballs: 0,
            repair_kits: 0,
            cargo_reserved: 0,
            minutes: 10,
            uptime: 100,
        }
    }
}
impl Settings {
    pub fn valid(&self) -> bool {
        self.second_cannon.is_none_or(|n| n < 7)
            && self.second_ball.is_none_or(|n| n < 7)
            && self.repair_tier < 7
            && (1..=100).contains(&self.trip_kills)
            && (1..=20).contains(&self.repair_ticks)
            && (1..=10).contains(&self.aggressors)
            && self.powered_charges <= 1_000_000
            && self.hull < 7
            && self.keel < 7
            && self.cargo.is_none_or(|v| v < 7)
            && self.cannon.is_none_or(|v| v < 7)
            && (1..=2).contains(&self.cannon_count)
            && self.cannonball < BALL_NAMES.len()
            && self.player_role < 3
            && self
                .crew
                .iter()
                .all(|c| c.person.is_none_or(|i| i < CREW.len()) && c.role < ROLES.len())
            && self.visible_sailing.is_none_or(|v| (1..=125).contains(&v))
            && self.cannonballs <= 1_000_000
            && self.repair_kits <= 10_000
            && self.cargo_reserved <= 240
            && (1..=120).contains(&self.minutes)
            && self.uptime <= 100
    }
    pub fn hp(&self) -> u32 {
        HULL_HP[self.boat.index()][self.hull]
            + if self.boat == Boat::Raft {
                0
            } else {
                KEEL_HP[self.boat.index() - 1][self.keel]
            }
    }
    pub fn armour(&self) -> u32 {
        if self.boat == Boat::Raft {
            0
        } else {
            FLAT_ARMOUR[self.keel]
        }
    }
    pub fn cargo_capacity(&self) -> u32 {
        self.cargo.map(|c| CARGO[self.boat.index()][c]).unwrap_or(0)
    }
    pub fn crew_limit(&self, base: u32) -> usize {
        [40, 55, 70, 85, 95]
            .into_iter()
            .filter(|v| base >= *v)
            .count()
            .min(self.boat.crew())
    }
    pub fn required_sailing(&self) -> u32 {
        let mut level = self.boat.level().max(HULL_LEVELS[self.hull]);
        if self.boat != Boat::Raft {
            level = level.max(KEEL_LEVELS[self.keel]);
        }
        if let Some(c) = self.cargo {
            level = level.max(CARGO_LEVELS[c]);
        }
        if let Some(c) = self.cannon {
            level = level.max(CANNON_LEVELS[c]);
            if self.cannon_count == 2 {
                level = level.max(CANNON_LEVELS[self.second_cannon.unwrap_or(c)]);
            }
        }
        if self.attractor {
            level = level.max(50);
        }
        level
    }
    /// Structural conflicts, separately from skill/access assumptions. Each crew
    /// member can occupy one seat and each active role has one operator.
    pub fn conflicts(&self) -> Vec<String> {
        if !self.valid() {
            return vec!["Invalid boat settings.".into()];
        }
        let mut errors = Vec::new();
        if self.cargo.is_none()
            && (self.cannonballs > 0 && self.player_role != 1 || self.repair_kits > 0)
        {
            errors.push("Cargo supplies require an installed cargo hold.".into());
        }
        if self.cannon.is_some() && self.cannon_count > self.boat.cannons() {
            errors.push("Only a sloop can use two cannons on the same target side.".into());
        }
        if self.boat == Boat::Raft {
            let facilities = usize::from(self.cargo.is_some())
                + self.cannon.map(|_| self.cannon_count).unwrap_or(0)
                + usize::from(self.repair_workbench)
                + usize::from(self.perildance);
            if facilities > 1 {
                errors.push("A raft has one facility hotspot; choose cargo or a cannon.".into());
            }
            if self.attractor {
                errors.push("A ballistic attractor requires a skiff or sloop.".into());
            }
        }
        if self.cargo_reserved > self.cargo_capacity() {
            errors.push("Reserved cargo exceeds the selected hold capacity.".into());
        }
        let mut people = std::collections::BTreeSet::new();
        let mut roles = std::collections::BTreeSet::new();
        if self.player_role == 1 {
            roles.insert(3);
        }
        if self.player_role == 2 {
            roles.insert(1);
        }
        for c in &self.crew {
            if let Some(i) = c.person {
                if !people.insert(i) {
                    errors.push(format!("{} is assigned more than once.", CREW[i].name));
                }
                if c.role > 0 && !roles.insert(c.role) {
                    errors.push(format!("{} already has an operator.", ROLES[c.role]));
                }
                if c.role >= 3 {
                    if self.cannon.is_none() || c.role - 2 > self.cannon_count {
                        errors.push(format!("{} needs an installed cannon.", ROLES[c.role]));
                    }
                    if self.cannon.is_some_and(|tier| {
                        CREW[i].privateering
                            < CANNON_PRIVATEERING[if c.role == 4 {
                                self.second_cannon.unwrap_or(tier)
                            } else {
                                tier
                            }]
                    }) {
                        errors.push(format!(
                            "{} lacks the Privateering level for this cannon.",
                            CREW[i].name
                        ));
                    }
                    if self.cargo.is_none() {
                        errors.push("Crew cannons need cannonballs in a cargo hold.".into());
                    }
                }
                if c.role == 2 && !self.repair_workbench {
                    errors.push("Automatic crew repairs need the bosun's workbench.".into());
                }
            }
        }
        if people.len() > self.boat.crew() {
            errors.push(format!(
                "{} supports at most {} crew.",
                self.boat.name(),
                self.boat.crew()
            ));
        }
        if self.player_role == 1 && self.cannon.is_none() {
            errors.push("Install a cannon before assigning yourself to it.".into());
        }
        if self.cannon_count == 2 && self.second_ball.is_some_and(|n| n != self.cannonball) {
            errors
                .push("Both modelled cannon mounts must use the selected cannonball type.".into());
        }
        if !self.ammo_compatible() {
            errors.push("Selected cannonballs exceed the cannon’s tier.".into());
        }
        errors.sort();
        errors.dedup();
        errors
    }
    pub fn checks(&self, member: &Member) -> Vec<String> {
        let mut notes = self.conflicts();
        if !self.valid() {
            return notes;
        }
        match member.level("Sailing") {
            None => notes.push(
                "Enter your Sailing level in Your bank to check boat and crew requirements.".into(),
            ),
            Some(base) => {
                if base < self.required_sailing() {
                    notes.push(format!("Selected parts need {} Sailing; your recorded level is {base}. Check boostable installation requirements and your installed parts.",self.required_sailing()));
                }
                if base < self.boat.level() {
                    notes.push(format!(
                        "Using a {} requires {} unboosted Sailing.",
                        self.boat.name(),
                        self.boat.level()
                    ));
                }
                if self.crew.iter().filter(|c| c.person.is_some()).count() > self.crew_limit(base) {
                    notes.push(format!(
                        "Your boat and base Sailing level allow {} active crew.",
                        self.crew_limit(base)
                    ));
                }
            }
        }
        if self.crew.iter().any(|c| c.person.is_some() && c.role == 2) && self.repair_kits == 0 {
            notes.push("No repair kits recorded for the repair crew.".into());
        }
        notes
    }
    pub fn crew_max(&self, member: &Member, seat: usize) -> Result<u32, String> {
        if !self.valid() {
            return Err("Invalid boat settings.".into());
        }
        let conflicts = self.conflicts();
        if !conflicts.is_empty() {
            return Err(conflicts.join(" "));
        }
        let c = self.crew.get(seat).ok_or("Crew seat missing")?;
        let person = c.person.map(|i| &CREW[i]).ok_or("No crew selected")?;
        if c.role < 3 {
            return Err("This crewmate is not operating a cannon.".into());
        }
        let tier = self.cannon.ok_or("No cannon installed")?;
        let base = member.level("Sailing").ok_or("Sailing level unknown")?;
        let tier = if c.role == 4 {
            self.second_cannon.unwrap_or(tier)
        } else {
            tier
        };
        if self.visible_sailing.unwrap_or(base) < CANNON_LEVELS[tier] {
            return Err(format!(
                "Operating this cannon requires {} Sailing.",
                CANNON_LEVELS[tier]
            ));
        }
        if self.crew.iter().filter(|c| c.person.is_some()).count() > self.crew_limit(base) {
            return Err("Too many crew for your base Sailing level.".into());
        }
        if BALL_TIER[if c.role == 4 {
            self.second_ball.unwrap_or(self.cannonball)
        } else {
            self.cannonball
        }] > tier
        {
            return Err("Cannonballs exceed the cannon's tier.".into());
        }
        let mount = c.role - 3;
        let tier = if mount == 1 {
            self.second_cannon.unwrap_or(tier)
        } else {
            tier
        };
        let ball = if mount == 1 {
            self.second_ball.unwrap_or(self.cannonball)
        } else {
            self.cannonball
        };
        Ok(
            ((person.privateering * 12 + 8) * (CANNON_STRENGTH[tier] + BALL_STRENGTH[ball] + 64)
                + 320)
                / 640
                + u32::from(self.perildance),
        )
    }
    pub fn ammo_compatible(&self) -> bool {
        self.cannon.is_none_or(|c| {
            BALL_TIER[self.cannonball] <= c
                && (self.cannon_count < 2
                    || BALL_TIER[self.second_ball.unwrap_or(self.cannonball)]
                        <= self.second_cannon.unwrap_or(c))
        })
    }
    pub fn ammo_budget(&self) -> (f64, f64) {
        let guns = if self.cannon.is_some() {
            usize::from(self.player_role == 1)
                + self
                    .crew
                    .iter()
                    .filter(|c| c.person.is_some() && c.role >= 3)
                    .count()
        } else {
            0
        };
        let shots =
            f64::from(self.minutes) * 60.0 / 4.2 * f64::from(self.uptime) / 100.0 * guns as f64;
        (shots, shots * if self.attractor { 0.4 } else { 1.0 })
    }
    /// Update from named browser controls; all imported values remain bounded.
    pub fn update(&mut self, key: &str, value: &str) -> bool {
        let mut next = self.clone();
        let n = value.parse::<usize>().ok();
        match key {
            "cannon2" => {
                next.second_cannon = n;
            }
            "ball2" => {
                next.second_ball = n;
            }
            "kit" => {
                let Some(n) = n else { return false };
                next.repair_tier = n;
            }
            "kills" => {
                let Ok(n) = value.parse() else { return false };
                next.trip_kills = n;
            }
            "repair-ticks" => {
                let Ok(n) = value.parse() else { return false };
                next.repair_ticks = n;
            }
            "aggressors" => {
                let Ok(n) = value.parse() else { return false };
                next.aggressors = n;
            }
            "charges" => {
                let Ok(n) = value.parse() else { return false };
                next.powered_charges = n;
            }
            "manual-repairs" => next.manual_repairs = value == "true",
            "boat" => {
                let Some(b) = Boat::ALL.iter().find(|b| b.name() == value) else {
                    return false;
                };
                next.boat = *b;
            }
            "hull" => {
                let Some(n) = n else { return false };
                next.hull = n;
            }
            "keel" => {
                let Some(n) = n else { return false };
                next.keel = n;
            }
            "cargo" => {
                next.cargo = if value == "none" {
                    None
                } else {
                    let Some(n) = n else { return false };
                    Some(n)
                }
            }
            "cannon" => {
                next.cannon = if value == "none" {
                    None
                } else {
                    let Some(n) = n else { return false };
                    Some(n)
                }
            }
            "count" => {
                let Some(n) = n else { return false };
                next.cannon_count = n;
            }
            "ball" => {
                let Some(n) = n else { return false };
                next.cannonball = n;
                if next.second_ball.is_some() {
                    next.second_ball = Some(n);
                }
            }
            "player" => {
                let Some(n) = n else { return false };
                next.player_role = n;
            }
            "visible" => {
                next.visible_sailing = if value.is_empty() {
                    None
                } else {
                    let Ok(n) = value.parse() else { return false };
                    Some(n)
                }
            }
            "attractor" => next.attractor = value == "true",
            "perildance" => next.perildance = value == "true",
            "workbench" => next.repair_workbench = value == "true",
            "balls" => {
                let Ok(n) = value.parse() else { return false };
                next.cannonballs = n;
            }
            "kits" => {
                let Ok(n) = value.parse() else { return false };
                next.repair_kits = n;
            }
            "reserved" => {
                let Ok(n) = value.parse() else { return false };
                next.cargo_reserved = n;
            }
            "minutes" => {
                let Ok(n) = value.parse() else { return false };
                next.minutes = n;
            }
            "uptime" => {
                let Ok(n) = value.parse() else { return false };
                next.uptime = n;
            }
            _ => {
                let Some((field, seat)) = key.split_once('-') else {
                    return false;
                };
                let Some(c) = seat
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| next.crew.get_mut(i))
                else {
                    return false;
                };
                match field {
                    "person" => {
                        c.person = if value == "none" {
                            None
                        } else {
                            let Some(n) = n else { return false };
                            Some(n)
                        }
                    }
                    "role" => {
                        let Some(n) = n else { return false };
                        c.role = n;
                    }
                    _ => return false,
                }
            }
        }
        if next.valid() {
            *self = next;
            true
        } else {
            false
        }
    }
}
pub fn is_default(s: &Settings) -> bool {
    s == &Settings::default()
}

pub fn reduced_max(settings: &Settings, target: &Monster) -> Option<u32> {
    if !settings.valid() || !is_sea(target.id) {
        return None;
    }
    Some(
        target
            .max_hit
            .parse::<u32>()
            .ok()?
            .saturating_sub(settings.armour()),
    )
}
