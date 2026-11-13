use ironbound::{
    account::Member,
    defence::{self, Settings},
    planner::{self, Packed},
    prayer,
};
fn member(level: u32) -> Member {
    let mut m = Member::default();
    for skill in ["Defence", "Magic", "Prayer"] {
        m.level_overrides.insert(skill.into(), level);
    }
    m
}
fn set() -> Vec<Option<i32>> {
    vec![None; 14]
}
fn target(id: i32) -> &'static planner::Monster {
    planner::monster(id).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[test]
fn thirty_prayer_bonus_doubles_continuous_duration() {
    let settings = Settings {
        protection: "Melee".into(),
        ..Default::default()
    };
    close(prayer::drain_per_minute(0, &settings), 20.0);
    close(prayer::drain_per_minute(15, &settings), 40.0 / 3.0);
    close(prayer::drain_per_minute(30, &settings), 10.0);
    close(
        prayer::minutes(&member(99), &set(), &[], &settings).unwrap(),
        99.0 / 20.0,
    );
}
#[test]
fn prayer_rates_add_and_preserve_is_one_point_per_eighteen_seconds() {
    let settings = Settings {
        protection: "Melee".into(),
        offensive: "Piety".into(),
        preserve: true,
        ..Default::default()
    };
    close(prayer::drain_per_minute(0, &settings), 190.0 / 3.0);
    let chivalry = Settings {
        offensive: "Chivalry".into(),
        ..Default::default()
    };
    close(prayer::drain_per_minute(0, &chivalry), 20.0);
}
#[test]
fn restoration_uses_doses_and_level_floor_without_phantom_food_points() {
    let items = vec![
        Packed {
            id: 2434,
            quantity: 2,
            purpose: String::new(),
        },
        Packed {
            id: 3026,
            quantity: 1,
            purpose: String::new(),
        },
        Packed {
            id: 385,
            quantity: 4,
            purpose: String::new(),
        },
    ];
    // 74 Prayer: 25 points/prayer dose, 26/super-restore dose.
    assert_eq!(prayer::restore_points(74, &items), 278);
    let settings = Settings {
        protection: "Magic".into(),
        ..Default::default()
    };
    close(
        prayer::minutes(&member(74), &set(), &items, &settings).unwrap(),
        (74.0 + 278.0) / 20.0,
    );
}
#[test]
fn prayer_skill_gates_include_overheads_preserve_and_defence() {
    let mut m = member(40);
    let protection = Settings {
        protection: "Melee".into(),
        ..Default::default()
    };
    assert!(prayer::eligible_settings(&m, &protection).is_err());
    m.level_overrides.insert("Prayer".into(), 54);
    assert!(
        prayer::eligible_settings(
            &m,
            &Settings {
                preserve: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    m.level_overrides.insert("Prayer".into(), 99);
    assert!(
        prayer::eligible_settings(
            &m,
            &Settings {
                offensive: "Piety".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn moon_chain_uses_conditional_hit_probabilities_and_blood_healing_distribution() {
    let r = defence::estimate(
        &member(99),
        &set(),
        target(13011),
        "Chained slash",
        &Settings::default(),
    )
    .unwrap();
    // Independently calculated NPC/player rolls: (258+9)*(100+64), (99+8)*64.
    let p = 1.0 - 6850.0 / (2.0 * 43789.0);
    close(r.hit_chance, p);
    close(
        r.damage_per_second,
        (2.0 * p + 4.0 * p * p + 10.0 * p * p * p) / 3.6,
    );
    // Mean clamp(3*d,30,50), d uniformly 0..20, is 773/21.
    close(
        r.healing_per_second,
        (2.0 * p + 8.0 * p * p + (773.0 / 21.0) * p * p * p) / 3.6,
    );
}
#[test]
fn protection_prayer_does_not_zero_moon_typeless_slash() {
    let m = member(99);
    let a = defence::estimate(
        &m,
        &set(),
        target(13013),
        "Chained slash",
        &Settings::default(),
    )
    .unwrap();
    let b = defence::estimate(
        &m,
        &set(),
        target(13013),
        "Chained slash",
        &Settings {
            protection: "Melee".into(),
            ..Default::default()
        },
    )
    .unwrap();
    close(a.damage_per_second, b.damage_per_second);
}
#[test]
fn tank_armour_can_outweigh_a_modest_outgoing_dps_loss_at_blood_moon() {
    let m = member(99);
    let bare = set();
    let mut tank = set();
    tank[4] = Some(1127);
    tank[7] = Some(1079);
    let a = defence::estimate(
        &m,
        &bare,
        target(13011),
        "Chained slash",
        &Settings::default(),
    )
    .unwrap();
    let b = defence::estimate(
        &m,
        &tank,
        target(13011),
        "Chained slash",
        &Settings::default(),
    )
    .unwrap();
    assert!(b.hit_chance < a.hit_chance);
    assert!(b.damage_per_second < a.damage_per_second);
    assert!(b.healing_per_second < a.healing_per_second);
    assert!(
        defence::score(&m, &tank, target(13011), 13.0, &Settings::default())
            > defence::score(&m, &bare, target(13011), 14.0, &Settings::default())
    );
}
#[test]
fn prayer_preference_can_reward_more_prayer_bonus_without_inventing_incoming_damage() {
    let mut boosted = set();
    boosted[2] = Some(1712);
    let base = set();
    let m = member(99);
    let settings = Settings {
        protection: "Melee".into(),
        prayer_weight: 3,
        ..Default::default()
    };
    assert!(
        defence::score(&m, &boosted, target(11778), 10.0, &settings)
            > defence::score(&m, &base, target(11778), 10.0, &settings)
    );
    let no_weight = Settings {
        prayer_weight: 0,
        ..settings
    };
    close(
        defence::score(&m, &boosted, target(11778), 10.0, &no_weight),
        10.0,
    );
}
#[test]
fn unknown_prayer_interaction_is_unavailable_and_generic_melee_is_not_assumed_crush() {
    let mut generic = target(1676).clone();
    generic.id = 999_998;
    assert!(
        defence::estimate(&member(99), &set(), &generic, "Crush", &Settings::default()).is_ok()
    );
    assert!(
        defence::estimate(
            &member(99),
            &set(),
            &generic,
            "Crush",
            &Settings {
                protection: "Melee".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    generic.attack_styles = vec!["Melee".into()];
    assert!(
        defence::estimate(&member(99), &set(), &generic, "Melee", &Settings::default()).is_err()
    );
}
#[test]
fn graardor_ranged_uses_fifteen_to_thirty_five_not_zero_to_sixty() {
    let r = defence::estimate(
        &member(99),
        &set(),
        target(2215),
        "Ranged",
        &Settings::default(),
    )
    .unwrap();
    close(r.damage_per_second, r.hit_chance * 25.0 / 3.6);
    let blocked = defence::estimate(
        &member(99),
        &set(),
        target(2215),
        "Ranged",
        &Settings {
            protection: "Ranged".into(),
            ..Default::default()
        },
    )
    .unwrap();
    close(blocked.damage_per_second, 0.0);
}
#[test]
fn nex_selected_magic_scenario_retains_chip_and_rounds_each_hit() {
    assert_eq!(defence::attacks(target(11278)), vec!["Magic (Smoke / Ice)"]);
    let r = defence::estimate(
        &member(99),
        &set(),
        target(11278),
        "Magic (Smoke / Ice)",
        &Settings {
            protection: "Magic".into(),
            ..Default::default()
        },
    )
    .unwrap();
    // floor(d/2) averaged over all 34 damage rolls is exactly 8, not 8.25.
    close(r.damage_per_second, r.hit_chance * 8.0 / 2.4);
    assert!(r.damage_per_second > 0.0);
    assert!(
        defence::estimate(
            &member(99),
            &set(),
            target(11278),
            "Ranged",
            &Settings::default()
        )
        .is_err()
    );
}
#[test]
fn magic_defence_floors_seventy_and_thirty_percent_separately() {
    let mut m = member(71);
    m.level_overrides.insert("Defence".into(), 73);
    let r = defence::estimate(&m, &set(), target(1672), "Magic", &Settings::default()).unwrap();
    // floor(71*.7)+floor(73*.3)+8 = 78, whereas combined floor gives 79.
    let attack_roll = 109.0 * 137.0;
    let defence_roll = 78.0 * 64.0;
    close(
        r.hit_chance,
        1.0 - (defence_roll + 2.0) / (2.0 * (attack_roll + 1.0)),
    );
}
#[test]
fn unmodelled_incoming_equipment_passives_remain_blocked() {
    let mut gear = set();
    gear[5] = Some(12817);
    assert!(
        defence::estimate(
            &member(99),
            &gear,
            target(13011),
            "Chained slash",
            &Settings::default()
        )
        .is_err()
    );
    assert!(
        defence::score(
            &member(99),
            &gear,
            target(13011),
            20.0,
            &Settings::default()
        ) <= 0.001
    );
}
