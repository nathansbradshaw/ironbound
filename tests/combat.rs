use ironbound::{
    account,
    account::Member,
    combat,
    planner::{self, Boss, Options},
};
fn member(level: u32) -> Member {
    let mut m = Member::default();
    for s in [
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
        "Magic",
        "Prayer",
        "Hitpoints",
    ] {
        m.level_overrides.insert(s.into(), level);
    }
    m
}
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn monster(id: i32) -> &'static planner::Monster {
    planner::monsters().iter().find(|m| m.id == id).unwrap()
}
// Independent reference cases: OSRS Wiki's BasicRolls.test.ts (accurate stance).
#[test]
fn wiki_whip_reference_rolls() {
    for (level, roll, max) in [(1, 1752, 2), (99, 16060, 24)] {
        let r = combat::calculate(&member(level), &set(4151), "Melee", monster(1673), "").unwrap();
        assert_eq!(r.attack_roll, roll);
        assert_eq!(r.max_hit, max);
        assert_eq!(r.attack_ticks, 4);
    }
}
#[test]
fn wiki_trident_reference_rolls() {
    for (level, roll, max) in [(1, 948, 1), (99, 8690, 28)] {
        let r = combat::calculate(&member(level), &set(11905), "Magic", monster(1673), "").unwrap();
        assert_eq!(r.attack_roll, roll);
        assert_eq!(r.max_hit, max);
        assert_eq!(r.attack_ticks, 4);
    }
}
#[test]
fn wind_spell_uses_current_tier_and_elemental_weakness() {
    let r =
        combat::calculate(&member(99), &set(1381), "Magic", monster(1673), "Wind Wave").unwrap();
    assert_eq!(r.max_hit, 30);
    assert_eq!(r.attack_roll, 11988);
    assert_eq!(r.defence_roll, 530);
    assert_eq!(r.attack_ticks, 5);
    let lower =
        combat::calculate(&member(62), &set(1381), "Magic", monster(1673), "Wind Wave").unwrap();
    assert_eq!(lower.max_hit, 25);
}
#[test]
fn negative_bonuses_and_zero_accuracy_remain_finite() {
    for (a, d) in [(0, 0), (-100, 100), (100, -100), (-100, -100), (100, 100)] {
        let p = combat::accuracy(a, d);
        assert!(p.is_finite() && (0.0..=1.0).contains(&p));
    }
    assert_eq!(combat::accuracy(-100, 100), 0.0);
    assert_eq!(combat::accuracy(0, 0), 0.0);
}
#[test]
fn fang_toa_accuracy_and_damage_are_not_normal_sword_math() {
    let r = combat::calculate(&member(99), &set(26219), "Melee", monster(11778), "").unwrap();
    assert_eq!(r.attack_roll, 18590);
    assert_eq!(r.defence_roll, 12816);
    assert!(r.accuracy > 0.85 && r.accuracy < 0.9);
    assert_eq!(r.max_hit, 24);
    assert_eq!(r.attack_ticks, 5);
}
#[test]
fn invocation_scales_defence_roll_without_scaling_base_stats() {
    let m = monster(11778);
    let r0 = combat::calculate_at_invocation(&member(99), &set(26219), "Melee", m, "", 0).unwrap();
    let r150 =
        combat::calculate_at_invocation(&member(99), &set(26219), "Melee", m, "", 150).unwrap();
    assert_eq!(r150.defence_roll, 20505);
    assert!(r150.dps < r0.dps);
    assert_eq!(m.skills["def"], 80);
}
#[test]
fn keris_includes_kalphite_damage_and_breaching_accuracy() {
    let a = combat::calculate(&member(99), &set(25979), "Melee", monster(11719), "").unwrap();
    let b = combat::calculate(&member(99), &set(25981), "Melee", monster(11719), "").unwrap();
    assert_eq!(a.max_hit, 69);
    assert_eq!(a.attack_roll, 13420);
    assert_eq!(b.attack_roll, 17848);
    assert!(b.dps > a.dps);
}
#[test]
fn ruby_procs_depend_on_scaled_target_hp() {
    let mut s = set(9185);
    s[13] = Some(9242);
    let r = combat::calculate(&member(99), &s, "Ranged", monster(11730), "").unwrap();
    assert_eq!(r.max_hit, 100);
    assert_eq!(r.attack_ticks, 5);
    assert!(r.dps > 2.0);
    let mut small = monster(11730).clone();
    small.skills.insert("hp".into(), 100);
    let low = combat::calculate(&member(99), &s, "Ranged", &small, "").unwrap();
    assert!(low.dps < r.dps);
}
#[test]
fn incompatible_ammo_and_unmodelled_phases_return_explanations() {
    let mut s = set(861);
    s[13] = Some(9144);
    assert!(combat::calculate(&member(99), &s, "Ranged", monster(1672), "").is_err());
    let w = planner::monsters()
        .iter()
        .find(|m| m.name.contains("Warden"))
        .unwrap();
    assert!(combat::calculate(&member(99), &set(11905), "Magic", w, "").is_err());
}
#[test]
fn owned_tiers_never_recommend_unowned_items() {
    let bank = account::demo();
    let o = Options {
        boss: Boss::Barrows,
        ..Default::default()
    };
    for style in ["Magic", "Ranged", "Melee"] {
        for slot in 0..14 {
            for item in planner::owned_tiers(&bank, &o, style, slot) {
                assert!(bank.available(false).contains_key(&item.id));
            }
        }
    }
}
#[test]
fn unknown_skill_levels_do_not_produce_dps() {
    assert!(
        combat::calculate(&Member::default(), &set(11905), "Magic", monster(1673), "").is_err()
    );
}
#[test]
fn shadow_applies_its_accuracy_multiplier_and_five_tick_speed() {
    let s = set(27275);
    let normal = combat::calculate(&member(99), &s, "Magic", monster(1673), "").unwrap();
    let raid = combat::calculate(&member(99), &s, "Magic", monster(11789), "").unwrap();
    assert_eq!(normal.attack_roll, 18590);
    assert_eq!(raid.attack_roll, 22440);
    assert_eq!(normal.max_hit, 34);
    assert_eq!(normal.attack_ticks, 5);
}
#[test]
fn breaching_does_not_get_a_bonus_against_non_kalphites() {
    let base = combat::calculate(&member(99), &set(25979), "Melee", monster(11778), "").unwrap();
    let breach = combat::calculate(&member(99), &set(25981), "Melee", monster(11778), "").unwrap();
    assert_eq!(base.attack_roll, breach.attack_roll);
    assert_eq!(base.dps, breach.dps);
}
