use crate::account::{Member, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SLOTS: [&str; 14] = [
    "Head", "Cape", "Neck", "Weapon", "Body", "Shield", "", "Legs", "", "Hands", "Feet", "",
    "Ring", "Ammo",
];
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gear {
    pub id: i32,
    pub name: String,
    pub slot: String,
    #[serde(default, rename = "isTwoHanded")]
    pub two_handed: bool,
}
pub fn equipment() -> &'static [Gear] {
    static ITEMS: std::sync::OnceLock<Vec<Gear>> = std::sync::OnceLock::new();
    ITEMS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/equipment.json")).expect("bundled equipment")
    })
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
        1359 => "Rune pickaxe",
        _ => {
            return equipment()
                .iter()
                .find(|g| g.id == id)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| format!("Item {id}"));
        }
    }
    .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Boss {
    #[default]
    Barrows,
    Toa,
}
impl Boss {
    pub fn name(self) -> &'static str {
        match self {
            Self::Barrows => "Barrows",
            Self::Toa => "Tombs of Amascut",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Barrows => "barrows",
            Self::Toa => "toa",
        }
    }
    pub fn source(self) -> &'static str {
        match self {
            Self::Barrows => "https://oldschool.runescape.wiki/w/Barrows/Strategies",
            Self::Toa => "https://oldschool.runescape.wiki/w/Tombs_of_Amascut/Strategies",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub boss: Boss,
    pub shared: bool,
    pub fewer_switches: bool,
    pub food: usize,
    pub prayer_potions: usize,
    pub excluded: BTreeSet<i32>,
    pub access_override: Option<bool>,
    pub started_barrows: Option<bool>,
    pub prayer_unlocks: bool,
    pub raid_level: u32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            boss: Boss::Barrows,
            shared: false,
            fewer_switches: false,
            food: 8,
            prayer_potions: 3,
            excluded: BTreeSet::new(),
            access_override: None,
            started_barrows: None,
            prayer_unlocks: false,
            raid_level: 0,
        }
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
    pub fn exportable(&self) -> bool {
        self.missing.is_empty()
            && self.inventory.len() <= 28
            && self.checks.iter().all(|c| c.state == Some(true))
            && self.equipment[3].is_some()
    }
    pub fn status(&self) -> &'static str {
        if self.checks.iter().any(|c| c.state == Some(false)) || !self.missing.is_empty() {
            "Preparation needed"
        } else if self.checks.iter().any(|c| c.state.is_none()) {
            "Confirm your access"
        } else {
            "Meets starter guidelines"
        }
    }
}

fn can_use(id: i32, m: &Member) -> bool {
    let req: Vec<(&str, u32)> = match id {
        26219 | 27246 => vec![("Attack", 82)],
        11889 => vec![("Attack", 70)],
        21009 | 4587 | 1215 => vec![("Attack", 60)],
        25979 | 25981 | 27291 => vec![("Attack", 65)],
        11905 | 11907 | 22288 => vec![("Magic", 75)],
        12899 | 22292 => vec![("Magic", 78)],
        28585 => vec![("Magic", 62)],
        12658 | 1409 => vec![("Magic", 50), ("Attack", 50)],
        9185 | 26486 => vec![("Ranged", 61)],
        21902 => vec![("Ranged", 65)],
        11785 => vec![("Ranged", 70)],
        26374 => vec![("Ranged", 80)],
        861 | 12788 => vec![("Ranged", 50)],
        10828 => vec![("Defence", 55)],
        24271 => vec![("Defence", 70)],
        11665 | 11664 | 11663 | 8839 | 8840 | 8842 => vec![("Defence", 42)],
        4712 | 4714 | 4708 | 4856 | 4857 | 4858 | 4859 | 4868 | 4869 | 4870 | 4871 | 4874
        | 4875 | 4876 | 4877 => vec![("Magic", 70), ("Defence", 70)],
        4091 | 4093 | 4099 | 4101 | 4103 | 4109 | 4111 | 4113 => {
            vec![("Magic", 40), ("Defence", 20)]
        }
        2503 | 2497 => vec![("Ranged", 70), ("Defence", 40)],
        2499 | 2495 => vec![("Ranged", 60), ("Defence", 40)],
        1135 | 1099 => vec![("Ranged", 40)],
        11832 | 11834 => vec![("Defence", 65)],
        1127 | 1079 | 1201 => vec![("Defence", 40)],
        4722 => vec![("Defence", 70)],
        11840 => vec![("Defence", 60)],
        13239 | 13237 | 13235 => vec![("Defence", 75)],
        6920 => vec![("Magic", 60), ("Defence", 40)],
        19544 | 19547 | 19553 => vec![("Hitpoints", 75)],
        2491 => vec![("Ranged", 70)],
        10551 => vec![("Defence", 40)],
        12954 | 19722 => vec![("Attack", 60), ("Defence", 60)],
        29280 | 29283 | 29286 => vec![("Ranged", 60), ("Defence", 50)],
        _ => vec![],
    };
    req.iter().all(|(s, l)| m.level(s).is_some_and(|v| v >= *l))
}

// Reviewed, finite item priorities. This is a starter planner, not an exact-DPS optimiser.
fn candidates(style: &str, slot: usize, boss: Boss) -> &'static [i32] {
    match (style, slot) {
        ("Magic", 3) if boss == Boss::Barrows => {
            &[1381, 1405, 11905, 11907, 12899, 22288, 28585, 12658, 1409]
        }
        ("Magic", 3) => &[12899, 22292, 11905, 11907, 22288, 28585],
        ("Ranged", 3) => &[26374, 11785, 21902, 9185, 26486, 12788, 861],
        ("Melee", 3) => &[26219, 27246, 11889, 21009, 25981, 25979, 27291],
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
        ("Ranged", 4) => &[2503, 2499, 1135, 29280],
        ("Ranged", 7) => &[2497, 2495, 1099, 29283],
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
    }
}
pub fn catalog_ids() -> BTreeSet<i32> {
    [Boss::Barrows, Boss::Toa]
        .into_iter()
        .flat_map(|b| {
            ["Magic", "Ranged", "Melee"].into_iter().flat_map(move |s| {
                (0..14).flat_map(move |slot| candidates(s, slot, b).iter().copied())
            })
        })
        .collect()
}

fn style_set(
    style: &str,
    boss: Boss,
    m: &Member,
    owned: &BTreeMap<i32, u32>,
    catalog: &[Gear],
) -> Vec<Option<i32>> {
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
    if let Some(w) = set[3] {
        if catalog.iter().any(|g| g.id == w && g.two_handed) {
            set[5] = None;
        }
    }
    if style == "Ranged" {
        let bow = matches!(set[3], Some(861 | 12788));
        set[13] = if bow {
            &[892, 890][..]
        } else if boss == Boss::Toa {
            &[9242, 9243, 9144, 9143, 9142][..]
        } else {
            &[9144, 9143, 9142, 9243, 9242][..]
        }
        .iter()
        .copied()
        .find(|id| owned.get(id).copied().unwrap_or(0) >= 100);
    }
    set
}

pub fn plan(snapshot: &Snapshot, options: &Options) -> Plan {
    let mut out = Plan {
        equipment: vec![None; 14],
        inventory: vec![],
        phases: vec![],
        checks: vec![],
        warnings: vec![],
        missing: vec![],
        switches: 0,
        supported: 0,
        owned: 0,
        label: "Reviewed starter priorities".into(),
        spell: String::new(),
    };
    let Some(m) = snapshot.selected() else {
        out.missing.push("Select an account first.".into());
        return out;
    };
    let mut owned = snapshot.available(options.shared);
    for id in &options.excluded {
        owned.remove(id);
    }
    let catalog = equipment();
    out.owned = owned.len();
    out.supported = owned
        .keys()
        .filter(|id| {
            ["Magic", "Ranged", "Melee"]
                .iter()
                .any(|s| (0..14).any(|slot| candidates(s, slot, options.boss).contains(id)))
        })
        .count();
    let access = options.access_override.or_else(|| {
        m.quest(if options.boss == Boss::Barrows {
            "Priest in Peril"
        } else {
            "Beneath Cursed Sands"
        })
        .map(|s| s == 2)
    });
    out.checks.push(Check {
        name: if options.boss == Boss::Barrows {
            "Priest in Peril"
        } else {
            "Beneath Cursed Sands"
        }
        .into(),
        state: access,
        detail: "Quest access requirement".into(),
    });
    if options.boss == Boss::Barrows {
        out.checks.push(Check {
            name: "His Faithful Servants started".into(),
            state: options
                .started_barrows
                .or_else(|| m.quest("His Faithful Servants").map(|s| s != 1)),
            detail: "Start the miniquest to dig into the crypts".into(),
        });
    }
    for (skill, minimum) in if options.boss == Boss::Barrows {
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
    let primary = if options.boss == Boss::Barrows {
        "Magic"
    } else {
        "Melee"
    };
    out.equipment = style_set(primary, options.boss, m, &owned, &catalog);
    let ranged = style_set("Ranged", options.boss, m, &owned, &catalog);
    let magic = style_set("Magic", options.boss, m, &owned, &catalog);
    if out.equipment[3].is_none() {
        out.missing
            .push(format!("A supported, usable {primary} weapon"));
    }
    if ranged[3].is_none() {
        out.missing.push("A supported ranged weapon".into());
    }
    if ranged[13].is_none() {
        out.missing
            .push("At least 100 compatible bolts or arrows".into());
    }
    if options.boss == Boss::Toa && magic[3].is_none() {
        out.missing
            .push("A charged powered staff usable at your Magic level".into());
    }
    let mut sets = vec![("Ranged", ranged.clone())];
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
    let ranged = sets[0].1.clone();
    let magic = sets
        .iter()
        .find(|(s, _)| *s == "Magic")
        .map(|(_, s)| s.clone())
        .unwrap_or(magic);
    let mut allocated: BTreeMap<i32, u32> = BTreeMap::new();
    for id in out.equipment.iter().flatten() {
        *allocated.entry(*id).or_default() += 1;
    }
    for (style, set) in &sets {
        for (slot, id) in set.iter().enumerate() {
            if let Some(id) = id {
                if out.equipment[slot] != Some(*id) && !out.inventory.iter().any(|p| p.id == *id) {
                    out.inventory.push(Packed {
                        id: *id,
                        quantity: if slot == 13 { 100 } else { 1 },
                        purpose: format!("{style} switch"),
                    });
                    *allocated.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
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
        for (ids, purpose) in [
            (&[25981, 25979, 27291][..], "Kephri weapon"),
            (&[1215][..], "Core special attack"),
            (&[1359, 1275, 1271, 1269][..], "Het puzzle pickaxe"),
        ] {
            if let Some(id) = ids
                .iter()
                .find(|id| owned.contains_key(id) && can_use(**id, m))
            {
                if !allocated.contains_key(id) {
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
        allocated.insert(id, 100);
    }
    let mut reserve = |id: i32, q: u32, purpose: &str, required: bool| {
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
        if matches!(out.equipment[3], Some(1381 | 1405)) {
            let wave =
                m.level("Magic").unwrap_or(1) >= 62 && owned.get(&565).copied().unwrap_or(0) >= 150;
            out.spell = if wave { "Wind Wave" } else { "Wind Blast" }.into();
            reserve(
                if wave { 565 } else { 560 },
                150,
                "150 spell casts · editable trip assumption",
                true,
            );
        } else if matches!(out.equipment[3], Some(12658 | 1409)) {
            out.spell = "Iban Blast".into();
            reserve(560, 150, "150 spell casts", true);
            reserve(554, 750, "150 spell casts", true);
        } else {
            out.spell = "Built-in spell".into();
        }
    }
    reserve(8007, 1, "Exit teleport · requires your POH", true);
    if owned.contains_key(&12625) {
        reserve(12625, 1, "Stamina", false);
    }
    drop(reserve);
    let potions = [2434, 139, 141, 143, 3024, 3026, 3028, 3030];
    for _ in 0..options.prayer_potions.min(12) {
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
    for _ in 0..options.food.min(20) {
        if let Some(id) = [385, 3144, 373, 379].iter().find(|id| {
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
    // Every transition must fit, including 2h-weapon switches that displace a shield into the bag.
    let mut max_slots = out.inventory.len();
    let mut total = BTreeMap::new();
    for (slot, id) in out.equipment.iter().enumerate() {
        if let Some(id) = id {
            *total.entry(*id).or_insert(0u32) += if slot == 13 { 100 } else { 1 };
        }
    }
    for p in &out.inventory {
        *total.entry(p.id).or_insert(0) += p.quantity;
    }
    for phase in &out.phases {
        let mut carried = total.clone();
        for (slot, id) in phase.equipment.iter().enumerate() {
            if let Some(id) = id {
                let q = carried.entry(*id).or_default();
                *q = q.saturating_sub(if slot == 13 { 100 } else { 1 });
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
    out.warnings.push("Equipment is selected from a reviewed starter catalog. Set-effect optimisation and exact DPS comparisons are not included.".into());
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
pub fn goals(m: &Member, boss: Boss) -> Vec<Goal> {
    let mut result = vec![];
    for (id, title, skill, target, reason, link) in [
        (
            "prayer",
            "Build your Prayer foundation",
            "Prayer",
            70,
            "A useful preparation milestone for longer PvM goals. Piety also requires King's Ransom and Knight Waves.",
            "https://oldschool.runescape.wiki/w/Piety",
        ),
        (
            "herblore",
            "Make your own super restores",
            "Herblore",
            63,
            "Unlock the potion recipe; you still need snapdragons and red spiders' eggs.",
            "https://oldschool.runescape.wiki/w/Super_restore",
        ),
        (
            "magic",
            "Unlock a trident loadout",
            "Magic",
            75,
            "Meet the Magic requirement for a trident of the seas. Obtain and charge the staff separately.",
            "https://oldschool.runescape.wiki/w/Trident_of_the_seas",
        ),
    ] {
        let current = m.level(skill);
        let done = current.is_some_and(|v| v >= target);
        result.push(Goal {
            id,
            title,
            reason,
            next: match current {
                Some(_) if done => format!(
                    "{target} {skill} reached. Check remaining item and quest requirements."
                ),
                Some(v) => format!("Train {skill} from {v} to {target}."),
                None => format!("Import {skill} XP to evaluate this goal."),
            },
            progress: current.map(|v| v.min(target) * 100 / target).unwrap_or(0),
            complete: done,
            link,
        });
    }
    let name = if boss == Boss::Barrows {
        "Priest in Peril"
    } else {
        "Beneath Cursed Sands"
    };
    let done = m.quest(name) == Some(2);
    result.insert(0,Goal{id:"access",title:if boss==Boss::Barrows{"Open the road to Barrows"}else{"Open the Tombs of Amascut"},reason:"Encounter access is a hard requirement, separate from suggested combat levels.",next:if done{format!("{name} complete. Review the encounter preparation checklist.")}else{format!("Confirm or complete {name}; use the Wiki for its prerequisite chain.")},progress:if done{100}else{0},complete:done,link:if boss==Boss::Barrows{"https://oldschool.runescape.wiki/w/Priest_in_Peril"}else{"https://oldschool.runescape.wiki/w/Beneath_Cursed_Sands"}});
    result
}

pub fn raw_item_count(m: &Member) -> usize {
    let mut pairs = BTreeMap::new();
    for c in [&m.bank, &m.inventory, &m.equipment].into_iter().flatten() {
        crate::account::add_pairs(&mut pairs, c);
    }
    pairs.len()
}
pub fn is_stackable(id: i32) -> bool {
    matches!(
        id,
        556 | 554 | 560 | 565 | 562 | 563 | 9242 | 9243 | 9144 | 9143 | 9142 | 9141 | 892 | 890
    )
}
pub fn equipment_names() -> BTreeMap<i32, String> {
    equipment().iter().map(|g| (g.id, g.name.clone())).collect()
}
pub fn data_version() -> Value {
    serde_json::json!({"catalog":"starter-2026-11-09","selection":"reviewed item priorities","dps_simulation":false})
}
