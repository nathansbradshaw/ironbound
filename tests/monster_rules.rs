//! Independent arithmetic from pinned Wiki PlayerVsNPCCalc / HitDist / BaseCalc
//! at 74958b1. Fixtures retain real NPC stats; expected values are not engine output.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::Settings,
    encounters,
    monster_rules::{self, Transform},
    planner::{self, Boss, Options, Target},
};
fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
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
fn calc(id: i32, weapon: i32, ammo: Option<i32>, style: &str) -> combat::ResultStats {
    combat::calculate(
        &member(),
        &set(weapon, ammo),
        style,
        planner::monster(id).unwrap(),
        "",
    )
    .unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn kraken_ranged_rolls_use_sevenths_with_minimum_one_and_preserve_misses() {
    let r = calc(494, 9185, Some(9144), "Ranged");
    // Rapid rune crossbow + runite bolts: roll107*154=16478, max30.
    // Kraken defence10*364=3640. Summed transformed accurate rolls=61/31.
    let hit = 1.0 - 3642.0 / (2.0 * 16479.0);
    assert_eq!((r.attack_roll, r.defence_roll, r.max_hit), (16478, 3640, 4));
    near(r.accuracy, hit);
    near(r.dps, hit * 61.0 / 31.0 / 3.0);
    near(
        r.hit_distribution
            .iter()
            .filter(|(d, _)| *d == 0)
            .map(|(_, p)| p)
            .sum(),
        1.0 - hit,
    );
    let magic = calc(494, 11905, None, "Magic");
    assert_eq!(
        (magic.attack_roll, magic.defence_roll, magic.max_hit),
        (8690, 1940, 28)
    );
    near(
        magic.dps,
        (1.0 - 1942.0 / 17382.0) * (14.0 + 1.0 / 29.0) / 2.4,
    );
}

#[test]
fn emerged_kraken_is_distinct_from_small_cave_kraken_and_whirlpools() {
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Melee",
            planner::monster(494).unwrap(),
            ""
        )
        .is_err()
    );
    assert!(calc(492, 4151, None, "Melee").dps > 0.0);
    for id in [493, 496] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
        assert!(
            combat::calculate(
                &member(),
                &set(11905, None),
                "Magic",
                planner::monster(id).unwrap(),
                ""
            )
            .is_err()
        );
    }
}

#[test]
fn kraken_ruby_proc_is_reduced_and_recomputed_with_current_hp() {
    let r = calc(494, 9185, Some(9242), "Ranged");
    let hit = 1.0 - 3642.0 / 32958.0;
    // Ruby bolt strength gives base28; transformed normal sum53/29.
    // Full-health proc: floor255*.2=51, divided by7=7.
    assert_eq!(r.max_hit, 7);
    near(r.dps, (0.94 * hit * 53.0 / 29.0 + 0.06 * 7.0) / 3.0);
    let ruby = r.hp_dependent.unwrap();
    assert_eq!(ruby.outcomes(255), vec![(7, 0.06)]);
    assert_eq!(ruby.outcomes(30), vec![(1, 0.06)]);
    assert_eq!(ruby.outcomes(1), vec![(0, 0.06)]);
    let dynamic = attack_budget::expected_attacks(&r.hit_distribution, 255, 0, Some(ruby)).unwrap();
    let frozen = attack_budget::expected_attacks(&r.hit_distribution, 255, 0, None).unwrap();
    assert!(dynamic > frozen);
}

#[test]
fn corp_halves_each_rolled_melee_hit_while_magic_stays_full() {
    let whip = calc(319, 4151, None, "Slash");
    assert_eq!(
        (whip.attack_roll, whip.defence_roll, whip.max_hit),
        (16060, 84216, 12)
    );
    // floor(max(1,roll)/2), roll0..24: total144/25, including two zeros.
    near(whip.dps, (16060.0 / 168434.0) * 144.0 / 25.0 / 2.4);
    let magic = calc(319, 11905, None, "Magic");
    assert_eq!(
        (magic.attack_roll, magic.defence_roll, magic.max_hit),
        (8690, 76826, 28)
    );
    near(magic.dps, 8690.0 / 153654.0 * (14.0 + 1.0 / 29.0) / 2.4);
    assert!(monster_rules::corpbane(26219, "stab"));
    assert!(!monster_rules::corpbane(26219, "slash"));
    assert!(!monster_rules::corpbane(4151, "slash"));
    let target = planner::monster(319).unwrap();
    let mut ordinary = target.clone();
    ordinary.id = 5779;
    ordinary.name = "Independent ordinary target".into();
    for style in ["Stab", "Slash"] {
        let a = calc(319, 26219, None, style);
        let b = combat::calculate(&member(), &set(26219, None), style, &ordinary, "").unwrap();
        if style == "Stab" {
            near(a.dps, b.dps);
            assert_eq!(a.max_hit, b.max_hit);
        } else {
            assert_eq!(a.max_hit, b.max_hit / 2);
            assert!(a.dps < b.dps / 2.0);
        }
    }
}

#[test]
fn corp_diamond_is_halved_after_its_bonus_and_ruby_retains_full_damage() {
    let diamond = calc(319, 9185, Some(9243), "Ranged");
    // Base28, diamond32, per-hit halves give means196/29 and256/33.
    let hit = 16478.0 / (2.0 * 52317.0); // Heavy defence: (310+9)*(100+64)=52316.
    assert_eq!(diamond.defence_roll, 52316);
    assert_eq!(diamond.max_hit, 16);
    near(
        diamond.dps,
        (0.9 * hit * 196.0 / 29.0 + 0.1 * 256.0 / 33.0) / 3.0,
    );
    let ruby = calc(319, 9185, Some(9242), "Ranged");
    assert_eq!(ruby.max_hit, 100);
    near(ruby.dps, (0.94 * hit * 196.0 / 29.0 + 0.06 * 100.0) / 3.0);
    assert_eq!(ruby.hp_dependent.unwrap().outcomes(250), vec![(50, 0.06)]);
}

#[test]
fn zulrah_reroll_keeps_fifty_and_rerolls_every_larger_hit_into_six_outcomes() {
    assert_eq!(Transform::Zulrah.outcomes(50, 1.0), vec![(50, 1.0)]);
    let r = Transform::Zulrah.outcomes(51, 1.0);
    assert_eq!(
        r.iter().map(|(d, _)| *d).collect::<Vec<_>>(),
        [45, 46, 47, 48, 49, 50]
    );
    near(r.iter().map(|(_, p)| p).sum(), 1.0);
    near(r.iter().map(|(d, p)| f64::from(*d) * p).sum(), 47.5);
    for id in [2042, 2043, 2044] {
        let ruby = calc(id, 9185, Some(9242), "Ranged");
        assert_eq!(ruby.max_hit, 50);
        let proc = ruby.hp_dependent.unwrap();
        assert_eq!(proc.distribution_entries, 6);
        assert_eq!(
            proc.outcomes(500),
            (45..=50).map(|d| (d, 0.01)).collect::<Vec<_>>()
        );
        assert_eq!(proc.outcomes(250), vec![(50, 0.06)]);
        assert_eq!(proc.outcomes(200), vec![(40, 0.06)]);
        near(ruby.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
        assert!(
            attack_budget::expected_attacks(&ruby.hit_distribution, 500, 0, Some(proc))
                .unwrap()
                .is_finite()
        );
        assert!(
            combat::calculate(
                &member(),
                &set(4151, None),
                "Melee",
                planner::monster(id).unwrap(),
                ""
            )
            .is_err()
        );
    }
}

#[test]
fn zulrah_limits_high_magic_and_diamond_damage_after_their_bonuses() {
    let mut m = member();
    m.level_overrides.insert("Magic".into(), 200);
    let r = combat::calculate(
        &m,
        &set(11905, None),
        "Magic",
        planner::monster(2042).unwrap(),
        "",
    )
    .unwrap();
    // Base61. Accurate rolls1..50, accurate zero->1, eleven rolls51..61->47.5.
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (16669, 5871, 50)
    );
    let hit = 1.0 - 5873.0 / (2.0 * 16670.0);
    near(r.dps, hit * (1276.0 + 11.0 * 47.5) / 62.0 / 2.4);
    assert!(hit > 0.0);
    let mut ranged = member();
    ranged.level_overrides.insert("Ranged".into(), 200);
    let d = combat::calculate(
        &ranged,
        &set(9185, Some(9243)),
        "Ranged",
        planner::monster(2044).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(d.max_hit, 50);
    assert!(d.hit_distribution.iter().all(|(hit, _)| *hit <= 50));
}

#[test]
fn kings_and_kalphite_queen_use_recorded_defences_without_invented_immunity() {
    for id in [2265, 2266, 2267, 963, 965] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_none());
        for (weapon, ammo, style) in [
            (4151, None, "Melee"),
            (11905, None, "Magic"),
            (9185, Some(9144), "Ranged"),
        ] {
            assert!(calc(id, weapon, ammo, style).dps > 0.0, "{id}/{style}");
        }
    }
    assert!(calc(2267, 11905, None, "Magic").dps > calc(2265, 11905, None, "Magic").dps);
    assert!(calc(963, 25979, None, "Melee").max_hit > calc(963, 4151, None, "Melee").max_hit);
}

#[test]
fn dusk_immunities_flying_reach_and_style_coverage_are_explicit() {
    for id in [7851, 7887] {
        assert!(calc(id, 4151, None, "Melee").dps > 0.0);
        for (weapon, ammo, style) in [(11905, None, "Magic"), (9185, Some(9144), "Ranged")] {
            assert!(
                combat::calculate(
                    &member(),
                    &set(weapon, ammo),
                    style,
                    planner::monster(id).unwrap(),
                    ""
                )
                .is_err()
            );
        }
    }
    for id in [3162, 7852] {
        assert!(
            combat::calculate(
                &member(),
                &set(4151, None),
                "Melee",
                planner::monster(id).unwrap(),
                ""
            )
            .is_err()
        );
        assert!(calc(id, 11905, None, "Magic").dps > 0.0);
        assert!(calc(id, 9185, Some(9144), "Ranged").dps > 0.0);
        // Reach is independent of whether this particular weapon's own rules are enabled.
        assert!(monster_rules::check_weapon(id, "Melee", 3204).is_ok());
    }
    let o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 7851,
            style: "Magic".into(),
        }],
        ..Default::default()
    };
    let rows = coverage::windows(&o);
    assert!(!rows[0].supported);
    assert!(rows[0].reason.contains("immune"));
}

#[test]
fn reviewed_outgoing_windows_keep_incoming_and_full_fight_limits_visible() {
    for p in &monster_rules::registry().profiles {
        for id in &p.ids {
            let target = planner::monster(*id).unwrap();
            let window = encounters::window(*id).expect("documented outgoing scope");
            assert!(!window.description.is_empty());
            assert!(
                ironbound::defence::estimate(
                    &member(),
                    &set(4151, None),
                    target,
                    "Melee",
                    &Settings::default()
                )
                .is_err()
            );
        }
    }
    for id in [2042, 2043, 2044] {
        assert_eq!(
            encounters::window(id).unwrap().progress_group.as_deref(),
            Some("zulrah_forms")
        );
    }
    assert_eq!(
        encounters::window(7851).unwrap().progress_group,
        encounters::window(7887).unwrap().progress_group
    );
    let mut data: monster_rules::Registry =
        serde_json::from_str(include_str!("../assets/monster-rules.json")).unwrap();
    data.profiles[0].ids.push(2042);
    assert!(data.validate().is_err());
}

#[test]
fn alternate_forms_share_one_comparison_hp_workload() {
    for (ids, expected_hp) in [(&[2042, 2043, 2044][..], 500.0), (&[7851, 7887][..], 450.0)] {
        let mut comparison =
            ironbound::tradeoffs::compare(&account::demo(), &Options::default(), 4, 8);
        for side in [&mut comparison.left, &mut comparison.right] {
            let template = side.windows[0].clone();
            side.windows.clear();
            side.unmodelled_phases.clear();
            for id in ids {
                let mut window = template.clone();
                let (weapon, style) = if *id >= 7800 {
                    (4151, "Melee")
                } else {
                    (11905, "Magic")
                };
                let r = calc(*id, weapon, None, style);
                window.target_id = *id;
                window.style = style.into();
                window.name = planner::monster(*id).unwrap().name.clone();
                window.metrics.dps = Some(r.dps);
                window.metrics.attack = Some(r);
                window.metrics.incoming.clear();
                side.windows.push(window);
            }
        }
        let result = ironbound::encounter_summary::compare(&comparison);
        assert!(result.complete(), "{:?}", result.omissions);
        near(result.workloads.iter().map(|w| w.hp).sum(), expected_hp);
        assert!(result.left.unwrap().incoming_damage.is_none());
    }
}

#[test]
fn exposed_sire_vents_use_minimum_hits_and_successful_demonbane_finishers() {
    let vent = planner::monster(5914).unwrap();
    let r = calc(5914, 11905, None, "Magic");
    let hit = 1.0 - 578.0 / 17382.0;
    assert_eq!((r.attack_roll, r.defence_roll, r.max_hit), (8690, 576, 28));
    near(r.dps, hit * 21.0 / 2.4); // Uniform accurate damage14..28, not0..28.
    assert!(
        r.hit_distribution
            .iter()
            .all(|(d, _)| *d == 0 || (14..=28).contains(d))
    );
    for marked in [false, true] {
        let mut settings = Settings::default();
        settings.mark_darkness = marked;
        let r = combat::calculate_configured(
            &member(),
            &set(29594, None),
            "Magic",
            vent,
            "Dark Demonbane",
            0,
            &settings,
        )
        .unwrap();
        assert_eq!(r.max_hit, 50);
        assert!(r.accuracy < 1.0);
        assert!(r.hit_distribution.iter().all(|(d, _)| *d == 0 || *d == 50));
        near(r.dps, 50.0 * r.accuracy / 3.0);
    }
    assert!(combat::calculate(&member(), &set(29589, None), "Slash", vent, "").is_err());
    let o = Options::default();
    let b = attack_budget::for_window(vent, &r, &o, &set(11905, None)).unwrap();
    assert_eq!(b.entities, 4);
    near(
        b.attacks,
        4.0 * attack_budget::expected_attacks(&r.hit_distribution, 50, 0, None).unwrap(),
    );
}

#[test]
fn duke_uses_seventy_percent_demonbane_effectiveness_only_in_reviewed_awake_state() {
    let r = calc(12191, 29589, None, "Slash");
    // Accurate Emberlight roll14740 becomes21962 (+49%); base13 becomes19.
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (21962, 30956, 19)
    );
    near(r.dps, 21962.0 / 61914.0 * (9.5 + 1.0 / 20.0) / 2.4);
    assert_eq!(encounters::demonbane_vulnerability(12191), 70);
    assert_eq!(encounters::demonbane_vulnerability(14176), 120);
    assert!(encounters::limitation(planner::monster(12195).unwrap()).is_some());
    assert_eq!(
        [5889, 5891, 5908]
            .iter()
            .map(|id| encounters::progress_hp(*id, "Melee").unwrap())
            .sum::<f64>(),
        425.0
    );
    assert!(encounters::limitation(planner::monster(5886).unwrap()).is_some());
}

#[test]
fn npc_limiters_apply_to_each_hitsplat_before_multi_hit_totals() {
    assert_eq!(
        monster_rules::transform_splats(&[3, 3], 1.0, Transform::Corporeal),
        vec![(2, 1.0)]
    );
    let capped = monster_rules::transform_splats(&[60, 30], 1.0, Transform::Zulrah);
    assert_eq!(
        capped.iter().map(|(d, _)| *d).collect::<Vec<_>>(),
        [75, 76, 77, 78, 79, 80]
    );
    let torag = calc(319, 4747, None, "Crush");
    assert_eq!(
        (torag.attack_roll, torag.defence_roll, torag.max_hit),
        (16390, 52316, 11)
    );
    near(
        torag.dps,
        (16390.0 / 104634.0) * (30.0 / 12.0 + 36.0 / 13.0) / 3.0,
    );
    let mut karil = set(4734, Some(4740));
    for (slot, id) in [(0, 4732), (2, 12851), (4, 4736), (7, 4738)] {
        karil[slot] = Some(id);
    }
    let kraken = combat::calculate(
        &member(),
        &karil,
        "Ranged",
        planner::monster(494).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(kraken.max_hit, 3);
    near(kraken.dps, kraken.accuracy * (28.0 / 21.0 + 0.25) / 1.8);
    let mut m = member();
    m.level_overrides.insert("Ranged".into(), 200);
    let zulrah =
        combat::calculate(&m, &karil, "Ranged", planner::monster(2044).unwrap(), "").unwrap();
    assert_eq!(
        zulrah.max_hit, 58,
        "39+19 stays below the cap on each splat"
    );
    near(
        zulrah.dps,
        zulrah.accuracy * (19.5 + 1.0 / 40.0 + 0.25 * 382.0 / 40.0) / 1.8,
    );
}

#[test]
fn integrated_scythe_corp_reduction_precedes_the_three_hit_sum() {
    let r = calc(319, 22325, None, "Slash");
    // Base23 produces23/11/5 maxima on a large NPC. Corp halves these into
    // 11/5/2, not floor((23+11+5)/2)=19. Their accurate means sum to9.
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20790, 84216, 18)
    );
    near(r.dps, 20790.0 / 168434.0 * 9.0 / 3.0);
}
