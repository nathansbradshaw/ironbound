//! Public synthetic fixtures; no player bank data. Reference formulas:
//! https://github.com/weirdgloop/osrs-dps-calc/blob/main/src/lib/PlayerVsNPCCalc.ts
//! https://github.com/weirdgloop/osrs-dps-calc/blob/main/src/lib/BaseCalc.ts
use ironbound::{account::Member, combat, planner, rules};
fn member() -> Member {
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
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn set(weapon: i32, ammo: Option<i32>) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s[13] = ammo;
    s
}
fn vorkath() -> planner::Monster {
    planner::monsters()
        .iter()
        .find(|m| {
            m.name.starts_with("Vorkath")
                && m.attributes.contains(&"undead".into())
                && m.attributes.contains(&"dragon".into())
        })
        .unwrap()
        .clone()
}
fn calc(s: &[Option<i32>], style: &str, target: &planner::Monster) -> combat::ResultStats {
    combat::calculate(&member(), s, style, target, "").unwrap()
}
fn void(s: &mut [Option<i32>], head: i32, elite: bool) {
    s[0] = Some(head);
    s[4] = Some(if elite { 13072 } else { 8839 });
    s[7] = Some(if elite { 13073 } else { 8840 });
    s[9] = Some(8842);
}
#[test]
fn crossbow_capabilities_accept_real_bolts_not_only_rune_bolts() {
    for weapon in [21012, 25916, 25918, 26374, 21902, 28053, 11785] {
        for ammo in [
            877, 9140, 9141, 9142, 9143, 9144, 11875, 21316, 21905, 9242, 9243, 21944, 21946,
        ] {
            assert!(
                rules::ammo_ids(weapon).contains(&ammo),
                "{weapon} rejected {ammo}"
            );
            let result = calc(&set(weapon, Some(ammo)), "Ranged", &vorkath());
            assert!(result.dps.is_finite() && result.dps > 0.0);
        }
    }
    assert!(!rules::ammo_ids(9185).contains(&21944));
    assert!(rules::all_ammo_ids().contains(&21946));
}
#[test]
fn unmodelled_enchantment_is_not_misreported_as_incompatible() {
    assert!(rules::ammo_ids(26374).contains(&21942));
    let error = combat::calculate(
        &member(),
        &set(26374, Some(21942)),
        "Ranged",
        &vorkath(),
        "",
    )
    .unwrap_err();
    assert!(error.contains("compatible") && error.contains("effect is not modelled"));
}
#[test]
fn dhcb_salve_and_elite_void_stack_with_correct_flooring() {
    let mut s = set(21012, Some(21905));
    let target = vorkath();
    let basic = calc(&s, "Ranged", &target);
    assert_eq!(basic.attack_roll, 22116); // floor(107*(95+64)*1.30)
    assert_eq!(basic.max_hit, 38); // floor(floor((107*186+320)/640)*1.25)
    s[2] = Some(12018);
    let salve = calc(&s, "Ranged", &target);
    assert_eq!(salve.attack_roll, 26539);
    assert_eq!(salve.max_hit, 46);
    void(&mut s, 11664, true);
    let elite = calc(&s, "Ranged", &target);
    assert_eq!(elite.attack_roll, 29019);
    assert_eq!(elite.max_hit, 52);
    void(&mut s, 11664, false);
    let ordinary = calc(&s, "Ranged", &target);
    assert_eq!(ordinary.attack_roll, elite.attack_roll);
    assert_eq!(ordinary.max_hit, 50);
    assert!(elite.dps > ordinary.dps && ordinary.dps > salve.dps);
}
#[test]
fn dragonbane_and_salve_require_their_actual_target_attributes() {
    let mut target = vorkath();
    target.attributes.clear();
    let s = set(21012, Some(21905));
    let base = calc(&s, "Ranged", &target);
    assert_eq!(base.attack_roll, 17013);
    assert_eq!(base.max_hit, 31);
    let mut salve = s.clone();
    salve[2] = Some(12018);
    assert_eq!(
        calc(&salve, "Ranged", &target).attack_roll,
        base.attack_roll
    );
    for neck in [4081, 10588] {
        salve[2] = Some(neck);
        assert_eq!(calc(&salve, "Ranged", &vorkath()).attack_roll, 22116);
    }
}
#[test]
fn void_needs_every_slot_and_matching_style_and_excludes_broken_pieces() {
    let mut s = set(21012, Some(21905));
    void(&mut s, 11664, true);
    for slot in [0, 4, 7, 9] {
        let mut incomplete = s.clone();
        incomplete[slot] = None;
        assert!(!rules::set_effect(&incomplete, "void_ranged"));
    }
    s[9] = Some(31106);
    assert!(!rules::set_effect(&s, "void_ranged"));
    s[9] = Some(8842);
    s[0] = Some(11663);
    assert!(!rules::set_effect(&s, "void_ranged"));
    assert!(rules::set_effect(&s, "elite_void_magic"));
    s[0] = Some(20479);
    assert!(!rules::set_effect(&s, "void_ranged"));
    s[0] = Some(26475);
    s[4] = Some(27003);
    s[7] = Some(26471);
    s[9] = Some(26467);
    assert!(rules::set_effect(&s, "elite_void_ranged"));
}
#[test]
fn void_magic_damage_and_effective_accuracy_match_reviewed_formula() {
    let mut s = set(11905, None);
    void(&mut s, 11663, true);
    let target = planner::monster(11789).unwrap();
    let result = calc(&s, "Magic", target);
    let bonus = combat::bonuses(&s).offensive["magic"];
    assert_eq!(result.attack_roll, (110 * 29 / 20) * i64::from(bonus + 64));
    assert_eq!(result.max_hit, 29); // 28 base plus5% complete elite mage set.
    s[4] = Some(8839);
    assert_eq!(calc(&s, "Magic", target).max_hit, 28);
}
#[test]
fn confliction_uses_two_attack_rolls_sharing_defence_and_stationary_miss_state() {
    for (attack, defence) in [(10, 20), (20, 10), (10, 10), (1, 100)] {
        let count = (attack + 1) as f64;
        let two = (0..=defence)
            .map(|d| 1.0 - (((d.min(attack) + 1) as f64 / count).powi(2)))
            .sum::<f64>()
            / (defence + 1) as f64;
        let one = combat::accuracy(attack, defence);
        let expected = two / (1.0 + two - one);
        assert!((combat::confliction_accuracy(attack, defence) - expected).abs() < 1e-12);
        let mut success = one;
        for _ in 0..10000 {
            success = success * one + (1.0 - success) * two;
        }
        assert!((success - expected).abs() < 1e-12);
    }
    assert_eq!(combat::confliction_accuracy(0, 100), 0.0);
    assert!(combat::confliction_accuracy(10_000, 1) <= 1.0);
}
#[test]
fn confliction_applies_to_one_handed_magic_and_not_shadow_or_ranged() {
    let target = planner::monster(11789).unwrap();
    let mut s = set(11905, None);
    s[9] = Some(31106);
    let magic = calc(&s, "Magic", target);
    assert!(
        (magic.accuracy - combat::confliction_accuracy(magic.attack_roll, magic.defence_roll))
            .abs()
            < 1e-12
    );
    assert!(magic.accuracy > combat::accuracy(magic.attack_roll, magic.defence_roll));
    s[3] = Some(27275);
    let shadow = calc(&s, "Magic", target);
    assert_eq!(
        shadow.accuracy,
        combat::accuracy(shadow.attack_roll, shadow.defence_roll)
    );
    s[3] = Some(21012);
    s[13] = Some(21905);
    let ranged = calc(&s, "Ranged", &vorkath());
    assert_eq!(
        ranged.accuracy,
        combat::accuracy(ranged.attack_roll, ranged.defence_roll)
    );
}
#[test]
fn corrected_requirements_apply_to_real_variants_without_quest_checks() {
    let mut m = member();
    m.level_overrides.insert("Hitpoints".into(), 89);
    assert!(!planner::can_use(31106, &m));
    m.level_overrides.insert("Hitpoints".into(), 90);
    assert!(planner::can_use(31106, &m));
    m.level_overrides.insert("Ranged".into(), 69);
    for id in [21012, 25916, 25918] {
        assert!(!planner::can_use(id, &m));
    }
    m.level_overrides.insert("Ranged".into(), 70);
    for id in [21012, 25916, 25918] {
        assert!(planner::can_use(id, &m));
    }
    m.level_overrides.insert("Attack".into(), 41);
    assert!(!planner::can_use(26475, &m));
}
#[test]
fn demonbane_scales_from_reviewed_target_vulnerability_not_item_specific_branches() {
    let target = planner::monster(14176).unwrap();
    for weapon in [19675, 29589] {
        let s = set(weapon, None);
        let mut ordinary = target.clone();
        ordinary.attributes.clear();
        let base = calc(&s, "Slash", &ordinary);
        let demon = calc(&s, "Slash", target);
        assert_eq!(
            demon.attack_roll,
            base.attack_roll + base.attack_roll * 84 / 100
        );
        assert_eq!(demon.max_hit, base.max_hit + base.max_hit * 84 / 100);
        assert!(demon.dps > base.dps);
    }
}
#[test]
fn every_new_registered_set_piece_has_public_stats_and_the_expected_slot() {
    for set in &rules::registry().equipment_sets {
        for piece in &set.pieces {
            for id in &piece.ids {
                assert_eq!(
                    planner::gear(*id).unwrap().slot,
                    planner::SLOT_KEYS[piece.slot]
                );
                assert!(combat::stats(*id).is_some());
            }
        }
    }
}

#[test]
fn brimstone_and_confliction_compose_instead_of_overwriting_each_other() {
    let mut s = set(11905, None);
    s[9] = Some(31106);
    s[12] = Some(22975);
    let result = calc(&s, "Magic", planner::monster(11789).unwrap());
    let (a, d) = (result.attack_roll, result.defence_roll);
    let single = 0.75 * combat::accuracy(a, d) + 0.25 * combat::accuracy(a, d * 9 / 10);
    let double = 0.75 * combat::fang_accuracy(a, d) + 0.25 * combat::fang_accuracy(a, d * 9 / 10);
    assert!((result.accuracy - double / (1.0 + double - single)).abs() < 1e-12);
    assert!(result.accuracy > combat::confliction_accuracy(a, d));
}

#[test]
fn melee_void_and_imbued_magic_salve_use_their_own_formulas() {
    let target = vorkath();
    let mut melee = set(4151, None);
    void(&mut melee, 11665, false);
    melee[2] = Some(10588);
    let r = calc(&melee, "Melee", &target);
    let b = combat::bonuses(&melee);
    assert_eq!(
        r.attack_roll,
        ((110 * 11 / 10) * i64::from(b.offensive["slash"] + 64)) * 6 / 5
    );
    assert_eq!(
        r.max_hit,
        ((((107 * 11 / 10) * i64::from(b.other["str"] + 64) + 320) / 640) * 6 / 5) as i32
    );
    let mut magic = set(11905, None);
    magic[2] = Some(12018);
    magic[9] = Some(31106);
    let result = calc(&magic, "Magic", &target);
    assert_eq!(result.max_hit, 28 + 28 * 270 / 1000); //7%gauntlets +20%Salve, additive.
    let attack = 110 * i64::from(combat::bonuses(&magic).offensive["magic"] + 64);
    assert_eq!(result.attack_roll, attack * 6 / 5);
    magic[2] = Some(10588);
    let unimbued = calc(&magic, "Magic", &target);
    assert_eq!(unimbued.attack_roll, attack);
    assert_eq!(unimbued.max_hit, 28 + 28 * 70 / 1000);
}
