use ironbound::{account::Member, combat, defence::Settings, planner, rules};

fn member(level: u32) -> Member {
    let mut m = Member::default();
    for skill in [
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
        "Magic",
        "Prayer",
        "Hitpoints",
    ] {
        m.level_overrides.insert(skill.into(), level);
    }
    m
}
fn set(id: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(id);
    s
}
fn target(id: i32) -> &'static planner::Monster {
    planner::monster(id).unwrap()
}

#[test]
fn registry_is_consistent_with_public_item_catalog() {
    let r = rules::registry();
    r.validate().unwrap();
    for weapon in &r.weapons {
        for id in &weapon.ids {
            assert_eq!(planner::gear(*id).unwrap().slot, "weapon");
            assert!(combat::stats(*id).unwrap().speed > 0);
        }
    }
    for family in r.families.values() {
        for ammo in &family.ammo {
            assert_eq!(planner::gear(*ammo).unwrap().slot, "ammo");
        }
    }
}

#[test]
fn bad_rule_data_does_not_silently_enable_a_weapon() {
    let mut r: rules::Registry =
        serde_json::from_str(include_str!("../assets/weapon-rules.json")).unwrap();
    let first = r.weapons[0].ids[0];
    r.weapons[0].ids.push(first);
    assert!(r.validate().is_err());
    let mut r: rules::Registry =
        serde_json::from_str(include_str!("../assets/weapon-rules.json")).unwrap();
    r.families.get_mut("warped").unwrap().denominator = 0;
    assert!(r.validate().is_err());
}

#[test]
fn weapon_families_enforce_available_attack_types() {
    assert!(rules::weapon(11889).unwrap().permits("Crush"));
    assert!(rules::weapon(11889).unwrap().permits("Slash"));
    assert!(!rules::weapon(25979).unwrap().permits("Slash"));
    assert!(!rules::weapon(4151).unwrap().permits("Stab"));
    assert!(rules::weapon(4587).unwrap().permits("Stab"));
    assert!(combat::calculate(&member(99), &set(4151), "Crush", target(13013), "").is_err());
}

#[test]
fn explicit_types_use_the_weapons_real_stance() {
    // Hasta stab is controlled (+1 Attack/Strength), unlike a partisan's accurate stab.
    let hasta = combat::calculate(&member(99), &set(11889), "Stab", target(11778), "").unwrap();
    assert_eq!(hasta.attack_roll, 16092);
    assert_eq!(hasta.max_hit, 23);
    // Zombie axe crush is aggressive (+3 Strength, no Attack stance bonus).
    let axe = combat::calculate(&member(99), &set(28810), "Crush", target(13013), "").unwrap();
    assert_eq!(axe.attack_roll, 16478);
    assert_eq!(axe.max_hit, 34); // 29 before Blue Moon's -5 flat armour.
}

#[test]
fn fang_slash_keeps_damage_band_but_uses_normal_accuracy() {
    let r = combat::calculate(&member(99), &set(26219), "Slash", target(11778), "").unwrap();
    assert_eq!(r.attack_roll, 14873);
    assert_eq!(r.defence_roll, 19936);
    assert_eq!(r.max_hit, 25);
    let expected = 14873.0 / (2.0 * 19937.0);
    assert!((r.accuracy - expected).abs() < 1e-12);
}

#[test]
fn flat_armour_transforms_each_successful_hit_without_healing_misses() {
    let blue = combat::calculate(&member(99), &set(28810), "Crush", target(13013), "").unwrap();
    // Thirty equiprobable raw rolls 0..29; accurate zero becomes1, then each gains5.
    let conditional = 586.0 / 30.0;
    assert!((blue.dps - blue.accuracy * conditional / 3.0).abs() < 1e-12);
    let eclipse = combat::calculate(&member(99), &set(11889), "Stab", target(13012), "").unwrap();
    // Raw0..23 becomes max(roll-6,0); positive outcomes sum1..17=153.
    assert_eq!(eclipse.max_hit, 17);
    assert!((eclipse.dps - eclipse.accuracy * (153.0 / 24.0) / 2.4).abs() < 1e-12);
}

#[test]
fn melee_flat_armour_does_not_affect_magic() {
    let m = target(13013).clone();
    let mut without = m.clone();
    without.defensive.insert("flat_armour".into(), 0);
    let a = combat::calculate(&member(99), &set(11907), "Magic", &m, "").unwrap();
    let b = combat::calculate(&member(99), &set(11907), "Magic", &without, "").unwrap();
    assert_eq!(a.dps, b.dps);
    assert_eq!(a.max_hit, b.max_hit);
}

#[test]
fn upgraded_partisans_require_80_attack_and_remain_exact_items() {
    let m = member(75);
    assert!(planner::can_use(25979, &m));
    for id in [25981, 27291, 27287, 30891] {
        assert!(!planner::can_use(id, &m));
        assert!(planner::can_use(id, &member(80)));
    }
    let family = rules::weapon(30891).unwrap();
    assert_eq!(family.weapon.ids, vec![30891]);
}

#[test]
fn amascut_stats_are_only_enhanced_inside_toa() {
    let outside = combat::calculate(&member(99), &set(30891), "Stab", target(1673), "").unwrap();
    assert_eq!(outside.attack_roll, 13420); // Outside:58 stab, not108.
    assert_eq!(outside.max_hit, 18);
    let inside = combat::calculate(&member(99), &set(30891), "Stab", target(11719), "").unwrap();
    assert_eq!(inside.attack_roll, 18920);
    assert_eq!(inside.max_hit, 75); // Base22 -> floor(22*1.15)=25; triple-hit ceiling75.
}

#[test]
fn offensive_prayers_change_both_accuracy_and_damage() {
    let piety = Settings {
        offensive: "Piety".into(),
        ..Default::default()
    };
    let melee = combat::calculate_configured(
        &member(99),
        &set(4151),
        "Melee",
        target(1673),
        "",
        0,
        &piety,
    )
    .unwrap();
    assert_eq!(melee.attack_roll, 18834);
    assert_eq!(melee.max_hit, 29);
    let augury = Settings {
        offensive: "Augury".into(),
        ..Default::default()
    };
    let mage = combat::calculate_configured(
        &member(99),
        &set(11907),
        "Magic",
        target(1673),
        "",
        0,
        &augury,
    )
    .unwrap();
    assert_eq!(mage.attack_roll, 10586);
    assert_eq!(mage.max_hit, 29);
}

#[test]
fn cosmetic_variants_share_rules_without_changing_ownership_ids() {
    let plain = combat::calculate(&member(99), &set(4587), "Melee", target(1673), "").unwrap();
    for id in [20000, 28031] {
        let cosmetic = combat::calculate(&member(99), &set(id), "Melee", target(1673), "").unwrap();
        assert_eq!(plain.dps, cosmetic.dps);
    }
    assert_ne!(
        rules::weapon(4587).unwrap().weapon.ids[0],
        rules::weapon(20000).unwrap().weapon.ids[1]
    );
}

#[test]
fn generic_accurate_hit_proc_is_included_in_sanguinesti_damage() {
    // Current Wiki reference: normal33 cap, plus8 with probability1/5 on accurate hits.
    let r = combat::calculate(&member(99), &set(22323), "Magic", target(1673), "").unwrap();
    assert_eq!(r.max_hit, 41);
    let conditional = 16.5 + 1.0 / 34.0 + 1.6;
    assert!((r.dps - r.accuracy * conditional / 2.4).abs() < 1e-12);
    let holy = combat::calculate(&member(99), &set(25731), "Magic", target(1673), "").unwrap();
    assert_eq!(r.dps, holy.dps);
}

#[test]
fn flat_armour_and_procs_compose_for_a_single_hit() {
    let mut moon = target(13013).clone();
    moon.attributes.push("kalphite".into());
    let r = combat::calculate(&member(99), &set(25979), "Stab", &moon, "").unwrap();
    assert_eq!(r.max_hit, 74); // Base18*1.33=>23; triple69, then Blue's +5.
    // Keris outcome distribution is transformed before the accurate-zero floor and flat armour.
    let conditional = 23.0 / 2.0 * 53.0 / 51.0 + 1.0 / 24.0 + 5.0;
    assert!((r.dps - r.accuracy * conditional / 2.4).abs() < 1e-12);
}

#[test]
fn spell_selection_and_supplies_share_the_same_recipes() {
    let mut owned = std::collections::BTreeMap::from([(565, 150), (560, 150)]);
    let staff = set(1381);
    assert_eq!(rules::spell_name(&member(62), &staff, &owned), "Wind Wave");
    assert_eq!(rules::rune_supplies("Wind Wave", 150), vec![(565, 150)]);
    owned.insert(565, 149);
    assert_eq!(rules::spell_name(&member(62), &staff, &owned), "Wind Blast");
    assert!(rules::casting_supported(&member(41), &staff, &owned, 150));
    assert!(!rules::casting_supported(&member(40), &staff, &owned, 150));
    owned.insert(560, 149);
    assert!(!rules::casting_supported(&member(62), &staff, &owned, 150));
}

#[test]
fn fixed_spell_and_powered_staff_supplies_are_distinct() {
    let staff = set(12658);
    let mut owned = std::collections::BTreeMap::from([(560, 150), (554, 750)]);
    assert_eq!(rules::spell_name(&member(50), &staff, &owned), "Iban Blast");
    assert_eq!(
        rules::rune_supplies("Iban Blast", 150),
        vec![(554, 750), (560, 150)]
    );
    assert!(rules::casting_supported(&member(50), &staff, &owned, 150));
    owned.insert(554, 749);
    assert!(!rules::casting_supported(&member(50), &staff, &owned, 150));
    assert_eq!(rules::spell_name(&member(99), &set(11907), &owned), "");
    assert!(rules::rune_supplies("", 150).is_empty());
    assert!(rules::casting_supported(
        &member(75),
        &set(11907),
        &Default::default(),
        150
    ));
    assert!(!rules::casting_supported(
        &member(74),
        &set(11907),
        &Default::default(),
        150
    ));
}

#[test]
fn spell_tiers_and_family_enumeration_are_data_driven() {
    assert_eq!(rules::spell_max_hit("Wind Wave", 61), None);
    assert_eq!(rules::spell_max_hit("Wind Wave", 62), Some(17));
    assert_eq!(rules::spell_max_hit("Wind Wave", 75), Some(20));
    assert_eq!(rules::spell_max_hit("Wind Blast", 53), Some(15));
    assert!(rules::family_ids("partisan").contains(&30891));
    assert!(rules::family_ids("missing").is_empty());
}
