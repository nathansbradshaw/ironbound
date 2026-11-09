//! Continuous prayer use, with no flicking. Encounter drains are accounted for separately.
use crate::{account::Member, combat, defence::Settings, planner::Packed};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Prayer {
    pub name: String,
    pub drain: u32,
    pub level: u32,
    pub defence_level: u32,
    pub style: String,
    pub attack: i64,
    pub strength: i64,
    pub defence: i64,
    #[serde(default)]
    pub magic_defence: i64,
    pub magic_damage: i64,
}
pub fn prayers() -> &'static [Prayer] {
    static DATA: std::sync::OnceLock<Vec<Prayer>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/prayers.json")).expect("prayers")
    })
}
pub fn selected(settings: &Settings) -> &'static Prayer {
    prayers()
        .iter()
        .find(|p| p.name == settings.offensive)
        .unwrap_or(&prayers()[0])
}
pub fn eligible(member: &Member, prayer: &Prayer) -> bool {
    prayer.name == "None"
        || member.level("Prayer").is_some_and(|l| l >= prayer.level)
            && member
                .level("Defence")
                .is_some_and(|l| l >= prayer.defence_level)
}
pub fn eligible_settings(member: &Member, settings: &Settings) -> Result<(), String> {
    if !settings.valid() {
        return Err("Invalid combat settings.".into());
    }
    if !eligible(member, selected(settings)) {
        return Err("Selected boost prayer's skill requirements are unmet or unknown.".into());
    }
    let overhead_level = match settings.protection.as_str() {
        "Magic" => 37,
        "Ranged" => 40,
        "Melee" => 43,
        _ => 0,
    };
    let minimum = overhead_level.max(if settings.preserve { 55 } else { 0 });
    if minimum > 0 && !member.level("Prayer").is_some_and(|l| l >= minimum) {
        return Err(format!("Selected prayers require {minimum} Prayer."));
    }
    Ok(())
}
pub fn drain_per_minute(bonus: i32, settings: &Settings) -> f64 {
    let base = selected(settings).drain as f64
        + if settings.protection != "None" {
            20.0
        } else {
            0.0
        }
        + if settings.preserve { 10.0 / 3.0 } else { 0.0 };
    base * 30.0 / (30 + bonus).max(1) as f64
}
pub fn bonus(set: &[Option<i32>]) -> i32 {
    set.iter()
        .flatten()
        .filter_map(|id| combat::stats(*id))
        .map(|item| item.bonuses.get("prayer").copied().unwrap_or(0))
        .sum()
}
/// Ordinary potion restoration only: no Holy wrench, imbued Ring of the gods,
/// prayer cape, prayer regeneration, Sanfew serum or encounter-specific drain.
pub fn restore_points(level: u32, items: &[Packed]) -> u32 {
    items.iter().fold(0u32, |total, item| {
        let doses: u32 = match item.id {
            2434 | 3024 => 4,
            139 | 3026 => 3,
            141 | 3028 => 2,
            143 | 3030 => 1,
            _ => 0,
        };
        let each = if [3024, 3026, 3028, 3030].contains(&item.id) {
            level / 4 + 8
        } else {
            level / 4 + 7
        };
        total.saturating_add(doses.saturating_mul(each).saturating_mul(item.quantity))
    })
}
pub fn minutes(
    member: &Member,
    set: &[Option<i32>],
    items: &[Packed],
    settings: &Settings,
) -> Option<f64> {
    eligible_settings(member, settings).ok()?;
    let drain = drain_per_minute(bonus(set), settings);
    if drain <= 0.0 {
        return None;
    }
    let level = member.level("Prayer")?;
    Some(level.saturating_add(restore_points(level, items)) as f64 / drain)
}
