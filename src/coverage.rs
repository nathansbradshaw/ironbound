//! Mechanic coverage is independent of bank ownership and successful calculations.
use crate::planner::{self, Boss, Options};

pub const LIMITS: &str = "Basic attack estimates exclude full-fight movement, special attacks, supply consumption and unverified phase mechanics. Supported windows do not establish a complete encounter simulation.";

#[derive(Debug)]
pub struct Window {
    pub name: String,
    pub style: String,
    pub supported: bool,
    pub reason: String,
}
pub fn windows(options: &Options) -> Vec<Window> {
    let mut rows: Vec<_> = crate::recommendation::targets(options).into_iter().map(|(id, style)| {
        let target = planner::monster(id);
        let reason = target.and_then(crate::encounters::limitation);
        Window {
            name: target.map(|m| m.name.clone()).unwrap_or_else(|| format!("Unknown target {id}")),
            style: style.into(),
            supported: target.is_some() && reason.is_none(),
            reason: reason.unwrap_or(if target.is_some() { "Basic attack model available; this loadout still needs compatible gear, skills and spells." } else { "Target mapping missing." }).into(),
        }
    }).collect();
    if options.boss == Boss::Toa {
        rows.push(Window { name: "Wardens".into(), style: "Multiple styles".into(), supported: false, reason: "Packing only. Wardens phase mechanics are not verified; no outgoing DPS is enabled.".into() });
    }
    rows
}
pub fn summary(options: &Options) -> String {
    let rows = windows(options);
    format!(
        "{} / {} selected attack windows have a basic attack model. Full encounter mechanics remain partial.",
        rows.iter().filter(|r| r.supported).count(),
        rows.len()
    )
}
pub fn export_note(options: &Options) -> String {
    let unsupported: Vec<_> = windows(options)
        .into_iter()
        .filter(|r| !r.supported)
        .map(|r| format!("{} ({})", r.name, r.style))
        .collect();
    format!(
        "{} {}{}",
        summary(options),
        LIMITS,
        if unsupported.is_empty() {
            String::new()
        } else {
            format!(" Packing-only windows: {}.", unsupported.join(", "))
        }
    )
}
