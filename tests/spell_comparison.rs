use ironbound::{
    account, combat,
    defence::Settings,
    planner,
    rules::{self, SpellComparison},
};
use std::collections::BTreeMap;
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
#[test]
fn manual_element_and_spell_allow_hypothetical_rune_comparisons() {
    let bank = account::demo();
    let member = bank.selected().unwrap();
    let owned = bank.available(false);
    let target = planner::monster(1673).unwrap();
    let settings = Settings::default();
    let staff = set(1381);
    let choose = |choice: &SpellComparison| -> &'static str {
        rules::comparison_spell_name(member, &staff, &owned, &settings, target, 0, choice)
    };
    assert_eq!(choose(&SpellComparison::Auto), "Wind Wave");
    assert_eq!(
        choose(&SpellComparison::Element("water".into())),
        "Water Wave"
    );
    let wave = choose(&SpellComparison::Spell("Water Wave".into()));
    let strike = choose(&SpellComparison::Spell("Water Strike".into()));
    let a = combat::calculate(member, &staff, "Magic", target, wave).unwrap();
    let b = combat::calculate(member, &staff, "Magic", target, strike).unwrap();
    assert_eq!((a.max_hit, b.max_hit), (20, 8));
    assert!(a.dps > b.dps);
    let reason =
        rules::spell_comparison_requirement(member, &staff, &owned, &settings, wave).unwrap();
    assert!(
        reason.contains("Water rune ×1050") && reason.contains("hypothetical"),
        "{reason}"
    );
    assert_eq!(choose(&SpellComparison::Auto), "Wind Wave");
}
#[test]
fn selection_respects_weapon_families_levels_boosts_and_staff_runes() {
    let mut member = account::demo().selected().unwrap().clone();
    let target = planner::monster(1673).unwrap();
    let owned = BTreeMap::from([(565, 150), (555, 1050)]);
    let staff = set(1381);
    let mut settings = Settings::default();
    assert!(
        rules::spell_comparison_requirement(&member, &staff, &owned, &settings, "Water Wave")
            .is_none()
    );
    assert!(
        rules::spell_comparison_requirement(&member, &staff, &owned, &settings, "Fire Surge")
            .unwrap()
            .contains("95 Magic")
    );
    let powered = set(11907);
    assert_eq!(
        rules::comparison_spell_name(
            &member,
            &powered,
            &owned,
            &settings,
            target,
            0,
            &SpellComparison::Spell("Water Wave".into())
        ),
        ""
    );
    assert!(
        rules::spell_comparison_requirement(&member, &powered, &owned, &settings, "Water Wave")
            .unwrap()
            .contains("compatible")
    );
    member.level_overrides.insert("Magic".into(), 61);
    settings.boosts.magic = "Magic potion".into();
    let name = rules::comparison_spell_name(
        &member,
        &staff,
        &owned,
        &settings,
        target,
        0,
        &SpellComparison::Element("water".into()),
    );
    assert_eq!(name, "Water Wave");
    assert_eq!(
        combat::calculate_configured(&member, &staff, "Magic", target, name, 0, &settings)
            .unwrap()
            .max_hit,
        17
    );
    assert!(
        rules::spell_comparison_requirement(&member, &staff, &owned, &settings, name).is_none()
    );
}
#[test]
fn every_supported_spell_and_prayer_has_a_bundled_game_icon() {
    let icons: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../assets/book-icons.json")).unwrap();
    for name in rules::spells()
        .iter()
        .map(|s| s.name.as_str())
        .chain(
            ironbound::prayer::prayers()
                .iter()
                .filter(|p| p.name != "None")
                .map(|p| p.name.as_str()),
        )
        .chain([
            "Preserve",
            "Protect from Magic",
            "Protect from Missiles",
            "Protect from Melee",
        ])
    {
        assert!(
            icons
                .get(name)
                .is_some_and(|data| data.starts_with("data:image/png;base64,iVBORw0KGgo")),
            "missing PNG for {name}"
        );
    }
}
