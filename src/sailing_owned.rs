//! Account-private availability. Installed facilities and recruited crew cannot
//! be inferred from bank exports or shared loadout links.
use crate::{account::Snapshot, planner::Options, sailing::Settings};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const BALL_IDS: [i32; 7] = [31906, 31908, 2, 31910, 31912, 31914, 31916];
pub const KIT_IDS: [i32; 7] = [32300, 32301, 32302, 32303, 32304, 32305, 32306];
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Fleet {
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
        self.cannons.iter().all(|n| *n <= 2)
            && self.loaded.len() <= 14
            && self
                .loaded
                .iter()
                .all(|(id, q)| (BALL_IDS.contains(id) || KIT_IDS.contains(id)) && *q <= 1_000_000)
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
        for (id, q) in &m.sailing_owned.loaded {
            owned
                .entry(*id)
                .and_modify(|n| *n = n.saturating_add(*q).min(i32::MAX as u32))
                .or_insert(*q);
        }
    }
    owned.retain(|id, q| *q > 0 && !o.item_excluded(*id));
    owned
}
