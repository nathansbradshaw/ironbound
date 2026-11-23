//! Account-private availability. Installed facilities and recruited crew cannot
//! be inferred from bank exports or shared loadout links.
use crate::{account::Snapshot, planner::Options, sailing::Settings};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const BALL_IDS: [i32; 7] = [31906, 31908, 2, 31910, 31912, 31914, 31916];
pub const KIT_IDS: [i32; 7] = [31964, 31967, 31970, 31973, 31976, 31979, 31982];
// Earlier saves used the facility-icon IDs for manually confirmed aboard counts.
pub const LEGACY_KIT_ICONS: [i32; 7] = [32300, 32301, 32302, 32303, 32304, 32305, 32306];
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Fleet {
    pub profiles: Vec<BoatProfile>,
    pub active: Option<String>,
    pub boats: [bool; 3],
    pub hulls: [[bool; 7]; 3],
    pub keels: [[bool; 7]; 3],
    pub cargo: [[bool; 7]; 3],
    pub cannons: [u8; 7],
    pub crew: [bool; 10],
    pub attractor: bool,
    pub perildance: bool,
    pub workbench: bool,
    /// Supplies aboard and absent from the imported item containers.
    pub loaded: BTreeMap<i32, u32>,
}

impl Fleet {
    pub fn valid(&self) -> bool {
        self.profiles.len() <= 5
            && self
                .active
                .as_ref()
                .is_none_or(|name| self.profiles.iter().any(|p| p.name == *name))
            && self.profiles.iter().enumerate().all(|(i, p)| {
                p.valid()
                    && !self.profiles[..i]
                        .iter()
                        .any(|old| old.name.eq_ignore_ascii_case(&p.name))
            })
            && self.cannons.iter().all(|n| *n <= 2)
            && self.loaded.len() <= 21
            && self.loaded.iter().all(|(id, q)| {
                (BALL_IDS.contains(id) || KIT_IDS.contains(id) || LEGACY_KIT_ICONS.contains(id))
                    && *q <= 1_000_000
            })
    }
    pub fn confirm(&mut self, b: &Settings) {
        if !b.valid() {
            return;
        }
        let i = b.boat.index();
        self.boats[i] = true;
        self.hulls[i][b.hull] = true;
        if i > 0 {
            self.keels[i][b.keel] = true;
        }
        if let Some(c) = b.cargo {
            self.cargo[i][c] = true;
        }
        if let Some(c) = b.cannon {
            let mut counts = [0u8; 7];
            counts[c] += 1;
            if b.cannon_count == 2 {
                counts[b.second_cannon.unwrap_or(c)] += 1;
            }
            for (n, c) in self.cannons.iter_mut().zip(counts) {
                *n = (*n).max(c);
            }
        }
        for c in &b.crew {
            if let Some(p) = c.person {
                self.crew[p] = true;
            }
        }
        self.attractor |= b.attractor;
        self.perildance |= b.perildance;
        self.workbench |= b.repair_workbench;
    }
    pub fn toggle(&mut self, b: &Settings, key: &str, n: usize) -> bool {
        if !b.valid() {
            return false;
        }
        let i = b.boat.index();
        match key {
            "boat" if n < 3 => self.boats[n] = !self.boats[n],
            "hull" if n < 7 => self.hulls[i][n] = !self.hulls[i][n],
            "keel" if n < 7 && i > 0 => self.keels[i][n] = !self.keels[i][n],
            "cargo" if n < 7 => self.cargo[i][n] = !self.cargo[i][n],
            "cannon" if n < 7 => self.cannons[n] = (self.cannons[n] + 1) % 3,
            "crew" if n < 10 => self.crew[n] = !self.crew[n],
            "facility" => match n {
                0 => self.attractor = !self.attractor,
                1 => self.perildance = !self.perildance,
                2 => self.workbench = !self.workbench,
                _ => return false,
            },
            _ => return false,
        }
        true
    }
    pub fn permits(&self, b: &Settings) -> bool {
        if !self.valid() || !b.valid() {
            return false;
        }
        if !self.profiles.is_empty() {
            return self.profiles.iter().any(|p| p.matches(b));
        }
        let i = b.boat.index();
        if !self.boats[i]
            || !self.hulls[i][b.hull]
            || i > 0 && !self.keels[i][b.keel]
            || b.cargo.is_some_and(|c| !self.cargo[i][c])
            || b.attractor && !self.attractor
            || b.perildance && !self.perildance
            || b.repair_workbench && !self.workbench
            || b.crew
                .iter()
                .any(|c| c.person.is_some_and(|p| !self.crew[p]))
        {
            return false;
        }
        let mut need = [0u8; 7];
        if let Some(c) = b.cannon {
            need[c] += 1;
            if b.cannon_count == 2 {
                need[b.second_cannon.unwrap_or(c)] += 1;
            }
        }
        need.iter().zip(self.cannons).all(|(a, b)| *a <= b)
    }
}
pub fn stock(snapshot: &Snapshot, o: &Options) -> BTreeMap<i32, u32> {
    let mut owned = snapshot.available(o.shared);
    if let Some(m) = snapshot.selected() {
        for (id, q) in m.sailing_owned.loaded_stock() {
            owned
                .entry(id)
                .and_modify(|n| *n = n.saturating_add(q))
                .or_insert(q);
        }
    }
    for id in KIT_IDS {
        if let Some(q) = owned.remove(&(id + 1)) {
            let count = owned.entry(id).or_default();
            *count = count.saturating_add(q);
        }
    }
    owned.retain(|id, q| *q > 0 && !o.item_excluded(*id));
    owned
}

/// A real, fitted boat. Scenario values come from the current trip, not the saved boat.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoatProfile {
    #[serde(default)]
    pub upgrade: Option<Settings>,
    pub name: String,
    pub settings: Settings,
}
impl BoatProfile {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name == self.name.trim()
            && self.name.chars().count() <= 40
            && !self.name.chars().any(char::is_control)
            && self.upgrade.as_ref().is_none_or(valid_upgrade)
            && self.settings.valid()
            && self.settings.conflicts().is_empty()
    }
    pub fn for_trip(&self, trip: &Settings) -> Settings {
        let mut b = trip.clone();
        let p = &self.settings;
        b.boat = p.boat;
        b.hull = p.hull;
        b.keel = p.keel;
        b.cargo = p.cargo;
        b.cannon = p.cannon;
        b.cannon_count = p.cannon_count;
        b.second_cannon = p.second_cannon;
        b.attractor = p.attractor;
        b.perildance = p.perildance;
        b.repair_workbench = p.repair_workbench;
        b.player_role = p.player_role;
        b.crew = p.crew.clone();
        // Keep trip ammo when compatible; otherwise use the boat's recorded ammunition.
        if !b.ammo_compatible() {
            b.cannonball = p.cannonball;
            b.second_ball = p.second_ball;
        }
        b
    }
    pub fn matches(&self, b: &Settings) -> bool {
        let p = &self.settings;
        b.boat == p.boat
            && b.hull == p.hull
            && b.keel == p.keel
            && b.cargo == p.cargo
            && b.cannon == p.cannon
            && b.cannon_count == p.cannon_count
            && b.second_cannon.unwrap_or(b.cannon.unwrap_or(0))
                == p.second_cannon.unwrap_or(p.cannon.unwrap_or(0))
            && b.attractor == p.attractor
            && b.perildance == p.perildance
            && b.repair_workbench == p.repair_workbench
            && b.player_role == p.player_role
            && b.crew == p.crew
    }
}
/// A cancelable editor. Nothing is written to the account before Save boat.
#[derive(Clone, Debug)]
pub struct Draft {
    pub owner: String,
    pub index: Option<usize>,
    pub original: Option<BoatProfile>,
    pub remove_pending: bool,
    pub error: String,
    pub planning: bool,
    pub name: String,
    pub settings: Settings,
    pub step: usize,
    pub slot: String,
}
impl Fleet {
    pub fn save_boat(
        &mut self,
        index: Option<usize>,
        name: &str,
        settings: &Settings,
    ) -> Result<(), String> {
        let mut settings = settings.clone();
        // These quantities belong to a trip and can conflict with a newly fitted hold.
        settings.cannonballs = 0;
        settings.repair_kits = 0;
        settings.cargo_reserved = 0;
        let profile = BoatProfile {
            upgrade: self
                .profiles
                .get(index.unwrap_or(usize::MAX))
                .and_then(|p| p.upgrade.clone())
                .filter(|planned| {
                    !BoatProfile {
                        name: String::new(),
                        settings: planned.clone(),
                        upgrade: None,
                    }
                    .matches(&settings)
                }),
            name: name.trim().into(),
            settings,
        };
        if !profile.settings.valid() {
            return Err("Check the boat settings.".into());
        }
        let conflicts = profile.settings.conflicts();
        if !conflicts.is_empty() {
            return Err(conflicts.join(" "));
        }
        if !profile.valid() {
            return Err("Use a boat name of 1–40 characters.".into());
        }
        if self
            .profiles
            .iter()
            .enumerate()
            .any(|(i, p)| Some(i) != index && p.name.eq_ignore_ascii_case(&profile.name))
        {
            return Err("Choose a different name for this boat.".into());
        }
        let active = profile.name.clone();
        match index {
            Some(i) if i < self.profiles.len() => self.profiles[i] = profile,
            None if self.profiles.len() < 5 => self.profiles.push(profile),
            Some(_) => return Err("This boat changed. Open the editor again.".into()),
            None => {
                return Err("Your fleet already has five boats. Edit or remove one first.".into());
            }
        }
        self.active = Some(active);
        Ok(())
    }
}

impl Fleet {
    pub fn save_upgrade(&mut self, index: usize, b: &Settings) -> Result<(), String> {
        if !valid_upgrade(b) {
            return Err("Check the planned boat fittings.".into());
        }
        let p = self
            .profiles
            .get_mut(index)
            .ok_or("Boat no longer exists")?;
        p.upgrade = (!crate::sailing_materials::changed(b, Some(&p.settings)).is_empty())
            .then(|| b.clone());
        Ok(())
    }
}

impl Fleet {
    pub fn loaded_stock(&self) -> BTreeMap<i32, u32> {
        let mut out = self.loaded.clone();
        for (icon, item) in LEGACY_KIT_ICONS.into_iter().zip(KIT_IDS) {
            if let Some(q) = out.remove(&icon) {
                out.entry(item).or_insert(q);
            }
        }
        out
    }
}

fn valid_upgrade(b: &Settings) -> bool {
    if !b.valid() {
        return false;
    }
    let mut fittings = b.clone();
    fittings.crew = Default::default();
    fittings.player_role = 0;
    fittings.cannonballs = 0;
    fittings.repair_kits = 0;
    fittings.cargo_reserved = 0;
    fittings.cannonball = 0;
    fittings.second_ball = None;
    fittings.conflicts().is_empty()
}
