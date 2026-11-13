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
    result
}
#[derive(Clone, Debug)]
pub struct ResultStats {
    pub dps: f64,
    pub accuracy: f64,
    pub max_hit: i32,
    pub attack_ticks: i32,
    pub attack_roll: i64,
    pub defence_roll: i64,
    pub attack_type: String,
    pub spell: String,
    pub elemental: Option<ElementalStats>,
    /// Guaranteed-hit windows retain proc probabilities for exact alternating cycles.
    pub guaranteed_hits: Option<Vec<(i32, f64)>>,
}
#[derive(Clone, Debug)]
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
        hp += hp * (4 * invocation.min(150) as i32 / 10) / 100;
        if hp > 100 {
            let round = if hp > 300 { 10 } else { 5 };
            hp = (hp + round / 2) / round * round;
        }
    }
    Some(hp)
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
    let stats = stats(weapon).ok_or("Weapon stats unavailable.")?;
    let rule =
        rules::weapon(weapon).ok_or("This weapon's combat mechanics are not modelled yet.")?;
    if !rule.permits(style) {
        return Err("This weapon does not support the selected attack style.".into());
    }
    let requested_style = style;
    let style = rules::base_style(style);
    if style == "Ranged" && !settings.ranged_type.permits_weapon(weapon) {
        return Err(format!(
            "Choose a {} ranged weapon for this plan.",
            settings.ranged_type.label()
        ));
    }
    if style == "Magic" {
        if let Some(recipe) = rules::spells().iter().find(|recipe| recipe.name == spell) {
            if recipe.family != rule.weapon.family {
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
    let damage_bonus = target_bonus.map(|b| b.damage).unwrap_or([1, 1]);
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
            if void {
                effective_attack = effective_attack * 11 / 10;
                effective_strength = effective_strength * 11 / 10;
            }
            let a = effective_attack * (get(&b.offensive, kind) as i64 + 64);
            let max = (effective_strength * (get(&b.other, "str") as i64 + 64) + 320) / 640;
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
            let mut strength = level("Ranged")? * (100 + prayer_strength) / 100 + 8 + stance_bonus;
            if rules::set_effect(set, "elite_void_ranged") {
                strength = strength * 9 / 8;
            } else if void {
                strength = strength * 11 / 10;
            }
            let max = (strength * (get(&b.other, "ranged_str") as i64 + 64) + 320) / 640;
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
                        .find(|r| r.name == spell && r.family == rule.weapon.family)
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
                        .find(|recipe| recipe.name == spell && recipe.family == rule.weapon.family)
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
            let mut a = base_a * accuracy_bonus[0] / accuracy_bonus[1];
            if rule.family.formula == Formula::Elemental {
                let element = rules::spells()
                    .iter()
                    .find(|r| r.name == spell)
                    .and_then(|r| r.element.as_deref())
                    .ok_or("Spell element unavailable.")?;
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
        // Shared additive groups are combined in the multiplier stage below.
        if !target_bonus.is_some_and(|b| b.slayer_task)
            || !rule.weapon.multipliers.iter().any(|m| {
                m.damage && m.additive_with_task_damage && monster.attributes.contains(&m.attribute)
            })
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
    let min_hit = if fang { max_hit * 3 / 20 } else { 0 };
    if fang {
        max_hit -= min_hit;
    }
    let (mean, ceiling, hits) = if always_max {
        hit = 1.0;
        conditional_hit(
            max_hit,
            max_hit,
            flat_armour,
            &rule.family.procs,
            monster,
            mark_bonus.filter(|_| hit_rule.is_some_and(|rule| rule.mark_damage)),
        )
    } else {
        conditional_hit(
            min_hit,
            max_hit,
            flat_armour,
            &rule.family.procs,
            monster,
            mark_bonus,
        )
    };
    let mut expected = hit * mean;
    max_hit = ceiling;
    if style == "Ranged" && !rule.family.ammo.is_empty() {
        let zcb = rule.has(Effect::Zaryte);
        let chance = rules::bolt_proc_chance(set[13].unwrap_or(0), settings.kandarin_diary);
        match rules::ammunition(set[13].unwrap_or(0)) {
            AmmoEffect::Ruby => {
                let hp = target_hitpoints(monster, invocation).unwrap_or(1);
                let ruby = (hp * if zcb { 22 } else { 20 } / 100).min(if zcb { 110 } else { 100 });
                expected = expected * (1.0 - chance) + ruby as f64 * chance;
                max_hit = max_hit.max(ruby);
            }
            AmmoEffect::Diamond => {
                let diamond = max_hit * if zcb { 126 } else { 115 } / 100;
                expected = expected * (1.0 - chance)
                    + crate::effects::accurate_uniform_mean(diamond) * chance;
                max_hit = max_hit.max(diamond);
            }
            _ => {}
        }
    }
    Ok(ResultStats {
        dps: expected / (ticks as f64 * 0.6),
        accuracy: hit,
        max_hit,
        attack_ticks: ticks,
        attack_roll: attack,
        defence_roll: defence,
        attack_type: attack_type.into(),
        spell: if style == "Magic" {
            spell.into()
        } else {
            String::new()
        },
        elemental,
        guaranteed_hits: always_max.then_some(hits),
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
    let mut mean = 0.0;
    let mut ceiling = 0;
    let mut outcomes = vec![];
    for (damage, probability) in hits {
        let damage = if let Some((percent, vulnerability)) = mark_bonus {
            damage + (i64::from(damage) * percent / 100 * vulnerability / 100) as i32
        } else {
            damage
        };
        let damage = (damage - armour).max(0);
        mean += damage as f64 * probability;
        if probability > 0.0 {
            ceiling = ceiling.max(damage);
            outcomes.push((damage, probability));
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
