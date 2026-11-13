//! Independent goldens from the pinned calculator stages in SOURCES.md.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::{self, Settings},
    encounters,
    modifiers::{self, Reductions},
    planner::{self, Boss, Options, Target},
};
fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn gear(weapon: i32, ammo: Option<i32>) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s[13] = ammo;
    s
}
fn calc(id: i32, set: &[Option<i32>], style: &str, spell: &str) -> combat::ResultStats {
    combat::calculate(&member(), set, style, planner::monster(id).unwrap(), spell).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(r: &combat::ResultStats) -> f64 {
    r.hit_distribution
        .iter()
        .map(|(d, p)| f64::from(*d) * p)
        .sum()
}
#[test]
fn tail_crush_misses_deal_one_without_becoming_accurate() {
    let r = calc(14014, &gear(1434, None), "Crush", "");
    assert_eq!((r.attack_roll, r.defence_roll, r.max_hit), (13640, 8576, 9));
    let p = 1.0 - 8578.0 / 27282.0;
    close(r.accuracy, p);
    // Raw 0..20 rolls: accurate 0 becomes 1; 9..20 all become 9.
    close(r.dps, ((1.0 - p) + p * 145.0 / 21.0) / 2.4);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 1));
    let mut options = Options::default();
    options.combat.target_hp.insert(14014, 1);
    let b = attack_budget::for_window(
        planner::monster(14014).unwrap(),
        &r,
        &options,
        &gear(1434, None),
    )
    .unwrap();
    close(b.attacks, 1.0);
    close(b.seconds, 2.4);
}
#[test]
fn tail_requires_strict_crush_priority_and_a_crush_attack() {
    // Dharok's axe has 103 slash and only 95 crush, so choosing Crush isn't enough.
    let r = calc(14014, &gear(4718, None), "Crush", "");
    assert_eq!(r.max_hit, 4);
    assert!(r.hit_distribution.iter().any(|(d, p)| *d == 0 && *p > 0.0));
    let mut bonuses = combat::Bonuses::default();
    for (kind, n) in [("crush", 60), ("stab", 60), ("slash", 0)] {
        bonuses.offensive.insert(kind.into(), n);
    }
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "crush", &bonuses, "");
    assert_eq!(transform.outcomes(20, 1.0), vec![(4, 1.0)]);
    assert_eq!(transform.outcomes(0, 1.0), vec![(0, 1.0)]);
    bonuses.offensive.insert("stab".into(), 59);
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "crush", &bonuses, "");
    assert_eq!(transform.outcomes(0, 1.0), vec![(1, 1.0)]);
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "stab", &bonuses, "");
    assert_eq!(transform.outcomes(20, 1.0), vec![(4, 1.0)]);
}
#[test]
fn earth_tail_hits_have_nine_cap_but_keep_misses_at_zero() {
    let s = gear(1381, None);
    let earth = calc(14014, &s, "Magic", "Earth Bolt");
    assert_eq!(
        (earth.attack_roll, earth.defence_roll, earth.max_hit),
        (12787, 15576, 9)
    );
    let p = 12787.0 / 31154.0;
    close(earth.accuracy, p);
    close(earth.dps, p * 136.0 / 20.0 / 3.0);
    let fire = calc(14014, &s, "Magic", "Fire Bolt");
    assert_eq!(fire.max_hit, 4);
    close(fire.dps, 7992.0 / 31154.0 * 43.0 / 13.0 / 3.0);
    let pillar = calc(100161, &s, "Magic", "Earth Bolt");
    assert_eq!(pillar.max_hit, 11);
    close(pillar.dps, p * 165.0 / 20.0 / 3.0);
    assert!(
        pillar
            .hit_distribution
            .iter()
            .any(|(d, p)| *d == 0 && *p > 0.0)
    );
}
#[test]
fn independent_hammers_keep_both_miss_minima_and_pillar_rounding() {
    let s = gear(4747, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (16390, 8576, 18)
    );
    let p = 1.0 - 8578.0 / 32782.0;
    // The 23 raw maximum splits into independent 0..11 and 0..12 rolls.
    close(mean(&r), 2.0 * (1.0 - p) + p * (64.0 / 12.0 + 73.0 / 13.0));
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 2));
    let pillar = calc(100161, &s, "Crush", "");
    assert_eq!(pillar.max_hit, 22);
    close(
        mean(&pillar),
        2.0 * (1.0 - p) + p * (77.0 / 12.0 + 88.0 / 13.0),
    );
}
#[test]
fn chained_macuahuitl_miss_minimum_does_not_create_a_second_hit() {
    let s = gear(28997, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20350, 8576, 18)
    );
    let p = 1.0 - 8578.0 / 40702.0;
    let accurate = 73.0 / 13.0;
    close(
        mean(&r),
        (1.0 - p) + p * accurate + p * ((1.0 - p) + p * accurate),
    );
    close(
        r.hit_distribution
            .iter()
            .filter(|(d, _)| *d == 1)
            .map(|(_, p)| p)
            .sum(),
        1.0 - p,
    );
}
#[test]
fn scythe_caps_each_splat_and_does_not_borrow_crush_priority() {
    let s = gear(22325, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (10058, 8576, 12)
    );
    let p = 1.0 - 8578.0 / 20118.0;
    close(mean(&r), p * (91.0 / 25.0 + 43.0 / 13.0 + 19.0 / 7.0));
    let pillar = calc(100161, &s, "Crush", "");
    assert_eq!(pillar.max_hit, 15);
    close(mean(&pillar), p * (112.0 / 25.0 + 52.0 / 13.0 + 22.0 / 7.0));
}
#[test]
fn pillar_applies_after_tail_caps_and_after_each_ruby_proc() {
    let s = gear(9185, Some(9242));
    let r = calc(14014, &s, "Ranged", "");
    assert_eq!(r.max_hit, 4);
    assert_eq!(r.hp_dependent.unwrap().outcomes(300), vec![(4, 0.06)]);
    let p = calc(100161, &s, "Ranged", "");
    assert_eq!(p.max_hit, 5);
    let ruby = p.hp_dependent.unwrap();
    assert_eq!(ruby.outcomes(300), vec![(5, 0.06)]);
    assert_eq!(ruby.outcomes(10), vec![(2, 0.06)]);
    let head = calc(100160, &s, "Ranged", "");
    assert_eq!(head.hp_dependent.unwrap().outcomes(2500), vec![(130, 0.06)]);
    let body = calc(14017, &s, "Ranged", "");
    assert_eq!(body.hp_dependent.unwrap().outcomes(250), vec![(50, 0.06)]);
}
#[test]
fn protected_head_broken_tail_and_unverified_incoming_stay_separate() {
    let shield = calc(14011, &gear(9185, Some(9242)), "Ranged", "");
    assert_eq!((shield.dps, shield.max_hit), (0.0, 0));
    assert!(
        encounters::limitation(planner::monster(14015).unwrap())
            .unwrap()
            .contains("broken tail")
    );
    for id in [
        14009, 14014, 14017, 14011, 14015, 100160, 100161, 12596, 14147, 100162, 100163, 14150,
        14151,
    ] {
        assert!(
            defence::estimate(
                &member(),
                &gear(1434, None),
                planner::monster(id).unwrap(),
                "Magic",
                &Settings::default()
            )
            .is_err(),
            "{id}"
        );
    }
    for id in [14009, 100160, 14014, 100161, 14017] {
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &Reductions {
                bgs: 999,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], 120);
    }
}
#[test]
fn withdrawn_titans_multiply_ranged_attack_rolls_not_damage() {
    let s = gear(9185, Some(9144));
    for (close_id, far_id) in [(12596, 100162), (14147, 100163)] {
        let ordinary = calc(close_id, &s, "Ranged", "");
        let far = calc(far_id, &s, "Ranged", "");
        assert_eq!(
            (
                ordinary.attack_roll,
                far.attack_roll,
                far.defence_roll,
                far.max_hit
            ),
            (16478, 98868, 83276, 30)
        );
        close(ordinary.accuracy, 16478.0 / 166554.0);
        close(far.accuracy, 1.0 - 83278.0 / 197738.0);
        close(far.dps, far.accuracy * 466.0 / 31.0 / 3.0);
        assert_eq!(ordinary.max_hit, far.max_hit);
        assert!(
            combat::calculate(
                &member(),
                &gear(1434, None),
                "Crush",
                planner::monster(far_id).unwrap(),
                ""
            )
            .is_err()
        );
        assert_eq!(
            calc(far_id, &gear(1381, None), "Magic", "Fire Bolt").attack_roll,
            calc(close_id, &gear(1381, None), "Magic", "Fire Bolt").attack_roll
        );
    }
}
#[test]
fn titan_elemental_magic_accuracy_uses_bonus_not_levels_or_weakness() {
    let s = gear(1381, None);
    for id in [14150, 14151] {
        let r = calc(id, &s, "Magic", "Fire Bolt");
        close(r.accuracy, 0.4);
        let mut low = member();
        low.level_overrides.insert("Magic".into(), 13);
        close(
            combat::calculate(
                &low,
                &s,
                "Magic",
                planner::monster(id).unwrap(),
                "Fire Strike",
            )
            .unwrap()
            .accuracy,
            0.4,
        );
        let mut negative = s.clone();
        negative[4] = Some(10551);
        close(calc(id, &negative, "Magic", "Fire Bolt").accuracy, 0.3);
        let mut high = s.clone();
        for (slot, item) in [(1, 2412), (2, 12002), (4, 4091), (7, 4093), (10, 4097)] {
            high[slot] = Some(item);
        }
        assert_eq!(combat::bonuses(&high).offensive["magic"], 70);
        close(calc(id, &high, "Magic", "Fire Bolt").accuracy, 1.0);
        let mut reroll = s.clone();
        reroll[9] = Some(31106);
        close(calc(id, &reroll, "Magic", "Fire Bolt").accuracy, 0.6);
    }
    let fire = calc(14150, &s, "Magic", "Water Bolt");
    let ice = calc(14151, &s, "Magic", "Fire Bolt");
    assert_eq!((fire.max_hit, ice.max_hit), (24, 24));
    close(fire.dps, 0.4 * 301.0 / 25.0 / 3.0);
}
#[test]
fn elemental_void_accuracy_applies_once_and_requires_the_complete_set() {
    for (body, legs) in [(8839, 8840), (13072, 13073)] {
        let mut s = gear(1381, None);
        for (slot, item) in [(0, 11663), (4, body), (7, legs), (9, 8842)] {
            s[slot] = Some(item);
        }
        close(calc(14150, &s, "Magic", "Water Bolt").accuracy, 0.58);
        s[9] = None;
        close(calc(14150, &s, "Magic", "Water Bolt").accuracy, 0.4);
    }
    // A powered Shadow's intrinsic x3 equipment bonus enters the same formula.
    let s = gear(27275, None);
    close(calc(14151, &s, "Magic", "").accuracy, 1.0);
}
#[test]
fn new_windows_preserve_shared_pools_bounds_and_unsupported_burn_gates() {
    let entries = encounters::catalog();
    let titans = entries
        .iter()
        .find(|e| e.name == "The Royal Titans")
        .unwrap();
    assert_eq!(titans.targets.len(), 6);
    let o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 12596,
                style: "Ranged".into(),
            },
            Target {
                id: 100162,
                style: "Ranged".into(),
            },
        ],
        ..Default::default()
    };
    assert!(coverage::windows(&o).iter().all(|w| w.supported));
    assert_eq!(
        encounters::window(12596).unwrap().progress_group,
        encounters::window(100162).unwrap().progress_group
    );
    assert_eq!(
        combat::target_hp_bounds(planner::monster(100161).unwrap(), 0).unwrap(),
        (1, 300)
    );
    let mut s = gear(29000, Some(28991));
    for (slot, item) in [(0, 29055), (4, 29049), (7, 29052)] {
        s[slot] = Some(item);
    }
    for id in [14014, 100161, 100160] {
        assert!(
            combat::calculate(&member(), &s, "Ranged", planner::monster(id).unwrap(), "")
                .unwrap_err()
                .contains("Eclipse burn")
        );
    }
}

#[test]
fn blood_moon_cooldown_uses_accuracy_even_when_tail_misses_deal_damage() {
    let mut s = gear(28997, None);
    for (slot, item) in [(0, 29073), (4, 29067), (7, 29070)] {
        s[slot] = Some(item);
    }
    let r = calc(14014, &s, "Crush", "");
    let p = r.accuracy;
    assert!(p > 0.0 && p < 1.0);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 1));
    close(
        r.expected_attack_ticks,
        4.0 - 0.33 * p - 0.33 * 0.67 * p * p,
    );
    close(r.dps, mean(&r) / (0.6 * r.expected_attack_ticks));
}
#[test]
fn blue_moon_additions_keep_their_own_accuracy_and_npc_transforms() {
    let mut s = gear(28988, None);
    for (slot, item) in [(0, 29019), (4, 29013), (7, 29016)] {
        s[slot] = Some(item);
    }
    for id in [14014, 100161, 14150] {
        let target = planner::monster(id).unwrap();
        let mut settings = Settings {
            blue_moon_melee: "Stab".into(),
            ..Default::default()
        };
        let melee =
            combat::calculate_configured(&member(), &s, "Stab", target, "", 0, &settings).unwrap();
        let on = combat::calculate_configured(
            &member(),
            &s,
            "Magic",
            target,
            "Undead Grasp",
            0,
            &settings,
        )
        .unwrap();
        settings.blue_moon_melee = "Off".into();
        let off = combat::calculate_configured(
            &member(),
            &s,
            "Magic",
            target,
            "Undead Grasp",
            0,
            &settings,
        )
        .unwrap();
        close(on.dps, off.dps + off.accuracy * 0.5 * melee.dps);
        assert_eq!(on.max_hit, off.max_hit + melee.max_hit);
    }
}
