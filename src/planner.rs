use crate::account::{Member, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SLOTS: [&str; 14] = [
    "Head", "Cape", "Neck", "Weapon", "Body", "Shield", "", "Legs", "", "Hands", "Feet", "",
    "Ring", "Ammo",
];
pub const SLOT_KEYS: [&str; 14] = [
    "head", "cape", "neck", "weapon", "body", "shield", "", "legs", "", "hands", "feet", "",
    "ring", "ammo",
];
pub const GEAR_QUESTS: [&str; 5] = [
    "The Fremennik Isles",
    "The Fremennik Exiles",
    "Dragon Slayer I",
    "Lost City",
    "Underground Pass",
];
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gear {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub slot: String,
    #[serde(default, rename = "isTwoHanded")]
    pub two_handed: bool,
    #[serde(default)]
    pub requirements: Option<BTreeMap<String, u32>>,
}
pub fn equipment() -> &'static [Gear] {
    static ITEMS: std::sync::OnceLock<Vec<Gear>> = std::sync::OnceLock::new();
    ITEMS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/equipment.json")).expect("bundled equipment")
    })
}
pub fn gear(id: i32) -> Option<&'static Gear> {
    static INDEX: std::sync::OnceLock<BTreeMap<i32, usize>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            equipment()
                .iter()
                .enumerate()
                .map(|(i, g)| (g.id, i))
                .collect()
        })
        .get(&id)
        .map(|i| &equipment()[*i])
}
pub fn item_name(id: i32) -> String {
    match id {
        952 => "Spade",
        385 => "Shark",
        3144 => "Cooked karambwan",
        379 => "Lobster",
        373 => "Swordfish",
        8007 => "Teleport to house",
        2434 => "Prayer potion(4)",
        139 => "Prayer potion(3)",
        141 => "Prayer potion(2)",
        143 => "Prayer potion(1)",
        3024 => "Super restore(4)",
        3026 => "Super restore(3)",
        3028 => "Super restore(2)",
        3030 => "Super restore(1)",
        6685 => "Saradomin brew(4)",
        3040 => "Magic potion(4)",
        2444 => "Ranging potion(4)",
        2440 => "Super strength(4)",
        2442 => "Super defence(4)",
        2436 => "Super attack(4)",
        12625 => "Stamina potion(4)",
        556 => "Air rune",
        554 => "Fire rune",
        560 => "Death rune",
        565 => "Blood rune",
        562 => "Chaos rune",
        563 => "Law rune",
        9242 => "Ruby bolts (e)",
        9243 => "Diamond bolts (e)",
        9144 => "Runite bolts",
        9143 => "Adamant bolts",
        9142 => "Mithril bolts",
        892 => "Rune arrow",
        890 => "Adamant arrow",
        1275 => "Rune pickaxe",
        _ => {
            return equipment()
                .iter()
                .find(|g| g.id == id)
                .map(|g| {
                    if g.version.is_empty() {
                        g.name.clone()
                    } else {
                        format!("{} ({})", g.name, g.version)
                    }
                })
                .unwrap_or_else(|| {
                    crate::inventory::public_name(id).unwrap_or_else(|| format!("Item {id}"))
                });
        }
    }
    .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Boss {
    #[default]
    Barrows,
    Toa,
    Custom,
}
impl Boss {
    pub fn name(self) -> &'static str {
        match self {
            Self::Barrows => "Barrows",
            Self::Toa => "Tombs of Amascut",
            Self::Custom => "Custom encounter",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Barrows => "barrows",
            Self::Toa => "toa",
            Self::Custom => "custom",
        }
    }
    pub fn source(self) -> &'static str {
        match self {
            Self::Barrows => "https://oldschool.runescape.wiki/w/Barrows/Strategies",
            Self::Toa => "https://oldschool.runescape.wiki/w/Tombs_of_Amascut/Strategies",
            Self::Custom => "https://oldschool.runescape.wiki/w/Combat",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub id: i32,
    pub style: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monster {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub skills: BTreeMap<String, i32>,
    #[serde(default)]
    pub defensive: BTreeMap<String, i32>,
    #[serde(default)]
    pub weakness: Option<Weakness>,
    #[serde(default)]
    pub attributes: Vec<String>,
    #[serde(default)]
    pub offensive: BTreeMap<String, i32>,
    #[serde(default)]
    pub speed: i32,
    #[serde(default, rename = "style")]
    pub attack_styles: Vec<String>,
    #[serde(default)]
    pub max_hit: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weakness {
    pub element: String,
    pub severity: i32,
}
pub fn monsters() -> &'static [Monster] {
    static MONSTERS: std::sync::OnceLock<Vec<Monster>> = std::sync::OnceLock::new();
    MONSTERS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/monsters.json")).expect("monster catalog")
    })
}
pub fn monster(id: i32) -> Option<&'static Monster> {
    static INDEX: std::sync::OnceLock<BTreeMap<i32, usize>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            monsters()
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id, i))
                .collect()
        })
        .get(&id)
        .map(|i| &monsters()[*i])
}
impl Target {
    pub fn valid(&self) -> bool {
        crate::rules::STYLES.contains(&self.style.as_str())
            && monsters().iter().any(|m| m.id == self.id)
    }
    pub fn name(&self) -> String {
        monsters()
            .iter()
            .find(|m| m.id == self.id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "Unknown monster".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub confirmations: BTreeMap<String, String>,
    pub recommended: Option<bool>,
    pub ranking: crate::ranking::Preferences,
    pub combat: crate::defence::Settings,
    pub boss: Boss,
    pub shared: bool,
    pub fewer_switches: bool,
    pub food: usize,
    pub prayer_potions: usize,
    pub excluded: BTreeSet<i32>,
    pub exclude_void: bool,
    pub access_override: Option<bool>,
    pub started_barrows: Option<bool>,
    pub prayer_unlocks: bool,
    pub raid_level: u32,
    pub targets: Vec<Target>,
    pub gear_overrides: BTreeMap<String, i32>,
    pub inventory_overrides: BTreeMap<usize, i32>,
    pub inventory_quantities: BTreeMap<usize, u32>,
    pub max_switches: Option<usize>,
    #[serde(skip_serializing_if = "crate::efficient::is_default_tolerance")]
    pub switch_tolerance: u32,
    pub switch_slots: usize,
    pub focus_style: String,
    pub phase_priorities: BTreeMap<i32, u32>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            confirmations: BTreeMap::new(),
            recommended: None,
            ranking: Default::default(),
            combat: Default::default(),
            boss: Boss::Barrows,
            shared: false,
            fewer_switches: false,
            food: 8,
            prayer_potions: 3,
            excluded: BTreeSet::new(),
            exclude_void: false,
            access_override: None,
            started_barrows: None,
            prayer_unlocks: false,
            raid_level: 0,
            targets: vec![],
            gear_overrides: BTreeMap::new(),
            inventory_overrides: BTreeMap::new(),
            inventory_quantities: BTreeMap::new(),
            max_switches: None,
            switch_tolerance: crate::efficient::DEFAULT_TOLERANCE_PERCENT,
            switch_slots: 12,
            focus_style: "Balanced".into(),
            phase_priorities: BTreeMap::new(),
        }
    }
}
impl Options {
    pub fn item_excluded(&self, id: i32) -> bool {
        self.excluded.contains(&id) || self.exclude_void && crate::rules::void_piece(id)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Packed {
    pub id: i32,
    pub quantity: u32,
    pub purpose: String,
}
#[derive(Clone, Debug)]
pub struct Phase {
    pub name: String,
    pub style: String,
    pub equipment: Vec<Option<i32>>,
    pub note: String,
}
#[derive(Clone, Debug)]
pub struct Check {
    pub name: String,
    pub state: Option<bool>,
    pub detail: String,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub ammo_shots: u32,
    pub loaded_ammo: Option<Packed>,
    pub optimization: Option<crate::optimization::Report>,
    pub equipment: Vec<Option<i32>>,
    pub inventory: Vec<Packed>,
    pub phases: Vec<Phase>,
    pub checks: Vec<Check>,
    pub warnings: Vec<String>,
    pub missing: Vec<String>,
    pub switches: usize,
    pub supported: usize,
    pub owned: usize,
    pub label: String,
    pub spell: String,
}
impl Plan {
    /// Preview a phase from the saved starting bag without changing the export.
    pub fn preview_inventory(&self, phase: Option<usize>) -> Vec<Packed> {
        let mut inventory = self.inventory.clone();
        let Some(phase) = phase.and_then(|index| self.phases.get(index)) else {
            return inventory;
        };
        let mut freed = BTreeMap::new();
        // Equip incoming gear first, making room for displaced items (including
        // a shield removed by a two-handed weapon).
        for (slot, (start, target)) in self.equipment.iter().zip(&phase.equipment).enumerate() {
            if start == target {
                continue;
            }
            if let Some(index) = target.and_then(|id| {
                inventory
                    .iter()
                    .position(|item| item.id == id && item.quantity > 0)
            }) {
                let quantity = equipped_quantity(slot, target.unwrap(), self.ammo_shots);
                inventory[index].quantity = inventory[index].quantity.saturating_sub(quantity);
                if inventory[index].quantity == 0 {
                    inventory[index].id = 0;
                    freed.insert(slot, index);
                }
            }
        }
        for (slot, (start, target)) in self.equipment.iter().zip(&phase.equipment).enumerate() {
            if start == target {
                continue;
            }
            let Some(id) = *start else { continue };
            let quantity = equipped_quantity(slot, id, self.ammo_shots);
            if is_stackable(id) {
                if let Some(item) = inventory
                    .iter_mut()
                    .find(|item| item.id == id && item.quantity > 0)
                {
                    item.quantity += quantity;
                    continue;
                }
            }
            let item = Packed {
                id,
                quantity,
                purpose: "Unequipped starting gear".into(),
            };
            let index = freed
                .remove(&slot)
                .filter(|index| inventory[*index].quantity == 0)
                .or_else(|| {
                    inventory
                        .iter()
                        .position(|item| item.id <= 0 || item.quantity == 0)
                });
            if let Some(index) = index {
                inventory[index] = item;
            } else {
                inventory.push(item);
            }
        }
        inventory
    }

    pub fn inventory_slots(&self) -> usize {
        self.inventory
            .iter()
            .filter(|p| p.id > 0 && p.quantity > 0)
            .count()
    }
    pub fn exportable(&self) -> bool {
        self.missing.is_empty() && self.inventory.len() <= 28 && self.equipment[3].is_some()
    }
    pub fn status(&self) -> &'static str {
        if self.label.starts_with("Shared setup") {
            return "Shared setup · check your requirements";
        }
        if self.checks.iter().any(|c| c.state == Some(false)) || !self.missing.is_empty() {
            "Preparation needed"
        } else if self.checks.iter().any(|c| c.state.is_none()) {
            "Confirm your levels"
        } else if self.checks.is_empty() {
            "No encounter-specific checks"
        } else {
            "Meets starter guidelines"
        }
    }
}

pub fn can_use(id: i32, m: &Member) -> bool {
    if let Some(requirements) = crate::rules::equipment_requirements(id) {
        return requirements
            .iter()
            .all(|(skill, level)| m.level(skill).is_some_and(|n| n >= *level));
    }
    if let Some(rule) = crate::rules::weapon(id) {
        return rule
            .weapon
            .requirements
            .iter()
            .all(|(skill, level)| m.level(skill).is_some_and(|n| n >= *level));
    }
    if let Some(g) = gear(id) {
        if let Some(req) = &g.requirements {
            if req
                .iter()
                .any(|(s, l)| !m.level(s).is_some_and(|level| level >= *l))
            {
                return false;
            }
        } else if !catalog_ids().contains(&id) {
            return false;
        }
    }
    let req: Vec<(&str, u32)> = match id {
        12002 => vec![("Magic", 70)],
        2412 | 2413 | 2414 => vec![("Magic", 60)],
        10499 => vec![("Ranged", 50)],
        22109 => vec![("Ranged", 70)],
        1275 => vec![("Mining", 41)],
        1271 => vec![("Mining", 31)],
        1273 => vec![("Mining", 21)],
        1269 => vec![("Mining", 6)],
        1169 => vec![("Ranged", 20)],
        21791 | 21793 | 21795 => vec![("Magic", 75)],
        10828 => vec![("Defence", 55)],
        24271 => vec![("Defence", 70)],
        11665 | 11664 | 11663 | 8839 | 8840 | 8842 => vec![("Defence", 42)],
        4712 | 4714 | 4708 | 4856 | 4857 | 4858 | 4859 | 4868 | 4869 | 4870 | 4871 | 4874
        | 4875 | 4876 | 4877 => vec![("Magic", 70), ("Defence", 70)],
        4091 | 4093 | 4099 | 4101 | 4103 | 4109 | 4111 | 4113 => {
            vec![("Magic", 40), ("Defence", 20)]
        }
        2503 => vec![("Ranged", 70), ("Defence", 40)],
        2497 => vec![("Ranged", 70)],
        2501 => vec![("Ranged", 60), ("Defence", 40)],
        2495 => vec![("Ranged", 60)],
        2499 => vec![("Ranged", 50), ("Defence", 40)],
        2493 => vec![("Ranged", 50)],
        1135 => vec![("Ranged", 40), ("Defence", 40)],
        1099 => vec![("Ranged", 40)],
        11832 | 11834 => vec![("Defence", 65)],
        1127 | 1079 | 1201 => vec![("Defence", 40)],
        4722 => vec![("Defence", 70)],
        11840 => vec![("Defence", 60)],
        13239 | 13237 | 13235 => vec![("Defence", 75)],
        6920 => vec![("Magic", 60), ("Defence", 40)],
        19544 | 19547 | 19553 => vec![("Hitpoints", 75)],
        2491 => vec![("Ranged", 70)],
        4095 => vec![("Magic", 40), ("Defence", 20)],
        10551 => vec![("Defence", 40)],
        12954 | 19722 => vec![("Attack", 60), ("Defence", 60)],
        29280 | 29286 => vec![("Ranged", 60), ("Defence", 50)],
        29283 => vec![("Ranged", 60)],
        _ => vec![],
    };
    req.iter().all(|(s, l)| m.level(s).is_some_and(|v| v >= *l))
}
pub fn missing_combat_levels(snapshot: &Snapshot) -> Vec<&'static str> {
    [
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
        "Magic",
        "Prayer",
        "Hitpoints",
    ]
    .into_iter()
    .filter(|skill| snapshot.selected().and_then(|m| m.level(skill)).is_none())
    .collect()
}

// Reviewed, finite item priorities. This is a starter planner, not an exact-DPS optimiser.
fn candidates(style: &str, slot: usize, boss: Boss) -> Vec<i32> {
    if slot == 3 {
        return crate::rules::weapon_ids(style, boss == Boss::Barrows);
    }
    if slot == 13 && crate::rules::base_style(style) == "Ranged" {
        return crate::rules::all_ammo_ids();
    }
    let choices: &[i32] = match (crate::rules::base_style(style), slot) {
        ("Ranged", 13) => &[9144, 9143, 9142, 9243, 9242, 892, 890],
        ("Magic", 0) => &[4708, 4856, 4857, 4858, 4859, 10828, 1169],
        ("Ranged", 0) => &[10828, 1169],
        (_, 0) => &[24271, 10828, 1169],
        ("Magic", 1) => &[2413, 2412, 2414],
        ("Ranged", 1) => &[22109, 10499, 10498],
        (_, 1) => &[6570, 2413],
        ("Magic", 2) => &[12002, 6585, 1712, 1704, 1731],
        ("Ranged", 2) => &[19547, 6585, 1712, 1704, 1731],
        (_, 2) => &[19553, 6585, 1725, 1712, 1704, 1731],
        ("Magic", 4) => &[4712, 4868, 4869, 4870, 4871, 4091, 4101, 4111, 577],
        ("Magic", 7) => &[4714, 4874, 4875, 4876, 4877, 4093, 4103, 4113, 1011],
        ("Ranged", 4) => &[2503, 2501, 2499, 1135, 29280],
        ("Ranged", 7) => &[2497, 2495, 2493, 1099, 29283],
        (_, 4) => &[11832, 10551, 29280, 1127, 2503, 1135],
        (_, 7) => &[11834, 4722, 29283, 1079, 2497, 1099],
        ("Magic", 5) => &[3840, 3842, 1540],
        ("Ranged", 5) => &[3842, 1540],
        (_, 5) => &[12954, 19722, 8850, 3842, 1540],
        ("Magic", 9) => &[19544, 7462, 4095],
        (_, 9) => &[7462, 7461, 2491, 1065],
        ("Magic", 10) => &[13235, 6920, 2579, 3105],
        ("Ranged", 10) => &[13237, 2577, 29286, 3105],
        (_, 10) => &[13239, 11840, 3105],
        ("Magic", 12) => &[28313, 11770, 6731, 2550, 2570],
        ("Ranged", 12) => &[11771, 6733, 2550, 2570],
        (_, 12) => &[11773, 25264, 26770, 6737, 2550, 2570],
        _ => &[],
    };
    choices.to_vec()
}
pub fn catalog_ids() -> BTreeSet<i32> {
    [Boss::Barrows, Boss::Toa, Boss::Custom]
        .into_iter()
        .flat_map(|b| {
            ["Magic", "Ranged", "Melee"]
                .into_iter()
                .flat_map(move |s| (0..14).flat_map(move |slot| candidates(s, slot, b).into_iter()))
        })
        .collect()
}
#[derive(Clone, Debug)]
pub struct OwnedTier {
    pub id: i32,
    pub usable: bool,
    pub excluded: bool,
}
pub fn owned_tiers(
    snapshot: &Snapshot,
    options: &Options,
    style: &str,
    slot: usize,
) -> Vec<OwnedTier> {
    let Some(m) = snapshot.selected() else {
        return vec![];
    };
    let owned = snapshot.available(false);
    let ids: Vec<_> = if slot == 3 || slot == 13 {
        candidates(style, slot, options.boss).to_vec()
    } else {
        equipment()
            .iter()
            .filter(|g| g.slot == SLOT_KEYS[slot])
            .map(|g| g.id)
            .collect()
    };
    ids.iter()
        .filter(|id| owned.get(id).copied().unwrap_or(0) > 0)
        .map(|id| OwnedTier {
            id: *id,
            usable: can_use(*id, m)
                && (slot != 13
                    || (owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots && {
                        let set = equipment_for_style(snapshot, options, style);
                        crate::rules::ammo_ids(set[3].unwrap_or(0)).contains(id)
                    })),
            excluded: options.item_excluded(*id),
        })
        .collect()
}
/// Calculations see the same available stock as the planner, including shared
/// items and exclusions. This transient copy never leaves the local process.
pub(crate) fn member_with_stock(m: &Member, owned: &BTreeMap<i32, u32>) -> Member {
    let mut member = m.clone();
    member.bank = Some(
        owned
            .iter()
            .flat_map(|(id, q)| [i64::from(*id), i64::from(*q)])
            .collect(),
    );
    member.inventory = None;
    member.equipment = None;
    member
}
pub fn equipment_for_style(
    snapshot: &Snapshot,
    options: &Options,
    style: &str,
) -> Vec<Option<i32>> {
    let Some(m) = snapshot.selected() else {
        return vec![None; 14];
    };
    let mut owned = snapshot.available(options.shared);
    owned.retain(|id, _| !options.item_excluded(*id));
    let stock_member = member_with_stock(m, &owned);
    let m = &stock_member;
    let mut set = style_set(style, options.boss, m, &owned, equipment(), options);
    if options.fewer_switches && style != primary_style(options) {
        let primary = style_set(
            primary_style(options),
            options.boss,
            m,
            &owned,
            equipment(),
            options,
        );
        for slot in [0, 1, 2, 9, 10, 12] {
            set[slot] = primary[slot];
        }
    }
    set
}
pub fn primary_style(options: &Options) -> &str {
    match options.boss {
        Boss::Barrows => "Magic",
        Boss::Toa => "Melee",
        Boss::Custom => options
            .targets
            .first()
            .map(|t| t.style.as_str())
            .unwrap_or("Melee"),
    }
}

/// Compare complete owned loadouts when selecting a reviewed single boss.
/// Explicit attack-style edits remain unchanged; this is an opt-in selection action.
pub fn recommend_encounter(
    snapshot: &Snapshot,
    options: &Options,
    targets: &[Target],
) -> Vec<Target> {
    if targets.len() != 1 || !crate::recommendation::enabled(options) {
        return targets.to_vec();
    }
    let mut base = options.clone();
    base.boss = Boss::Custom;
    base.targets = targets.to_vec();
    let Some(profile) = crate::guidance::profile(&base) else {
        return targets.to_vec();
    };
    let Some(member) = snapshot.selected() else {
        return targets.to_vec();
    };
    if !missing_combat_levels(snapshot).is_empty() {
        return targets.to_vec();
    }
    let mut owned = snapshot.available(options.shared);
    owned.retain(|id, _| !options.item_excluded(*id));
    let stock_member = member_with_stock(member, &owned);
    let member = &stock_member;
    let mut best = None::<(f64, String)>;
    for style in crate::rules::STYLES {
        if !profile
            .roles
            .iter()
            .any(|r| r.style == crate::rules::base_style(style))
        {
            continue;
        }
        let mut trial = base.clone();
        trial.targets[0].style = style.into();
        let set = equipment_for_style(snapshot, &trial, style);
        let Some(value) = crate::recommendation::assess(member, &set, style, &trial, &owned) else {
            continue;
        };
        if best.as_ref().is_none_or(|(time, _)| value.time < *time) {
            best = Some((value.time, style.into()));
        }
    }
    best.map(|(_, style)| {
        vec![Target {
            id: targets[0].id,
            style,
        }]
    })
    .unwrap_or_else(|| targets.to_vec())
}

pub(crate) fn effective_options(o: &Options, m: &Member, owned: &BTreeMap<i32, u32>) -> Options {
    let mut options = o.clone();
    options.gear_overrides.retain(|key, id| {
        let Some((style, slot)) = key.split_once(':') else {
            return false;
        };
        let Some(slot) = slot.parse::<usize>().ok().filter(|n| *n < 14) else {
            return false;
        };
        crate::rules::STYLES.contains(&style)
            && (*id == 0
                || owned.get(id).copied().unwrap_or(0) > 0
                    && !o.item_excluded(*id)
                    && can_use(*id, m)
                    && gear(*id).is_some_and(|g| g.slot == SLOT_KEYS[slot]))
    });
    options
}
fn style_set(
    style: &str,
    boss: Boss,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    catalog: &[Gear],
    options: &Options,
) -> Vec<Option<i32>> {
    let effective = effective_options(options, m, owned);
    let options = &effective;
    let mut set = vec![None; 14];
    let slot_keys = [
        "head", "cape", "neck", "weapon", "body", "shield", "", "legs", "", "hands", "feet", "",
        "ring", "ammo",
    ];
    for (slot, value) in set.iter_mut().enumerate() {
        *value = candidates(style, slot, boss).iter().copied().find(|id| {
            owned.get(id).copied().unwrap_or(0) > 0
                && can_use(*id, m)
                && catalog
                    .iter()
                    .any(|g| g.id == *id && g.slot == slot_keys[slot])
        });
    }
    // Armour and accessories come from the complete owned equipment catalog, not weapon presets.
    for slot in [0, 1, 2, 4, 5, 7, 9, 10, 12] {
        let current = set[slot]
            .map(|id| equipment_score(id, style))
            .unwrap_or_default();
        if let Some((id, score)) = catalog
            .iter()
            .filter(|g| g.slot == slot_keys[slot] && owned.contains_key(&g.id) && can_use(g.id, m))
            .map(|g| (g.id, equipment_score(g.id, style)))
            .max_by_key(|(id, score)| (*score, std::cmp::Reverse(*id)))
        {
            if score > current {
                set[slot] = Some(id);
            }
        }
    }
    if crate::recommendation::enabled(options) {
        crate::guidance::seed(&mut set, style, m, owned, options);
    }
    align_spell_weapon(&mut set, style, m, owned, options);
    align_ranged_weapon(&mut set, style, m, owned, options);
    for (slot, value) in set.iter_mut().enumerate() {
        if let Some(id) = options.gear_overrides.get(&format!("{style}:{slot}")) {
            if *id == 0 {
                *value = None;
            } else if owned.get(id).copied().unwrap_or(0) > 0
                && can_use(*id, m)
                && catalog
                    .iter()
                    .any(|g| g.id == *id && g.slot == slot_keys[slot])
            {
                *value = Some(*id);
            }
        }
    }
    if let Some(w) = set[3] {
        if catalog.iter().any(|g| g.id == w && g.two_handed) {
            set[5] = None;
        }
    }
    if style == "Ranged" {
        set[13] = crate::rules::ammo_ids(set[3].unwrap_or(0))
            .iter()
            .copied()
            .find(|id| {
                owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots
                    && can_use(*id, m)
                    && crate::rules::ammunition(*id) != crate::rules::AmmoEffect::Unsupported
            })
            .or_else(|| {
                crate::rules::ammo_ids(set[3].unwrap_or(0))
                    .iter()
                    .copied()
                    .find(|id| {
                        owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots
                            && can_use(*id, m)
                    })
            });
        if let Some(id) = options.gear_overrides.get("Ranged:13") {
            let allowed = crate::rules::ammo_ids(set[3].unwrap_or(0)).contains(id);
            if allowed && owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots {
                set[13] = Some(*id);
            }
        }
    }
    refine_dps(&mut set, style, boss, m, owned, catalog, options);
    set
}
fn align_spell_weapon(
    set: &mut [Option<i32>],
    style: &str,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) {
    let selection = &options.combat.spell_selection;
    if crate::rules::base_style(style) != "Magic"
        || selection.is_auto()
        || options.gear_overrides.contains_key(&format!("{style}:3"))
        || set[3].is_some_and(|id| selection.permits_weapon(id))
    {
        return;
    }
    set[3] = owned
        .keys()
        .copied()
        .filter(|id| {
            can_use(*id, member)
                && crate::rules::weapon(*id).is_some_and(|r| r.family.style == "Magic")
                && selection.permits_weapon(*id)
        })
        .max_by_key(|id| {
            let mut trial = set.to_vec();
            trial[3] = Some(*id);
            let spell = crate::rules::spell_name_configured(member, &trial, owned, &options.combat);
            let missing: u64 =
                crate::rules::rune_supplies_for_set(spell, options.combat.spell_casts, &trial)
                    .iter()
                    .map(|(id, q)| u64::from(q.saturating_sub(owned.get(id).copied().unwrap_or(0))))
                    .sum();
            (
                crate::rules::casting_supported_configured(
                    member,
                    &trial,
                    owned,
                    options.combat.spell_casts,
                    &options.combat,
                ),
                std::cmp::Reverse(missing),
                equipment_score(*id, style),
                std::cmp::Reverse(*id),
            )
        });
}
fn ranged_ammo_for_weapon(
    weapon: i32,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    shots: u32,
) -> Option<i32> {
    let allowed = crate::rules::ammo_ids(weapon);
    let available =
        |id: &&i32| owned.get(id).copied().unwrap_or(0) >= shots && can_use(**id, member);
    allowed
        .iter()
        .filter(available)
        .find(|id| crate::rules::ammunition(**id) != crate::rules::AmmoEffect::Unsupported)
        .or_else(|| allowed.iter().find(available))
        .copied()
}
fn align_ranged_weapon(
    set: &mut [Option<i32>],
    style: &str,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) {
    let selection = options.combat.ranged_type;
    if style != "Ranged"
        || selection.key().is_none()
        || options.gear_overrides.contains_key("Ranged:3")
    {
        return;
    }
    let ready = |id| {
        let rule = crate::rules::weapon(id).unwrap();
        if !rule.family.internal_ammo.is_empty() {
            crate::rules::internal_ammo(member, id, &options.combat).is_some_and(|dart| {
                options.combat.loaded_ammo_confirmed
                    || owned.get(&dart).copied().unwrap_or(0) >= options.combat.ammo_shots
            })
        } else {
            rule.family.ammo.is_empty()
                || ranged_ammo_for_weapon(id, member, owned, options.combat.ammo_shots).is_some()
        }
    };
    if !set[3].is_some_and(|id| selection.permits_weapon(id) && ready(id)) {
        set[3] = owned
            .keys()
            .copied()
            .filter(|id| can_use(*id, member) && selection.permits_weapon(*id))
            .max_by_key(|id| {
                (
                    ready(*id),
                    equipment_score(*id, style),
                    std::cmp::Reverse(*id),
                )
            });
    }
    set[13] =
        set[3].and_then(|id| ranged_ammo_for_weapon(id, member, owned, options.combat.ammo_shots));
}
fn refine_dps(
    set: &mut Vec<Option<i32>>,
    style: &str,
    boss: Boss,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    catalog: &[Gear],
    options: &Options,
) {
    // Refine ordinary armour and joint set/weapon combinations independently.
    // A one-slot search cannot enter or leave interdependent effects reliably.
    let seeds = crate::set_search::candidates(set, style, m, owned, options);
    refine_dps_seed(set, style, boss, m, owned, catalog, options);
    if seeds.is_empty() {
        return;
    }
    let mut trials = vec![set.clone()];
    for candidate in seeds {
        let mut seed = candidate.equipment;
        let mut pinned = options.clone();
        for slot in candidate.slots {
            pinned
                .gear_overrides
                .insert(format!("{style}:{slot}"), seed[slot].unwrap_or(0));
        }
        refine_dps_seed(&mut seed, style, boss, m, owned, catalog, &pinned);
        trials.push(seed);
    }
    let Some(target) = refinement_target(style, boss, options).and_then(monster) else {
        return;
    };
    let judge_magic = judge_magic_set(style, boss, m, owned, catalog, options);
    let chosen = choose_equipment(
        &trials,
        style,
        m,
        owned,
        options,
        target,
        judge_magic.as_ref(),
    );
    *set = trials[chosen].clone();
}

fn refinement_target(style: &str, boss: Boss, options: &Options) -> Option<i32> {
    Some(match boss {
        Boss::Barrows => {
            if style == "Ranged" {
                1672
            } else {
                1673
            }
        }
        Boss::Toa => match style {
            "Ranged" => 11730,
            "Magic" => 11789,
            _ => 11778,
        },
        Boss::Custom => options.targets.iter().find(|t| t.style == style)?.id,
    })
}

fn judge_magic_set(
    style: &str,
    boss: Boss,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    catalog: &[Gear],
    options: &Options,
) -> Option<Vec<Option<i32>>> {
    let relevant: Vec<_> = options
        .targets
        .iter()
        .filter(|t| t.style == style)
        .collect();
    (!relevant.is_empty()
        && relevant.iter().all(|t| crate::yama::is_judge_window(t.id))
        && crate::rules::base_style(style) == "Melee")
        .then(|| style_set("Magic", boss, m, owned, catalog, options))
}

fn choose_equipment(
    trials: &[Vec<Option<i32>>],
    style: &str,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
    target: &Monster,
    judge_magic: Option<&Vec<Option<i32>>>,
) -> usize {
    if !crate::recommendation::enabled(options) {
        let metrics: Vec<_> = trials
            .iter()
            .map(|set| {
                let dps = crate::combat::calculate_configured(
                    m,
                    set,
                    style,
                    target,
                    crate::rules::spell_name_for_target(
                        m,
                        set,
                        owned,
                        &options.combat,
                        target,
                        options.raid_level,
                    ),
                    options.raid_level,
                    &options.combat,
                )
                .ok()
                .map(|r| r.dps);
                crate::ranking::equipment_metrics(set, dps, options.ranking.defence)
            })
            .collect();
        if options.ranking.weights.dps > 0.0 && metrics.iter().all(|row| row.dps.is_none()) {
            return 0;
        }
        let scores = crate::ranking::rank(&metrics, &options.ranking.weights).scores;
        let mut best = 0;
        for i in 1..trials.len() {
            if scores[i].is_some()
                && (scores[i] > scores[best]
                    || scores[i] == scores[best]
                        && equipment_tiebreak(&trials[i], style)
                            > equipment_tiebreak(&trials[best], style))
            {
                best = i;
            }
        }
        return best;
    }
    if let Some(magic) = judge_magic {
        if let Some((index, _, _)) = trials
            .iter()
            .enumerate()
            .filter_map(|(i, set)| {
                crate::yama::judge_cycle(m, set, magic, options, owned)
                    .ok()
                    .map(|cycle| {
                        (
                            i,
                            cycle.seconds,
                            crate::guidance::quality(style, set, options),
                        )
                    })
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)))
        {
            return index;
        }
    }
    let relevant: Vec<_> = options
        .targets
        .iter()
        .filter(|t| t.style == style)
        .collect();
    if !relevant.is_empty()
        && relevant.iter().all(|t| crate::yama::is_judge_window(t.id))
        && crate::rules::base_style(style) == "Magic"
    {
        if let Some((index, _)) = trials
            .iter()
            .enumerate()
            .filter_map(|(i, set)| {
                crate::combat::calculate_configured(
                    m,
                    set,
                    style,
                    target,
                    crate::rules::spell_name_for_target(
                        m,
                        set,
                        owned,
                        &options.combat,
                        target,
                        options.raid_level,
                    ),
                    options.raid_level,
                    &options.combat,
                )
                .ok()
                .map(|r| (i, r.dps))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
        {
            return index;
        }
    }
    crate::recommendation::choose(trials, style, m, options, owned)
}

fn refine_dps_seed(
    set: &mut Vec<Option<i32>>,
    style: &str,
    boss: Boss,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    catalog: &[Gear],
    options: &Options,
) {
    let Some(target) = refinement_target(style, boss, options).and_then(monster) else {
        return;
    };
    // Judge melee is a separate attack commitment from the main slash set.
    // Magic refinement has no reverse dependency, so this comparison is acyclic.
    let judge_magic = judge_magic_set(style, boss, m, owned, catalog, options);
    let choose = |trials: &[Vec<Option<i32>>]| {
        choose_equipment(
            trials,
            style,
            m,
            owned,
            options,
            target,
            judge_magic.as_ref(),
        )
    };
    if crate::recommendation::enabled(options) {
        let trials = guided_trials(set, style, m, owned, options);
        let chosen = choose(&trials);
        *set = trials[chosen].clone();
    }
    for _ in 0..2 {
        let mut changed = false;
        for slot in [3, 0, 1, 2, 4, 5, 7, 9, 10, 12, 13] {
            if options
                .gear_overrides
                .contains_key(&format!("{style}:{slot}"))
                || slot == 13 && style != "Ranged"
            {
                continue;
            }
            if slot == 5 && set[3].is_some_and(|id| gear(id).is_some_and(|g| g.two_handed)) {
                continue;
            }
            let ids: Vec<_> = if slot == 3 || slot == 13 {
                candidates(style, slot, boss).to_vec()
            } else {
                catalog
                    .iter()
                    .filter(|g| g.slot == SLOT_KEYS[slot])
                    .map(|g| g.id)
                    .collect()
            };
            let mut trials = vec![set.clone()];
            for id in ids {
                if owned.get(&id).copied().unwrap_or(0) == 0 || !can_use(id, m) {
                    continue;
                }
                if slot == 3 && style == "Ranged" && !options.combat.ranged_type.permits_weapon(id)
                {
                    continue;
                }
                if slot == 13 && owned.get(&id).copied().unwrap_or(0) < options.combat.ammo_shots {
                    continue;
                }
                if slot == 13
                    && (crate::rules::ammunition(id) == crate::rules::AmmoEffect::Unsupported
                        || !crate::rules::ammo_ids(set[3].unwrap_or(0)).contains(&id))
                {
                    continue;
                }
                let mut trial = set.clone();
                trial[slot] = Some(id);
                if slot == 3 {
                    if gear(id).is_some_and(|g| g.two_handed)
                        && options
                            .gear_overrides
                            .get(&format!("{style}:5"))
                            .is_some_and(|id| *id > 0)
                    {
                        continue;
                    }
                    if gear(id).is_some_and(|g| g.two_handed) {
                        trial[5] = None;
                    } else if trial[5].is_none()
                        && !options.gear_overrides.contains_key(&format!("{style}:5"))
                    {
                        trial[5] = catalog
                            .iter()
                            .filter(|g| {
                                g.slot == "shield" && owned.contains_key(&g.id) && can_use(g.id, m)
                            })
                            .max_by_key(|g| equipment_score(g.id, style))
                            .map(|g| g.id);
                    }
                }
                let ammo: Vec<Option<i32>> = if slot == 3 && style == "Ranged" {
                    if options.gear_overrides.contains_key("Ranged:13") {
                        vec![set[13]]
                    } else {
                        let allowed = crate::rules::ammo_ids(id);
                        if allowed.is_empty() {
                            vec![None]
                        } else {
                            allowed
                                .iter()
                                .filter(|id| {
                                    owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots
                                        && can_use(**id, m)
                                        && crate::rules::ammunition(**id)
                                            != crate::rules::AmmoEffect::Unsupported
                                })
                                .map(|id| Some(*id))
                                .collect()
                        }
                    }
                } else {
                    vec![trial[13]]
                };
                for a in ammo {
                    trial[13] = a;
                    if !crate::rules::casting_supported_configured(
                        m,
                        &trial,
                        owned,
                        options.combat.spell_casts,
                        &options.combat,
                    ) {
                        continue;
                    }
                    if slot == 3 && crate::recommendation::enabled(options) {
                        trials.extend(guided_trials(&trial, style, m, owned, options));
                    }
                    trials.push(trial.clone());
                }
            }
            let chosen = choose(&trials);
            if trials[chosen] != *set {
                *set = trials[chosen].clone();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn guided_trials(
    set: &[Option<i32>],
    style: &str,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) -> Vec<Vec<Option<i32>>> {
    let mut guided = set.to_vec();
    crate::guidance::seed(&mut guided, style, member, owned, options);
    let mut trials = vec![set.to_vec(), guided.clone()];
    for trial in &mut trials {
        align_spell_weapon(trial, style, member, owned, options);
        align_ranged_weapon(trial, style, member, owned, options);
        for (key, id) in &options.gear_overrides {
            if let Some(slot) = key
                .strip_prefix(&format!("{style}:"))
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|slot| *slot < 14)
            {
                trial[slot] = if *id == 0 { None } else { Some(*id) };
            }
        }
        if trial[3].is_some_and(|id| gear(id).is_some_and(|g| g.two_handed)) {
            trial[5] = None;
        }
    }
    trials.sort();
    trials.dedup();
    // Preserve the caller's seed if every attack estimate is unavailable.
    if let Some(index) = trials.iter().position(|trial| trial == set) {
        trials.swap(0, index);
    }
    trials
}
// Offensive stats seed the search; defence resolves ties before free prayer
// bonus. Prayer economy only trades away combat stats through the explicit
// Options::ranking weights, never through an unconditional prayer bonus.
fn equipment_score(id: i32, style: &str) -> (i32, i32, i32) {
    let Some(s) = crate::combat::stats(id) else {
        return (0, 0, 0);
    };
    let get = |m: &BTreeMap<String, i32>, k: &str| m.get(k).copied().unwrap_or(0);
    let score = match crate::rules::base_style(style) {
        "Magic" => get(&s.bonuses, "magic_str") * 100 + get(&s.offensive, "magic") * 10,
        "Ranged" => get(&s.bonuses, "ranged_str") * 100 + get(&s.offensive, "ranged") * 10,
        _ => {
            get(&s.bonuses, "str") * 100
                + ["stab", "slash", "crush"]
                    .iter()
                    .map(|k| get(&s.offensive, k))
                    .max()
                    .unwrap_or(0)
                    * 10
        }
    };
    (score, s.defensive.values().sum(), get(&s.bonuses, "prayer"))
}

fn equipment_tiebreak(set: &[Option<i32>], style: &str) -> (i32, i32, i32) {
    set.iter().flatten().fold((0, 0, 0), |total, id| {
        let (offence, defence, prayer) = equipment_score(*id, style);
        (total.0 + defence, total.1 + offence, total.2 + prayer)
    })
}

pub fn plan(snapshot: &Snapshot, options: &Options) -> Plan {
    // Manual confirmations never influence gear selection or packing.
    let mut options = options.clone();
    options.confirmations.clear();
    // Tolerance selects among plans; it does not change an individual plan.
    options.switch_tolerance = crate::efficient::DEFAULT_TOLERANCE_PERCENT;
    let key = serde_json::to_string(&(snapshot, &options)).unwrap_or_default();
    thread_local! {static CACHE:std::cell::RefCell<std::collections::VecDeque<(String,Plan)>>=const{std::cell::RefCell::new(std::collections::VecDeque::new())};}
    if let Some(p) = CACHE.with(|c| {
        c.borrow()
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, p)| p.clone())
    }) {
        return p;
    }
    let result = plan_uncached(snapshot, &options);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= 8 {
            c.pop_front();
        }
        c.push_back((key, result.clone()));
    });
    result
}
fn pack_profile_supplies(
    out: &mut Plan,
    allocated: &mut BTreeMap<i32, u32>,
    owned: &BTreeMap<i32, u32>,
    options: &Options,
    styles: &[&str],
    profile: &crate::guidance::Profile,
    magic_boost_useful: bool,
) -> usize {
    let mut packed_roles = BTreeSet::new();
    let mut prayer_bottles = 0usize;
    for role in &profile.supply_roles {
        if !role.styles.is_empty()
            && !role
                .styles
                .iter()
                .any(|style| styles.iter().any(|s| crate::rules::base_style(s) == style))
            || role.skipped_by.iter().any(|id| packed_roles.contains(id))
            || role.requires_magic_benefit && !magic_boost_useful
        {
            continue;
        }
        let reserved = options.food.min(20)
            + options
                .prayer_potions
                .min(12)
                .saturating_sub(prayer_bottles);
        if !role.required && out.inventory.len().saturating_add(reserved) >= 28 {
            continue;
        }
        let chosen = role.ids.iter().copied().find(|id| {
            owned.get(id).copied().unwrap_or(0) > allocated.get(id).copied().unwrap_or(0)
        });
        if let Some(id) = chosen {
            out.inventory.push(Packed {
                id,
                quantity: 1,
                purpose: role.label.clone(),
            });
            *allocated.entry(id).or_default() += 1;
            if role.prayer_ids.contains(&id) {
                prayer_bottles += 1;
            }
            packed_roles.insert(role.id.clone());
        }
        if role.required {
            out.checks.push(Check {
                name: role.label.clone(),
                state: Some(chosen.is_some()),
                detail: chosen.map(item_name).unwrap_or_else(|| {
                    "No supported owned bottle; prepare protection before entering.".into()
                }),
            });
            if chosen.is_none() {
                out.warnings.push(format!("{} is missing. This packing list does not establish that the encounter is ready.",role.label));
            }
        }
    }
    prayer_bottles
}
fn plan_uncached(snapshot: &Snapshot, options: &Options) -> Plan {
    let mut out = Plan {
        ammo_shots: options.combat.ammo_shots,
        loaded_ammo: None,
        optimization: None,
        equipment: vec![None; 14],
        inventory: vec![],
        phases: vec![],
        checks: vec![],
        warnings: vec![],
        missing: vec![],
        switches: 0,
        supported: 0,
        owned: 0,
        label: "Owned gear with supported DPS comparisons".into(),
        spell: String::new(),
    };
    let Some(m) = snapshot.selected() else {
        out.missing.push("Select an account first.".into());
        return out;
    };
    let missing_levels = missing_combat_levels(snapshot);
    if !missing_levels.is_empty() {
        out.label = "Combat levels required".into();
        out.missing.push(format!(
            "Enter your combat levels: {}",
            missing_levels.join(", ")
        ));
        return out;
    }
    let mut owned = snapshot.available(options.shared);
    owned.retain(|id, _| !options.item_excluded(*id));
    let catalog = equipment();
    let stock_member = member_with_stock(m, &owned);
    let m = &stock_member;
    let profile = crate::guidance::profile(options);
    out.owned = owned.len();
    out.supported = owned
        .keys()
        .filter(|id| {
            ["Magic", "Ranged", "Melee"]
                .iter()
                .any(|s| (0..14).any(|slot| candidates(s, slot, options.boss).contains(id)))
        })
        .count();
    for (skill, minimum) in if options.boss == Boss::Custom {
        vec![]
    } else if options.boss == Boss::Barrows {
        vec![("Magic", 50), ("Ranged", 50), ("Prayer", 43)]
    } else {
        vec![
            ("Attack", 70),
            ("Strength", 70),
            ("Defence", 70),
            ("Ranged", 70),
            ("Magic", 75),
            ("Prayer", 43),
        ]
    } {
        out.checks.push(Check {
            name: format!("{minimum} {skill}"),
            state: m.level(skill).map(|v| v >= minimum),
            detail: format!(
                "Starter guideline · current {}",
                m.level(skill)
                    .map(|v| v.to_string())
                    .unwrap_or("unknown".into())
            ),
        });
    }
    if m.bank.is_none() {
        out.checks.push(Check {
            name: "Bank observation".into(),
            state: None,
            detail: "Bank missing; only carried and worn items are known".into(),
        });
    }
    let targets: Vec<_> = options
        .targets
        .iter()
        .filter(|t| t.valid())
        .take(crate::rules::MAX_TARGETS)
        .collect();
    if options.boss == Boss::Custom
        && (targets.is_empty() || targets.len() != options.targets.len())
    {
        out.missing.push(format!(
            "Select 1–{} valid monsters and their attack styles.",
            crate::rules::MAX_TARGETS
        ));
    }
    let primary = if options.boss == Boss::Custom {
        targets.first().map(|t| t.style.as_str()).unwrap_or("Melee")
    } else if options.boss == Boss::Barrows {
        "Magic"
    } else {
        "Melee"
    };
    out.equipment = style_set(primary, options.boss, m, &owned, catalog, options);
    let ranged = style_set("Ranged", options.boss, m, &owned, catalog, options);
    let magic = style_set("Magic", options.boss, m, &owned, catalog, options);
    if out.equipment[3].is_none() {
        out.missing
            .push(format!("A supported, usable {primary} weapon"));
    }
    let needs_ranged = options.boss != Boss::Custom || targets.iter().any(|t| t.style == "Ranged");
    if needs_ranged && ranged[3].is_none() {
        out.missing
            .push(if options.combat.ranged_type.key().is_none() {
                "A supported ranged weapon".into()
            } else {
                format!(
                    "A supported, usable {} ranged weapon from your bank",
                    options.combat.ranged_type.label()
                )
            });
    }
    if needs_ranged && ranged[3].is_some_and(|id| !options.combat.ranged_type.permits_weapon(id)) {
        out.missing.push(format!(
            "The pinned ranged weapon does not match {}. Choose a compatible weapon or Auto.",
            options.combat.ranged_type.label()
        ));
    }
    if needs_ranged
        && ranged[3].is_some_and(|id| !crate::rules::ammo_ids(id).is_empty())
        && ranged[13].is_none()
    {
        out.missing.push(format!(
            "At least {} compatible bolts or arrows",
            options.combat.ammo_shots
        ));
    }
    if (options.boss == Boss::Toa
        || (options.boss == Boss::Custom && targets.iter().any(|t| t.style == "Magic")))
        && magic[3].is_none()
    {
        out.missing
            .push("A usable supported Magic weapon or spell with sufficient runes".into());
    }
    let mut sets = if options.boss == Boss::Custom {
        crate::rules::STYLES
            .into_iter()
            .filter(|style| targets.iter().any(|t| t.style == *style))
            .map(|style| {
                (
                    style,
                    style_set(style, options.boss, m, &owned, catalog, options),
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![("Ranged", ranged.clone())]
    };
    if options.boss == Boss::Toa {
        sets.push(("Magic", magic.clone()));
    }
    if options.fewer_switches {
        for (_, set) in &mut sets {
            for slot in [0, 1, 2, 9, 10, 12] {
                set[slot] = out.equipment[slot];
            }
        }
    }
    if options.max_switches.is_some() || crate::yama::has_judge_cycle(options) {
        let mut ideals = vec![crate::optimization::StyleSet {
            style: primary.into(),
            equipment: out.equipment.clone(),
        }];
        for (style, set) in &sets {
            if !ideals.iter().any(|s| s.style == *style) {
                ideals.push(crate::optimization::StyleSet {
                    style: (*style).into(),
                    equipment: set.clone(),
                });
            }
        }
        if ideals.iter().all(|s| s.equipment[3].is_some()) {
            match crate::optimization::solve(m, &owned, ideals, options) {
                Ok((chosen, report)) => {
                    if let Some(primary) = chosen.iter().find(|s| s.style == primary) {
                        out.equipment = primary.equipment.clone();
                    }
                    for (style, set) in &mut sets {
                        if let Some(chosen) = chosen.iter().find(|s| s.style == *style) {
                            *set = chosen.equipment.clone();
                        }
                    }
                    out.optimization = Some(report);
                }
                Err(e) => out.missing.push(e),
            }
        }
    }
    let ranged = sets
        .iter()
        .find(|(s, _)| *s == "Ranged")
        .map(|(_, s)| s.clone())
        .unwrap_or(ranged);
    let magic = sets
        .iter()
        .find(|(s, _)| *s == "Magic")
        .map(|(_, s)| s.clone())
        .unwrap_or(magic);
    let mut allocated: BTreeMap<i32, u32> = BTreeMap::new();
    let mut essentials = BTreeMap::<i32, u32>::new();
    for id in out.equipment.iter().flatten() {
        *allocated.entry(*id).or_default() += 1;
    }
    for (style, set) in &sets {
        for (slot, id) in set.iter().enumerate() {
            if let Some(id) = id {
                if out.equipment[slot] != Some(*id) && !out.inventory.iter().any(|p| p.id == *id) {
                    out.inventory.push(Packed {
                        id: *id,
                        quantity: equipped_quantity(slot, *id, options.combat.ammo_shots),
                        purpose: format!("{style} switch"),
                    });
                    *allocated.entry(*id).or_default() +=
                        equipped_quantity(slot, *id, options.combat.ammo_shots);
                }
            }
        }
    }
    out.switches = out.inventory.len();
    if options.boss == Boss::Barrows {
        let mage = out.equipment.clone();
        out.phases.push(Phase{name:"Dharok · Guthan · Torag · Verac · Karil".into(),style:"Magic".into(),equipment:mage,note:"Use the selected spell or powered staff. Prayer drain makes efficient trips valuable.".into()});
        out.phases.push(Phase {
            name: "Ahrim".into(),
            style: "Ranged".into(),
            equipment: ranged,
            note: "Switch to your ranged equipment and compatible ammunition.".into(),
        });
    } else if options.boss == Boss::Custom {
        for target in targets {
            if let Some((_, set)) = sets.iter().find(|(style, _)| *style == target.style) {
                out.phases.push(Phase{
                    name:target.name(),style:target.style.clone(),equipment:set.clone(),
                    note:crate::encounters::window(target.id).map(|window|window.description.clone())
                        .unwrap_or_else(||"User-selected attack style. Check immunities, protection, special items and encounter mechanics.".into())
                });
            }
        }
        out.warnings.push("Custom groups share one carried equipment pool. Standard attack estimates and verified defence profiles guide choices; special mechanics and supply consumption remain partial.".into());
    } else {
        for (name, style, set, note) in [
            (
                "Ba-ba",
                "Melee",
                out.equipment.clone(),
                "Use your stab weapon; bring a pickaxe for the Het puzzle.",
            ),
            (
                "Kephri",
                "Melee",
                out.equipment.clone(),
                "A Keris partisan is a useful dedicated weapon if available.",
            ),
            (
                "Zebak",
                "Ranged",
                ranged.clone(),
                "This export uses one ammunition type; bolt rotations are not modelled.",
            ),
            (
                "Akkha",
                "All three styles",
                magic.clone(),
                "Carry all three sets. Rotate manually as protection changes; final stand uses melee.",
            ),
            (
                "Wardens",
                "Magic / Ranged",
                ranged.clone(),
                "Phase 2 needs magic and ranged. Exposed core uses melee; phase 3 uses ranged.",
            ),
        ] {
            out.phases.push(Phase {
                name: name.into(),
                style: style.into(),
                equipment: set,
                note: note.into(),
            });
        }
        let mut keris = crate::rules::family_ids("partisan");
        let value = |id: i32| {
            let mut trial = out.equipment.clone();
            trial[3] = Some(id);
            monster(11719)
                .and_then(|target| {
                    crate::combat::calculate_configured(
                        m,
                        &trial,
                        "Melee",
                        target,
                        "",
                        options.raid_level,
                        &options.combat,
                    )
                    .ok()
                })
                .map(|r| r.dps)
                .unwrap_or(0.0)
        };
        keris.sort_by(|a, b| {
            value(*b)
                .partial_cmp(&value(*a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (ids, purpose) in [
            (&keris[..], "Kephri weapon"),
            (&[1215][..], "Core special attack"),
            (&[1275, 1271, 1273, 1269][..], "Het puzzle pickaxe"),
        ] {
            if let Some(id) = ids
                .iter()
                .find(|id| owned.contains_key(id) && can_use(**id, m))
            {
                if purpose == "Het puzzle pickaxe" {
                    essentials.insert(*id, 1);
                }
                if !allocated.contains_key(id) {
                    if options.max_switches.is_some()
                        && purpose != "Het puzzle pickaxe"
                        && out.inventory.len() >= options.switch_slots
                    {
                        continue;
                    }
                    out.inventory.push(Packed {
                        id: *id,
                        quantity: 1,
                        purpose: purpose.into(),
                    });
                    allocated.insert(*id, 1);
                }
                if purpose == "Kephri weapon" {
                    if let Some(p) = out.phases.iter_mut().find(|p| p.name == "Kephri") {
                        p.equipment[3] = Some(*id);
                    }
                }
            } else if purpose == "Het puzzle pickaxe" {
                out.missing
                    .push("A supported pickaxe for the Het puzzle".into());
            }
        }
        if options.raid_level > 150 {
            out.warnings.push("This starter plan supports raid levels 0–150. Higher invocations require a separate reviewed plan.".into());
        }
        out.warnings.push("ToA is a packing and phase plan, not a raid simulator. Invocation mechanics, spec energy, defence drains, salts and supply consumption need your judgement.".into());
    }
    // Deduct worn ammunition properly; a worn ammo slot is one stack, not one projectile.
    if let Some(id) = out.equipment[13] {
        allocated.insert(id, equipped_quantity(13, id, options.combat.ammo_shots));
    }
    let mut spell_runes = BTreeMap::<i32, u32>::new();
    let mut trip_spells = std::collections::BTreeSet::new();
    let mut mark_applies = false;
    for set in std::iter::once(&out.equipment).chain(out.phases.iter().map(|p| &p.equipment)) {
        if !set[3]
            .and_then(crate::rules::weapon)
            .is_some_and(|r| r.family.style == "Magic")
        {
            continue;
        }
        let mut selected_spells = std::collections::BTreeSet::new();
        for (id, style) in crate::recommendation::targets(options) {
            if crate::rules::base_style(style) == "Magic" {
                if let Some(target) = monster(id) {
                    selected_spells.insert(crate::rules::spell_name_for_target(
                        m,
                        set,
                        &owned,
                        &options.combat,
                        target,
                        options.raid_level,
                    ));
                }
            }
        }
        if selected_spells.is_empty() {
            selected_spells.insert(crate::rules::spell_name_configured(
                m,
                set,
                &owned,
                &options.combat,
            ));
        }
        mark_applies |= options.combat.mark_darkness
            && set[3]
                .and_then(crate::rules::weapon)
                .is_some_and(|r| r.has(crate::rules::Effect::DemonbaneSpell));
        for spell in selected_spells {
            trip_spells.insert(if spell.is_empty() {
                "Built-in spell"
            } else {
                spell
            });
            if !crate::rules::casting_supported_configured(
                m,
                set,
                &owned,
                options.combat.spell_casts,
                &options.combat,
            ) {
                let level = crate::rules::spells()
                    .iter()
                    .find(|s| s.name == spell)
                    .map(|s| s.level);
                out.missing.push(format!(
                    "{} is required for this spell template, with sufficient runes for {} casts.",
                    level
                        .map(|l| format!("{l} Magic"))
                        .unwrap_or("A usable charged staff".into()),
                    options.combat.spell_casts
                ));
            }
            for (id, quantity) in
                crate::rules::rune_supplies_for_set(spell, options.combat.spell_casts, set)
            {
                spell_runes
                    .entry(id)
                    .and_modify(|q| *q = (*q).max(quantity))
                    .or_insert(quantity);
            }
        }
    }
    out.spell = trip_spells.into_iter().collect::<Vec<_>>().join(" / ");
    if out.spell.is_empty()
        && !options.combat.spell_selection.is_auto()
        && crate::recommendation::targets(options)
            .iter()
            .any(|(_, style)| crate::rules::base_style(style) == "Magic")
    {
        out.spell = match &options.combat.spell_selection {
            crate::rules::SpellSelection::Spell(name) => name.clone(),
            crate::rules::SpellSelection::Element(element) => format!("{element} spell"),
            _ => String::new(),
        };
        out.missing.push(format!(
            "A compatible owned spellcasting weapon for {}",
            out.spell
        ));
    }
    if mark_applies {
        for (id, quantity) in crate::rules::mark_darkness_supplies() {
            *spell_runes.entry(id).or_default() += quantity;
        }
        out.checks.push(Check{name:"Mark of Darkness".into(),state:Some(crate::rules::mark_darkness_supported(m,&owned)),detail:"One pre-cast is packed. You choose the spellbook and cast it manually; recasts consume more runes.".into()});
        if !crate::rules::mark_darkness_supported(m, &owned) {
            out.missing.push("Mark of Darkness needs its Magic level and one cast's cosmic/soul runes, in addition to attack-spell runes.".into());
        }
    }
    let mut reserve = |id: i32, q: u32, purpose: &str, required: bool| {
        if required {
            essentials
                .entry(id)
                .and_modify(|need| *need = (*need).max(q))
                .or_insert(q);
        }
        let used = allocated.get(&id).copied().unwrap_or(0);
        let available = owned.get(&id).copied().unwrap_or(0).saturating_sub(used);
        if available >= q {
            out.inventory.push(Packed {
                id,
                quantity: q,
                purpose: purpose.into(),
            });
            *allocated.entry(id).or_default() += q;
        } else if required {
            out.missing
                .push(format!("{} × {q} ({available} available)", item_name(id)));
        }
    };
    if options.boss == Boss::Barrows {
        reserve(952, 1, "Dig into the crypts", true);
    }
    for (id, quantity) in spell_runes {
        reserve(id, quantity, "Spell casts · editable trip assumption", true);
    }
    if let Some(id) = [
        8007, 2552, 2554, 2556, 2558, 2560, 2562, 2564, 2566, 3853, 3855, 3857, 3859, 3861, 3863,
        3865, 3867,
    ]
    .into_iter()
    .find(|id| owned.get(id).copied().unwrap_or(0) > 0)
    {
        reserve(id, 1, "Exit teleport · check destination and charges", true);
    } else {
        reserve(8007, 1, "Exit teleport · requires your POH", true);
    }
    if owned.contains_key(&12625) && profile.is_none_or(|p| !p.skip_stamina) {
        reserve(12625, 1, "Stamina", false);
    }
    drop(reserve);
    let styles: Vec<_> = std::iter::once(primary)
        .chain(sets.iter().map(|(style, _)| *style))
        .collect();
    let magic_boost_useful = sets
        .iter()
        .filter(|(style, _)| crate::rules::base_style(style) == "Magic")
        .any(|(style, set)| {
            set[3]
                .and_then(crate::rules::weapon)
                .is_some_and(|r| r.family.formula == crate::rules::Formula::Powered)
                || options.targets.iter().any(|t| {
                    t.style == *style
                        && !crate::encounters::hit_rule(t.id).is_some_and(|r| r.always_max)
                })
        });
    let supplied_prayer = profile
        .map(|p| {
            pack_profile_supplies(
                &mut out,
                &mut allocated,
                &owned,
                options,
                &styles,
                p,
                magic_boost_useful,
            )
        })
        .unwrap_or(0);
    let mut potions: Vec<i32> = profile
        .into_iter()
        .flat_map(|p| {
            p.supply_roles
                .iter()
                .flat_map(|r| r.prayer_ids.iter().copied())
        })
        .collect();
    potions.extend([2434, 139, 141, 143, 3024, 3026, 3028, 3030]);
    for _ in supplied_prayer.min(options.prayer_potions.min(12))..options.prayer_potions.min(12) {
        if let Some(id) = potions.iter().find(|id| {
            owned.get(id).copied().unwrap_or(0) > allocated.get(id).copied().unwrap_or(0)
        }) {
            out.inventory.push(Packed {
                id: *id,
                quantity: 1,
                purpose: "Prayer restoration".into(),
            });
            *allocated.entry(*id).or_default() += 1;
        } else {
            out.missing
                .push("Another prayer-restoration potion bottle".into());
            break;
        }
    }
    let mut foods = profile
        .filter(|p| !p.food_ids.is_empty())
        .map(|p| p.food_ids.clone())
        .unwrap_or_else(|| vec![385, 3144, 373, 379]);
    if profile.is_some_and(|p| !p.food_ids.is_empty()) {
        foods.sort_by_key(|id| {
            (
                std::cmp::Reverse(
                    crate::supplies::food_healing(*id, m.level("Hitpoints"))
                        .map(|v| v.0)
                        .unwrap_or(0),
                ),
                *id,
            )
        });
    }
    for _ in 0..options.food.min(20) {
        if let Some(id) = foods.iter().find(|id| {
            owned.get(id).copied().unwrap_or(0) > allocated.get(id).copied().unwrap_or(0)
        }) {
            out.inventory.push(Packed {
                id: *id,
                quantity: 1,
                purpose: "Food".into(),
            });
            *allocated.entry(*id).or_default() += 1;
        } else {
            out.missing
                .push("More supported food for the requested slots".into());
            break;
        }
    }
    for (index, id) in &options.inventory_overrides {
        if *index >= 28 || *id < 0 {
            continue;
        }
        if *index >= out.inventory.len() && *id > 0 {
            out.inventory.resize_with(*index + 1, || Packed {
                id: 0,
                quantity: 0,
                purpose: String::new(),
            });
            out.inventory[*index] = Packed {
                id: *id,
                quantity: 1,
                purpose: "Manual item addition".into(),
            };
            continue;
        }
        if *index < out.inventory.len() {
            if *id == 0 {
                out.inventory[*index].id = 0;
                out.inventory[*index].quantity = 0;
            } else {
                out.inventory[*index] = Packed {
                    id: *id,
                    quantity: 1,
                    purpose: "Manual item replacement".into(),
                };
            }
        }
    }
    for (index, q) in &options.inventory_quantities {
        if let Some(item) = out.inventory.get_mut(*index) {
            if is_stackable(item.id) {
                item.quantity = (*q).clamp(1, i32::MAX as u32);
            }
        }
    }
    while out.inventory.last().is_some_and(|p| p.quantity == 0) {
        out.inventory.pop();
    }
    if let Some(profile) = profile {
        for role in profile.supply_roles.iter().filter(|r| r.required) {
            let packed = out
                .inventory
                .iter()
                .any(|p| p.quantity > 0 && role.ids.contains(&p.id));
            if let Some(check) = out.checks.iter_mut().find(|c| c.name == role.label) {
                check.state = Some(packed);
                if !packed {
                    check.detail =
                        "The edited inventory has no supported poison-protection bottle.".into();
                }
            }
        }
        if !profile.supply_roles.is_empty() {
            out.warnings.push("Spellbook/prayer unlocks, staff charges, special attacks and potion timing need manual confirmation. Packed boosts are not a full-duration trip simulation.".into());
        }
    }
    if let Some(limit) = options.max_switches {
        let sets: Vec<_> = std::iter::once(&out.equipment)
            .chain(out.phases.iter().map(|p| &p.equipment))
            .collect();
        let maximum = sets
            .iter()
            .flat_map(|a| sets.iter().map(move |b| crate::optimization::changes(a, b)))
            .max()
            .unwrap_or(0);
        if maximum > limit {
            out.missing.push(format!(
                "The final phase plan needs {maximum} changed slots; your limit is {limit}."
            ));
        }
        let carried: BTreeSet<_> = out
            .phases
            .iter()
            .flat_map(|p| {
                p.equipment
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, id)| id.filter(|id| out.equipment[slot] != Some(*id)))
            })
            .collect();
        if carried.len() > options.switch_slots {
            out.missing.push(format!(
                "The final phase plan needs {} switch-gear slots; your limit is {}.",
                carried.len(),
                options.switch_slots
            ));
        }
    }
    for (id, q) in essentials {
        if out
            .inventory
            .iter()
            .filter(|p| p.id == id)
            .map(|p| p.quantity as u64)
            .sum::<u64>()
            < q as u64
        {
            out.missing.push(format!(
                "The edited inventory still needs {} × {q} for this trip.",
                item_name(id)
            ));
        }
    }
    // Every transition must fit, including 2h-weapon switches that displace a shield into the bag.
    let mut max_slots = out.inventory.len();
    let mut total = BTreeMap::new();
    for (slot, id) in out.equipment.iter().enumerate() {
        if let Some(id) = id {
            *total.entry(*id).or_insert(0u32) +=
                equipped_quantity(slot, *id, options.combat.ammo_shots);
        }
    }
    for p in &out.inventory {
        *total.entry(p.id).or_insert(0) += p.quantity;
    }
    for weapon in std::iter::once(&out.equipment)
        .chain(out.phases.iter().map(|p| &p.equipment))
        .filter_map(|set| set[3])
    {
        if !crate::rules::weapon(weapon).is_some_and(|r| !r.family.internal_ammo.is_empty()) {
            continue;
        }
        if let Some(dart) = crate::rules::internal_ammo(m, weapon, &options.combat) {
            let q = options.combat.ammo_shots;
            out.loaded_ammo = Some(Packed {
                id: dart,
                quantity: q,
                purpose: "Load before leaving; stored inside the weapon".into(),
            });
            let loaded_ok = options.combat.loaded_ammo_confirmed
                || owned.get(&dart).copied().unwrap_or(0)
                    >= q.saturating_add(total.get(&dart).copied().unwrap_or(0));
            if !loaded_ok {
                out.missing.push(format!("{} × {q} needed to load the blowpipe in addition to carried darts, or confirm its existing contents.",item_name(dart)));
            }
            out.checks.push(Check{name:"Loaded darts".into(),state:Some(loaded_ok),detail:format!("{} × {q}. {} Verify remaining darts and charges; these are not worn in the ammo slot.",item_name(dart),if options.combat.loaded_ammo_confirmed{"Existing contents confirmed by you."}else{"Load from your bank before leaving."})});
        } else {
            out.missing.push(format!(
                "Select compatible blowpipe darts or import at least {} owned darts to load.",
                options.combat.ammo_shots
            ));
        }
        break;
    }
    for (id, quantity) in &total {
        if *quantity > owned.get(id).copied().unwrap_or(0) {
            out.missing.push(format!(
                "Not enough {} for the edited setup.",
                item_name(*id)
            ));
        }
    }
    for phase in &out.phases {
        let mut carried = total.clone();
        for (slot, id) in phase.equipment.iter().enumerate() {
            if let Some(id) = id {
                let q = carried.entry(*id).or_default();
                let need = equipped_quantity(slot, *id, options.combat.ammo_shots);
                if *q < need {
                    out.missing.push(format!(
                        "{} needs {} in the carried pool.",
                        phase.name,
                        item_name(*id)
                    ));
                }
                *q = q.saturating_sub(need);
            }
        }
        let slots: usize = carried
            .iter()
            .map(|(id, q)| {
                if is_stackable(*id) {
                    usize::from(*q > 0)
                } else {
                    *q as usize
                }
            })
            .sum();
        max_slots = max_slots.max(slots);
    }
    if max_slots > 28 {
        out.missing.push(format!("{max_slots} inventory slots needed across gear changes; maximum 28. Reduce switches or supplies."));
    }
    out.warnings.push("Charged item IDs do not reveal remaining charges. Check staff charges, teleport destination and supplies before leaving.".into());
    if m.potion_storage.as_ref().is_some_and(|p| !p.is_empty()) {
        out.warnings.push("Potion storage is imported as doses, not bottles. Withdraw/decant potions into your bank and refresh before packing them.".into());
    }
    out.warnings.push("Armour and accessories use owned equipment stats. Weapon behaviours are reviewed separately. Combat calculations and switch-budget search have the coverage described above.".into());
    out
}

#[derive(Clone, Debug)]
pub struct Goal {
    pub id: &'static str,
    pub title: &'static str,
    pub reason: &'static str,
    pub next: String,
    pub progress: u32,
    pub complete: bool,
    pub link: &'static str,
}
pub fn goals(snapshot: &Snapshot, boss: Boss) -> Vec<Goal> {
    if boss == Boss::Custom {
        return vec![];
    }
    let Some(m) = snapshot.selected() else {
        return vec![];
    };
    let owned = snapshot.available(false);
    let mut result = vec![];
    if boss == Boss::Barrows {
        let hilt = equipment().iter().any(|g| {
            g.name
                .strip_prefix("Ghommal's hilt ")
                .and_then(|n| n.parse::<u32>().ok())
                .is_some_and(|n| n >= 2)
                && owned.contains_key(&g.id)
        });
        result.push(Goal{id:"barrows-hilt",title:"Stop the Barrows prayer drain",reason:"Ghommal's hilt 2 or higher prevents the Barrows area prayer drain while wielded.",next:if hilt{"Bring your hilt and wield it for the drain.".into()}else{"Complete the medium Combat Achievements and claim Ghommal's hilt 2.".into()},progress:if hilt{100}else{0},complete:hilt,link:"https://oldschool.runescape.wiki/w/Ghommal%27s_hilt_2"});
        let magic = m.level("Magic");
        result.push(Goal {
            id: "barrows-magic",
            title: "Unlock Wind Wave",
            reason: "An additional air-spell option for Barrows. Keep its blood-rune use in mind.",
            next: if magic.is_some_and(|l| l >= 62) {
                "62 Magic reached. Bring blood runes and an air staff to cast Wind Wave.".into()
            } else {
                "Train Magic to 62 for Wind Wave; gather blood runes for your trip.".into()
            },
            progress: magic.unwrap_or(0).min(62) * 100 / 62,
            complete: magic.is_some_and(|l| l >= 62),
            link: "https://oldschool.runescape.wiki/w/Wind_Wave",
        });
    } else {
        let charged = [12899, 22292, 11905, 11907, 22288, 28585]
            .iter()
            .any(|id| owned.contains_key(id) && can_use(*id, m));
        result.push(Goal{id:"toa-magic",title:"Bring a powered staff",reason:"Akkha and Wardens require a magic option. This starter plan supports charged tridents and the warped sceptre.",next:if charged{"A usable powered staff is available. Check its remaining charges.".into()}else{"Obtain and charge a supported powered staff, and meet its Magic requirement.".into()},progress:if charged{100}else{0},complete:charged,link:"https://oldschool.runescape.wiki/w/Tombs_of_Amascut/Strategies"});
        let stab = [26219, 27246, 11889]
            .iter()
            .any(|id| owned.contains_key(id) && can_use(*id, m));
        result.push(Goal{id:"toa-stab",title:"Improve your ToA stab weapon",reason:"A Zamorakian hasta or Osmumten's fang gives this starter catalog an upgrade over its basic partisan route.",next:if stab{"A supported upgrade is available; compare it in your equipment plan.".into()}else{"Work toward a Zamorakian spear and the Barbarian Training needed to turn it into a hasta. A fang is another route.".into()},progress:if stab{100}else{0},complete:stab,link:"https://oldschool.runescape.wiki/w/Zamorakian_hasta"});
    }
    result
}

pub fn raw_item_count(m: &Member) -> usize {
    let mut pairs = BTreeMap::new();
    for c in [&m.bank, &m.inventory, &m.equipment].into_iter().flatten() {
        crate::account::add_pairs(&mut pairs, c);
    }
    pairs.len()
}
/// Projectiles use the selected stack budget; nonstackable ammo-slot items
/// (such as blessings) are single pieces of equipment.
pub fn equipped_quantity(slot: usize, id: i32, ammo_shots: u32) -> u32 {
    if slot == 13 && is_stackable(id) {
        ammo_shots
    } else {
        1
    }
}
pub fn is_stackable(id: i32) -> bool {
    if crate::rules::all_ammo_ids().contains(&id)
        || crate::rules::all_internal_ammo_ids().contains(&id)
    {
        return true;
    }
    if crate::inventory::public_name(id)
        .is_some_and(|name| name.ends_with(" rune") || name.ends_with(" runes"))
    {
        return true;
    }
    matches!(
        id,
        556 | 554 | 560 | 565 | 562 | 563 | 9242 | 9243 | 9144 | 9143 | 9142 | 9141 | 892 | 890
    )
}
pub fn equipment_names() -> BTreeMap<i32, String> {
    equipment().iter().map(|g| (g.id, g.name.clone())).collect()
}
pub fn data_version() -> Value {
    serde_json::json!({"catalog":"starter-2026-11-09","selection":"owned equipment, supported DPS refinement and bounded switch search","dps_simulation":false})
}
