//! Locally computed standard sustained attacks. Unsupported effects return an explanation.
use crate::{
    account::Member,
    planner::Monster,
    rules::{self, AmmoEffect, Effect, Formula},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EquipmentStats {
    pub id: i32,
    #[serde(default)]
    pub speed: i32,
    pub category: String,
    pub bonuses: BTreeMap<String, i32>,
    pub offensive: BTreeMap<String, i32>,
    pub defensive: BTreeMap<String, i32>,
}
pub fn equipment_stats() -> &'static [EquipmentStats] {
    static DATA: std::sync::OnceLock<Vec<EquipmentStats>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/combat-equipment.json"))
            .expect("combat equipment")
    })
}
pub fn stats(id: i32) -> Option<&'static EquipmentStats> {
    static INDEX: std::sync::OnceLock<BTreeMap<i32, usize>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let mut map = BTreeMap::new();
        for (i, s) in equipment_stats().iter().enumerate() {
            map.entry(s.id).or_insert(i);
        }
        map
    });
    index.get(&id).map(|i| &equipment_stats()[*i])
}
#[derive(Clone, Debug, Default)]
pub struct Bonuses {
    pub offensive: BTreeMap<String, i32>,
    pub defensive: BTreeMap<String, i32>,
    pub other: BTreeMap<String, i32>,
}
pub fn bonuses(set: &[Option<i32>]) -> Bonuses {
    bonuses_in_context(set, false)
}
fn bonuses_in_context(set: &[Option<i32>], toa: bool) -> Bonuses {
    let mut result = Bonuses::default();
    for id in set.iter().flatten() {
        if let Some(g) = stats(*id) {
            for (map, dest) in [
                (&g.offensive, &mut result.offensive),
                (&g.defensive, &mut result.defensive),
                (&g.bonuses, &mut result.other),
            ] {
                for (k, v) in map {
                    *dest.entry(k.clone()).or_default() += v;
                }
            }
            if let Some(rule) = rules::weapon(*id) {
                for adjustment in &rule.weapon.adjustments {
                    if (adjustment.context == "outside_toa" && !toa)
                        || (adjustment.context == "toa" && toa)
                    {
                        for (key, amount) in &adjustment.offensive {
                            *result.offensive.entry(key.clone()).or_default() += amount;
                        }
                        for (key, amount) in &adjustment.bonuses {
                            *result.other.entry(key.clone()).or_default() += amount;
                        }
                    }
                }
            }
        }
    }
    if rules::set_effect(set, "verac_damned") {
        *result.other.entry("prayer".into()).or_default() += 7;
    }
    result
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ResultStats {
    /// Marginal per-attack damage, including misses and reviewed bolt effects.
    pub hit_distribution: Vec<(i32, f64)>,
    pub hp_dependent: Option<HpDependentHit>,
    pub dps: f64,
    /// Delayed Eclipse damage at steady state, with five-stack blocking.
    #[serde(default)]
    pub burn_dps: f64,
    #[serde(default)]
    pub burn: Option<crate::burn::BurnAttack>,
    /// Proc potential before overhealing and target remaining-HP caps.
    pub potential_healing_per_second: f64,
    pub accuracy: f64,
    pub max_hit: i32,
    pub attack_ticks: i32,
    /// Mean cooldown, including Bloodrager's accuracy-dependent one-tick reduction.
    pub expected_attack_ticks: f64,
    pub attack_roll: i64,
    pub defence_roll: i64,
    pub attack_type: String,
    pub spell: String,
    pub elemental: Option<ElementalStats>,
    /// Guaranteed-hit windows retain proc probabilities for exact alternating cycles.
    pub guaranteed_hits: Option<Vec<(i32, f64)>>,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct HpDependentHit {
    pub chance: f64,
    pub percent: i32,
    pub cap: i32,
    pub divisor: i32,
    pub transform: crate::monster_rules::Transform,
    /// Number of final entries contributed by this proc at the displayed HP.
    pub distribution_entries: usize,
}
impl HpDependentHit {
    pub fn outcomes(self, hp: u32) -> Vec<(i32, f64)> {
        let raw = ((i64::from(hp) * i64::from(self.percent) / 100).min(i64::from(self.cap)) as i32)
            / self.divisor;
        self.transform.outcomes(raw, self.chance)
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ElementalStats {
    pub element: String,
    pub base_max: i32,
    pub weakness_percent: i32,
    pub damage_before_weakness: i32,
    pub weakness_damage: i32,
    pub base_attack_roll: i64,
    pub weakness_accuracy: i64,
    pub tome: bool,
}
/// Catalog HP under the same solo/path-zero ToA scaling used by bolt effects.
pub fn target_hitpoints(monster: &Monster, invocation: u32) -> Option<i32> {
    let mut hp = monster.skills.get("hp").copied().filter(|hp| *hp > 0)?;
    if crate::encounters::mechanics().toa_ids.contains(&monster.id) {
        let scale = if crate::encounters::window(monster.id)
            .is_some_and(|w| w.damage_model == crate::encounters::DamageModel::WardenCore)
        {
            1
        } else {
            4
        };
        hp += hp * (scale * invocation.min(150) as i32 / 10) / 100;
        if hp > 100 {
            let round = if hp > 300 { 10 } else { 5 };
            hp = (hp + round / 2) / round * round;
        }
    }
    Some(hp)
}
/// Phase bounds are shared by outgoing damage, current-HP inputs and budgets.
pub fn target_hp_bounds(monster: &Monster, invocation: u32) -> Result<(u32, u32), String> {
    let full = target_hitpoints(monster, invocation).ok_or("Target HP unavailable.")? as u32;
    let window = crate::encounters::window(monster.id);
    let max = window.and_then(|w| w.start_hp).unwrap_or(full);
    let min = window.and_then(|w| w.min_hp).unwrap_or(1);
    if min == 0 || min > max || max > full {
        return Err("Invalid phase HP bounds.".into());
    }
    Ok((min, max))
}
pub fn current_target_hp(
    monster: &Monster,
    invocation: u32,
    settings: &crate::defence::Settings,
) -> Result<u32, String> {
    let (min, max) = target_hp_bounds(monster, invocation)?;
    let hp = settings.target_hp.get(&monster.id).copied().unwrap_or(max);
    if hp < min || hp > max {
        return Err(format!(
            "Current target HP must be {min}–{max} for this attack window."
        ));
    }
    Ok(hp)
}
/// Normal accuracy includes negative attack/defence rolls.
pub fn accuracy(mut a: i64, mut d: i64) -> f64 {
    if a < 0 {
        a = (a + 2).min(0);
    }
    if d < 0 {
        d = (d + 2).min(0);
    }
    let normal = |a: i64, d: i64| {
        if a > d {
            1.0 - (d + 2) as f64 / (2.0 * (a + 1) as f64)
        } else {
            a as f64 / (2.0 * (d + 1) as f64)
        }
    };
    match (a >= 0, d >= 0) {
        (true, true) => normal(a, d),
        (true, false) => 1.0 - 1.0 / (-d + 1) as f64 / (a + 1) as f64,
        (false, true) => 0.0,
        (false, false) => normal(-d, -a),
    }
    .clamp(0.0, 1.0)
}
pub fn calculate(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
) -> Result<ResultStats, String> {
    calculate_at_invocation(member, set, style, monster, spell, 0)
}
pub fn calculate_at_invocation(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
    invocation: u32,
) -> Result<ResultStats, String> {
    calculate_configured(
        member,
        set,
        style,
        monster,
        spell,
        invocation,
        &Default::default(),
    )
}
pub fn calculate_configured(
    member: &Member,
    set: &[Option<i32>],
    style: &str,
    monster: &Monster,
    spell: &str,
    invocation: u32,
    settings: &crate::defence::Settings,
) -> Result<ResultStats, String> {
    if !settings.valid() {
        return Err("Invalid combat settings.".into());
    }
    let source_monster = monster;
    let hp = current_target_hp(monster, invocation, settings)?;
    let scaled = crate::monster_rules::hp_scaled(monster, hp)?;
    let monster = scaled.as_ref().unwrap_or(monster);
    let reduced = settings
        .reductions
        .get(&monster.id)
        .map(|reductions| crate::modifiers::reduce(monster, reductions))
        .transpose()?;
    let monster = reduced.as_ref().unwrap_or(monster);
    if settings.mark_darkness && !member.level("Magic").is_some_and(|level| level >= 59) {
        return Err("Mark of Darkness requires 59 Magic.".into());
    }
    let prayer = crate::prayer::selected(settings);
    crate::prayer::eligible_settings(member, settings)?;
    if set.len() != 14 {
        return Err("Equipment needs 14 slots.".into());
    }
    let weapon = set[3].ok_or("Choose a weapon.")?;
    if settings.bulwark_block && rules::equipped_effect(set, "bulwark") {
        return Err(
            "Dinh's Block stance prevents outgoing attacks. Disable Block to compare attacks."
                .into(),
        );
    }
    let stats = stats(weapon).ok_or("Weapon stats unavailable.")?;
    let rule = rules::weapon_for_style(weapon, style)
        .ok_or("This weapon's combat mechanics are not modelled for this style.")?;
    if !rule.permits(style) {
        return Err("This weapon does not support the selected attack style.".into());
    }
    let requested_style = style;
    let style = rules::base_style(style);
    crate::monster_rules::check_weapon(monster.id, style, weapon)?;
    if style == "Ranged" && !settings.ranged_type.permits_weapon(weapon) {
        return Err(format!(
            "Choose a {} ranged weapon for this plan.",
            settings.ranged_type.label()
        ));
    }
    if style == "Magic" {
        if let Some(recipe) = rules::spells().iter().find(|recipe| recipe.name == spell) {
            if !rules::spell_compatible(&rule, recipe, set) {
                return Err(format!(
                    "This weapon cannot cast {}. Choose a supported spell or a compatible spellcasting weapon.",
                    recipe.name
                ));
            }
        }
    }
    let hit_rule = crate::encounters::hit_rule(monster.id);
    if hit_rule.is_some_and(|rule| !rule.allowed_styles.iter().any(|allowed| allowed == style)) {
        return Err(
            "This attack style is blocked or not modelled for this encounter window.".into(),
        );
    }
    let always_max = hit_rule.is_some_and(|rule| rule.always_max);
    let damage_model = crate::encounters::window(monster.id)
        .map(|w| w.damage_model)
        .unwrap_or_default();
    let damage_divisor = damage_model.divisor();
    let warden_p2 = damage_model == crate::encounters::DamageModel::WardensP2;
    let demon = monster
        .attributes
        .iter()
        .any(|attribute| attribute == "demon");
    let demonbane_spell = style == "Magic" && rule.has(Effect::DemonbaneSpell);
    if demonbane_spell && !demon {
        return Err("Demonbane spells can only target demons.".into());
    }
    let vulnerability = i64::from(crate::encounters::demonbane_vulnerability(monster.id));
    let spell_factor = rule.weapon.demonbane_spell_factor.unwrap_or(1);
    let mark_bonus =
        (demonbane_spell && settings.mark_darkness).then_some((25 * spell_factor, vulnerability));
    let void = rules::set_effect(
        set,
        match style {
            "Magic" => "void_magic",
            "Ranged" => "void_ranged",
            _ => "void_melee",
        },
    );
    let target_bonus = crate::effects::target_bonus(set, style, monster, settings);
    let accuracy_bonus = target_bonus.map(|b| b.accuracy).unwrap_or([1, 1]);
    let damage_bonus = if rule.has(Effect::StrengthRanged) {
        crate::effects::target_bonus(set, "Melee", monster, settings)
    } else {
        target_bonus
    }
    .map(|b| b.damage)
    .unwrap_or([1, 1]);
    let prayer_attack = if prayer.style == style {
        prayer.attack
    } else {
        0
    };
    let prayer_strength = if prayer.style == style {
        prayer.strength
    } else {
        0
    };
    if let Some(reason) = set
        .iter()
        .flatten()
        .find_map(|id| rules::unsupported_equipment(*id))
    {
        return Err(reason.into());
    }
    if monster.skills.is_empty() {
        return Err("Monster combat stats unavailable.".into());
    }
    let flat_armour = monster
        .defensive
        .get(if style == "Magic" {
            "magic_resistance"
        } else {
            "flat_armour"
        })
        .copied()
        .unwrap_or(0);
    if flat_armour != 0 && style == "Ranged" {
        return Err("This combination of flat armour and weapon effects needs a separate hit-distribution model.".into());
    }
    if let Some(reason) = crate::encounters::limitation(monster) {
        return Err(reason.into());
    }
    let toa = crate::encounters::mechanics().toa_ids.contains(&monster.id);
    let mut b = bonuses_in_context(set, toa);
    if style == "Ranged"
        && rule.family.quiver_ammo
        && set[13].is_some_and(|ammo| rule.family.ammo.contains(&ammo))
        && rules::equipped_effect(set, "charged_quiver")
    {
        *b.offensive.entry("ranged".into()).or_default() += 10;
        *b.other.entry("ranged_str".into()).or_default() += 1;
    }
    if !rule.family.internal_ammo.is_empty() {
        let dart = rules::internal_ammo(member, weapon, settings).ok_or_else(|| {
            format!(
                "Choose compatible loaded darts, or provide at least {} owned darts to load.",
                settings.ammo_shots
            )
        })?;
        let dart_stats = self::stats(dart).ok_or("Loaded ammunition stats unavailable.")?;
        *b.other.entry("ranged_str".into()).or_default() +=
            dart_stats.bonuses.get("ranged_str").copied().unwrap_or(0);
    }
    if rules::set_effect(set, "elite_void_magic") {
        *b.other.entry("magic_str".into()).or_default() += 50;
    }
    // Self-ammunition weapons retain e.g. a blessing's prayer, but do not fire arrows/bolts.
    if style == "Ranged" && rule.family.ammo.is_empty() {
        if let Some(ammo) = set[13].and_then(self::stats) {
            *b.other.entry("ranged_str".into()).or_default() -=
                ammo.bonuses.get("ranged_str").copied().unwrap_or(0);
            *b.offensive.entry("ranged".into()).or_default() -=
                ammo.offensive.get("ranged").copied().unwrap_or(0);
        }
    }
    if style == "Magic" && rule.has(Effect::Shadow) {
        let factor = if toa { 4 } else { 3 };
        *b.offensive.entry("magic".into()).or_default() *= factor;
        let magic = b.other.entry("magic_str".into()).or_default();
        *magic = (*magic * factor).min(1000);
    }
    if style == "Magic" {
        if rule.has(Effect::SmokeStaff)
            && rules::spells()
                .iter()
                .any(|r| r.name == spell && r.family == "wind")
        {
            *b.other.entry("magic_str".into()).or_default() += 100;
        }
        if rules::spells()
            .iter()
            .any(|r| r.name == spell && r.family == "ancient")
        {
            let pieces = rules::registry()
                .equipment_effects
                .iter()
                .filter(|r| r.effect == "virtus_ancient")
                .filter(|r| {
                    r.slot
                        .and_then(|slot| set[slot])
                        .is_some_and(|id| r.ids.contains(&id))
                })
                .count() as i32;
            *b.other.entry("magic_str".into()).or_default() += 30 * pieces;
        }
        *b.other.entry("magic_str".into()).or_default() += if prayer.style == style {
            prayer.magic_damage as i32 * 10
        } else {
            0
        };
        // Salve's magic damage is additive with other magic damage, and is not
        // multiplied by Shadow; melee/ranged Salve apply after the base max hit.
        *b.other.entry("magic_str".into()).or_default() +=
            target_bonus.map(|b| b.magic_damage_per_mille).unwrap_or(0);
    }
    let get = |m: &BTreeMap<String, i32>, k: &str| m.get(k).copied().unwrap_or(0);
    let level = |s: &str| {
        crate::modifiers::level(member, s, &settings.boosts)
            .map(|v| v as i64)
            .ok_or_else(|| format!("Enter your {s} level."))
    };
    let mut elemental = None;
    let mut obsidian_accuracy = 0;
    let (attack_type, attack, max_hit, ticks, def_level, def_bonus) = match style {
        "Melee" => {
            let kind = if requested_style == "Melee" {
                rule.family.attack_types[0].as_str()
            } else {
                rule.family
                    .attack_types
                    .iter()
                    .find(|t| t.eq_ignore_ascii_case(requested_style))
                    .unwrap()
                    .as_str()
            };
            let stance = rule.stance(kind);
            let mut effective_attack =
                level("Attack")? * (100 + prayer_attack) / 100 + 8 + stance.attack_bonus();
            let mut effective_strength =
                level("Strength")? * (100 + prayer_strength) / 100 + 8 + stance.strength_bonus();
            if rule.has(Effect::Soulreaper) {
                effective_strength +=
                    level("Strength")? * i64::from(settings.soulreaper_stacks) * 6 / 100;
            }
            if void {
                effective_attack = effective_attack * 11 / 10;
                effective_strength = effective_strength * 11 / 10;
            }
            let a = effective_attack * (get(&b.offensive, kind) as i64 + 64);
            let mut max = (effective_strength * (get(&b.other, "str") as i64 + 64) + 320) / 640;
            if rule.has(Effect::ObsidianWeapon) && rules::set_effect(set, "obsidian_armour") {
                obsidian_accuracy = a / 10;
                max = max * damage_bonus[0] / damage_bonus[1] + max / 10;
            }
            (
                kind,
                a,
                max.max(0) as i32,
                stats.speed.max(1),
                get(&monster.skills, "def"),
                get(&monster.defensive, kind),
            )
        }
        "Ranged" => {
            if !rule.family.ammo.is_empty() {
                let ammo = set[13].ok_or("Choose compatible ammunition.")?;
                if !rule.family.ammo.contains(&ammo) {
                    return Err("Weapon/ammunition combination not supported.".into());
                }
                if rules::ammunition(ammo) == AmmoEffect::Unsupported {
                    return Err("This ammunition is compatible, but its enchanted-bolt effect is not modelled yet. Choose supported bolts for a DPS estimate.".into());
                }
            }
            let stance_bonus = if settings.ranged_stance == "Accurate" {
                3
            } else {
                0
            };
            let mut effective = level("Ranged")? * (100 + prayer_attack) / 100 + 8 + stance_bonus;
            if void {
                effective = effective * 11 / 10;
            }
            let a = effective * (get(&b.offensive, "ranged") as i64 + 64);
            let strength_ranged = rule.has(Effect::StrengthRanged);
            let mut strength = level(if strength_ranged {
                "Strength"
            } else {
                "Ranged"
            })? * (100 + prayer_strength)
                / 100
                + 8
                + stance_bonus;
            if rules::set_effect(set, "elite_void_ranged") {
                strength = strength * 9 / 8;
            } else if void {
                strength = strength * 11 / 10;
            }
            let max = (strength
                * (get(&b.other, if strength_ranged { "str" } else { "ranged_str" }) as i64 + 64)
                + 320)
                / 640;
            (
                rule.family.attack_types[0].as_str(),
                a,
                max.max(0) as i32,
                (stats.speed - i32::from(rule.family.rapid && settings.ranged_stance == "Rapid"))
                    .max(1),
                get(&monster.skills, "def"),
                *monster
                    .defensive
                    .get(&rule.family.attack_types[0])
                    .ok_or("Selected ranged defence category is unavailable for this target.")?,
            )
        }
        "Magic" => {
            let mage = level("Magic")?;
            let base = match rule.family.formula {
                Formula::Elemental => {
                    let recipe = rules::spells()
                        .iter()
                        .find(|r| r.name == spell && rules::spell_compatible(&rule, r, set))
                        .ok_or("Choose a supported elemental spell.")?;
                    if mage < i64::from(recipe.level) {
                        return Err(format!("{} requires {} Magic.", recipe.name, recipe.level));
                    }
                    // Boosts unlock casts and improve accuracy; damage tiers use base Magic.
                    let base_magic = member.level("Magic").ok_or("Enter your Magic level.")?;
                    recipe
                        .tiers
                        .iter()
                        .rev()
                        .find(|(level, _)| base_magic >= *level)
                        .or_else(|| recipe.tiers.first())
                        .map(|(_, max)| *max)
                        .ok_or("Spell damage tier unavailable.")?
                }
                Formula::Powered => if rule.family.offset_before_division {
                    (rule.family.numerator * mage + rule.family.offset) / rule.family.denominator
                } else {
                    rule.family.numerator * mage / rule.family.denominator + rule.family.offset
                }
                .max(1),
                Formula::Fixed => rule.family.offset,
                Formula::Spell => {
                    if rule.weapon.requirements.iter().any(|(skill, minimum)| {
                        !member.level(skill).is_some_and(|level| level >= *minimum)
                    }) {
                        return Err(
                            "This spellcasting weapon's skill requirements are unmet or unknown."
                                .into(),
                        );
                    }
                    let recipe = rules::spells()
                        .iter()
                        .find(|recipe| {
                            recipe.name == spell && rules::spell_compatible(&rule, recipe, set)
                        })
                        .ok_or("Choose a supported spell for this weapon.")?;
                    if mage < i64::from(recipe.level) {
                        return Err(format!("{} requires {} Magic.", recipe.name, recipe.level));
                    }
                    rules::spell_max_hit(spell, mage as u32)
                        .ok_or("Spell damage tier unavailable.")?
                }
                _ => return Err("Spell or powered staff not supported.".into()),
            };
            let mut max = base + base * get(&b.other, "magic_str") as i64 / 1000;
            max = max * damage_bonus[0] / damage_bonus[1];
            let mut effective = mage * (100 + prayer_attack) / 100 + rule.family.accuracy_bonus;
            if void {
                effective = effective * 29 / 20;
            }
            let base_a = effective * (get(&b.offensive, "magic") as i64 + 64);
            let smoke = rule.has(Effect::SmokeStaff)
                && rules::spells()
                    .iter()
                    .any(|r| r.name == spell && r.family == "wind");
            // Smoke accuracy adds to Salve's additive percentage. Task accuracy
            // instead multiplies after smoke and dragonbane, with its own floor.
            let on_task = target_bonus.is_some_and(|bonus| bonus.slayer_task);
            let mut a = if smoke {
                if on_task {
                    base_a * 110 / 100
                } else {
                    base_a * (accuracy_bonus[0] * 10 + accuracy_bonus[1]) / (accuracy_bonus[1] * 10)
                }
            } else if on_task {
                base_a
            } else {
                base_a * accuracy_bonus[0] / accuracy_bonus[1]
            };
            // Magic dragonbane applies before additive elemental weakness,
            // unlike the later melee/ranged multiplier stage.
            for multiplier in &rule.weapon.multipliers {
                if monster.attributes.contains(&multiplier.attribute) {
                    if multiplier.accuracy {
                        a = a * multiplier.numerator / multiplier.denominator;
                    }
                    if multiplier.damage {
                        max = max * multiplier.numerator / multiplier.denominator;
                    }
                }
            }
            if on_task {
                a = a * accuracy_bonus[0] / accuracy_bonus[1];
            }
            if let Some(element) = rules::spells()
                .iter()
                .find(|r| r.name == spell)
                .and_then(|r| r.element.as_deref())
            {
                let weakness_percent = monster
                    .weakness
                    .as_ref()
                    .filter(|w| w.element == element)
                    .map(|w| w.severity)
                    .unwrap_or(0);
                let weakness_accuracy = base_a * i64::from(weakness_percent) / 100;
                let weakness_damage = base * i64::from(weakness_percent) / 100;
                let focus = rules::elemental_focus(set).filter(|f| f.element == element);
                elemental = Some(ElementalStats {
                    element: element.into(),
                    base_max: base as i32,
                    weakness_percent,
                    damage_before_weakness: max as i32,
                    weakness_damage: weakness_damage as i32,
                    base_attack_roll: base_a,
                    weakness_accuracy,
                    tome: focus.is_some(),
                });
                a += weakness_accuracy;
                max += weakness_damage;
                if let Some(focus) = focus {
                    max = max * (100 + focus.damage_percent) / 100;
                    // Tome accuracy precedes the additive weakness bonus.
                    a = (a - weakness_accuracy) * (100 + focus.accuracy_percent) / 100
                        + weakness_accuracy;
                }
            }
            (
                "magic",
                a,
                max.max(0) as i32,
                rule.family.ticks.unwrap_or(stats.speed).max(1),
                get(
                    &monster.skills,
                    if crate::encounters::mechanics()
                        .magic_defence_uses_defence
                        .contains(&crate::encounters::source_npc_id(monster.id))
                    {
                        "def"
                    } else {
                        "magic"
                    },
                ),
                get(&monster.defensive, "magic"),
            )
        }
        _ => return Err("Select one attack style.".into()),
    };
    let mut defence = (def_level as i64 + 9) * (def_bonus as i64 + 64);
    if toa {
        defence = defence * (250 + invocation.min(150) as i64) / 250;
    }
    let (mut attack, mut max_hit) = rule.scale_equipment(set, attack, max_hit);
    if demonbane_spell {
        let percent =
            20 * spell_factor * if settings.mark_darkness { 2 } else { 1 } * vulnerability / 100;
        attack += attack * percent / 100;
    }
    if style != "Magic" {
        attack = attack * accuracy_bonus[0] / accuracy_bonus[1];
        attack += obsidian_accuracy;
        // Shared additive groups are combined in the multiplier stage below.
        if !(rule.has(Effect::ObsidianWeapon) && rules::set_effect(set, "obsidian_armour"))
            && (!target_bonus.is_some_and(|b| b.slayer_task)
                || !rule.weapon.multipliers.iter().any(|m| {
                    m.damage
                        && m.additive_with_task_damage
                        && monster.attributes.contains(&m.attribute)
                }))
        {
            max_hit = (i64::from(max_hit) * damage_bonus[0] / damage_bonus[1]) as i32;
        }
    }
    let (attack, max_hit) = rule
        .family
        .target_scaling
        .as_ref()
        .map(|scaling| scaling.apply(monster, attack, max_hit))
        .unwrap_or((attack, max_hit));
    let (mut attack, mut max_hit) = (attack, max_hit);
    if warden_p2 {
        if let Some(scaling) = &rule.family.target_scaling {
            // The P2 accuracy multiplier applies twice; damage scales once.
            attack = scaling.apply(monster, attack, max_hit).0;
        }
    }
    if rule.has(Effect::Demonbane)
        && monster
            .attributes
            .iter()
            .any(|attribute| attribute == "demon")
    {
        let percent = rule.weapon.demonbane_percent.unwrap_or(0)
            * i64::from(crate::encounters::demonbane_vulnerability(monster.id))
            / 100;
        attack += attack * percent / 100;
        max_hit += (i64::from(max_hit) * percent / 100) as i32;
    }
    let keris =
        rule.has(Effect::KalphiteDamage) && monster.attributes.iter().any(|a| a == "kalphite");
    if keris {
        max_hit = max_hit * rule.weapon.kalphite_damage_percent.unwrap_or(133) / 100;
        if rule.has(Effect::KalphiteAccuracy) {
            attack = attack * 133 / 100;
        }
    }
    for multiplier in &rule.weapon.multipliers {
        if style == "Magic" {
            continue;
        }
        if monster.attributes.contains(&multiplier.attribute) {
            if multiplier.accuracy {
                attack = attack * multiplier.numerator / multiplier.denominator;
            }
            if multiplier.damage {
                max_hit = if multiplier.additive_with_task_damage
                    && target_bonus.is_some_and(|b| b.slayer_task)
                {
                    crate::effects::combined_bonus(
                        max_hit,
                        damage_bonus,
                        [multiplier.numerator, multiplier.denominator],
                    )
                } else {
                    (max_hit as i64 * multiplier.numerator / multiplier.denominator) as i32
                };
            }
        }
    }
    if style == "Melee" && attack_type == "crush" {
        let weight: i64 = rules::registry()
            .equipment_effects
            .iter()
            .filter(|e| {
                e.effect == "inquisitor"
                    && e.slot
                        .is_some_and(|slot| set[slot].is_some_and(|id| e.ids.contains(&id)))
            })
            .map(|e| e.weight)
            .sum();
        attack = attack * (200 + weight) / 200;
        max_hit = (i64::from(max_hit) * (200 + weight) / 200) as i32;
    }
    let always_max = always_max || crate::monster_rules::guaranteed_max(monster.id, attack_type);
    let mut hit = accuracy(attack, defence);
    let brimstone = style == "Magic" && rules::equipped_effect(set, "brimstone_accuracy");
    if brimstone {
        hit = 0.75 * hit + 0.25 * accuracy(attack, defence * 9 / 10);
    }
    let fang = rule.has(Effect::Fang);
    if fang && attack_type == "stab" {
        hit = if toa {
            1.0 - (1.0 - hit).powi(2)
        } else {
            fang_accuracy(attack, defence)
        };
    }
    if style == "Magic"
        && rules::equipped_effect(set, "confliction_accuracy")
        && crate::planner::gear(weapon).is_some_and(|g| !g.two_handed)
        && !always_max
    {
        // The ring rolls independently each cast. Average the unbuffed and
        // after-miss transition probabilities before solving the steady state.
        let double = if brimstone {
            0.75 * fang_accuracy(attack, defence) + 0.25 * fang_accuracy(attack, defence * 9 / 10)
        } else {
            fang_accuracy(attack, defence)
        };
        hit = double / (1.0 + double - hit);
    }
    if crate::monster_rules::guaranteed_accuracy(monster.id, attack_type) {
        hit = 1.0;
    }
    let mut min_hit = if fang { max_hit * 3 / 20 } else { 0 };
    if fang {
        max_hit -= min_hit;
    }
    let respiratory = crate::monster_rules::profile(monster.id)
        .is_some_and(|p| p.kind == crate::monster_rules::Kind::Respiratory);
    let respiratory_finisher = respiratory
        && (rule.has(Effect::Demonbane)
            || demonbane_spell
            || rule
                .weapon
                .multipliers
                .iter()
                .any(|m| m.attribute == "demon"));
    if respiratory_finisher {
        max_hit = monster.skills.get("hp").copied().unwrap_or(50);
        min_hit = max_hit;
    } else if respiratory {
        min_hit += max_hit / 2;
    }
    if warden_p2 {
        (min_hit, max_hit) = crate::effects::wardens_p2_bounds(attack, defence, max_hit);
        hit = 1.0;
    }
    let splat_base_max = max_hit;
    let mut damage_factor = [1, 1];
    if style == "Melee" && rules::set_effect(set, "dharok") {
        let hp = member
            .level("Hitpoints")
            .filter(|hp| (1..=99).contains(hp))
            .ok_or("Enter your base Hitpoints level for Dharok's set (1–99).")?;
        let current = settings.current_hitpoints.unwrap_or(hp);
        damage_factor = [
            (10_000 + (i64::from(hp) - i64::from(current)) * i64::from(hp)).max(0),
            10_000,
        ];
    }
    if rule.has(Effect::ObsidianWeapon) && rules::equipped_effect(set, "berserker_necklace") {
        damage_factor = [6, 5];
    }
    let frostweaver = style == "Magic"
        && rules::set_effect(set, "blue_moon")
        && settings.blue_moon_melee != "Off"
        && rules::spells()
            .iter()
            .any(|r| r.name == spell && r.frostweaver_percent > 0)
        && !hit_rule.is_some_and(|r| !r.allowed_styles.iter().any(|s| s == "Melee"))
        && crate::monster_rules::check_weapon(monster.id, "Melee", weapon).is_ok();
    let twinflame = style == "Magic"
        && rule.has(Effect::Twinflame)
        && rules::spells().iter().any(|r| {
            r.name == spell
                && r.family == "wind"
                && ["Bolt", "Blast", "Wave"]
                    .iter()
                    .any(|suffix| r.name.ends_with(suffix))
        });
    if twinflame && (always_max || warden_p2) {
        return Err(
            "Twinflame's extra hit in guaranteed-hit and Wardens P2 windows is not verified yet."
                .into(),
        );
    }
    let per_splat_npc = rule.has(Effect::TwinHits)
        || twinflame
        || rule.has(Effect::Macuahuitl)
        || rule.has(Effect::Scythe)
        || frostweaver
        || style == "Ranged" && rules::set_effect(set, "karil_damned");
    let hit_effects = crate::effects::SetHitEffects {
        npc_transform: if per_splat_npc {
            crate::monster_rules::transform_for(monster.id, weapon, attack_type)
        } else {
            Default::default()
        },
        damage_factor,
        critical: style == "Magic" && rules::set_effect(set, "ahrim_damned"),
        extra_half: style == "Ranged" && rules::set_effect(set, "karil_damned"),
        extra_two_fifths: twinflame,
        final_divisor: if damage_divisor > 1
            && (twinflame || style == "Ranged" && rules::set_effect(set, "karil_damned"))
        {
            damage_divisor
        } else {
            1
        },
    };
    let distribution_max = max_hit;
    let (_mean, ceiling, mut hits) = if always_max {
        hit = 1.0;
        conditional_hit(
            max_hit,
            max_hit,
            flat_armour,
            &rule.family.procs,
            monster,
            mark_bonus.filter(|_| hit_rule.is_some_and(|rule| rule.mark_damage)),
            &hit_effects,
        )
    } else {
        conditional_hit(
            min_hit,
            max_hit,
            flat_armour,
            if respiratory_finisher {
                &[]
            } else {
                &rule.family.procs
            },
            monster,
            mark_bonus.filter(|_| !respiratory_finisher),
            &hit_effects,
        )
    };
    let mut distribution: Vec<_> = hits.iter().map(|(damage, p)| (*damage, p * hit)).collect();
    let mut hp_dependent = None;
    if hit < 1.0 {
        distribution.push((0, 1.0 - hit));
    }
    max_hit = ceiling;
    let multi_hit_effects = crate::effects::SetHitEffects {
        npc_transform: hit_effects.npc_transform,
        final_divisor: damage_divisor,
        ..Default::default()
    };
    if rule.has(Effect::Scythe) {
        if warden_p2 {
            return Err("Scythe Wardens P2 hit transforms are not verified.".into());
        }
        let size = crate::equipment_effects::target_size(monster.id)
            .ok_or("Scythe needs verified target size metadata.")?
            .min(3);
        let mut parts = Vec::new();
        for index in 0..size {
            let max = splat_base_max / (1 << index);
            let (_, _, splats) = conditional_hit(
                if always_max { max } else { 0 },
                max,
                flat_armour,
                &[],
                monster,
                None,
                &multi_hit_effects,
            );
            let mut part: Vec<_> = splats.into_iter().map(|(d, p)| (d, p * hit)).collect();
            if hit < 1.0 {
                part.push((0, 1.0 - hit));
            }
            parts.push(part);
        }
        distribution = crate::equipment_effects::combine_hits(&parts);
        max_hit = distribution
            .iter()
            .filter(|(_, p)| *p > 0.0)
            .map(|(d, _)| *d)
            .max()
            .unwrap_or(0);
        if always_max {
            hits = distribution.clone();
        }
        hit = 1.0 - (1.0 - hit).powi(size as i32);
    }
    if style == "Melee" && rules::set_effect(set, "verac") {
        let (_, proc_max, proc_hits) = conditional_hit(
            if always_max { distribution_max + 1 } else { 1 },
            distribution_max + 1,
            flat_armour,
            &[],
            monster,
            None,
            &Default::default(),
        );
        for (_, p) in &mut distribution {
            *p *= 0.75;
        }
        distribution.extend(proc_hits.iter().map(|(d, p)| (*d, *p * 0.25)));
        hit = hit * 0.75 + 0.25;
        max_hit = proc_max.max(max_hit);
        for (_, p) in &mut hits {
            *p *= 0.75;
        }
        hits.extend(proc_hits.into_iter().map(|(d, p)| (d, p * 0.25)));
    }
    let chained_weapon = rule.has(Effect::Macuahuitl);
    if chained_weapon && (warden_p2 || always_max) {
        return Err(
            "Dual macuahuitl guaranteed-hit and special-phase transforms are not verified.".into(),
        );
    }
    let blood_proc = if chained_weapon && rules::set_effect(set, "blood_moon") {
        // 33% per accurate splat; the second splat must pass both accuracy checks.
        0.33 * hit + 0.33 * 0.67 * hit * hit
    } else {
        0.0
    };
    if rule.has(Effect::TwinHits) || chained_weapon {
        let mut parts = Vec::new();
        let twin_effects = crate::effects::SetHitEffects {
            npc_transform: hit_effects.npc_transform,
            final_divisor: damage_divisor,
            ..Default::default()
        };
        max_hit = 0;
        for max in [
            distribution_max / 2,
            distribution_max - distribution_max / 2,
        ] {
            let (_, ceiling, outcomes) = conditional_hit(
                if always_max { max } else { 0 },
                max,
                flat_armour,
                &[],
                monster,
                None,
                &twin_effects,
            );
            max_hit += ceiling;
            let mut part: Vec<_> = outcomes.into_iter().map(|(d, p)| (d, p * hit)).collect();
            if hit < 1.0 {
                part.push((0, 1.0 - hit));
            }
            parts.push(part);
        }
        distribution = parts[0]
            .iter()
            .flat_map(|(a, p)| parts[1].iter().map(move |(b, q)| (a + b, p * q)))
            .collect();
        if chained_weapon {
            // Misses on the first splat suppress the entire second hit. Keep
            // accurate zero-damage first hits eligible for the second roll.
            distribution.clear();
            distribution.push((0, 1.0 - hit));
            let first_max = distribution_max / 2;
            let (_, _, first) = conditional_hit(
                if always_max { first_max } else { 0 },
                first_max,
                flat_armour,
                &[],
                monster,
                None,
                &twin_effects,
            );
            for (a, p) in first {
                for (b, q) in &parts[1] {
                    distribution.push((a + b, p * hit * q));
                }
            }
        }
        if always_max {
            hits = distribution.clone();
        }
        if !chained_weapon {
            hit = 1.0 - (1.0 - hit).powi(2);
        }
    }
    if always_max
        && (hit_effects.critical || hit_effects.extra_half || rules::set_effect(set, "verac"))
    {
        // These encounter windows replace the attack distribution with its
        // maximum after equipment procs, as in the reference calculator.
        distribution = vec![(max_hit, 1.0)];
        hits = vec![(max_hit, 1.0)];
    }
    let mut combined_final_damage = false;
    if frostweaver {
        let chance = rules::spells()
            .iter()
            .find(|r| r.name == spell)
            .map(|r| f64::from(r.frostweaver_percent) / 100.0)
            .unwrap_or(0.0);
        if chance > 0.0 {
            // Frostweaver is conditional on an accurate spell. The free spear
            // strike rolls its own melee accuracy, with the same gear/prayer.
            let melee = calculate_configured(
                member,
                set,
                &settings.blue_moon_melee,
                source_monster,
                "",
                invocation,
                settings,
            )?;
            if damage_divisor > 1 {
                for (damage, _) in &mut hits {
                    *damage /= damage_divisor;
                }
                max_hit /= damage_divisor;
                combined_final_damage = true;
            }
            distribution = vec![(0, 1.0 - hit)];
            for (damage, p) in &hits {
                distribution.push((*damage, p * hit * (1.0 - chance)));
                for (extra, q) in &melee.hit_distribution {
                    distribution.push((damage + extra, p * hit * chance * q));
                }
            }
            max_hit += melee.max_hit;
            if always_max {
                hits = distribution.clone();
            }
        }
    }
    // Collapse equal totals before finite-HP planning. In particular, two
    // independent hammers have many roll pairs but few distinct damage totals.
    let mut totals = BTreeMap::new();
    for (damage, p) in distribution {
        *totals.entry(damage).or_insert(0.0) += p;
    }
    distribution = totals.into_iter().collect();
    if style == "Ranged" && !rule.family.ammo.is_empty() {
        let zcb = rule.has(Effect::Zaryte);
        let chance = rules::bolt_proc_chance(set[13].unwrap_or(0), settings.kandarin_diary);
        match rules::ammunition(set[13].unwrap_or(0)) {
            AmmoEffect::Ruby => {
                if crate::encounters::window(monster.id)
                    .is_some_and(|w| w.resource != crate::encounters::Resource::Hitpoints)
                {
                    return Err("Ruby bolt effects against shield and charge counters are unverified. Choose other supported ammunition.".into());
                }
                let hp = hp as i32;
                let ruby = (hp * if zcb { 22 } else { 20 } / 100).min(if zcb { 110 } else { 100 });
                for (_, p) in &mut distribution {
                    *p *= 1.0 - chance;
                }
                distribution.push((ruby, chance));
                hp_dependent = Some(HpDependentHit {
                    chance,
                    percent: if zcb { 22 } else { 20 },
                    cap: if zcb { 110 } else { 100 },
                    divisor: 1,
                    transform: Default::default(),
                    distribution_entries: 1,
                });
                max_hit = max_hit.max(ruby);
            }
            AmmoEffect::Diamond => {
                let diamond = max_hit * if zcb { 126 } else { 115 } / 100;
                for (_, p) in &mut distribution {
                    *p *= 1.0 - chance;
                }
                let (_, _, proc_hits) =
                    conditional_hit(0, diamond, 0, &[], monster, None, &Default::default());
                distribution.extend(
                    proc_hits
                        .into_iter()
                        .map(|(damage, p)| (damage, p * chance)),
                );
                max_hit = max_hit.max(diamond);
            }
            effect @ (AmmoEffect::Opal
            | AmmoEffect::Pearl
            | AmmoEffect::Dragonstone
            | AmmoEffect::Onyx) => {
                if always_max || warden_p2 {
                    return Err(
                        "This enchanted bolt's special-phase hit transforms are not verified yet."
                            .into(),
                    );
                }
                let immune = (effect == AmmoEffect::Dragonstone
                    && monster
                        .attributes
                        .iter()
                        .any(|a| a == "dragon" || a == "fiery"))
                    || (effect == AmmoEffect::Onyx
                        && monster.attributes.iter().any(|a| a == "undead"));
                if !immune {
                    let ranged = level("Ranged")? as i32;
                    let extra = match effect {
                        AmmoEffect::Opal => ranged / if zcb { 9 } else { 10 },
                        AmmoEffect::Pearl => {
                            let divisor = if monster.attributes.iter().any(|a| a == "fiery") {
                                15
                            } else {
                                20
                            };
                            ranged / (divisor - if zcb { 2 } else { 0 })
                        }
                        AmmoEffect::Dragonstone => ranged * 2 / if zcb { 9 } else { 10 },
                        _ => 0,
                    };
                    let accurate_only =
                        matches!(effect, AmmoEffect::Dragonstone | AmmoEffect::Onyx);
                    let mut next = Vec::new();
                    // Bonus damage precedes accurate-zero conversion. In
                    // particular a successful rolled zero becomes `extra`,
                    // not `extra + 1`. Keep it separate from a rolled one.
                    let p = 1.0 / f64::from(splat_base_max - min_hit + 1);
                    for damage in min_hit..=splat_base_max {
                        next.push((damage.max(1), p * hit * (1.0 - chance)));
                        if effect == AmmoEffect::Onyx {
                            let proc_max = ceiling * if zcb { 132 } else { 120 } / 100;
                            let (_, _, proc_hits) = conditional_hit(
                                0,
                                proc_max,
                                0,
                                &[],
                                monster,
                                None,
                                &Default::default(),
                            );
                            next.extend(
                                proc_hits
                                    .into_iter()
                                    .map(|(d, q)| (d, q * p * hit * chance)),
                            );
                        } else {
                            next.push(((damage + extra).max(1), p * hit * chance));
                        }
                    }
                    if hit < 1.0 {
                        if accurate_only {
                            next.push((0, 1.0 - hit));
                        } else {
                            next.push((0, (1.0 - hit) * (1.0 - chance)));
                            // Opal and pearl can proc on a failed accuracy roll.
                            next.push((extra, (1.0 - hit) * chance));
                        }
                    }
                    distribution = next;
                    max_hit = distribution
                        .iter()
                        .filter(|(_, p)| *p > 0.0)
                        .map(|(d, _)| *d)
                        .max()
                        .unwrap_or(0);
                }
            }
            _ => {}
        }
    }
    crate::monster_rules::apply(
        monster.id,
        weapon,
        attack_type,
        &mut distribution,
        &mut hp_dependent,
        per_splat_npc,
    );
    // NPC limiters can lower or reroll the previously calculated maximum.
    if crate::monster_rules::profile(monster.id).is_some() {
        max_hit = distribution
            .iter()
            .filter(|(_, p)| *p > 0.0)
            .map(|(d, _)| *d)
            .max()
            .unwrap_or(0);
    }
    if damage_divisor > 1
        && hit_effects.final_divisor == 1
        && !rule.has(Effect::TwinHits)
        && !rule.has(Effect::Scythe)
        && !chained_weapon
        && !combined_final_damage
    {
        for (damage, _) in &mut distribution {
            *damage /= damage_divisor;
        }
        max_hit /= damage_divisor;
        if let Some(proc) = &mut hp_dependent {
            proc.divisor = damage_divisor;
        }
    }
    let instant_defeat = crate::monster_rules::instant_defeat(monster.id, weapon);
    if let Some(damage) = instant_defeat {
        if blood_proc > 0.0 {
            return Err(
                "Blood Moon timing when an NPC is defeated by its first hitsplat is unverified."
                    .into(),
            );
        }
        distribution = vec![(damage, 1.0)];
        hits = distribution.clone();
        hp_dependent = None;
        max_hit = damage;
        hit = 1.0;
    }
    let expected = distribution
        .iter()
        .map(|(damage, p)| *damage as f64 * p)
        .sum::<f64>();
    let expected_ticks = f64::from(ticks) - blood_proc;
    let burn_dps = if style == "Ranged"
        && rules::set_effect(set, "eclipse_moon")
        && !crate::moon::burn_immune(monster.id)
        && instant_defeat.is_none()
        && !crate::encounters::window(monster.id).is_some_and(|w| w.invulnerable)
    {
        if crate::encounters::window(monster.id).is_some_and(|w| {
            w.npc_damage.is_some() || w.resource != crate::encounters::Resource::Hitpoints
        }) {
            return Err("Eclipse burn timing and pulse reductions against this special state or non-HP counter are unverified.".into());
        }
        if ![
            crate::encounters::DamageModel::Standard,
            crate::encounters::DamageModel::VerifiedStandard,
            crate::encounters::DamageModel::HalfDamage,
        ]
        .contains(&damage_model)
        {
            return Err(
                "Eclipse burn timing/reduction is unverified for this special attack window."
                    .into(),
            );
        }
        crate::moon::burn_dps(hit, ticks)
    } else {
        0.0
    };
    let burn = if burn_dps > 0.0 {
        let transform = crate::monster_rules::transform_for(monster.id, weapon, attack_type);
        let divisor = if damage_model == crate::encounters::DamageModel::HalfDamage {
            2
        } else {
            1
        };
        let mut outcomes = vec![crate::burn::BurnHit {
            damage: 0,
            probability: 1.0 - hit,
            applies_burn: false,
        }];
        for (damage, p) in &hits {
            for (damage, p) in transform.outcomes(*damage, p * hit) {
                for (applies_burn, chance) in [(false, 0.8), (true, 0.2)] {
                    outcomes.push(crate::burn::BurnHit {
                        damage: damage / divisor as i32,
                        probability: p * chance,
                        applies_burn,
                    });
                }
            }
        }
        Some(crate::burn::BurnAttack {
            hits: outcomes,
            pulse: crate::burn::PulseRule { transform, divisor },
        })
    } else {
        None
    };
    Ok(ResultStats {
        hit_distribution: distribution,
        hp_dependent,
        dps: expected / (expected_ticks * 0.6) + burn_dps,
        burn_dps,
        burn,
        potential_healing_per_second: if style == "Melee" && rules::set_effect(set, "guthan") {
            expected / (ticks as f64 * 0.6) / 4.0
        } else {
            0.0
        },
        accuracy: hit,
        max_hit,
        attack_ticks: ticks,
        expected_attack_ticks: expected_ticks,
        attack_roll: attack,
        defence_roll: defence,
        attack_type: attack_type.into(),
        spell: if style == "Magic" {
            spell.into()
        } else {
            String::new()
        },
        elemental,
        guaranteed_hits: (always_max || instant_defeat.is_some()).then_some(hits),
    })
}
/// Shared single-hit distribution: composable procs, accurate-zero handling, then NPC armour.
fn conditional_hit(
    min: i32,
    max: i32,
    armour: i32,
    procs: &[rules::HitProc],
    monster: &Monster,
    mark_bonus: Option<(i64, i64)>,
    set_effects: &crate::effects::SetHitEffects,
) -> (f64, i32, Vec<(i32, f64)>) {
    if max <= 0 {
        return (0.0, 0, vec![(0, 1.0)]);
    }
    let mut hits: Vec<_> = (min..=max)
        .map(|damage| (damage, 1.0 / (max - min + 1) as f64))
        .collect();
    for before in [true, false] {
        if !before {
            for (damage, _) in &mut hits {
                *damage = (*damage).max(1);
            }
        }
        for proc in procs.iter().filter(|p| {
            p.before_accurate_zero == before
                && p.attribute
                    .as_ref()
                    .is_none_or(|a| monster.attributes.contains(a))
        }) {
            let chance = proc.numerator as f64 / proc.denominator as f64;
            hits = hits
                .into_iter()
                .flat_map(|(damage, probability)| {
                    let changed = match proc.operation {
                        rules::ProcOperation::Add => damage + proc.amount,
                        rules::ProcOperation::Multiply => damage * proc.amount,
                    };
                    [
                        (damage, probability * (1.0 - chance)),
                        (changed, probability * chance),
                    ]
                })
                .collect();
        }
    }
    let hits = crate::effects::set_hit_outcomes(&hits, set_effects);
    let mut mean = 0.0;
    let mut ceiling = 0;
    let mut outcomes = vec![];
    for (splats, probability) in hits {
        let splats: Vec<i32> = splats
            .into_iter()
            .map(|damage| {
                let damage = if let Some((percent, vulnerability)) = mark_bonus {
                    damage + (i64::from(damage) * percent / 100 * vulnerability / 100) as i32
                } else {
                    damage
                };
                (damage - armour).max(0) / set_effects.final_divisor
            })
            .collect();
        for (damage, probability) in
            crate::monster_rules::transform_splats(&splats, probability, set_effects.npc_transform)
        {
            mean += damage as f64 * probability;
            if probability > 0.0 {
                ceiling = ceiling.max(damage);
                outcomes.push((damage, probability));
            }
        }
    }
    (mean, ceiling, outcomes)
}
pub fn fang_accuracy(a: i64, d: i64) -> f64 {
    let a = if a < 0 { (a + 2).min(0) } else { a };
    let d = if d < 0 { (d + 2).min(0) } else { d };
    if a >= 0 && d < 0 {
        return 1.0 - 1.0 / (-d + 1) as f64 / (a + 1) as f64;
    }
    if a < 0 && d >= 0 {
        return 0.0;
    }
    if a < 0 && d < 0 {
        let (a, d) = (-d as f64, -a as f64);
        return (if a < d {
            a * (d * 6.0 - 2.0 * a + 5.0) / (6.0 * (d + 1.0) * (d + 1.0))
        } else {
            1.0 - (d + 2.0) * (2.0 * d + 3.0) / (6.0 * (d + 1.0) * (a + 1.0))
        })
        .clamp(0.0, 1.0);
    }
    let (a, d) = (a as f64, d as f64);
    (if a > d {
        1.0 - (d + 2.0) * (2.0 * d + 3.0) / (6.0 * (a + 1.0) * (a + 1.0))
    } else {
        a * (4.0 * a + 5.0) / (6.0 * (a + 1.0) * (d + 1.0))
    })
    .clamp(0.0, 1.0)
}
/// Long-run single-target accuracy: after a miss, two attack rolls share one
/// defence roll. The first cast and target/spell switches reset this state.
pub fn confliction_accuracy(attack: i64, defence: i64) -> f64 {
    let single = accuracy(attack, defence);
    let double = fang_accuracy(attack, defence);
    (double / (1.0 + double - single)).clamp(0.0, 1.0)
}
