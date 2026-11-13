use ironbound::{alternatives::Alternative, tooltip};
use std::collections::BTreeMap;

fn alternative() -> Alternative {
    Alternative {
        id: 11772,
        style: "Magic".into(),
        slot: 12,
        dps_delta: Some(0.0),
        dps_reason: None,
        prayer_delta: 0.41,
        defence_delta: BTreeMap::from([("crush".into(), 8), ("magic".into(), -5)]),
        max_changes: 5,
        fits_budget: true,
        note: None,
    }
}

#[test]
fn tooltip_colors_benefits_and_costs_including_inverse_prayer_drain() {
    tooltip::install_icons(include_str!("../assets/item-icons-pack.json")).unwrap();
    let mut choice = alternative();
    let html = tooltip::render(
        "Ring: Current\nChosen for this phase.",
        Some(11770),
        &[choice.clone()],
        Some(4),
    );
    assert!(html.contains("tooltip-gain\">+8</strong>"));
    assert!(html.contains("tooltip-loss\">-5</strong>"));
    assert!(html.contains("tooltip-loss\">+0.41</strong>"));
    assert!(html.contains("tooltip-neutral\">Same DPS</strong>"));
    assert_eq!(html.matches("data:image/png;base64,").count(), 2);
    assert!(html.contains("data-action=\"pick-gear:Magic:12:11772\""));
    assert!(html.contains("aria-label=\"Equip Warrior ring (i)\""));
    assert!(!html.contains("Green = better") && !html.contains("Red = worse"));
    choice.prayer_delta = -0.2;
    choice.dps_delta = Some(0.123);
    let html = tooltip::render("Current", None, &[choice], None);
    assert!(html.contains("tooltip-gain\">-0.20</strong>"));
    assert!(html.contains("tooltip-gain\">+0.123</strong>"));
}

#[test]
fn tooltip_keeps_unknown_metrics_and_budget_warnings_explicit_and_escapes_text() {
    let mut choice = alternative();
    choice.dps_delta = None;
    choice.fits_budget = false;
    choice.note = Some("<img src=x onerror=alert(1)> & details".into());
    let html = tooltip::render("<script>\nA & B", None, &[choice], Some(4));
    assert!(html.contains("tooltip-neutral\">Unavailable</strong>"));
    assert!(html.contains("Needs 5 changes"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt; &amp; details"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img src=x"));
}

#[test]
fn shared_missing_dps_reason_is_shown_once_and_distinct_failures_stay_local() {
    let mut a = alternative();
    a.dps_delta = None;
    a.dps_reason = Some("Xarth needs protection states <reviewed>.".into());
    let mut b = a.clone();
    b.id = 11770;
    let html = tooltip::render("Weapon", None, &[a.clone(), b.clone()], None);
    assert_eq!(html.matches("Xarth needs protection states").count(), 1);
    assert_eq!(html.matches("DPS comparison unavailable").count(), 1);
    assert!(!html.contains("<reviewed>"));
    assert!(html.contains("&lt;reviewed&gt;"));
    b.dps_reason = Some("Insufficient runes.".into());
    let html = tooltip::render("Weapon", None, &[a, b], None);
    assert!(html.contains("Xarth needs protection states") && html.contains("Insufficient runes."));
    assert!(!html.contains("DPS comparison unavailable"));
}
