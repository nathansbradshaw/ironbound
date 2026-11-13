//! Item IDs are data. Calculation code implements reusable mechanics, not individual weapons.
use crate::account::Member;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STYLES: [&str; 6] = ["Melee", "Stab", "Slash", "Crush", "Ranged", "Magic"];
pub const MAX_TARGETS: usize = 64;
pub fn base_style(style: &str) -> &str {
    if ["Stab", "Slash", "Crush"].contains(&style) {
        "Melee"
    } else {
        style
    }
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Formula {
    Standard,
    #[serde(alias = "wind")]
    Elemental,
    Fixed,
    Powered,
    Spell,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    KalphiteDamage,
    KalphiteAccuracy,
    Fang,
    Shadow,
    Zaryte,
    Demonbane,
    DemonbaneSpell,
    Scythe,
    Soulreaper,
    Macuahuitl,
    StrengthRanged,
    TwinHits,
    ObsidianWeapon,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcOperation {
    Add,
    Multiply,
}
#[derive(Debug, Deserialize)]
pub struct HitProc {
    pub numerator: u32,
    pub denominator: u32,
    pub amount: i32,
    pub operation: ProcOperation,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub before_accurate_zero: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    #[default]
    Accurate,
    Aggressive,
    Controlled,
}
impl Stance {
    pub fn attack_bonus(self) -> i64 {
        match self {
            Self::Accurate => 3,
            Self::Controlled => 1,
            Self::Aggressive => 0,
        }
    }
    pub fn strength_bonus(self) -> i64 {
        match self {
            Self::Accurate => 0,
            Self::Controlled => 1,
            Self::Aggressive => 3,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct ContextAdjustment {
    pub context: String,
    #[serde(default)]
    pub offensive: BTreeMap<String, i32>,
    #[serde(default)]
    pub bonuses: BTreeMap<String, i32>,
}
#[derive(Debug, Deserialize)]
pub struct Multiplier {
    pub attribute: String,
    pub numerator: i64,
    pub denominator: i64,
    #[serde(default)]
    pub accuracy: bool,
    #[serde(default)]
    pub damage: bool,
    #[serde(default)]
    pub additive_with_task_damage: bool,
}
/// Add equipped-piece weights before applying a single floored multiplier.
#[derive(Debug, Deserialize)]
pub struct EquipmentScaling {
    pub effect: String,
    pub accuracy_denominator: i64,
    pub damage_denominator: i64,
}
#[derive(Debug, Deserialize)]
pub struct Family {
    #[serde(default)]
    pub internal_ammo: Vec<i32>,
    #[serde(default)]
    pub target_scaling: Option<crate::effects::TargetScaling>,
    pub style: String,
    pub attack_types: Vec<String>,
    #[serde(default)]
    pub stances: BTreeMap<String, Stance>,
    #[serde(default)]
    pub procs: Vec<HitProc>,
    pub formula: Formula,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub equipment_scaling: Vec<EquipmentScaling>,
    #[serde(default)]
    pub ammo: Vec<i32>,
    #[serde(default)]
    pub rapid: bool,
    #[serde(default)]
    pub numerator: i64,
    #[serde(default = "one")]
    pub denominator: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub offset_before_division: bool,
    #[serde(default)]
    pub accuracy_bonus: i64,
    #[serde(default)]
    pub ticks: Option<i32>,
}
fn one() -> i64 {
    1
}
#[derive(Debug, Deserialize)]
pub struct Weapon {
    pub ids: Vec<i32>,
    pub family: String,
    #[serde(default)]
    pub alternate_families: Vec<String>,
    #[serde(default)]
    pub spell_families: Vec<String>,
    #[serde(default)]
    pub supplied_runes: Vec<i32>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    pub requirements: BTreeMap<String, u32>,
    #[serde(default)]
    pub adjustments: Vec<ContextAdjustment>,
    #[serde(default)]
    pub multipliers: Vec<Multiplier>,
    #[serde(default)]
    pub kalphite_damage_percent: Option<i32>,
    #[serde(default)]
    pub demonbane_percent: Option<i64>,
    #[serde(default)]
    pub demonbane_spell_factor: Option<i64>,
}
#[derive(Debug, Deserialize)]
pub struct SetPiece {
    pub slot: usize,
    pub ids: Vec<i32>,
}
#[derive(Debug, Deserialize)]
pub struct EquipmentSet {
    pub effect: String,
    pub pieces: Vec<SetPiece>,
    #[serde(default)]
    pub search_style: Option<String>,
    #[serde(default)]
    pub partial: bool,
    #[serde(default)]
    pub weapon_families: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct EquipmentEffect {
    pub ids: Vec<i32>,
    pub effect: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub weight: i64,
    #[serde(default)]
    pub slot: Option<usize>,
}
#[derive(Debug, Deserialize)]
pub struct EquipmentRequirements {
    pub ids: Vec<i32>,
    pub requirements: BTreeMap<String, u32>,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmmoEffect {
    None,
    Ruby,
    Diamond,
    Opal,
    Pearl,
    Dragonstone,
    Onyx,
    Unsupported,
}
#[derive(Debug, Deserialize)]
pub struct Ammunition {
    pub ids: Vec<i32>,
    pub effect: AmmoEffect,
    #[serde(default)]
    pub proc_chance: f64,
}
#[derive(Debug, Deserialize)]
pub struct ElementalFocus {
    pub ids: Vec<i32>,
    pub element: String,
    pub rune: i32,
    pub damage_percent: i64,
    pub accuracy_percent: i64,
}
#[derive(Debug, Deserialize)]
pub struct Registry {
    pub elemental_focuses: Vec<ElementalFocus>,
    #[serde(default)]
    pub target_bonuses: Vec<crate::effects::TargetBonus>,
    pub families: BTreeMap<String, Family>,
    pub weapons: Vec<Weapon>,
    pub ammunition: Vec<Ammunition>,
    #[serde(default)]
    pub equipment_effects: Vec<EquipmentEffect>,
    #[serde(default)]
    pub equipment_requirements: Vec<EquipmentRequirements>,
    #[serde(default)]
    pub equipment_sets: Vec<EquipmentSet>,
}
impl Registry {
    pub fn validate(&self) -> Result<(), String> {
        let mut focuses = std::collections::BTreeSet::new();
        if self.elemental_focuses.iter().any(|f| {
            f.ids.is_empty()
                || f.ids.iter().any(|id| *id <= 0 || !focuses.insert(*id))
                || !["air", "water", "earth", "fire"].contains(&f.element.as_str())
                || f.rune <= 0
                || !(0..=100).contains(&f.damage_percent)
                || !(0..=100).contains(&f.accuracy_percent)
        }) {
            return Err("Invalid elemental focus".into());
        }
        if self.target_bonuses.iter().any(|b| {
            !b.valid()
                || !self
                    .equipment_effects
                    .iter()
                    .any(|e| e.effect == b.equipment_effect)
        }) {
            return Err("Invalid target-bonus condition or factor".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for (name, family) in &self.families {
            if !["Melee", "Ranged", "Magic"].contains(&family.style.as_str())
                || family.attack_types.is_empty()
                || (family.style == "Ranged"
                    && (family.attack_types.len() != 1
                        || !["light", "standard", "heavy"]
                            .contains(&family.attack_types[0].as_str())))
                || family.denominator <= 0
                || family.ticks.is_some_and(|n| n <= 0)
                || (!family.internal_ammo.is_empty()
                    && (!family.ammo.is_empty()
                        || family.style != "Ranged"
                        || family.internal_ammo.iter().any(|id| *id <= 0)))
                || family.target_scaling.as_ref().is_some_and(|s| !s.valid())
            {
                return Err(format!("Invalid weapon family {name}"));
            }
            for kind in family.stances.keys() {
                if !family.attack_types.contains(kind) {
                    return Err(format!("Unknown stance attack type in {name}"));
                }
            }
            for scaling in &family.equipment_scaling {
                if scaling.effect.is_empty()
                    || scaling.accuracy_denominator <= 0
                    || scaling.damage_denominator <= 0
                    || !self
                        .equipment_effects
                        .iter()
                        .any(|e| e.effect == scaling.effect)
                {
                    return Err(format!("Invalid equipment scaling in {name}"));
                }
            }
            if family.procs.len() > 8
                || family.procs.iter().any(|p| {
                    p.denominator == 0
                        || p.numerator > p.denominator
                        || p.amount <= 0
                        || p.amount > 1000
                })
            {
                return Err(format!("Invalid proc in {name}"));
            }
        }
        for weapon in &self.weapons {
            if weapon
                .multipliers
                .iter()
                .filter(|m| m.additive_with_task_damage)
                .count()
                > 1
            {
                return Err("Multiple additive task groups need an explicit combined stage".into());
            }
            if !self.families.contains_key(&weapon.family)
                || weapon
                    .alternate_families
                    .iter()
                    .any(|f| !self.families.contains_key(f))
                || weapon.ids.is_empty()
                || weapon.ids.iter().any(|id| *id <= 0 || !ids.insert(*id))
            {
                return Err(format!(
                    "Missing family or repeated weapon ID in {}",
                    weapon.family
                ));
            }
            if weapon
                .multipliers
                .iter()
                .any(|m| m.denominator <= 0 || m.numerator <= 0 || m.attribute.is_empty())
                || weapon
                    .adjustments
                    .iter()
                    .any(|a| !["toa", "outside_toa"].contains(&a.context.as_str()))
            {
                return Err(format!("Invalid effect data in {}", weapon.family));
            }
        }
        let mut ammo_ids = std::collections::BTreeSet::new();
        for ammo in &self.ammunition {
            if !ammo.proc_chance.is_finite()
                || !(0.0..=1.0).contains(&ammo.proc_chance)
                || (!matches!(ammo.effect, AmmoEffect::None | AmmoEffect::Unsupported)
                    && ammo.proc_chance == 0.0)
                || ammo.ids.is_empty()
                || ammo.ids.iter().any(|id| *id <= 0 || !ammo_ids.insert(*id))
            {
                return Err("Invalid or duplicate ammunition ID".into());
            }
        }
        for family in self.families.values() {
            if family.ammo.iter().any(|id| !ammo_ids.contains(id)) {
                return Err("Missing ammunition effect definition".into());
            }
        }
        for effect in &self.equipment_effects {
            if effect.weight < 0 || effect.slot.is_some_and(|slot| slot >= 14) {
                return Err("Invalid equipment effect weight or slot".into());
            }
        }
        let mut equipment_ids = std::collections::BTreeSet::new();
        for set in &self.equipment_sets {
            let mut slots = std::collections::BTreeSet::new();
            if set.effect.is_empty()
                || set.pieces.is_empty()
                || set
                    .search_style
                    .as_ref()
                    .is_some_and(|s| !STYLES.contains(&s.as_str()))
                || set
                    .weapon_families
                    .iter()
                    .any(|f| !self.families.contains_key(f))
                || set.pieces.iter().any(|piece| {
                    piece.slot >= 14 || piece.ids.is_empty() || !slots.insert(piece.slot)
                })
            {
                return Err("Invalid equipment set".into());
            }
        }
        for group in &self.equipment_requirements {
            if group.ids.is_empty()
                || group
                    .ids
                    .iter()
                    .any(|id| *id <= 0 || !equipment_ids.insert(*id))
                || group.requirements.is_empty()
                || group
                    .requirements
                    .values()
                    .any(|level| !(1..=99).contains(level))
            {
                return Err("Invalid equipment requirements".into());
            }
        }
        Ok(())
    }
}
pub fn registry() -> &'static Registry {
    static DATA: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let rules: Registry = serde_json::from_str(include_str!("../assets/weapon-rules.json"))
            .expect("weapon rules");
        rules.validate().expect("valid weapon rules");
        rules
    })
}
pub struct Rule {
    pub family: &'static Family,
    pub weapon: &'static Weapon,
}
impl Rule {
    pub fn scale_equipment(
        &self,
        set: &[Option<i32>],
        mut attack: i64,
        mut max_hit: i32,
    ) -> (i64, i32) {
        for scaling in &self.family.equipment_scaling {
            let weight: i64 = registry()
                .equipment_effects
                .iter()
                .filter(|e| e.effect == scaling.effect)
                .filter(|e| match e.slot {
                    Some(slot) => set
                        .get(slot)
                        .copied()
                        .flatten()
                        .is_some_and(|id| e.ids.contains(&id)),
                    None => set.iter().flatten().any(|id| e.ids.contains(id)),
                })
                .map(|e| e.weight)
                .sum();
            attack =
                attack * (scaling.accuracy_denominator + weight) / scaling.accuracy_denominator;
            max_hit = (i64::from(max_hit) * (scaling.damage_denominator + weight)
                / scaling.damage_denominator) as i32;
        }
        (attack, max_hit)
    }
    pub fn has(&self, effect: Effect) -> bool {
        self.family.effects.contains(&effect) || self.weapon.effects.contains(&effect)
    }
    pub fn permits(&self, style: &str) -> bool {
        family_permits(self.family, style)
            || self
                .weapon
                .alternate_families
                .iter()
                .filter_map(|name| registry().families.get(name))
                .any(|family| family_permits(family, style))
    }
    pub fn accepts_recipe(&self, recipe: &SpellRecipe) -> bool {
        recipe.family == self.weapon.family || self.weapon.spell_families.contains(&recipe.family)
    }
    pub fn stance(&self, attack_type: &str) -> Stance {
        self.family
            .stances
            .get(attack_type)
            .copied()
            .unwrap_or_default()
    }
}
fn family_permits(family: &Family, style: &str) -> bool {
    base_style(style) == family.style
        && (style == base_style(style)
            || family
                .attack_types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(style)))
}
pub fn spell_compatible(rule: &Rule, recipe: &SpellRecipe, set: &[Option<i32>]) -> bool {
    rule.accepts_recipe(recipe)
        && (recipe.family != "ancient"
            || rule.weapon.family != "wind"
            || set_effect(set, "ahrim_damned"))
}
pub fn equipment_effect(id: i32, effect: &str) -> bool {
    registry()
        .equipment_effects
        .iter()
        .any(|r| r.effect == effect && r.ids.contains(&id))
}
/// A passive only applies when its item occupies the reviewed equipment slot.
pub fn equipped_effect(set: &[Option<i32>], effect: &str) -> bool {
    registry().equipment_effects.iter().any(|rule| {
        rule.effect == effect
            && match rule.slot {
                Some(slot) => set
                    .get(slot)
                    .copied()
                    .flatten()
                    .is_some_and(|id| rule.ids.contains(&id)),
                None => set.iter().flatten().any(|id| rule.ids.contains(id)),
            }
    })
}
pub fn void_piece(id: i32) -> bool {
    registry().equipment_sets.iter().any(|rule| {
        (rule.effect.starts_with("void_") || rule.effect.starts_with("elite_void_"))
            && rule.pieces.iter().any(|piece| piece.ids.contains(&id))
    })
}

pub fn set_effect(set: &[Option<i32>], effect: &str) -> bool {
    registry().equipment_sets.iter().any(|rule| {
        rule.effect == effect
            && rule.pieces.iter().all(|piece| {
                set.get(piece.slot)
                    .copied()
                    .flatten()
                    .is_some_and(|id| piece.ids.contains(&id))
            })
    })
}
/// Reviewed requirements supplement missing or incomplete public item metadata.
/// Cosmetic/charge variants share data without implying ownership of another ID.
pub fn equipment_requirements(id: i32) -> Option<&'static BTreeMap<String, u32>> {
    registry()
        .equipment_requirements
        .iter()
        .find(|group| group.ids.contains(&id))
        .map(|group| &group.requirements)
}
pub fn unsupported_equipment(id: i32) -> Option<&'static str> {
    registry()
        .equipment_effects
        .iter()
        .find(|r| r.effect == "unsupported" && r.ids.contains(&id))
        .map(|r| r.reason.as_str())
}
pub fn weapon(id: i32) -> Option<Rule> {
    let r = registry();
    static INDEX: std::sync::OnceLock<BTreeMap<i32, usize>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        r.weapons
            .iter()
            .enumerate()
            .flat_map(|(i, w)| w.ids.iter().map(move |id| (*id, i)))
            .collect()
    });
    let w = &r.weapons[*index.get(&id)?];
    Some(Rule {
        family: r.families.get(&w.family)?,
        weapon: w,
    })
}
pub fn weapon_for_style(id: i32, style: &str) -> Option<Rule> {
    let mut rule = weapon(id)?;
    if !family_permits(rule.family, style) {
        rule.family = rule
            .weapon
            .alternate_families
            .iter()
            .filter_map(|name| registry().families.get(name))
            .find(|family| family_permits(family, style))?;
    }
    Some(rule)
}
pub fn ammunition(id: i32) -> AmmoEffect {
    registry()
        .ammunition
        .iter()
        .find(|a| a.ids.contains(&id))
        .map(|a| a.effect)
        .unwrap_or(AmmoEffect::None)
}
pub fn bolt_proc_chance(id: i32, diary: bool) -> f64 {
    let base = registry()
        .ammunition
        .iter()
        .find(|a| a.ids.contains(&id))
        .map(|a| a.proc_chance)
        .unwrap_or(0.0);
    (base * if diary { 1.1 } else { 1.0 }).clamp(0.0, 1.0)
}
pub fn weapon_ids(style: &str, allow_spells: bool) -> Vec<i32> {
    registry()
        .weapons
        .iter()
        .flat_map(|w| w.ids.iter().copied())
        .filter(|id| {
            weapon_for_style(*id, style).is_some_and(|r| {
                r.permits(style)
                    && (allow_spells
                        || ["blue_moon", "ancient"].contains(&r.weapon.family.as_str())
                        || !matches!(r.family.formula, Formula::Elemental | Formula::Fixed))
            })
        })
        .collect()
}
pub fn ammo_ids(weapon_id: i32) -> &'static [i32] {
    weapon(weapon_id)
        .map(|w| w.family.ammo.as_slice())
        .unwrap_or(&[])
}
pub const DEFAULT_LOADED_SHOTS: u32 = 100;
pub fn all_internal_ammo_ids() -> Vec<i32> {
    registry()
        .families
        .values()
        .flat_map(|f| f.internal_ammo.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
pub fn internal_ammo(
    member: &Member,
    weapon_id: i32,
    settings: &crate::defence::Settings,
) -> Option<i32> {
    let rule = weapon(weapon_id)?;
    if rule.family.internal_ammo.is_empty() {
        return None;
    }
    if let Some(id) = settings.loaded_dart {
        return rule.family.internal_ammo.contains(&id).then_some(id);
    }
    let mut owned = BTreeMap::new();
    for container in [&member.bank, &member.inventory, &member.equipment]
        .into_iter()
        .flatten()
    {
        crate::account::add_pairs(&mut owned, container);
    }
    rule.family
        .internal_ammo
        .iter()
        .copied()
        .filter(|id| owned.get(id).copied().unwrap_or(0) >= settings.ammo_shots)
        .max_by_key(|id| {
            crate::combat::stats(*id)
                .and_then(|s| s.bonuses.get("ranged_str"))
                .copied()
                .unwrap_or(0)
        })
}
pub fn all_ammo_ids() -> Vec<i32> {
    registry()
        .families
        .values()
        .flat_map(|family| family.ammo.iter().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub const DEFAULT_CASTS: u32 = 150;
/// Saved weapon-category constraint shared by planning and combat comparisons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangedType {
    #[default]
    Auto,
    Light,
    Standard,
    Heavy,
}
impl RangedType {
    pub fn key(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Light => Some("light"),
            Self::Standard => Some("standard"),
            Self::Heavy => Some("heavy"),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Light => "Light",
            Self::Standard => "Standard",
            Self::Heavy => "Heavy",
        }
    }
    pub fn permits_weapon(self, id: i32) -> bool {
        weapon(id).is_some_and(|rule| {
            rule.family.style == "Ranged"
                && self
                    .key()
                    .is_none_or(|kind| rule.family.attack_types.iter().any(|t| t == kind))
        })
    }
}
/// One source of truth for calculation, planning, rune packing and sharing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SpellSelection {
    #[default]
    Auto,
    Element(String),
    Spell(String),
}
impl SpellSelection {
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
    pub fn valid(&self) -> bool {
        match self {
            Self::Auto => true,
            Self::Element(element) => ["air", "water", "earth", "fire"].contains(&element.as_str()),
            Self::Spell(name) => spells().iter().any(|recipe| recipe.name == *name),
        }
    }
    pub fn permits_weapon(&self, id: i32) -> bool {
        let Some(rule) = weapon(id) else {
            return false;
        };
        if rule.family.style != "Magic" {
            return false;
        }
        if self.is_auto() {
            return true;
        }
        match self {
            Self::Auto => true,
            Self::Element(_) => rule.family.formula == Formula::Elemental,
            Self::Spell(name) => spells()
                .iter()
                .find(|recipe| recipe.name == *name)
                .is_some_and(|recipe| rule.accepts_recipe(recipe)),
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct SpellRecipe {
    pub name: String,
    pub family: String,
    pub level: u32,
    pub runes: BTreeMap<i32, u32>,
    pub tiers: Vec<(u32, i64)>,
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub frostweaver_percent: u32,
}
pub fn spells() -> &'static [SpellRecipe] {
    static DATA: std::sync::OnceLock<Vec<SpellRecipe>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let spells: Vec<SpellRecipe> =
            serde_json::from_str(include_str!("../assets/spell-recipes.json"))
                .expect("spell recipes");
        let mut names = std::collections::BTreeSet::new();
        for spell in &spells {
            assert!(
                names.insert(spell.name.clone())
                    && registry().families.contains_key(&spell.family)
                    && spell.level > 0
                    && !spell.tiers.is_empty()
                    && spell
                        .element
                        .as_deref()
                        .is_none_or(|e| ["air", "water", "earth", "fire"].contains(&e)),
                "valid spell recipe"
            );
            assert!(
                spell.runes.iter().all(|(id, q)| *id > 0 && *q > 0)
                    && spell.tiers.windows(2).all(|pair| pair[0].0 < pair[1].0),
                "valid rune recipe and ordered damage tiers"
            );
        }
        spells
    })
}
pub fn family_ids(name: &str) -> Vec<i32> {
    registry()
        .weapons
        .iter()
        .filter(|w| w.family == name)
        .flat_map(|w| w.ids.iter().copied())
        .collect()
}
/// Unconfigured callers use the default cast budget. Family order is recipe preference.
pub fn spell_name(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
) -> &'static str {
    spell_name_at_level(set, owned, member.level("Magic"), DEFAULT_CASTS)
}
pub fn spell_name_configured(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
) -> &'static str {
    if let Some(name) = manual_spell_name(member, set, owned, settings, settings.spell_casts) {
        return name;
    }
    spell_name_at_level(
        set,
        owned,
        crate::modifiers::level(member, "Magic", &settings.boosts),
        settings.spell_casts,
    )
}
fn spell_name_at_level(
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    magic: Option<u32>,
    casts: u32,
) -> &'static str {
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return "";
    };
    let recipes: Vec<_> = spells()
        .iter()
        .filter(|s| spell_compatible(&rule, s, set))
        .collect();
    if recipes.is_empty() {
        return "";
    }
    recipes
        .iter()
        .find(|s| magic.is_some_and(|level| level >= s.level) && has_runes(s, set, owned, casts))
        .or_else(|| recipes.last())
        .map(|s| s.name.as_str())
        .unwrap_or("")
}
fn has_runes(
    spell: &SpellRecipe,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
) -> bool {
    rune_supplies_for_set(&spell.name, casts, set)
        .iter()
        .all(|(id, quantity)| owned.get(id).copied().unwrap_or(0) >= *quantity)
}
/// Staff-provided runes and charged tomes do not consume inventory rune stacks.
pub fn rune_supplies_for_set(spell: &str, casts: u32, set: &[Option<i32>]) -> Vec<(i32, u32)> {
    let supplied = set
        .get(3)
        .copied()
        .flatten()
        .and_then(weapon)
        .map(|r| r.weapon.supplied_runes.as_slice())
        .unwrap_or(&[]);
    let tome = elemental_focus(set).map(|f| f.rune);
    rune_supplies(spell, casts)
        .into_iter()
        .filter(|(id, _)| !supplied.contains(id) && tome != Some(*id))
        .collect()
}
pub fn elemental_focus(set: &[Option<i32>]) -> Option<&'static ElementalFocus> {
    let id = set.get(5).copied().flatten()?;
    registry()
        .elemental_focuses
        .iter()
        .find(|focus| focus.ids.contains(&id))
}
/// Compare one strongest affordable spell per element using the combat engine.
/// Non-elemental families retain their reviewed recipe preference.
pub fn spell_name_for_target(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
    target: &crate::planner::Monster,
    invocation: u32,
) -> &'static str {
    let fallback = spell_name_configured(member, set, owned, settings);
    if !settings.spell_selection.is_auto() {
        return fallback;
    }
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return fallback;
    };
    if rule.family.formula != Formula::Elemental {
        return fallback;
    }
    elemental_candidates(member, set, owned, settings)
        .into_iter()
        .enumerate()
        .filter_map(|(index, s)| {
            crate::combat::calculate_configured(
                member, set, "Magic", target, &s.name, invocation, settings,
            )
            .ok()
            .map(|r| (s.name.as_str(), r.dps, index))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.2.cmp(&a.2)))
        .map(|(name, _, _)| name)
        .unwrap_or(fallback)
}
pub fn elemental_candidates(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
) -> Vec<&'static SpellRecipe> {
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return vec![];
    };
    if rule.family.formula != Formula::Elemental {
        return vec![];
    }
    let magic = crate::modifiers::level(member, "Magic", &settings.boosts).unwrap_or(0);
    let mut elements = std::collections::BTreeSet::new();
    spells()
        .iter()
        .filter(|s| {
            spell_compatible(&rule, s, set)
                && magic >= s.level
                && has_runes(s, set, owned, settings.spell_casts)
        })
        .filter(|s| s.element.is_none() || elements.insert(s.element.as_deref()))
        .collect()
}
fn manual_spell_name(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
    casts: u32,
) -> Option<&'static str> {
    if set
        .get(3)
        .copied()
        .flatten()
        .and_then(weapon)
        .is_some_and(|r| r.family.style != "Magic")
    {
        return None;
    }
    match &settings.spell_selection {
        SpellSelection::Auto => None,
        SpellSelection::Spell(name) => spells()
            .iter()
            .find(|s| s.name == *name)
            .map(|s| s.name.as_str()),
        SpellSelection::Element(element) => {
            let magic = crate::modifiers::level(member, "Magic", &settings.boosts).unwrap_or(0);
            let recipes: Vec<_> = spells()
                .iter()
                .filter(|s| s.element.as_deref() == Some(element.as_str()))
                .collect();
            recipes
                .iter()
                .find(|s| magic >= s.level && has_runes(s, set, owned, casts))
                .or_else(|| recipes.iter().find(|s| magic >= s.level))
                .or_else(|| recipes.last())
                .map(|s| s.name.as_str())
        }
    }
}
/// Missing supplies do not hide theoretical damage, but must block a ready/exportable plan.
pub fn spell_requirement(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
    name: &str,
) -> Option<String> {
    let recipe = spells().iter().find(|s| s.name == name)?;
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return Some("Requires a compatible spellcasting weapon.".into());
    };
    if !spell_compatible(&rule, recipe, set) {
        return Some("Requires a compatible spellcasting weapon.".into());
    }
    let Some(magic) = crate::modifiers::level(member, "Magic", &settings.boosts) else {
        return Some("Enter your Magic level.".into());
    };
    if magic < recipe.level {
        return Some(format!("Requires {} Magic.", recipe.level));
    }
    let missing: Vec<_> = rune_supplies_for_set(name, settings.spell_casts, set)
        .into_iter()
        .filter_map(|(id, need)| {
            let short = need.saturating_sub(owned.get(&id).copied().unwrap_or(0));
            (short > 0).then(|| format!("{} ×{short}", crate::planner::item_name(id)))
        })
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "Missing runes for {} casts: {}. Prepare these runes before using this plan.",
            settings.spell_casts,
            missing.join(", ")
        )
    })
}
/// Explain a missing element row without treating spells as bank items.
pub fn elemental_unavailability(
    member: &Member,
    set: &[Option<i32>],
    settings: &crate::defence::Settings,
    element: &str,
) -> String {
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return "Choose a supported spellcasting weapon.".into();
    };
    if rule.family.formula == Formula::Powered {
        return "Powered staff: no elemental spell bonus.".into();
    }
    if rule.family.formula != Formula::Elemental {
        return "This weapon uses a non-elemental spell.".into();
    }
    let Some(magic) = crate::modifiers::level(member, "Magic", &settings.boosts) else {
        return "Enter your Magic level.".into();
    };
    let minimum = spells()
        .iter()
        .filter(|s| spell_compatible(&rule, s, set) && s.element.as_deref() == Some(element))
        .map(|s| s.level)
        .min();
    match minimum {
        Some(level) if magic < level => format!("Requires at least {level} Magic."),
        Some(_) => format!("Insufficient runes for {} casts.", settings.spell_casts),
        None => "Elemental casting is not supported for this weapon.".into(),
    }
}
pub fn rune_supplies(spell: &str, casts: u32) -> Vec<(i32, u32)> {
    spells()
        .iter()
        .find(|s| s.name == spell)
        .map(|s| {
            s.runes
                .iter()
                .map(|(id, quantity)| (*id, quantity.saturating_mul(casts)))
                .collect()
        })
        .unwrap_or_default()
}
/// One explicit Mark of Darkness precast; maintaining it longer needs recasts.
pub fn mark_darkness_supplies() -> Vec<(i32, u32)> {
    vec![(564, 1), (566, 1)]
}
pub fn mark_darkness_supported(member: &Member, owned: &BTreeMap<i32, u32>) -> bool {
    member.level("Magic").is_some_and(|level| level >= 59)
        && mark_darkness_supplies()
            .iter()
            .all(|(id, quantity)| owned.get(id).copied().unwrap_or(0) >= *quantity)
}
pub fn casting_supported(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
) -> bool {
    casting_at_level(member, set, owned, casts, member.level("Magic"), None)
}
pub fn casting_supported_configured(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
    settings: &crate::defence::Settings,
) -> bool {
    if let Some(rule) = set.get(3).copied().flatten().and_then(weapon) {
        if rule.family.style == "Ranged" {
            return settings.ranged_type.permits_weapon(set[3].unwrap());
        }
    }
    casting_at_level(
        member,
        set,
        owned,
        casts,
        crate::modifiers::level(member, "Magic", &settings.boosts),
        manual_spell_name(member, set, owned, settings, casts),
    )
}
pub fn casting_supported_for_style(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
    style: &str,
) -> bool {
    if base_style(style) == "Melee" {
        return set
            .get(3)
            .copied()
            .flatten()
            .and_then(|id| weapon_for_style(id, style))
            .is_some();
    }
    casting_supported_configured(member, set, owned, settings.spell_casts, settings)
}
fn casting_at_level(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
    magic: Option<u32>,
    selected: Option<&str>,
) -> bool {
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return false;
    };
    if rule.family.style != "Magic" {
        return true;
    }
    if rule
        .weapon
        .requirements
        .iter()
        .any(|(skill, minimum)| !member.level(skill).is_some_and(|level| level >= *minimum))
    {
        return false;
    }
    if rule.family.formula == Formula::Powered {
        return selected.is_none() && magic.is_some();
    }
    let name = selected.unwrap_or_else(|| spell_name_at_level(set, owned, magic, casts));
    spells().iter().find(|s| s.name == name).is_some_and(|s| {
        spell_compatible(&rule, s, set)
            && magic.is_some_and(|level| level >= s.level)
            && has_runes(s, set, owned, casts)
    })
}
pub fn spell_max_hit(name: &str, magic: u32) -> Option<i64> {
    spells().iter().find(|s| s.name == name).and_then(|s| {
        if magic < s.level {
            return None;
        }
        s.tiers
            .iter()
            .rev()
            .find(|(minimum, _)| magic >= *minimum)
            .map(|(_, max)| *max)
    })
}
