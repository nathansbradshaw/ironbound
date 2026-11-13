//! Local gear-budget screening; this is not an Items Kept on Death calculator.
use crate::{
    account::Snapshot,
    bestiary::Readiness,
    combat,
    encounters::Entry,
    planner::{self, Boss, Options},
    rules,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
pub const CAPS: [u32; 4] = [1_000, 10_000, 25_000, 50_000];
pub const DEFAULT_CAP: u32 = 10_000;
pub const CAUTION: &str = "Three valuable gear pieces, plus disposable fillers. Assumes unskulled, a normal world and PK Skull Prevention enabled; no fourth item from Protect Item. Confirm Items Kept on Death in game. Supplies, ammunition, runes, charges, entry fees and loot can add risk; this is not a death-loss guarantee.";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exposure {
    None,
    Route,
    Combat,
}
pub fn exposure(name: &str) -> Exposure {
    match name {
        "King Black Dragon" => Exposure::Route,
        "Artio"
        | "Callisto"
        | "Calvar'ion"
        | "Vet'ion"
        | "Spindel"
        | "Venenatis"
        | "Chaos Elemental"
        | "Chaos Fanatic"
        | "Crazy Archaeologist"
        | "Scorpia" => Exposure::Combat,
        _ => Exposure::None,
    }
}
#[derive(Deserialize)]
pub struct Prices {
    pub captured_at: String,
    pub prices: BTreeMap<i32, u64>,
}
pub fn prices() -> &'static Prices {
    static DATA: OnceLock<Prices> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/wilderness-prices.json"))
            .expect("public gear prices")
    })
}
/// Missing prices and untradeable gear always consume a valuable-item slot.
/// Ammunition is priced as the entire configured stack, not one projectile.
pub fn disposable(id: i32, quantity: u32, cap: u32) -> bool {
    prices().prices.get(&id).is_some_and(|price| {
        *price > 0 && price.saturating_mul(u64::from(quantity)) <= u64::from(cap)
    })
}
pub fn gear_quantity(id: i32, shots: u32) -> u32 {
    if planner::gear(id).is_some_and(|g| g.slot == "ammo") {
        shots
    } else {
        1
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Kit {
    pub allowed: BTreeSet<i32>,
    pub filler_cap: u32,
    pub screened_shots: u32,
}
impl Kit {
    pub fn valid(&self) -> bool {
        CAPS.contains(&self.filler_cap)
            && (1..=10_000).contains(&self.screened_shots)
            && self.allowed.len() <= 14
            && self.allowed.iter().all(|id| planner::gear(*id).is_some())
            && self
                .allowed
                .iter()
                .filter(|id| {
                    !disposable(
                        **id,
                        gear_quantity(**id, self.screened_shots),
                        self.filler_cap,
                    )
                })
                .count()
                <= 3
    }
    pub fn from_set(set: &[Option<i32>], filler_cap: u32) -> Self {
        Self {
            allowed: set.iter().flatten().copied().collect(),
            filler_cap,
            screened_shots: crate::rules::DEFAULT_LOADED_SHOTS,
        }
    }
    pub fn permits(&self, id: i32) -> bool {
        planner::gear(id).is_none() || self.allowed.contains(&id)
    }
}
#[derive(Clone, Debug)]
pub struct Preview {
    pub kit: Kit,
    pub equipment: Vec<Option<i32>>,
    pub style: String,
    pub dps: f64,
    pub readiness: Readiness,
    pub full_readiness: Readiness,
}
pub fn neutral(preferences: &Options) -> Options {
    Options {
        boss: Boss::Custom,
        shared: preferences.shared,
        excluded: preferences.excluded.clone(),
        exclude_void: preferences.exclude_void,
        recommended: Some(false),
        ..Default::default()
    }
}
/// Enumerate at most C(11,3) valuable-piece subsets from each owned baseline,
/// filling the other slots with owned disposable gear. This is a bounded screen.
pub fn preview(
    snapshot: &Snapshot,
    preferences: &Options,
    entry: &Entry,
    cap: u32,
    full: Readiness,
) -> Option<Preview> {
    if exposure(&entry.name) == Exposure::None
        || !CAPS.contains(&cap)
        || !planner::missing_combat_levels(snapshot).is_empty()
        || entry.targets.is_empty()
    {
        return None;
    }
    let options = neutral(preferences);
    let mut owned = snapshot.available(options.shared);
    owned.retain(|id, _| !options.item_excluded(*id));
    let member = planner::member_with_stock(snapshot.selected()?, &owned);
    let mut cheap = options.clone();
    cheap.excluded.extend(
        planner::equipment()
            .iter()
            .filter(|g| !disposable(g.id, gear_quantity(g.id, options.combat.ammo_shots), cap))
            .map(|g| g.id),
    );
    let mut best: Option<Preview> = None;
    for style in ["Stab", "Slash", "Crush", "Ranged", "Magic"] {
        let full_set = planner::equipment_for_style(snapshot, &options, style);
        let cheap_set = planner::equipment_for_style(snapshot, &cheap, style);
        let candidates: Vec<_> = full_set
            .iter()
            .enumerate()
            .filter_map(|(slot, id)| {
                id.filter(|id| !disposable(*id, gear_quantity(*id, options.combat.ammo_shots), cap))
                    .map(|id| (slot, id))
            })
            .collect();
        for mask in 0u32..(1u32 << candidates.len()) {
            if mask.count_ones() > 3 {
                continue;
            }
            let mut set = cheap_set.clone();
            for (index, (slot, id)) in candidates.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    set[*slot] = Some(*id);
                }
            }
            let Some(weapon) = set[3] else {
                continue;
            };
            if planner::gear(weapon).is_some_and(|g| g.two_handed) {
                set[5] = None;
            }
            if style == "Ranged" {
                let ammo = rules::ammo_ids(weapon);
                if !ammo.is_empty() {
                    if !set[13].is_some_and(|id| ammo.contains(&id)) {
                        set[13] = ammo.iter().copied().find(|id| {
                            owned.get(id).copied().unwrap_or(0) >= options.combat.ammo_shots
                                && planner::can_use(*id, &member)
                                && disposable(*id, options.combat.ammo_shots, cap)
                        });
                    }
                    if set[13].is_none() {
                        continue;
                    }
                } else {
                    set[13] = None;
                }
            } else {
                set[13] = None;
            }
            let kit = Kit::from_set(&set, cap);
            if !kit.valid() {
                continue;
            }
            let mut dps = f64::INFINITY;
            let mut supported = true;
            for target in &entry.targets {
                let Some(monster) = planner::monster(target.id) else {
                    supported = false;
                    break;
                };
                let spell = rules::spell_name_for_target(
                    &member,
                    &set,
                    &owned,
                    &options.combat,
                    monster,
                    0,
                );
                if style == "Magic"
                    && rules::spell_requirement(&member, &set, &owned, &options.combat, spell)
                        .is_some()
                {
                    supported = false;
                    break;
                }
                match combat::calculate_configured(
                    &member,
                    &set,
                    style,
                    monster,
                    spell,
                    0,
                    &options.combat,
                ) {
                    Ok(result) if result.dps.is_finite() => dps = dps.min(result.dps),
                    _ => {
                        supported = false;
                        break;
                    }
                }
            }
            if !supported || !dps.is_finite() || dps <= 0.0 {
                continue;
            }
            let Some(readiness) =
                crate::bestiary::rate_setup(&member, &entry.name, style, &set, dps)
            else {
                continue;
            };
            if best.as_ref().is_none_or(|old| {
                readiness < old.readiness
                    || readiness == old.readiness
                        && (dps > old.dps + 1e-8
                            || (dps - old.dps).abs() < 1e-8
                                && defence_score(&set) > defence_score(&old.equipment))
            }) {
                best = Some(Preview {
                    kit,
                    equipment: set,
                    style: style.into(),
                    dps,
                    readiness,
                    full_readiness: full,
                });
            }
        }
    }
    if let Some(p) = &mut best {
        p.readiness =
            crate::bestiary::rate_setup(&member, &entry.name, &p.style, &p.equipment, p.dps)?;
    }
    best
}
fn defence_score(set: &[Option<i32>]) -> i32 {
    combat::bonuses(set).defensive.values().sum()
}
/// Lock the actual planner to the screened set and remove previous trip edits.
pub fn apply(options: &mut Options, entry: &Entry, preview: &Preview) {
    options.wilderness = Some(preview.kit.clone());
    options.boss = Boss::Custom;
    options.targets = entry
        .targets
        .iter()
        .map(|t| planner::Target {
            id: t.id,
            style: preview.style.clone(),
        })
        .collect();
    options.combat = Default::default();
    options.recommended = Some(false);
    options.gear_overrides.clear();
    for style in rules::STYLES {
        for (slot, id) in preview.equipment.iter().enumerate() {
            options
                .gear_overrides
                .insert(format!("{style}:{slot}"), id.unwrap_or(0));
        }
    }
    options.inventory_overrides.clear();
    options.inventory_quantities.clear();
    options.confirmations.clear();
}
/// Check real carried equipment quantities, including manually added duplicates.
/// Non-equipment supplies are intentionally outside this gear-only constraint.
pub fn violations(plan: &planner::Plan, kit: &Kit) -> Vec<String> {
    let mut carried = BTreeMap::<i32, u32>::new();
    for (slot, id) in plan.equipment.iter().enumerate() {
        if let Some(id) = id {
            *carried.entry(*id).or_default() +=
                planner::equipped_quantity(slot, *id, plan.ammo_shots);
        }
    }
    for item in &plan.inventory {
        if item.quantity > 0 && planner::gear(item.id).is_some() {
            let q = carried.entry(item.id).or_default();
            *q = q.saturating_add(item.quantity);
        }
    }
    let mut valuable = 0u32;
    let mut errors = vec![];
    for (id, quantity) in carried {
        if !kit.allowed.contains(&id) {
            errors.push(format!(
                "{} is outside the selected wilderness kit.",
                planner::item_name(id)
            ));
        }
        if !disposable(id, quantity, kit.filler_cap) {
            valuable = valuable.saturating_add(if planner::is_stackable(id) {
                1
            } else {
                quantity
            });
        }
    }
    if valuable > 3 {
        errors.push(format!(
            "Carried equipment uses {valuable} valuable pieces/stacks; the limit is 3."
        ));
    }
    errors
}
