//! Independent per-roll reference arithmetic from Wiki PlayerVsNPCCalc;
//! synthetic owned banks test joint discovery separately from damage formulas.
use ironbound::{
    account::{self, Member},
    combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    prayer, rules,
};

fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn bank(ids: &[i32]) -> account::Snapshot {
    account::Snapshot {
        members: vec![Member {
            name: "Armour regression".into(),
            bank: Some(ids.iter().flat_map(|id| [i64::from(*id), 1000]).collect()),
            ..member()
        }],
        selected: "Armour regression".into(),
        fetched_at: String::new(),
        source: "Synthetic".into(),
    }
}
fn set(ids: &[(usize, i32)]) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for (slot, id) in ids {
        s[*slot] = Some(*id);
    }
    s
}
fn options(style: &str) -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 26,
            style: style.into(),
        }],
        recommended: Some(false),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    }
}
fn calculate(
    s: &[Option<i32>],
    style: &str,
    settings: &Settings,
    spell: &str,
) -> combat::ResultStats {
    combat::calculate_configured(
        &member(),
        s,
        style,
        planner::monster(26).unwrap(),
        spell,
        0,
        settings,
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn mean(max: i32, transform: impl Fn(i32) -> i32) -> f64 {
    (0..=max).map(|n| transform(n) as f64).sum::<f64>() / (max + 1) as f64
}

#[test]
fn dharok_scales_each_damage_roll_using_explicit_current_health() {
    let s = set(&[(0, 4716), (3, 4718), (4, 4720), (7, 4722)]);
    let full = calculate(&s, "Slash", &Settings::default(), "");
    assert_eq!(
        (full.attack_roll, full.max_hit, full.attack_ticks),
        (18370, 28, 7)
    );
    let low = calculate(
        &s,
        "Slash",
        &Settings {
            current_hitpoints: Some(1),
            ..Default::default()
        },
        "",
    );
    assert_eq!(low.max_hit, 55);
    close(
        low.dps,
        full.accuracy * mean(28, |n| (n * 19702 / 10000).max(1)) / 4.2,
    );
    assert!(low.dps > full.dps * 1.8);
    for slot in [0, 4, 7] {
        let mut partial = s.clone();
        partial[slot] = None;
        assert_eq!(
            calculate(
                &partial,
                "Slash",
                &Settings {
                    current_hitpoints: Some(1),
                    ..Default::default()
                },
                ""
            )
            .max_hit,
            28
        );
    }
    assert!(
        !Settings {
            current_hitpoints: Some(0),
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Settings {
            current_hitpoints: Some(256),
            ..Default::default()
        }
        .valid()
    );
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(old.current_hitpoints, None);
}

#[test]
fn verac_bypasses_accuracy_on_a_quarter_of_attacks_and_adds_one_damage() {
    let mut s = set(&[(0, 4753), (3, 4755), (4, 4757), (7, 4759)]);
    let r = calculate(&s, "Crush", &Settings::default(), "");
    assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (16060, 24, 5));
    let normal = combat::accuracy(16060, 1216);
    close(r.accuracy, 0.75 * normal + 0.25);
    close(
        r.dps,
        (0.75 * normal * mean(23, |n| n.max(1)) + 0.25 * 12.5) / 3.0,
    );
    s[2] = Some(12851);
    assert_eq!(prayer::bonus(&s), 28);
    s[0] = None;
    assert_eq!(prayer::bonus(&s), 18); // -3 helm and -7 full-set amulet bonus.
}

#[test]
fn karil_and_ahrim_amulet_procs_use_per_roll_floors_and_require_every_piece() {
    let mut karil = set(&[
        (0, 4732),
        (2, 12851),
        (3, 4734),
        (4, 4736),
        (7, 4738),
        (13, 4740),
    ]);
    let r = calculate(&karil, "Ranged", &Settings::default(), "");
    assert_eq!(r.max_hit, 30);
    let expected = (0..=20)
        .map(|n| 0.75 * n.max(1) as f64 + 0.25 * (n.max(1) + (n / 2).max(1)) as f64)
        .sum::<f64>()
        / 21.0;
    close(r.dps, r.accuracy * expected / 1.8);
    karil[2] = None;
    assert_eq!(
        calculate(&karil, "Ranged", &Settings::default(), "").max_hit,
        20
    );
    let mut ahrim = set(&[(0, 4708), (2, 12851), (3, 4710), (4, 4712), (7, 4714)]);
    let r = calculate(&ahrim, "Magic", &Settings::default(), "Wind Wave");
    assert_eq!(r.max_hit, 27);
    close(
        r.dps,
        r.accuracy * (0.75 * mean(21, |n| n.max(1)) + 0.25 * mean(21, |n| (n * 13 / 10).max(1)))
            / 3.0,
    );
    ahrim[2] = None;
    assert_eq!(
        calculate(&ahrim, "Magic", &Settings::default(), "Wind Wave").max_hit,
        21
    );
}

#[test]
fn guthan_exposes_potential_healing_without_promising_actual_recovery() {
    let mut s = set(&[(0, 4724), (3, 4726), (4, 4728), (7, 4730)]);
    let r = calculate(&s, "Stab", &Settings::default(), "");
    close(r.potential_healing_per_second, r.dps / 4.0);
    s[7] = None;
    assert_eq!(
        calculate(&s, "Stab", &Settings::default(), "").potential_healing_per_second,
        0.0
    );
}

#[test]
fn torag_rolls_two_independent_hits_and_applies_armour_to_each_splat() {
    let s = set(&[(3, 4747)]);
    let r = calculate(&s, "Crush", &Settings::default(), "");
    let accuracy = combat::accuracy(r.attack_roll, r.defence_roll);
    assert_eq!(r.max_hit, 23);
    close(
        r.dps,
        accuracy * (mean(11, |n| n.max(1)) + mean(12, |n| n.max(1))) / 3.0,
    );
    close(r.accuracy, 1.0 - (1.0 - accuracy).powi(2));
    let target = planner::monster(13012).unwrap();
    let moon = combat::calculate(&member(), &s, "Crush", target, "").unwrap();
    assert_eq!(moon.max_hit, 23 - 2 * target.defensive["flat_armour"]);
}

#[test]
fn inquisitor_uses_current_piece_weights_only_for_crush_without_a_mace_multiplier() {
    for weapon in [1434, 24417] {
        for pieces in [
            vec![],
            vec![(0, 24419, 1)],
            vec![(4, 24420, 2)],
            vec![(7, 24421, 2)],
            vec![(0, 24419, 1), (4, 24420, 2), (7, 24421, 2)],
        ] {
            let mut s = set(&[(3, weapon)]);
            let mut weight = 0;
            for (slot, id, w) in pieces {
                s[slot] = Some(id);
                weight += w;
            }
            let b = combat::bonuses(&s);
            let attack = 110 * i64::from(b.offensive["crush"] + 64);
            let max = (107 * i64::from(b.other["str"] + 64) + 320) / 640;
            let r = calculate(&s, "Crush", &Settings::default(), "");
            assert_eq!(r.attack_roll, attack * (200 + weight) / 200);
            assert_eq!(r.max_hit, (max * (200 + weight) / 200) as i32);
        }
    }
    let s = set(&[(3, 11889), (0, 24419), (4, 24420), (7, 24421)]);
    let b = combat::bonuses(&s);
    assert_eq!(
        calculate(&s, "Slash", &Settings::default(), "").attack_roll,
        108 * i64::from(b.offensive["slash"] + 64)
    );
}

#[test]
fn obsidian_necklace_stacks_after_the_armour_and_salve_additive_bonus() {
    let s = set(&[(0, 21298), (2, 11128), (3, 6523), (4, 21301), (7, 21304)]);
    let r = calculate(&s, "Stab", &Settings::default(), "");
    assert_eq!(r.max_hit, 27);
    close(r.dps, r.accuracy * mean(23, |n| (n * 6 / 5).max(1)) / 2.4);
    let mut salve = s.clone();
    salve[2] = Some(10588);
    let b = combat::bonuses(&salve);
    let base_attack = 110 * i64::from(b.offensive["stab"] + 64);
    let base_max = (107 * i64::from(b.other["str"] + 64) + 320) / 640;
    let r = calculate(&salve, "Stab", &Settings::default(), "");
    assert_eq!(r.attack_roll, base_attack * 6 / 5 + base_attack / 10);
    assert_eq!(r.max_hit, (base_max * 6 / 5 + base_max / 10) as i32);
}

#[test]
fn complete_barrows_sets_are_discovered_and_health_changes_the_dharok_choice() {
    let s = bank(&[4151, 10828, 10551, 4087, 7462, 4716, 4718, 4720, 4722]);
    for recommended in [true, false] {
        let mut o = options("Slash");
        o.recommended = Some(recommended);
        let healthy = planner::equipment_for_style(&s, &o, "Slash");
        assert!(!rules::set_effect(&healthy, "dharok"));
        o.combat.current_hitpoints = Some(1);
        let low = planner::equipment_for_style(&s, &o, "Slash");
        assert!(rules::set_effect(&low, "dharok"), "{low:?}");
        o.excluded.insert(4716);
        assert!(!rules::set_effect(
            &planner::equipment_for_style(&s, &o, "Slash"),
            "dharok"
        ));
    }
    let s = bank(&[4753, 4755, 4757, 4759, 4151, 10828, 10551, 4087, 7462]);
    let mut o = options("Melee");
    o.targets[0].id = 1673;
    let chosen = planner::equipment_for_style(&s, &o, "Melee");
    assert!(rules::set_effect(&chosen, "verac"), "{chosen:?}");
}

#[test]
fn crystal_weapon_and_full_or_partial_armour_compete_as_joint_loadouts() {
    let s = bank(&[
        20997, 11212, 25865, 23971, 23975, 23979, 27226, 27229, 27232, 7462,
    ]);
    for recommended in [true, false] {
        let mut o = options("Ranged");
        o.targets[0].id = 1672;
        o.recommended = Some(recommended);
        let chosen = planner::equipment_for_style(&s, &o, "Ranged");
        assert_eq!(chosen[3], Some(25865));
        assert_eq!(chosen[4], Some(23975));
        assert_eq!(chosen[7], Some(23979));
        o.gear_overrides.insert("Ranged:0".into(), 27226);
        let partial = planner::equipment_for_style(&s, &o, "Ranged");
        assert_eq!(partial[0], Some(27226));
        assert_eq!(partial[4], Some(23975));
    }
}

#[test]
fn obsidian_weapon_armour_and_necklace_can_beat_a_stat_seed() {
    let s = bank(&[
        4587, 6523, 21298, 21301, 21304, 11128, 10828, 10551, 4087, 7462, 6585,
    ]);
    let o = options("Melee");
    let chosen = planner::equipment_for_style(&s, &o, "Melee");
    assert_eq!(chosen[3], Some(6523));
    assert!(
        rules::set_effect(&chosen, "obsidian_necklace_loadout"),
        "{chosen:?}"
    );
}

#[test]
fn broken_barrows_and_crystal_cannot_unlock_sets_or_weapons() {
    for id in [4890, 4914, 4938, 4866, 4962, 4998, 23973, 23977, 23981] {
        if planner::gear(id).is_some_and(|g| g.slot == "weapon") {
            assert!(rules::weapon(id).is_none());
        }
    }
    let s = bank(&[4716, 4890, 4720, 4722, 4151, 10828, 10551, 4087]);
    let mut o = options("Slash");
    o.combat.current_hitpoints = Some(1);
    let chosen = planner::equipment_for_style(&s, &o, "Slash");
    assert!(!rules::set_effect(&chosen, "dharok"));
    o.gear_overrides.insert("Slash:3".into(), 4151);
    assert_eq!(planner::equipment_for_style(&s, &o, "Slash")[3], Some(4151));
}
