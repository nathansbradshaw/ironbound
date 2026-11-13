//! Item IDs are data. Calculation code implements reusable mechanics, not individual weapons.
use crate::account::Member;
use serde::Deserialize;
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
    Wind,
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
pub struct Registry {
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
                || (matches!(ammo.effect, AmmoEffect::Ruby | AmmoEffect::Diamond)
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
        base_style(style) == self.family.style
            && (style == base_style(style)
                || self
                    .family
                    .attack_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(style)))
    }
    pub fn stance(&self, attack_type: &str) -> Stance {
        self.family
            .stances
            .get(attack_type)
            .copied()
            .unwrap_or_default()
    }
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
            weapon(*id).is_some_and(|r| {
                r.permits(style)
                    && (allow_spells || !matches!(r.family.formula, Formula::Wind | Formula::Fixed))
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
        .filter(|id| owned.get(id).copied().unwrap_or(0) >= DEFAULT_LOADED_SHOTS)
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
#[derive(Debug, Deserialize)]
pub struct SpellRecipe {
    pub name: String,
    pub family: String,
    pub level: u32,
    pub runes: BTreeMap<i32, u32>,
    pub tiers: Vec<(u32, i64)>,
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
                    && !spell.tiers.is_empty(),
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
/// Spell selection uses the common 150-cast trip template. Family order is recipe preference.
pub fn spell_name(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
) -> &'static str {
    spell_name_at_level(set, owned, member.level("Magic"))
}
pub fn spell_name_configured(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &crate::defence::Settings,
) -> &'static str {
    spell_name_at_level(
        set,
        owned,
        crate::modifiers::level(member, "Magic", &settings.boosts),
    )
}
fn spell_name_at_level(
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    magic: Option<u32>,
) -> &'static str {
    let Some(rule) = set.get(3).copied().flatten().and_then(weapon) else {
        return "";
    };
    let recipes: Vec<_> = spells()
        .iter()
        .filter(|s| s.family == rule.weapon.family)
        .collect();
    if recipes.is_empty() {
        return "";
    }
    recipes
        .iter()
        .find(|s| magic.is_some_and(|level| level >= s.level) && has_runes(s, owned, DEFAULT_CASTS))
        .or_else(|| recipes.last())
        .map(|s| s.name.as_str())
        .unwrap_or("")
}
fn has_runes(spell: &SpellRecipe, owned: &BTreeMap<i32, u32>, casts: u32) -> bool {
    spell.runes.iter().all(|(id, quantity)| {
        quantity
            .checked_mul(casts)
            .is_some_and(|need| owned.get(id).copied().unwrap_or(0) >= need)
    })
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
    casting_at_level(member, set, owned, casts, member.level("Magic"))
}
pub fn casting_supported_configured(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
    settings: &crate::defence::Settings,
) -> bool {
    casting_at_level(
        member,
        set,
        owned,
        casts,
        crate::modifiers::level(member, "Magic", &settings.boosts),
    )
}
fn casting_at_level(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    casts: u32,
    magic: Option<u32>,
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
        return magic.is_some();
    }
    let name = spell_name_at_level(set, owned, magic);
    spells()
        .iter()
        .find(|s| s.name == name)
        .is_some_and(|s| magic.is_some_and(|level| level >= s.level) && has_runes(s, owned, casts))
}
pub fn spell_max_hit(name: &str, magic: u32) -> Option<i64> {
    spells().iter().find(|s| s.name == name).and_then(|s| {
        s.tiers
            .iter()
            .rev()
            .find(|(minimum, _)| magic >= *minimum)
            .map(|(_, max)| *max)
    })
}
