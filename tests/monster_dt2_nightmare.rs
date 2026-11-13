//! Independent rounding, HP/counter and variant checks against SOURCES.md.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::{self, Settings},
    encounters::{self, Resource},
    modifiers::{self, Reductions},
    monster_rules,
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
fn calc(id: i32, weapon: i32, ammo: Option<i32>, style: &str, spell: &str) -> combat::ResultStats {
    combat::calculate(
        &member(),
        &gear(weapon, ammo),
        style,
        planner::monster(id).unwrap(),
        spell,
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn leviathan_front_caps_damage_outcomes_including_bolt_procs() {
    let r = calc(100130, 9185, Some(9144), "Ranged", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (16478, 23051, 10)
    );
    let p = 16478.0 / 46104.0;
    close(r.dps, p * 256.0 / 31.0 / 3.0);
    let ruby = calc(100130, 9185, Some(9242), "Ranged", "");
    assert_eq!(ruby.max_hit, 10);
    close(ruby.dps, (0.94 * p * 236.0 / 29.0 + 0.06 * 10.0) / 3.0);
    let proc = ruby.hp_dependent.unwrap();
    assert_eq!(proc.outcomes(900), vec![(10, 0.06)]);
    assert_eq!(proc.outcomes(20), vec![(4, 0.06)]);
    let diamond = calc(100130, 9185, Some(9243), "Ranged", "");
    close(
        diamond.dps,
        (0.9 * p * 236.0 / 29.0 + 0.1 * 276.0 / 33.0) / 3.0,
    );
}

#[test]
fn leviathan_rear_magic_doubles_damage_without_guaranteeing_accuracy() {
    let rear = calc(100131, 1381, None, "Magic", "Fire Bolt");
    assert_eq!(
        (rear.attack_roll, rear.defence_roll, rear.max_hit),
        (7992, 58136, 24)
    );
    close(rear.accuracy, 7992.0 / 116274.0);
    close(rear.dps, 7992.0 / 116274.0 * 158.0 / 13.0 / 3.0);
    let normal = calc(12214, 1381, None, "Magic", "Fire Bolt");
    close(rear.dps, normal.dps * 2.0);
    let o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 100131,
            style: "Ranged".into(),
        }],
        ..Default::default()
    };
    assert!(!coverage::windows(&o)[0].supported);
    assert!(coverage::windows(&o)[0].reason.contains("not verified"));
}

#[test]
fn leviathan_melee_quest_and_enrage_do_not_borrow_normal_ranged_rules() {
    for weapon in [4151, 3204, 22325] {
        assert!(
            combat::calculate(
                &member(),
                &gear(weapon, None),
                "Melee",
                planner::monster(12214).unwrap(),
                ""
            )
            .is_err()
        );
    }
    for id in [12215, 100132] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
    }
    assert_eq!(
        combat::target_hp_bounds(planner::monster(12214).unwrap(), 0).unwrap(),
        (181, 900)
    );
    assert_eq!(
        combat::target_hp_bounds(planner::monster(100132).unwrap(), 0).unwrap(),
        (1, 180)
    );
    let r = calc(12214, 9185, Some(9144), "Ranged", "");
    assert!(
        attack_budget::for_window(
            planner::monster(12214).unwrap(),
            &r,
            &Options::default(),
            &gear(9185, Some(9144))
        )
        .is_err()
    );
}

#[test]
fn whisperer_specials_floor_each_hit_after_the_earth_weakness() {
    let normal = calc(12204, 1381, None, "Magic", "Earth Bolt");
    assert_eq!(
        (normal.attack_roll, normal.defence_roll, normal.max_hit),
        (12787, 13986, 19)
    );
    let p = 12787.0 / 27974.0;
    close(normal.dps, p * 191.0 / 20.0 / 3.0);
    let outside = calc(100135, 1381, None, "Magic", "Earth Bolt");
    let inside = calc(100136, 1381, None, "Magic", "Earth Bolt");
    assert_eq!((outside.max_hit, inside.max_hit), (5, 7));
    close(outside.dps, p * 48.0 / 20.0 / 3.0);
    close(inside.dps, p * 68.0 / 20.0 / 3.0);
    assert!(outside.dps < normal.dps * 0.3);
    assert!(inside.dps < normal.dps * 0.4);
}

#[test]
fn whisperer_specials_reduce_scythe_splats_before_summing() {
    let r = calc(100135, 22325, None, "Slash", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20790, 94276, 10)
    );
    // Max23/11/5 becomes6/3/1, rather than floor(39*30%)=11.
    close(
        r.dps,
        20790.0 / 188554.0 * (72.0 / 24.0 + 15.0 / 12.0 + 2.0 / 6.0) / 3.0,
    );
}

#[test]
fn whisperer_special_ruby_effects_retain_the_transform_at_each_hp() {
    for (id, max, at105) in [(100135, 30, 6), (100136, 40, 8)] {
        let r = calc(id, 9185, Some(9242), "Ranged", "");
        assert_eq!(r.max_hit, max);
        let proc = r.hp_dependent.unwrap();
        assert_eq!(proc.outcomes(105), vec![(at105, 0.06)]);
        assert_eq!(proc.outcomes(5), vec![(0, 0.06)]);
    }
}

#[test]
fn whisperer_enrage_uses_140_hp_and_separate_debuff_assumptions() {
    let mut settings = Settings::default();
    settings.reductions.insert(
        12204,
        Reductions {
            accursed: true,
            ..Default::default()
        },
    );
    let s = gear(1381, None);
    let main = combat::calculate_configured(
        &member(),
        &s,
        "Magic",
        planner::monster(12204).unwrap(),
        "Earth Bolt",
        0,
        &settings,
    )
    .unwrap();
    let enrage = combat::calculate_configured(
        &member(),
        &s,
        "Magic",
        planner::monster(100137).unwrap(),
        "Earth Bolt",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!((main.defence_roll, enrage.defence_roll), (11988, 13986));
    settings.reductions.insert(
        100137,
        Reductions {
            accursed: true,
            ..Default::default()
        },
    );
    let reapplied = combat::calculate_configured(
        &member(),
        &s,
        "Magic",
        planner::monster(100137).unwrap(),
        "Earth Bolt",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!(reapplied.defence_roll, 11988);
    let ruby = calc(100137, 9185, Some(9242), "Ranged", "");
    assert_eq!(ruby.hit_distribution.last(), Some(&(28, 0.06)));
    assert_eq!(
        combat::target_hp_bounds(planner::monster(100137).unwrap(), 0).unwrap(),
        (1, 140)
    );
    assert!(encounters::limitation(planner::monster(12206).unwrap()).is_some());
}

#[test]
fn nightmare_shields_are_explicit_counters_and_ruby_is_not_guessed() {
    for (id, start, progress) in [(9425, 2000, 6000.0), (9416, 400, 1200.0)] {
        let w = encounters::window(id).unwrap();
        assert_eq!(w.resource, Resource::Shield);
        assert_eq!(w.entity_count, 3);
        assert_eq!(encounters::progress_hp(id, "Melee"), Some(progress));
        assert_eq!(
            combat::target_hp_bounds(planner::monster(id).unwrap(), 0).unwrap(),
            (1, start)
        );
        assert_eq!(calc(id, 9185, Some(9144), "Ranged", "").max_hit, 30);
        let err = combat::calculate(
            &member(),
            &gear(9185, Some(9242)),
            "Ranged",
            planner::monster(id).unwrap(),
            "",
        )
        .unwrap_err();
        assert!(err.contains("shield and charge"), "{err}");
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &Reductions {
                bgs: 1000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], 120);
    }
    let final_phase = calc(100151, 9185, Some(9242), "Ranged", "");
    assert_eq!(final_phase.hit_distribution.last(), Some(&(30, 0.06)));
    assert_eq!(
        encounters::progress_hp(9416, "Melee").unwrap()
            + encounters::progress_hp(100151, "Melee").unwrap(),
        1350.0
    );
}

#[test]
fn totems_have_guaranteed_magic_and_double_post_weakness_damage() {
    for id in [9435, 100150] {
        let r = calc(id, 1381, None, "Magic", "Fire Bolt");
        assert_eq!((r.max_hit, r.accuracy), (40, 1.0));
        close(r.dps, 422.0 / 21.0 / 3.0); // accurate rolls0..20 become2,2,4,...40.
        let trident = calc(id, 11905, None, "Magic", "");
        assert_eq!((trident.max_hit, trident.accuracy), (56, 1.0));
        let mut bad = gear(1381, None);
        bad[4] = Some(10551);
        bad[7] = Some(1079);
        bad[0] = Some(1163);
        bad[5] = Some(1201);
        bad[10] = Some(4131);
        let armoured = combat::calculate(
            &member(),
            &bad,
            "Magic",
            planner::monster(id).unwrap(),
            "Fire Bolt",
        )
        .unwrap();
        assert!(armoured.attack_roll < 0);
        assert_eq!(armoured.accuracy, 1.0);
        close(armoured.dps, r.dps);
    }
}

#[test]
fn shared_totem_id_does_not_merge_normal_and_phosani_charge() {
    for (id, charge, total) in [(9435, 200, 2400.0), (100150, 300, 3600.0)] {
        let target = planner::monster(id).unwrap();
        assert_eq!(target.skills["hp"], charge);
        assert_eq!(encounters::source_npc_id(id), 9435);
        let w = encounters::window(id).unwrap();
        assert_eq!((w.resource, w.entity_count), (Resource::Charge, 12));
        assert_eq!(encounters::progress_hp(id, "Magic"), Some(total));
        let mut o = Options::default();
        o.combat.target_hp.insert(id, 1);
        let r = calc(id, 1381, None, "Magic", "Fire Bolt");
        let budget = attack_budget::for_window(target, &r, &o, &gear(1381, None)).unwrap();
        assert_eq!(
            (
                budget.start_hp,
                budget.stop_hp,
                budget.entities,
                budget.attacks
            ),
            (1, 0, 12, 12.0)
        );
        close(budget.seconds, 36.0);
    }
    assert!(!planner::monster(100150).unwrap().name.contains("Phosani"));
    let a = encounters::catalog()
        .iter()
        .find(|e| e.name == "Nightmare")
        .unwrap();
    assert!(a.targets.iter().any(|t| t.id == 100150));
    assert!(!a.targets.iter().any(|t| t.id == 9435));
}

#[test]
fn phosani_crush_guarantees_maximum_damage_without_guaranteeing_a_kill() {
    let mut weak = member();
    weak.level_overrides.insert("Strength".into(), 1);
    let s = gear(25979, None);
    for id in [9466, 9467, 9469] {
        let r = combat::calculate(&weak, &s, "Crush", planner::monster(id).unwrap(), "").unwrap();
        assert_eq!(r.accuracy, 1.0);
        assert!(r.max_hit > 0 && r.max_hit < 20);
        assert_eq!(r.hit_distribution, vec![(r.max_hit, 1.0)]);
        let hp = planner::monster(id).unwrap().skills["hp"] as u32;
        close(
            attack_budget::expected_attacks(&r.hit_distribution, hp, 0, None).unwrap(),
            f64::from(hp.div_ceil(r.max_hit as u32)),
        );
    }
    assert!(!monster_rules::guaranteed_max(9466, "slash"));
    for id in [9454, 9455, 9453, 9468] {
        assert!(!monster_rules::guaranteed_max(id, "crush"));
    }
    for id in [9468, 9436] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
    }
    assert!(
        encounters::limitation(planner::monster(9468).unwrap())
            .unwrap()
            .contains("unweakened parasite")
    );
}

#[test]
fn protected_bodies_and_incoming_limits_remain_explicit() {
    for id in [100152, 100153] {
        let r = calc(id, 9185, Some(9242), "Ranged", "");
        assert_eq!((r.max_hit, r.dps), (0, 0.0));
        assert_eq!(r.hp_dependent.unwrap().outcomes(200), vec![(0, 0.06)]);
    }
    for id in [12214, 12204, 9416, 9425, 9469, 100132, 9468, 9436] {
        assert!(
            defence::estimate(
                &member(),
                &gear(4151, None),
                planner::monster(id).unwrap(),
                "Magic",
                &Settings::default()
            )
            .is_err()
        );
    }
}

#[test]
fn shield_charge_and_enrage_assumptions_survive_setup_links_and_exports() {
    let o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 100150,
                style: "Magic".into(),
            },
            Target {
                id: 100137,
                style: "Magic".into(),
            },
        ],
        ..Default::default()
    };
    let snapshot = account::demo();
    let plan = planner::plan(&snapshot, &o);
    let setup = ironbound::share::Setup::from_plan(&plan, &o);
    let shared = ironbound::share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(shared.options().targets, o.targets);
    let note = coverage::export_note(&o);
    assert!(note.contains("300 charge points"), "{note}");
    assert!(note.contains("140 HP"), "{note}");
}

#[test]
fn moon_followups_apply_each_targets_transform_to_the_correct_attack_style() {
    let mut s = gear(28988, None);
    for (slot, id) in [(0, 29019), (4, 29013), (7, 29016)] {
        s[slot] = Some(id);
    }
    for id in [100135, 9435] {
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

#[test]
fn new_special_and_counter_states_do_not_inherit_unverified_burn_pulses() {
    let mut s = gear(29000, Some(28991));
    for (slot, id) in [(0, 29010), (4, 29004), (7, 29007)] {
        s[slot] = Some(id);
    }
    for id in [100130, 100135, 100136, 9425, 9416, 9435, 100150] {
        let err = combat::calculate(&member(), &s, "Ranged", planner::monster(id).unwrap(), "")
            .unwrap_err();
        assert!(
            err.contains("pulse reductions") && err.contains("unverified"),
            "{id}: {err}"
        );
    }
    let ordinary =
        combat::calculate(&member(), &s, "Ranged", planner::monster(26).unwrap(), "").unwrap();
    assert!(ordinary.burn.is_some());
}
