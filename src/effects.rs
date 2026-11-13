//! Pure, staged effects. Parameters live in data; every division truncates at
//! the same boundary as the reference engine. No item IDs belong here.
use crate::planner::Monster;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct SetHitEffects {
    pub npc_transform: crate::monster_rules::Transform,
    pub damage_factor: [i64; 2],
    pub critical: bool,
    pub extra_half: bool,
    pub extra_two_fifths: bool,
    pub final_divisor: i32,
}
impl Default for SetHitEffects {
    fn default() -> Self {
        Self {
            npc_transform: Default::default(),
            damage_factor: [1, 1],
            critical: false,
            extra_half: false,
            extra_two_fifths: false,
            final_divisor: 1,
        }
    }
}
/// Per-roll floors, not a multiplier of max hit or average damage. Retain
/// Karil's second splat so accurate zeros and flat armour apply separately.
pub fn set_hit_outcomes(hits: &[(i32, f64)], effects: &SetHitEffects) -> Vec<(Vec<i32>, f64)> {
    let mut result = Vec::new();
    for &(damage, p) in hits {
        let critical = if effects.critical {
            vec![(damage, 0.75), (damage * 13 / 10, 0.25)]
        } else {
            vec![(damage, 1.0)]
        };
        for (damage, q) in critical {
            let damage =
                (i64::from(damage) * effects.damage_factor[0] / effects.damage_factor[1]) as i32;
            if effects.extra_half {
                result.push((vec![damage.max(1)], p * q * 0.75));
                result.push((vec![damage.max(1), (damage / 2).max(1)], p * q * 0.25));
            } else if effects.extra_two_fifths {
                // Twinflame follows the accurate-zero rule. Its second splat
                // shares the first damage roll and may remain zero.
                let first = damage.max(1);
                result.push((vec![first, first * 2 / 5], p * q));
            } else {
                result.push((vec![damage.max(1)], p * q));
            }
        }
    }
    result
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuadraticPercent {
    pub base: i64,
    pub linear_numerator: i64,
    pub linear_offset: i64,
    pub linear_divisor: i64,
    pub inner_numerator: i64,
    pub inner_divisor: i64,
    pub inner_offset: i64,
    pub square_divisor: i64,
    pub cap: i64,
}
impl QuadraticPercent {
    pub fn valid(&self) -> bool {
        [self.linear_divisor, self.inner_divisor, self.square_divisor]
            .iter()
            .all(|v| *v > 0 && *v <= 10_000)
            && [
                self.base,
                self.linear_numerator,
                self.linear_offset,
                self.inner_numerator,
                self.inner_offset,
                self.cap,
            ]
            .iter()
            .all(|v| (0..=10_000).contains(v))
    }
    pub fn percent(&self, stat: i64) -> i64 {
        let linear = (self.linear_numerator * stat - self.linear_offset) / self.linear_divisor;
        let inner = self.inner_numerator * stat / self.inner_divisor - self.inner_offset;
        (self.base + linear - inner * inner / self.square_divisor).clamp(0, self.cap)
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetScaling {
    pub skill: String,
    pub offensive: String,
    pub cap: i64,
    #[serde(default)]
    pub attribute_caps: BTreeMap<String, i64>,
    pub accuracy: QuadraticPercent,
    pub damage: QuadraticPercent,
}
impl TargetScaling {
    pub fn valid(&self) -> bool {
        !self.skill.is_empty()
            && !self.offensive.is_empty()
            && (1..=1000).contains(&self.cap)
            && self.attribute_caps.values().all(|c| (1..=1000).contains(c))
            && self.accuracy.valid()
            && self.damage.valid()
    }
    pub fn stat(&self, target: &Monster) -> i64 {
        let cap = target
            .attributes
            .iter()
            .filter_map(|a| self.attribute_caps.get(a))
            .copied()
            .max()
            .unwrap_or(self.cap);
        i64::from(
            target
                .skills
                .get(&self.skill)
                .copied()
                .unwrap_or(0)
                .max(target.offensive.get(&self.offensive).copied().unwrap_or(0)),
        )
        .clamp(0, cap)
    }
    pub fn apply(&self, target: &Monster, attack: i64, max: i32) -> (i64, i32) {
        let stat = self.stat(target);
        (
            attack * self.accuracy.percent(stat) / 100,
            (i64::from(max) * self.damage.percent(stat) / 100) as i32,
        )
    }
}
/// The accurate-zero rule applies to successful diamond-bolt proc rolls too.
/// Misses remain outside this conditional distribution.
pub fn accurate_uniform_mean(max: i32) -> f64 {
    if max <= 0 {
        return 0.0;
    }
    f64::from(max) / 2.0 + 1.0 / (f64::from(max) + 1.0)
}

/// Ordered mutually exclusive target bonuses (e.g. Salve before Slayer).
/// The condition and rational factors are data; this evaluator has no item IDs.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetBonus {
    pub equipment_effect: String,
    pub styles: Vec<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub slayer_task: bool,
    pub accuracy: [i64; 2],
    pub damage: [i64; 2],
    #[serde(default)]
    pub magic_damage_per_mille: i32,
}
impl TargetBonus {
    pub fn valid(&self) -> bool {
        !self.equipment_effect.is_empty()
            && !self.styles.is_empty()
            && self
                .styles
                .iter()
                .all(|s| ["Melee", "Ranged", "Magic"].contains(&s.as_str()))
            && self
                .accuracy
                .iter()
                .chain(&self.damage)
                .all(|n| (1..=1000).contains(n))
            && (0..=1000).contains(&self.magic_damage_per_mille)
    }
}
pub fn target_bonus(
    set: &[Option<i32>],
    style: &str,
    target: &Monster,
    settings: &crate::defence::Settings,
) -> Option<&'static TargetBonus> {
    crate::rules::registry()
        .target_bonuses
        .iter()
        .find(|bonus| {
            bonus.styles.iter().any(|s| s == style)
                && bonus
                    .attribute
                    .as_ref()
                    .is_none_or(|a| target.attributes.contains(a))
                && (!bonus.slayer_task
                    || crate::slayer::on_task(settings, target.id)
                        && crate::encounters::slayer_eligible(target.id))
                && crate::rules::equipped_effect(set, &bonus.equipment_effect)
        })
}
/// Add two percentage bonuses sharing a stacking group, truncating only once.
/// Used by ranged dragonbane + Slayer; other target multipliers still multiply.
pub fn combined_bonus(value: i32, a: [i64; 2], b: [i64; 2]) -> i32 {
    let numerator = a[0] * b[1] + (b[0] - b[1]) * a[1];
    (i64::from(value) * numerator / (a[1] * b[1])) as i32
}

/// Pinned Wiki P2 shield formula: guaranteed accuracy, with accuracy roll
/// contributing an integer 15–40% damage band and a further 20% upper range.
pub fn wardens_p2_bounds(attack_roll: i64, defence_roll: i64, base_max: i32) -> (i32, i32) {
    let delta = attack_roll.saturating_sub(defence_roll / 3).max(0);
    let percent = (15 + delta.saturating_mul(25) / 42_000).clamp(15, 40);
    let base_max = base_max.max(0);
    (
        (i64::from(base_max) * percent / 100) as i32,
        (i64::from(base_max) * (percent + 20) / 100) as i32,
    )
}
