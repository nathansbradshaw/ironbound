//! Escaped, structured item comparisons. Icons come from the bundled public pack.
use crate::{alternatives::Alternative, planner};
use std::{collections::BTreeMap, sync::OnceLock};

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn item_icon(id: i32) -> String {
    static ICONS: OnceLock<BTreeMap<i32, String>> = OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/item-icons-pack.json"))
            .expect("public icon pack")
    });
    match icons.get(&id) {
        Some(data) => {
            format!(r#"<img src="data:image/png;base64,{data}" alt="" width="32" height="32">"#)
        }
        None => "<span class=\"unknown-item\" aria-hidden=\"true\">?</span>".into(),
    }
}

fn stat_icon(kind: &str) -> String {
    let path = match kind {
        "dps" | "slash" => "M5 19 19 5M13 5h6v6M5 13l6 6M4 20l3-3",
        "stab" => "M5 19 18 6l1-3-3 1L3 17m2-4 6 6",
        "crush" => "m5 19 9-9m-4-4 5-3 6 6-3 5zM3 17l4 4",
        "magic" => "m5 19 9-9m0-7 1 4 4 1-4 1-1 4-1-4-4-1 4-1zM4 4v4M2 6h4",
        "ranged" => "M5 3c17 5 17 13 0 18L9 12 5 3m4 9h12m-4-4 4 4-4 4",
        _ => "M10 3h4v6h6v4h-6v8h-4v-8H4V9h6z",
    };
    format!(
        r#"<svg class="tooltip-stat-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="{path}"/></svg>"#
    )
}

fn metric(kind: &str, label: &str, value: &str, benefit: f64) -> String {
    let tone = if benefit > 0.0 {
        "gain"
    } else if benefit < 0.0 {
        "loss"
    } else {
        "neutral"
    };
    let wide = if matches!(kind, "dps" | "prayer") {
        " tooltip-metric-wide"
    } else {
        ""
    };
    format!(
        r#"<span class="tooltip-metric{wide}">{}<span>{}</span><strong class="tooltip-{tone}">{}</strong></span>"#,
        stat_icon(kind),
        esc(label),
        esc(value)
    )
}

pub fn render(
    base: &str,
    current: Option<i32>,
    choices: &[Alternative],
    cap: Option<usize>,
) -> String {
    let (title, reason) = base.split_once('\n').unwrap_or((base, ""));
    let mut html = format!(
        r#"<div class="tooltip-item tooltip-current">{}<strong>{}</strong></div><p class="tooltip-reason">{}</p>"#,
        current.map(item_icon).unwrap_or_default(),
        esc(title),
        esc(reason)
    );
    if choices.is_empty() {
        return html;
    }
    html.push_str(r#"<div class="tooltip-alternatives"><strong class="tooltip-heading">Owned alternatives</strong><p class="tooltip-context">Key changes vs this item · one attack window</p>"#);
    for choice in choices {
        html.push_str(&format!(
            r#"<button type="button" class="tooltip-choice" data-action="pick-gear:{}:{}:{}" data-slot="{}" aria-label="Equip {}"><span class="tooltip-item">{}<strong>{}</strong><span class="tooltip-equip-label">Equip</span></span><span class="tooltip-metrics">"#,
            esc(&choice.style), choice.slot, choice.id, choice.slot,
            esc(&planner::item_name(choice.id)), item_icon(choice.id), esc(&planner::item_name(choice.id))
        ));
        let (dps, benefit) = match choice.dps_delta {
            Some(delta) if delta.abs() < 0.0005 => ("Same DPS".into(), 0.0),
            Some(delta) => (format!("{delta:+.3}"), delta),
            None => ("Unverified".into(), 0.0),
        };
        html.push_str(&metric("dps", "DPS", &dps, benefit));
        // Keep the existing two-highlight policy, preserving both sides of a tradeoff.
        let mut defence: Vec<_> = choice
            .defence_delta
            .iter()
            .filter(|(_, delta)| **delta != 0)
            .collect();
        defence.sort_by_key(|(_, delta)| std::cmp::Reverse(delta.abs()));
        let gain = defence.iter().copied().find(|(_, delta)| **delta > 0);
        let loss = defence.iter().copied().find(|(_, delta)| **delta < 0);
        let highlights = if let (Some(gain), Some(loss)) = (gain, loss) {
            vec![gain, loss]
        } else {
            defence.into_iter().take(2).collect()
        };
        for (kind, delta) in highlights {
            let label = match kind.as_str() {
                "stab" => "Stab def",
                "slash" => "Slash def",
                "crush" => "Crush def",
                "magic" => "Magic def",
                "ranged" => "Ranged def",
                _ => "Defence",
            };
            html.push_str(&metric(
                kind,
                label,
                &format!("{delta:+}"),
                f64::from(*delta),
            ));
        }
        if !choice.defence_delta.is_empty() && choice.prayer_delta.abs() >= 0.005 {
            html.push_str(&metric(
                "prayer",
                "Prayer drain / min",
                &format!("{:+.2}", choice.prayer_delta),
                -choice.prayer_delta,
            ));
        }
        html.push_str("</span>");
        if !choice.fits_budget {
            let warning = if cap.is_some_and(|cap| choice.max_changes > cap) {
                format!(
                    "Needs {} changes · exceeds your switch limit",
                    choice.max_changes
                )
            } else {
                "Rebuild setup / check bag space".into()
            };
            html.push_str(&format!(
                r#"<span class="tooltip-warning">{}</span>"#,
                esc(&warning)
            ));
        }
        if let Some(note) = &choice.note {
            let mut compact = note.split_whitespace().collect::<Vec<_>>().join(" ");
            if compact.chars().count() > 140 {
                compact = compact.chars().take(139).collect();
                compact.push('…');
            }
            html.push_str(&format!(
                r#"<span class="tooltip-note">{}</span>"#,
                esc(&compact)
            ));
        }
        html.push_str("</button>");
    }
    html.push_str("</div>");
    html
}

/// Keep interactive comparisons beside their source, or scroll them into the
/// available vertical space. Clamping a full-size panel can cover its trigger
/// and intercept the click that should open the equipment/inventory picker.
#[derive(Debug)]
pub struct Position {
    pub left: f64,
    pub top: f64,
    pub max_height: f64,
}
pub fn position(anchor: (f64, f64, f64, f64), viewport: (f64, f64), size: (f64, f64)) -> Position {
    let (x, y, w, h) = anchor;
    let (vw, vh) = viewport;
    let (tw, th) = size;
    let gap = 8.0;
    let screen_height = (vh - gap * 2.0).max(1.0);
    let right = x + w + gap;
    let left = x - tw - gap;
    if right + tw <= vw - gap || left >= gap {
        let max_height = th.min(screen_height);
        return Position {
            left: if right + tw <= vw - gap { right } else { left },
            top: y.clamp(gap, (vh - max_height - gap).max(gap)),
            max_height,
        };
    }
    let above = (y - gap * 2.0).max(1.0);
    let below = (vh - y - h - gap * 2.0).max(1.0);
    let use_above = above >= th || above >= below;
    let max_height = th.min(if use_above { above } else { below });
    Position {
        left: (x + (w - tw) / 2.0).clamp(gap, (vw - tw - gap).max(gap)),
        top: if use_above {
            (y - gap - max_height).max(gap)
        } else {
            y + h + gap
        },
        max_height,
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    fn assert_accessible(a: (f64, f64, f64, f64), v: (f64, f64), s: (f64, f64)) {
        let p = position(a, v, s);
        assert!(p.left >= 8.0 && p.top >= 8.0);
        assert!(p.left + s.0 <= v.0 - 8.0 + 0.01 && p.top + p.max_height <= v.1 - 8.0 + 0.01);
        assert!(
            p.left >= a.0 + a.2
                || p.left + s.0 <= a.0
                || p.top >= a.1 + a.3
                || p.top + p.max_height <= a.1,
            "comparison must not cover the clickable source: {p:?}"
        );
    }
    #[test]
    fn desktop_prefers_space_beside_the_item() {
        let a = (365.0, 350.0, 48.0, 48.0);
        let p = position(a, (1280.0, 720.0), (360.0, 420.0));
        assert_eq!(p.left, 421.0);
        assert_accessible(a, (1280.0, 720.0), (360.0, 420.0));
        assert_accessible((1100.0, 650.0, 48.0, 48.0), (1280.0, 720.0), (360.0, 420.0));
    }
    #[test]
    fn mobile_limits_height_instead_of_covering_the_item() {
        for y in [20.0, 200.0, 350.0, 700.0] {
            assert_accessible((160.0, y, 48.0, 48.0), (390.0, 844.0), (360.0, 700.0));
        }
        assert_accessible((120.0, 100.0, 48.0, 48.0), (320.0, 320.0), (300.0, 500.0));
    }
}
