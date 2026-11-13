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
    html.push_str(r#"<div class="tooltip-alternatives"><strong class="tooltip-heading">Owned alternatives</strong><p class="tooltip-context">Key changes vs this item · one attack window</p><p class="tooltip-legend"><span class="tooltip-gain">Green = better</span> · <span class="tooltip-loss">Red = worse</span></p>"#);
    for choice in choices {
        html.push_str(&format!(
            r#"<div class="tooltip-choice"><div class="tooltip-item">{}<strong>{}</strong></div><div class="tooltip-metrics">"#,
            item_icon(choice.id), esc(&planner::item_name(choice.id))
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
        html.push_str("</div>");
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
                r#"<p class="tooltip-warning">{}</p>"#,
                esc(&warning)
            ));
        }
        if let Some(note) = &choice.note {
            let mut compact = note.split_whitespace().collect::<Vec<_>>().join(" ");
            if compact.chars().count() > 140 {
                compact = compact.chars().take(139).collect();
                compact.push('…');
            }
            html.push_str(&format!(r#"<p class="tooltip-note">{}</p>"#, esc(&compact)));
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");
    html
}
