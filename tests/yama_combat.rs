//! Public synthetic Yama combat cases. Exact floors follow the Wiki calculator's
//! Demonbane distribution and guaranteed-max blocks, not a blanket damage multiplier.
use ironbound::{account::Member, combat, defence::Settings, encounters, planner, rules};
use std::collections::BTreeMap;
fn member() -> Member {
    let mut m = Member::default();
    for skill in [
        "Attack",
        "Strength",
        "Defence",
        "Magic",
        "Ranged",
        "Prayer",
        "Hitpoints",
    ] {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn staff() -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(29594);
    s
}
fn result(s: &[Option<i32>], id: i32, spell: &str, marked: bool) -> combat::ResultStats {
    combat::calculate_configured(
        &member(),
        s,
        "Magic",
        planner::monster(id).unwrap(),
        spell,
        0,
        &Settings {
            mark_darkness: marked,
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn purging_is_eligible_at_actual_weapon_levels_without_quest_checks() {
    let mut m = member();
    m.level_overrides.insert("Magic".into(), 76);
    assert!(!planner::can_use(29594, &m));
    m.level_overrides.insert("Magic".into(), 77);
    m.level_overrides.insert("Attack".into(), 49);
    assert!(!planner::can_use(29594, &m));
    m.level_overrides.insert("Attack".into(), 50);
    assert!(planner::can_use(29594, &m));
    assert_eq!(
        rules::weapon(29594).unwrap().family.formula,
        rules::Formula::Spell
    );
}
#[test]
fn spell_selection_and_casting_require_the_selected_recipes_real_runes() {
    let m = member();
    let s = staff();
    let mut runes = BTreeMap::from([(554, 1050), (566, 300)]);
    assert_eq!(rules::spell_name(&m, &s, &runes), "Dark Demonbane");
    assert!(rules::casting_supported(&m, &s, &runes, 150));
    assert_eq!(
        rules::rune_supplies("Dark Demonbane", 150),
        vec![(554, 1050), (566, 300)]
    );
    runes.insert(566, 150);
    assert_eq!(rules::spell_name(&m, &s, &runes), "Superior Demonbane");
    runes.insert(554, 450);
    assert_eq!(rules::spell_name(&m, &s, &runes), "Inferior Demonbane");
    runes.insert(566, 149);
    assert!(!rules::casting_supported(&m, &s, &runes, 150));
}
#[test]
fn purging_unmarked_is_accuracy_only_not_free_demonbane_damage() {
    let r = result(&staff(), 14176, "Dark Demonbane", false);
    assert_eq!(r.attack_roll, 16143); //108*(37+64), then+48% (40%*Yama120%).
    assert_eq!(r.max_hit, 33); //30base +10% visible staffdamage; noextra40%damage.
    assert_eq!(r.attack_ticks, 5);
    let superior = result(&staff(), 14176, "Superior Demonbane", false);
    assert_eq!(superior.max_hit, 25);
    let inferior = result(&staff(), 14176, "Inferior Demonbane", false);
    assert_eq!(inferior.max_hit, 17);
}
#[test]
fn marked_yama_accuracy_and_each_hit_use_separate_exact_floors() {
    let r = result(&staff(), 14176, "Dark Demonbane", true);
    assert_eq!(r.attack_roll, 21379); //base10908 +96% (80%*120%).
    assert_eq!(r.max_hit, 52); //33+floor(floor(33*.50)*1.20).
    let mean = (0..=33)
        .map(|raw| {
            let hit = raw.max(1);
            (hit + (hit * 50 / 100) * 120 / 100) as f64
        })
        .sum::<f64>()
        / 34.0;
    assert!((r.dps - r.accuracy * mean / 3.0).abs() < 1e-12);
    assert!(r.dps > result(&staff(), 14176, "Dark Demonbane", false).dps);
}
#[test]
fn demonbane_does_not_cast_on_non_demons_or_use_an_unrelated_spell() {
    let s = staff();
    let non_demon = planner::monster(11730).unwrap();
    assert!(
        combat::calculate(&member(), &s, "Magic", non_demon, "Dark Demonbane")
            .unwrap_err()
            .contains("only target demons")
    );
    assert!(
        combat::calculate(
            &member(),
            &s,
            "Magic",
            planner::monster(14176).unwrap(),
            "Wind Wave"
        )
        .unwrap_err()
        .contains("supported spell")
    );
}
#[test]
fn mark_is_explicit_level_checked_and_backwards_compatible() {
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert!(!old.mark_darkness);
    let marked = Settings {
        mark_darkness: true,
        ..Default::default()
    };
    let mut m = member();
    m.level_overrides.insert("Magic".into(), 58);
    assert!(
        combat::calculate_configured(
            &m,
            &staff(),
            "Magic",
            planner::monster(14176).unwrap(),
            "Inferior Demonbane",
            0,
            &marked
        )
        .unwrap_err()
        .contains("59 Magic")
    );
    let mut runes = BTreeMap::from([(564, 1), (566, 1)]);
    assert!(rules::mark_darkness_supported(&member(), &runes));
    runes.remove(&564);
    assert!(!rules::mark_darkness_supported(&member(), &runes));
    assert_eq!(rules::mark_darkness_supplies(), vec![(564, 1), (566, 1)]);
    let roundtrip: Settings =
        serde_json::from_str(&serde_json::to_string(&marked).unwrap()).unwrap();
    assert!(roundtrip.mark_darkness);
}
#[test]
fn judge_guarantees_maximum_for_its_open_style_without_confliction_rerolls() {
    let mut s = staff();
    s[9] = Some(31106);
    let r = result(&s, 100012, "Dark Demonbane", true);
    assert_eq!(r.accuracy, 1.0);
    assert_eq!(r.max_hit, 35); //Judge's max override excludesMark damage.
    assert!((r.dps - 35.0 / 3.0).abs() < 1e-12);
    let mut melee = vec![None; 14];
    melee[3] = Some(29589);
    let r = combat::calculate(
        &member(),
        &melee,
        "Slash",
        planner::monster(100013).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(r.accuracy, 1.0);
    assert!((r.dps - r.max_hit as f64 / 2.4).abs() < 1e-12);
    assert!(
        combat::calculate(
            &member(),
            &melee,
            "Slash",
            planner::monster(100012).unwrap(),
            ""
        )
        .is_err()
    );
    let mut ranged = vec![None; 14];
    ranged[3] = Some(9185);
    ranged[13] = Some(9144);
    assert!(
        combat::calculate(
            &member(),
            &ranged,
            "Ranged",
            planner::monster(14180).unwrap(),
            ""
        )
        .is_err()
    );
}
#[test]
fn judge_still_preserves_probabilistic_sanguinesti_bonus_instead_of_guaranteeing_proc() {
    let mut s = vec![None; 14];
    s[3] = Some(22323);
    let r = combat::calculate(
        &member(),
        &s,
        "Magic",
        planner::monster(100012).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(r.max_hit, 41);
    assert!((r.dps - (33.0 + 8.0 / 5.0) / 2.4).abs() < 1e-12);
}
#[test]
fn flare_mark_bonus_is_guaranteed_but_nested_floor_is_not_a_blanket_double() {
    assert_eq!(encounters::demonbane_vulnerability(100014), 200);
    let unmarked = result(&staff(), 100014, "Dark Demonbane", false);
    assert_eq!(unmarked.accuracy, 1.0);
    assert_eq!(unmarked.max_hit, 33);
    let marked = result(&staff(), 100014, "Dark Demonbane", true);
    assert_eq!(marked.accuracy, 1.0);
    assert_eq!(marked.max_hit, 65);
    assert!((marked.dps - 65.0 / 3.0).abs() < 1e-12);
    let mut gauntlets = staff();
    gauntlets[9] = Some(31106);
    let r = result(&gauntlets, 100014, "Dark Demonbane", true);
    assert_eq!(r.max_hit, 69);
    assert_eq!(r.accuracy, 1.0);
}
#[test]
fn mark_does_not_modify_powered_staffs_or_melee_demonbane() {
    let mut s = vec![None; 14];
    s[3] = Some(11905);
    let unmarked = result(&s, 14176, "", false);
    let marked = result(&s, 14176, "", true);
    assert_eq!(unmarked.max_hit, marked.max_hit);
    assert_eq!(unmarked.attack_roll, marked.attack_roll);
    s[3] = Some(29589);
    let target = planner::monster(14176).unwrap();
    let plain =
        combat::calculate_configured(&member(), &s, "Slash", target, "", 0, &Settings::default())
            .unwrap();
    let marked = combat::calculate_configured(
        &member(),
        &s,
        "Slash",
        target,
        "",
        0,
        &Settings {
            mark_darkness: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plain.max_hit, marked.max_hit);
    assert_eq!(plain.attack_roll, marked.attack_roll);
}
