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
        let reason = target.and_then(crate::encounters::limitation)
            .or_else(|| crate::monster_rules::style_limitation(id, style));
        Window {
            name: target.map(|m| m.name.clone()).unwrap_or_else(|| format!("Unknown target {id}")),
            style: style.into(),
            supported: target.is_some() && reason.is_none(),
            reason: reason.unwrap_or_else(|| crate::encounters::window(id).map(|w| w.description.as_str()).unwrap_or(if target.is_some() { "Basic attack model available; this loadout still needs compatible gear, skills and spells." } else { "Target mapping missing." })).into(),
        }
    }).collect();
    if options.boss == Boss::Toa {
        for (id, style) in [
            (11751, "Ranged"),
            (11753, "Magic"),
            (11753, "Ranged"),
            (11755, "Melee"),
            (11761, "Ranged"),
        ] {
            if let Some(target) = planner::monster(id) {
                let reason = crate::encounters::limitation(target);
                rows.push(Window { name: target.name.clone(), style: style.into(), supported: reason.is_none(), reason: reason.unwrap_or("Reviewed outgoing attack window; timing, incoming specials and full-phase workloads remain partial.").into() });
            }
        }
        rows.push(Window { name: "Wardens".into(), style: "Full encounter".into(), supported: false, reason: "Whole-fight scenarios are available separately in Combat. Timing and incoming execution assumptions still need gameplay calibration; outgoing windows alone do not establish a validated full-fight prediction.".into() });
    }
    rows
}
pub fn summary(options: &Options) -> String {
    let rows = windows(options);
    format!(
        "{} / {} basic attack windows",
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
