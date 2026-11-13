use ironbound::{
    account, combat,
    defence::Settings,
    effects,
    planner::{self, Boss, Options, Target},
    rules,
};
#[test]
fn independently_sourced_wiki_golden_cases() {
    let suite: ironbound::validation::ReferenceSuite =
        serde_json::from_str(include_str!("fixtures/wiki-reference.json")).unwrap();
    assert_eq!(suite.cases.len(), 24);
    let errors: Vec<_> = suite
        .cases
        .iter()
        .filter_map(|case| case.check().err())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
fn member() -> account::Member {
    let mut m = account::Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
#[test]
fn target_scaling_caps_input_and_uses_offensive_magic_and_xerician_cap() {
    let curve = rules::weapon(20997)
        .unwrap()
        .family
        .target_scaling
        .as_ref()
        .unwrap();
    let mut target = planner::monster(3127).unwrap().clone();
    target.skills.insert("magic".into(), 0);
    target.offensive.insert("magic".into(), 250);
    assert_eq!(curve.stat(&target), 250);
    assert_eq!(curve.apply(&target, 10000, 100), (14000, 215));
    target.offensive.insert("magic".into(), 1000);
    assert_eq!(curve.stat(&target), 250);
    target.attributes.push("xerician".into());
    assert_eq!(curve.stat(&target), 350);
    assert_eq!(curve.apply(&target, 10000, 100), (14000, 248));
    // All truncation boundaries matter: above-cap input must not weaken the bow.
    target.offensive.insert("magic".into(), 350);
    assert_eq!(curve.apply(&target, 10000, 100), (14000, 248));
}
#[test]
fn curve_validation_rejects_bad_coefficients_before_calculation() {
    let mut registry: rules::Registry =
        serde_json::from_str(include_str!("../assets/weapon-rules.json")).unwrap();
    registry
        .families
        .get_mut("target_bow")
        .unwrap()
        .target_scaling
        .as_mut()
        .unwrap()
        .accuracy
        .square_divisor = 0;
    assert!(registry.validate().is_err());
}
#[test]
fn ranged_stance_is_real_speed_accuracy_damage_tradeoff_and_old_settings_remain_rapid() {
    let rapid: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(rapid.ranged_stance, "Rapid");
    let mut accurate = rapid.clone();
    accurate.ranged_stance = "Accurate".into();
    let mut set = vec![None; 14];
    set[3] = Some(25865);
    let target = planner::monster(415).unwrap();
    let a =
        combat::calculate_configured(&member(), &set, "Ranged", target, "", 0, &accurate).unwrap();
    let r = combat::calculate_configured(&member(), &set, "Ranged", target, "", 0, &rapid).unwrap();
    assert_eq!((a.attack_roll, a.max_hit, a.attack_ticks), (21120, 29, 5));
    assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (20544, 28, 4));
    assert!(r.dps > a.dps);
    accurate.ranged_stance = "Impossible".into();
    assert!(!accurate.valid());
}
#[test]
fn dragonbane_does_not_apply_to_unrelated_monsters_and_current_levels_override_old_cache() {
    let mut set = vec![None; 14];
    set[3] = Some(22978);
    let mut dragon = planner::monster(8059).unwrap().clone();
    let a = combat::calculate(&member(), &set, "Stab", &dragon, "").unwrap();
    dragon.attributes.clear();
    let b = combat::calculate(&member(), &set, "Stab", &dragon, "").unwrap();
    assert_eq!(a.attack_roll, b.attack_roll * 6 / 5);
    assert_eq!(a.max_hit, b.max_hit * 6 / 5);
    let mut m = member();
    m.level_overrides.insert("Attack".into(), 77);
    assert!(!planner::can_use(22978, &m));
    m.level_overrides.insert("Attack".into(), 78);
    assert!(planner::can_use(22978, &m));
    m.level_overrides.insert("Ranged".into(), 84);
    assert!(!planner::can_use(20997, &m));
    m.level_overrides.insert("Ranged".into(), 85);
    assert!(planner::can_use(20997, &m));
}
#[test]
fn diamond_proc_includes_accurate_zero_without_turning_misses_into_hits() {
    for max in [1, 2, 17, 32, 100] {
        let brute = (0..=max).map(|hit| f64::from(hit.max(1))).sum::<f64>() / f64::from(max + 1);
        assert!((effects::accurate_uniform_mean(max) - brute).abs() < 1e-12);
    }
}
#[test]
fn magic_defence_level_exception_is_data_driven_and_does_not_leak() {
    let mut t = planner::monster(415).unwrap().clone();
    t.skills.insert("def".into(), 200);
    t.skills.insert("magic".into(), 1);
    let mut set = vec![None; 14];
    set[3] = Some(11905);
    let ordinary = combat::calculate(&member(), &set, "Magic", &t, "").unwrap();
    t.id = 9118; // Prifddinas rabbit uses Defence for its magic defence roll.
    let special = combat::calculate(&member(), &set, "Magic", &t, "").unwrap();
    assert_eq!(ordinary.defence_roll, 10 * 64);
    assert_eq!(special.defence_roll, 209 * 64);
}
#[test]
fn recommendations_discover_owned_lance_and_target_scaled_bow() {
    let mut s = account::demo();
    s.members[0] = member();
    s.members[0].name = "Public parity bank".into();
    s.selected = s.members[0].name.clone();
    s.members[0].bank = Some(vec![22978, 1, 11889, 1, 20997, 1, 25865, 1, 11212, 500]);
    for (target, style, wanted) in [
        (8059, "Stab", 22978),
        (3127, "Ranged", 20997),
        (415, "Ranged", 25865),
    ] {
        let o = Options {
            boss: Boss::Custom,
            targets: vec![Target {
                id: target,
                style: style.into(),
            }],
            recommended: Some(true),
            ..Default::default()
        };
        let set = planner::equipment_for_style(&s, &o, style);
        assert_eq!(set[3], Some(wanted), "target{target}");
        if wanted == 20997 {
            assert_eq!(set[13], Some(11212));
            assert_eq!(set[5], None);
        }
    }
}

#[test]
fn task_armour_boosts_require_exact_target_eligibility_and_correct_imbue() {
    let m = member();
    let target = planner::monster(415).unwrap();
    let mut settings = Settings::default();
    settings.slayer_targets.insert(415);
    for (weapon, style, head, should_boost) in [
        (4151, "Melee", 8921, true),
        (11905, "Magic", 8921, false),
        (11905, "Magic", 11865, true),
        (9185, "Ranged", 11865, true),
    ] {
        let mut set = vec![None; 14];
        set[3] = Some(weapon);
        set[0] = Some(head);
        if style == "Ranged" {
            set[13] = Some(9144);
        }
        let before = combat::calculate(&m, &set, style, target, "").unwrap();
        let after =
            combat::calculate_configured(&m, &set, style, target, "", 0, &settings).unwrap();
        assert_eq!(after.dps > before.dps, should_boost, "{style}/{head}");
        let mut unrelated = target.clone();
        unrelated.id = 3127;
        let a = combat::calculate(&m, &set, style, &unrelated, "").unwrap();
        let b =
            combat::calculate_configured(&m, &set, style, &unrelated, "", 0, &settings).unwrap();
        assert_eq!(a.dps, b.dps);
        settings.slayer_targets.insert(100014); // Magic flare is not task eligible.
        let flare = planner::monster(100014).unwrap();
        if style == "Magic" {
            assert_eq!(
                combat::calculate(&m, &set, style, flare, "").unwrap().dps,
                combat::calculate_configured(&m, &set, style, flare, "", 0, &settings)
                    .unwrap()
                    .dps
            );
        }
    }
}
#[test]
fn salve_wins_without_stacking_and_dragonbane_task_damage_is_additive() {
    let mut set = vec![None; 14];
    set[0] = Some(11865);
    set[3] = Some(21012);
    set[13] = Some(21905);
    let target = planner::monster(8059).unwrap();
    let mut task = Settings::default();
    task.slayer_targets.insert(8059);
    let actual =
        combat::calculate_configured(&member(), &set, "Ranged", target, "", 0, &task).unwrap();
    // Base31, Slayer15% + DHCB25% =floor31*1.40=43, not successive floors44.
    assert_eq!(actual.max_hit, 43);
    set[2] = Some(12018);
    let salve = combat::calculate(&member(), &set, "Ranged", target, "").unwrap();
    let both =
        combat::calculate_configured(&member(), &set, "Ranged", target, "", 0, &task).unwrap();
    assert_eq!(both.max_hit, salve.max_hit);
    assert_eq!(both.attack_roll, salve.attack_roll);
    assert_eq!(both.dps, salve.dps);
}
#[test]
fn eligible_owned_slayer_helmet_is_discovered_and_task_metadata_is_sorted() {
    let ids = &ironbound::encounters::mechanics().slayer_monster_ids;
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    let mut s = account::demo();
    s.members[0] = member();
    s.members[0].name = "Public task bank".into();
    s.selected = s.members[0].name.clone();
    s.members[0].bank = Some(vec![4151, 1, 10828, 1, 11865, 1]);
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 415,
            style: "Melee".into(),
        }],
        recommended: Some(true),
        ..Default::default()
    };
    o.combat.slayer_targets.insert(415);
    assert_eq!(
        planner::equipment_for_style(&s, &o, "Melee")[0],
        Some(11865)
    );
}
