//! Pinned Wiki scaling/Vardorvis.ts, Math.ts, incoming NPC data and the KQ /
//! Hydra / Muspah strategy pages cited in SOURCES.md. Arithmetic is independent.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::{self, Settings},
    encounters, modifiers,
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
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn hydra_quarters_rolled_hits_after_bolt_effects_instead_of_quartering_average_dps() {
    let normal = calc(8615, 9185, Some(9144), "Ranged");
    let shield = calc(100100, 9185, Some(9144), "Ranged");
    let hit = 1.0 - 11883.0 / 32958.0; // 109*109 defence; 107*154 attack.
    assert_eq!(
        (normal.attack_roll, normal.defence_roll, normal.max_hit),
        (16478, 11881, 30)
    );
    assert_eq!(shield.max_hit, 7);
    close(shield.dps, hit * 105.0 / 31.0 / 3.0);
    assert!(shield.dps < normal.dps / 4.0);
    let ruby = calc(100100, 9185, Some(9242), "Ranged");
    assert_eq!(ruby.max_hit, 25);
    close(ruby.dps, (0.94 * hit * 91.0 / 29.0 + 0.06 * 25.0) / 3.0);
    assert_eq!(ruby.hp_dependent.unwrap().divisor, 4);
    let diamond = calc(100100, 9185, Some(9243), "Ranged");
    assert_eq!(diamond.max_hit, 8);
    close(
        diamond.dps,
        (0.9 * hit * 91.0 / 29.0 + 0.1 * 120.0 / 33.0) / 3.0,
    );
}

#[test]
fn hydra_quarters_each_scythe_splat_before_adding_the_attack() {
    let r = calc(100100, 22325, None, "Slash");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20790, 23326, 8)
    );
    // 23/11/5 maxima become5/2/1. Accurate means60/24 +12/12 +2/6.
    close(
        r.dps,
        20790.0 / 46654.0 * (60.0 / 24.0 + 12.0 / 12.0 + 2.0 / 6.0) / 3.0,
    );
}

#[test]
fn hydra_stage_bounds_and_ruby_defaults_use_one_1100_hp_workload() {
    for (id, min, max) in [
        (8615, 826, 1100),
        (8619, 551, 825),
        (8620, 276, 550),
        (8621, 1, 275),
    ] {
        let target = planner::monster(id).unwrap();
        assert_eq!(combat::target_hp_bounds(target, 0).unwrap(), (min, max));
        let r = calc(id, 9185, Some(9242), "Ranged");
        let expected = (max * 20 / 100).min(100) as i32;
        assert_eq!(r.hit_distribution.last().copied(), Some((expected, 0.06)));
        let b = attack_budget::for_window(target, &r, &Options::default(), &set(9185, Some(9242)))
            .unwrap();
        assert_eq!((b.start_hp, b.stop_hp), (max, max - 275));
        let mut settings = Settings::default();
        settings.target_hp.insert(id, max + 1);
        assert!(
            combat::calculate_configured(
                &member(),
                &set(9185, Some(9242)),
                "Ranged",
                target,
                "",
                0,
                &settings
            )
            .is_err()
        );
        if min > 1 {
            settings.target_hp.insert(id, min - 1);
            assert!(combat::current_target_hp(target, 0, &settings).is_err());
        }
    }
    assert_eq!(
        [8615, 8619, 8620, 8621]
            .iter()
            .map(|id| encounters::progress_hp(*id, "Ranged").unwrap())
            .sum::<f64>(),
        1100.0
    );
    assert_eq!(calc(8621, 9185, Some(9242), "Ranged").max_hit, 55);
}

#[test]
fn existing_sire_stages_no_longer_borrow_full_boss_hp_for_ruby_procs() {
    let r = calc(5891, 9185, Some(9242), "Ranged");
    assert_eq!(r.hit_distribution.last().copied(), Some((42, 0.06))); // 210HP, not425.
    let final_phase = calc(5908, 9185, Some(9242), "Ranged");
    assert_eq!(
        final_phase.hit_distribution.last().copied(),
        Some((28, 0.06))
    ); // 140HP.
}

#[test]
fn synthetic_chemical_states_keep_real_npc_identity_and_share_round_trip() {
    let target = planner::monster(100101).unwrap();
    assert_eq!(encounters::source_npc_id(target.id), 8619);
    assert_eq!(target.skills, planner::monster(8619).unwrap().skills);
    assert!(target.name.contains("Before chemical"));
    let o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 100101,
            style: "Ranged".into(),
        }],
        ..Default::default()
    };
    let snapshot = account::demo();
    let p = planner::plan(&snapshot, &o);
    let setup = ironbound::share::Setup::from_plan(&p, &o);
    let shared = ironbound::share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(shared.options().targets, o.targets);
    assert!(coverage::export_note(&o).contains("825 HP"));
}

#[test]
fn vardorvis_catalog_and_current_stats_have_exact_full_fraction_rounding() {
    let target = planner::monster(12223).unwrap();
    assert_eq!((target.skills["def"], target.skills["str"]), (215, 270));
    let curve = encounters::window(12223)
        .unwrap()
        .hp_scaling
        .as_ref()
        .unwrap();
    for (hp, def, strength) in [
        (700, 215, 270),
        (699, 214, 270),
        (350, 180, 315),
        (1, 145, 359),
        (0, 145, 360),
    ] {
        let stats = curve.levels(hp).unwrap();
        assert_eq!((stats["def"], stats["str"]), (def, strength));
    }
    assert!(curve.levels(701).is_err());
    let gear = set(4151, None);
    for (hp, roll) in [(700, 28896), (350, 24381), (1, 19866)] {
        let mut settings = Settings::default();
        settings.target_hp.insert(12223, hp);
        let r = combat::calculate_configured(&member(), &gear, "Slash", target, "", 0, &settings)
            .unwrap();
        assert_eq!(r.defence_roll, roll);
        assert_eq!(r.attack_roll, 16060);
        close(r.accuracy, 16060.0 / (2.0 * (roll as f64 + 1.0)));
        settings.reductions.insert(
            12223,
            modifiers::Reductions {
                bgs: 100,
                ..Default::default()
            },
        );
        let drained =
            combat::calculate_configured(&member(), &gear, "Slash", target, "", 0, &settings)
                .unwrap();
        assert_eq!(
            drained.defence_roll, roll,
            "Vardorvis's drain floor follows current scaled defence"
        );
    }
    assert!(encounters::limitation(planner::monster(12224).unwrap()).is_some());
}

#[test]
fn vardorvis_dynamic_budget_solves_miss_loops_and_finishing_overkill() {
    let gear = set(4151, None);
    let target = planner::monster(12223).unwrap();
    let mut o = Options::default();
    o.combat.target_hp.insert(12223, 3);
    let b = attack_budget::for_dynamic_window(&member(), &gear, "Slash", target, "", &o).unwrap();
    let hit = 16060.0 / 39734.0;
    // Below10HP, def145. Accurate rolls0,1 both deal1. E3=704/(625*p).
    close(b.attacks, 704.0 / (625.0 * hit));
    close(b.seconds, b.attacks * 2.4);
    let r =
        combat::calculate_configured(&member(), &gear, "Slash", target, "", 0, &o.combat).unwrap();
    assert!(attack_budget::for_window(target, &r, &o, &gear).is_err());
    o.combat.target_hp.clear();
    let full = calc(12223, 4151, None, "Slash");
    let dynamic =
        attack_budget::for_dynamic_window(&member(), &gear, "Slash", target, "", &o).unwrap();
    let frozen = attack_budget::expected_attacks(&full.hit_distribution, 700, 0, None).unwrap();
    assert!(
        dynamic.attacks < frozen,
        "Falling defence must improve accuracy during the workload"
    );
    close(dynamic.seconds, dynamic.attacks * 2.4);
}

#[test]
fn muspah_normal_forms_use_defences_and_the_prayer_shield_remains_blocked() {
    for id in [12077, 12078] {
        assert_eq!(calc(id, 9185, Some(9144), "Ranged").max_hit, 30);
    }
    assert!(
        calc(12077, 9185, Some(9144), "Ranged").dps > calc(12078, 9185, Some(9144), "Ranged").dps
    );
    assert!(calc(12078, 11905, None, "Magic").dps > calc(12077, 11905, None, "Magic").dps);
    let final_phase = calc(12080, 9185, Some(9242), "Ranged");
    assert_eq!(
        final_phase.hit_distribution.last().copied(),
        Some((25, 0.06))
    );
    assert_eq!(final_phase.max_hit, 28);
    assert!(encounters::limitation(planner::monster(12079).unwrap()).is_some());
    assert!(encounters::limitation(planner::monster(12074).unwrap()).is_some());
    for id in [12077, 12078, 12080] {
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &modifiers::Reductions {
                dwh: 20,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], 125);
    }
    assert_eq!(encounters::progress_hp(12077, "Ranged"), Some(723.0));
    assert_eq!(encounters::progress_hp(12080, "Ranged"), Some(127.0));
}

#[test]
fn queen_magic_and_ranged_guarantee_accuracy_but_melee_does_not() {
    let gear = set(4151, None);
    for id in [963, 965] {
        let target = planner::monster(id).unwrap();
        for kind in ["Magic", "Ranged"] {
            let r =
                defence::estimate(&member(), &gear, target, kind, &Settings::default()).unwrap();
            assert_eq!(r.hit_chance, 1.0);
            close(r.damage_per_second, 15.5 / 2.4);
            let settings = Settings {
                protection: kind.into(),
                ..Default::default()
            };
            let protected = defence::estimate(&member(), &gear, target, kind, &settings).unwrap();
            assert_eq!(protected.damage_per_second, 0.0);
        }
        let melee =
            defence::estimate(&member(), &gear, target, "Stab", &Settings::default()).unwrap();
        close(melee.hit_chance, 1.0 - 6850.0 / 39554.0);
        assert!(melee.damage_per_second < 15.5 / 2.4);
        assert!(
            !defence::profile(id).unwrap().automatic,
            "A selected scenario must not invent an attack mixture"
        );
    }
}

#[test]
fn kings_have_separate_protected_basic_attack_scenarios() {
    let gear = set(4151, None);
    for (id, attack, protection, max) in [
        (2265, "Ranged", "Ranged", 30.0),
        (2266, "Magic", "Magic", 50.0),
        (2267, "Slash", "Melee", 26.0),
    ] {
        let target = planner::monster(id).unwrap();
        let r = defence::estimate(&member(), &gear, target, attack, &Settings::default()).unwrap();
        assert!(r.hit_chance > 0.0 && r.hit_chance < 1.0);
        close(r.damage_per_second, r.hit_chance * max / 2.0 / 2.4);
        let protected = defence::estimate(
            &member(),
            &gear,
            target,
            attack,
            &Settings {
                protection: protection.into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(protected.damage_per_second, 0.0);
        assert!(protected.note.contains("Spinolyps"));
    }
    assert!(
        defence::estimate(
            &member(),
            &gear,
            planner::monster(2042).unwrap(),
            "Ranged",
            &Settings::default()
        )
        .is_err()
    );
}

#[test]
fn moon_followups_apply_chemical_and_hp_scaling_once_to_each_style() {
    let mut gear = set(28988, None);
    for (slot, id) in [(0, 29019), (4, 29013), (7, 29016)] {
        gear[slot] = Some(id);
    }
    for id in [100100, 12223] {
        let target = planner::monster(id).unwrap();
        let mut settings = Settings {
            blue_moon_melee: "Stab".into(),
            ..Default::default()
        };
        if id == 12223 {
            settings.target_hp.insert(id, 350);
        }
        let melee =
            combat::calculate_configured(&member(), &gear, "Stab", target, "", 0, &settings)
                .unwrap();
        let on = combat::calculate_configured(
            &member(),
            &gear,
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
            &gear,
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

#[test]
fn dynamic_vardorvis_burn_uses_falling_defence_and_recommendations_share_the_budget() {
    let mut gear = set(29000, Some(28991));
    for (slot, id) in [(0, 29010), (4, 29004), (7, 29007)] {
        gear[slot] = Some(id);
    }
    let mut options = Options::default();
    options.boss = Boss::Custom;
    options.targets = vec![Target {
        id: 12223,
        style: "Ranged".into(),
    }];
    let m = member();
    let target = planner::monster(12223).unwrap();
    let budget =
        attack_budget::for_dynamic_window(&m, &gear, "Ranged", target, "", &options).unwrap();
    let dynamic = budget.burn.as_ref().unwrap();
    let frozen = combat::calculate(&m, &gear, "Ranged", target, "").unwrap();
    let fixed = attack_budget::for_hp(&frozen, 700, 0).unwrap();
    assert!(budget.attacks < fixed.attacks);
    assert!(dynamic.burn_damage > 0.0);
    let mut owned: std::collections::BTreeMap<_, _> =
        gear.iter().flatten().map(|id| (*id, 1)).collect();
    owned.insert(28991, 1000);
    let assessed =
        ironbound::recommendation::assess(&m, &gear, "Ranged", &options, &owned).unwrap();
    close(assessed.time, budget.seconds);
    let bow = set(9185, Some(9144));
    owned.insert(9185, 1);
    owned.insert(9144, 1000);
    let other =
        attack_budget::for_dynamic_window(&m, &bow, "Ranged", target, "", &options).unwrap();
    let assessed = ironbound::recommendation::assess(&m, &bow, "Ranged", &options, &owned).unwrap();
    close(assessed.time, other.seconds);
    assert!(other.burn.is_none());
    options.combat.target_hp.insert(12223, 1);
    let low = attack_budget::for_dynamic_window(&m, &gear, "Ranged", target, "", &options).unwrap();
    close(low.burn.as_ref().unwrap().burn_damage, 0.0);
    assert!(low.attacks < budget.attacks);
    let again =
        attack_budget::for_dynamic_window(&m, &gear, "Ranged", target, "", &Options::default())
            .unwrap();
    close(again.seconds, budget.seconds);
    options.combat.target_hp.insert(12223, 700);
    options.combat.ranged_stance = "Accurate".into();
    let accurate =
        attack_budget::for_dynamic_window(&m, &gear, "Ranged", target, "", &options).unwrap();
    assert_ne!(
        accurate.seconds, budget.seconds,
        "stance must invalidate the cached timing"
    );
}

#[test]
fn chemical_comparisons_share_the_source_npcs_slayer_assignment() {
    let mut settings = Settings::default();
    let mut gear = set(9185, Some(9144));
    gear[0] = Some(11865); // Slayer helmet (i).
    ironbound::slayer::set_task(&mut settings, 8619, true);
    for id in [8619, 100101] {
        assert!(ironbound::slayer::on_task(&settings, id));
        assert!(
            ironbound::effects::target_bonus(
                &gear,
                "Ranged",
                planner::monster(id).unwrap(),
                &settings,
            )
            .unwrap()
            .slayer_task
        );
    }
    ironbound::slayer::set_task(&mut settings, 100101, false);
    assert!(settings.slayer_targets.is_empty());
    // Older setup links may store the comparison key; read and clear those too.
    settings.slayer_targets.insert(100101);
    assert!(ironbound::slayer::on_task(&settings, 8619));
    ironbound::slayer::set_task(&mut settings, 8619, false);
    assert!(settings.slayer_targets.is_empty());
}
