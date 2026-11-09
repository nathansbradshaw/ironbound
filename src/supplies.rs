//! Potential restoration per carried slot, not a consumption or fight simulation.
use crate::{account::Member, inventory::Category};
use serde::Deserialize;
use std::{cmp::Ordering, collections::BTreeMap, sync::OnceLock};
#[derive(Deserialize)]
pub struct Food {
    pub per_bite: i32,
    pub bites: u32,
    pub delayed: i32,
    pub scaling: String,
}
pub fn food(id: i32) -> Option<&'static Food> {
    static FOODS: OnceLock<BTreeMap<i32, Food>> = OnceLock::new();
    FOODS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../assets/food-healing.json"))
                .expect("public food data")
        })
        .get(&id)
}
pub struct Value {
    pub role: &'static str,
    pub score: Option<u32>,
    pub secondary: u32,
    pub detail: String,
    family: String,
}
impl Value {
    pub fn compare(&self, other: &Self) -> Ordering {
        self.role
            .cmp(other.role)
            .then_with(|| {
                if self.role == "Other effects" {
                    self.family.cmp(&other.family)
                } else {
                    Ordering::Equal
                }
            })
            .then_with(|| other.score.cmp(&self.score))
            .then_with(|| other.secondary.cmp(&self.secondary))
    }
}
pub fn food_healing(id: i32, hp: Option<u32>) -> Option<(u32, u32, u32, u32)> {
    let profile = food(id)?;
    let instant = match profile.scaling.as_str() {
        "anglerfish" => {
            let hp = hp?;
            hp / 10
                + match hp {
                    0..=24 => 2,
                    25..=49 => 4,
                    50..=74 => 6,
                    75..=92 => 8,
                    _ => 13,
                }
        }
        "strawberry" => hp?.saturating_mul(6).div_ceil(100),
        "watermelon" => hp?.saturating_mul(5).div_ceil(100),
        "" => u32::try_from(profile.per_bite).ok()?,
        _ => return None,
    };
    let delayed = profile.delayed.max(0) as u32;
    Some((
        (instant + delayed) * profile.bites,
        instant,
        delayed,
        profile.bites,
    ))
}
pub fn value(category: Category, id: i32, name: &str, member: &Member) -> Value {
    let mut value = Value {
        role: "",
        score: None,
        secondary: 0,
        detail: String::new(),
        family: String::new(),
    };
    if category == Category::Food {
        if let Some((total, instant, delayed, bites)) = food_healing(id, member.level("Hitpoints"))
        {
            value.score = Some(total);
            value.secondary = instant;
            value.detail = format!(
                "{total} HP/slot{}",
                if delayed > 0 {
                    format!(" · {instant} now + {delayed} delayed")
                } else if bites > 1 {
                    format!(" · {bites} bites, {instant} HP each")
                } else {
                    String::new()
                }
            );
        } else {
            value.detail = "Healing unverified at your levels".into();
        }
    } else if category == Category::Potions {
        let name = name.to_lowercase();
        let (family, doses) = name
            .rsplit_once('(')
            .and_then(|(family, q)| {
                q.trim_end_matches(')')
                    .parse::<u32>()
                    .ok()
                    .filter(|q| (1..=4).contains(q))
                    .map(|q| (family.trim(), q))
            })
            .unwrap_or((name.as_str(), 1));
        value.family = family.into();
        value.secondary = doses;
        let prayer = crate::prayer::restore_points(
            member.level("Prayer").unwrap_or(1),
            &[crate::planner::Packed {
                id,
                quantity: 1,
                purpose: String::new(),
            }],
        );
        if prayer > 0 && member.level("Prayer").is_some() {
            value.role = "Prayer restoration";
            value.score = Some(prayer);
            value.detail = format!("{prayer} Prayer/slot · {doses} doses");
        } else if family.contains("saradomin brew")
            || family.contains("guthix rest")
            || family.contains("armadyl brew")
        {
            value.role = "Healing";
            let each = if family.contains("guthix rest") {
                Some(5)
            } else {
                member
                    .level("Hitpoints")
                    .map(|hp| hp * if family.contains("saradomin") { 15 } else { 10 } / 100 + 2)
            };
            value.score = each.map(|v| v * doses);
            value.detail = each
                .map(|v| {
                    format!(
                        "{} HP/slot · {doses} doses{}",
                        v * doses,
                        if family.contains("brew") {
                            " · drains combat stats"
                        } else {
                            ""
                        }
                    )
                })
                .unwrap_or_else(|| format!("{doses} doses · set Hitpoints to compare"));
        } else {
            let plain = family.strip_prefix("divine ").unwrap_or(family);
            if plain.contains("combat") {
                value.role = "Melee boosts";
                let skills = if plain.starts_with("super ") {
                    vec!["Attack", "Strength", "Defence"]
                } else {
                    vec!["Attack", "Strength"]
                };
                let boosts: Option<Vec<_>> = skills
                    .iter()
                    .map(|skill| {
                        member.level(skill).map(|level| {
                            if plain.starts_with("super ") {
                                level * 15 / 100 + 5
                            } else {
                                level * 10 / 100 + 3
                            }
                        })
                    })
                    .collect();
                value.score = boosts.as_ref().map(|values| values.iter().sum());
                value.detail = boosts
                    .map(|values| {
                        format!(
                            "{}/dose · {doses} doses{}",
                            skills
                                .iter()
                                .zip(values)
                                .map(|(skill, n)| format!("+{n} {skill}"))
                                .collect::<Vec<_>>()
                                .join(" / "),
                            if family.starts_with("divine ") {
                                " · costs HP"
                            } else {
                                ""
                            }
                        )
                    })
                    .unwrap_or_else(|| format!("{doses} doses · boost unverified"));
                return value;
            }
            let stat = if plain.contains("attack") {
                Some(("Attack boosts", "Attack"))
            } else if plain.contains("strength") {
                Some(("Strength boosts", "Strength"))
            } else if plain.contains("defence") {
                Some(("Defence boosts", "Defence"))
            } else if plain.contains("ranging") {
                Some(("Ranged boosts", "Ranged"))
            } else if plain.contains("magic potion") {
                Some(("Magic boosts", "Magic"))
            } else {
                None
            };
            if let Some((role, skill)) = stat {
                value.role = role;
                let boost = member.level(skill).map(|level| {
                    if skill == "Magic" {
                        4
                    } else if plain.starts_with("super ") {
                        level * 15 / 100 + 5
                    } else if skill == "Ranged" {
                        level * 10 / 100 + 4
                    } else {
                        level * 10 / 100 + 3
                    }
                });
                value.score = boost;
                value.detail = boost
                    .map(|n| {
                        format!(
                            "+{n} {skill}/dose · {doses} doses{}",
                            if family.starts_with("divine ") {
                                " · costs HP"
                            } else {
                                ""
                            }
                        )
                    })
                    .unwrap_or_else(|| format!("{doses} doses · boost unverified"));
            } else {
                value.role = "Other effects";
                value.score = Some(doses);
                value.detail = format!("{doses} doses/slot · effect not ranked");
            }
        }
    }
    value
}
