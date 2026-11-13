//! Independent arithmetic from the pinned equipment/combat sources in
//! docs/recent-equipment.md. Synthetic accounts and banks only.
use ironbound::{
    account::{self, Member, Snapshot},
    attack_budget, combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    rules,
};

fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn set(items: &[(usize, i32)]) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for (slot, id) in items {
        s[*slot] = Some(*id);
    }
    s
}
fn target() -> planner::Monster {
    let mut t = planner::monster(26).unwrap().clone();
    t.attributes.clear();
    t.weakness = None;
    t.defensive.clear();
    for key in [
        "stab", "slash", "crush", "magic", "light", "standard", "heavy",
    ] {
        t.defensive.insert(key.into(), 0);
    }
    t.skills.insert("def".into(), 99);
    t.skills.insert("magic".into(), 99);
    t
}
fn calc(
    s: &[Option<i32>],
    style: &str,
    spell: &str,
    t: &planner::Monster,
    settings: &Settings,
) -> combat::ResultStats {
    combat::calculate_configured(&member(), s, style, t, spell, 0, settings).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(max: i32, transform: impl Fn(i32) -> i32) -> f64 {
    (0..=max).map(|d| transform(d) as f64).sum::<f64>() / f64::from(max + 1)
}
fn distribution(r: &combat::ResultStats) {
    close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    close(
        r.dps,
        r.hit_distribution
            .iter()
            .map(|(d, p)| *d as f64 * p)
            .sum::<f64>()
            / (r.expected_attack_ticks * 0.6),
    );
    assert!(r.hit_distribution.iter().all(|(d, p)| *d >= 0 && *p >= 0.0));
}
fn bank(ids: &[(i32, i64)]) -> Snapshot {
    Snapshot {
        members: vec![Member {
            name: "Recent equipment fixture".into(),
            bank: Some(
                ids.iter()
                    .flat_map(|(id, q)| [i64::from(*id), *q])
                    .collect(),
            ),
            ..member()
        }],
        selected: "Recent equipment fixture".into(),
        fetched_at: String::new(),
        source: "Synthetic".into(),
    }
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

#[test]
fn twinflame_correlates_the_second_hit_and_preserves_six_tick_timing() {
    let r = calc(
        &set(&[(3, 30634)]),
        "Magic",
        "Wind Wave",
        &target(),
        &Settings::default(),
    );
    assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (9028, 30, 6));
    close(
        r.dps,
        r.accuracy * mean(22, |d| d.max(1) + d.max(1) * 2 / 5) / 3.6,
    );
    // Not two independent uniform distributions: both splats share one roll.
    assert!(
        r.hit_distribution
            .iter()
            .all(|(d, _)| (0..=22).any(|n| *d == 0 || *d == n.max(1) + n.max(1) * 2 / 5))
    );
    distribution(&r);
    let finite = attack_budget::for_hp(&r, 1, 0).unwrap();
    close(finite.attacks, 1.0 / r.accuracy);
    close(finite.seconds, 3.6 / r.accuracy);
}

#[test]
fn twinflame_only_adds_a_hit_to_elemental_bolt_blast_and_wave() {
    for (spell, max) in [
        ("Wind Strike", 8),
        ("Fire Bolt", 18),
        ("Earth Blast", 23),
        ("Wind Wave", 30),
        ("Fire Surge", 26),
        ("Entangle", 5),
    ] {
        let r = calc(
            &set(&[(3, 30634)]),
            "Magic",
            spell,
            &target(),
            &Settings::default(),
        );
        assert_eq!(r.max_hit, max, "{spell}");
        assert_eq!(r.attack_ticks, 6);
        distribution(&r);
    }
    let error = combat::calculate(
        &member(),
        &set(&[(3, 30634)]),
        "Magic",
        &target(),
        "Ice Barrage",
    )
    .unwrap_err();
    assert!(error.contains("cannot cast"));
}

#[test]
fn twinflame_applies_magic_armour_and_acid_reduction_to_each_splat() {
    let mut t = target();
    t.defensive.insert("magic_resistance".into(), 3);
    let r = calc(
        &set(&[(3, 30634)]),
        "Magic",
        "Wind Wave",
        &t,
        &Settings::default(),
    );
    assert_eq!(r.max_hit, 24); // (22-3) + (floor(22*.4)-3)
    close(
        r.dps,
        r.accuracy
            * mean(22, |d| {
                (d.max(1) - 3).max(0) + (d.max(1) * 2 / 5 - 3).max(0)
            })
            / 3.6,
    );
    t.defensive.remove("magic_resistance");
    t.id = 100020;
    let acid = calc(
        &set(&[(3, 30634)]),
        "Magic",
        "Wind Wave",
        &t,
        &Settings::default(),
    );
    close(
        acid.dps,
        acid.accuracy * mean(22, |d| d.max(1) / 2 + (d.max(1) * 2 / 5) / 2) / 3.6,
    );
    distribution(&acid);
}

#[test]
fn smoke_accuracy_is_additive_with_salve_but_task_is_a_later_multiplier() {
    let mut t = target();
    t.attributes.push("undead".into());
    let salve = calc(
        &set(&[(3, 30634), (2, 12018)]),
        "Magic",
        "Wind Wave",
        &t,
        &Settings::default(),
    );
    assert_eq!(salve.attack_roll, 10670); // floor(8208 * (1+.20+.10))
    assert_eq!(salve.max_hit, 36); // floor(20*1.30)=26, plus floor(26*.4)=10
    let settings = Settings {
        slayer_targets: [26].into_iter().collect(),
        ..Default::default()
    };
    let task = calc(
        &set(&[(3, 30634), (0, 11865)]),
        "Magic",
        "Wind Wave",
        &t,
        &settings,
    );
    assert_eq!(task.attack_roll, 10792); // floor(floor(108*79*1.10)*1.15)
    assert_eq!(task.max_hit, 35); // floor(22*1.15)=25, then 10
    let smoke = calc(
        &set(&[(3, 11998)]),
        "Magic",
        "Wind Wave",
        &target(),
        &Settings::default(),
    );
    assert_eq!((smoke.max_hit, smoke.attack_ticks), (22, 5));
}

#[test]
fn twinflame_free_runes_do_not_double_spell_costs_or_supply_air() {
    let s = set(&[(3, 30634)]);
    let water = rules::rune_supplies_for_set("Water Wave", 37, &s);
    let fire = rules::rune_supplies_for_set("Fire Wave", 37, &s);
    assert_eq!(water, vec![(556, 185), (565, 37)]);
    assert_eq!(fire, water);
    let o = options("Magic");
    let mut o = Options {
        combat: Settings {
            spell_casts: 37,
            spell_selection: rules::SpellSelection::Spell("Water Wave".into()),
            ..Default::default()
        },
        ..o
    };
    let snapshot = bank(&[(30634, 1), (556, 185), (565, 37), (8007, 1)]);
    o.gear_overrides.insert("Magic:3".into(), 30634);
    let plan = planner::plan(&snapshot, &o);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(plan.equipment[3], Some(30634));
    assert_eq!(
        plan.inventory
            .iter()
            .find(|i| i.id == 565)
            .unwrap()
            .quantity,
        37
    );
    assert!(!plan.inventory.iter().any(|i| i.id == 555 || i.id == 554));
    assert!(
        ironbound::export::inventory_setup(&plan, &o, "Twinflame regression")
            .unwrap()
            .contains("correlated 40% second hit")
    );
}

#[test]
fn twinflame_keeps_unverified_guaranteed_and_p2_extra_hits_gated() {
    for id in [100012, 11753] {
        let error = combat::calculate(
            &member(),
            &set(&[(3, 30634)]),
            "Magic",
            planner::monster(id).unwrap(),
            "Wind Wave",
        )
        .unwrap_err();
        assert!(error.contains("Twinflame"), "{error}");
    }
}

#[test]
fn dragon_wand_applies_dragonbane_before_weakness_and_task_floors() {
    let mut t = target();
    t.attributes.push("dragon".into());
    t.weakness = Some(planner::Weakness {
        element: "air".into(),
        severity: 50,
    });
    let s = set(&[(3, 30070)]);
    let r = calc(&s, "Magic", "Wind Wave", &t, &Settings::default());
    assert_eq!((r.attack_roll, r.max_hit), (19440, 40));
    let settings = Settings {
        slayer_targets: [26].into_iter().collect(),
        ..Default::default()
    };
    let task = calc(
        &set(&[(3, 30070), (0, 11865)]),
        "Magic",
        "Wind Wave",
        &t,
        &settings,
    );
    assert_eq!((task.attack_roll, task.max_hit), (22522, 45));
    t.attributes.clear();
    assert_eq!(
        calc(&s, "Magic", "Wind Wave", &t, &Settings::default()).max_hit,
        32
    );
    t.attributes.push("dragon".into());
    t.weakness = None;
    assert_eq!(
        calc(&s, "Magic", "Ice Barrage", &t, &Settings::default()).max_hit,
        46
    );
}

#[test]
fn scorching_bow_uses_dragon_arrows_and_additive_task_damage() {
    let mut t = target();
    t.attributes.push("demon".into());
    let s = set(&[(3, 29591), (13, 11212), (0, 11865)]);
    let off = calc(&s, "Ranged", "", &t, &Settings::default());
    let task = calc(
        &s,
        "Ranged",
        "",
        &t,
        &Settings {
            slayer_targets: [26].into_iter().collect(),
            ..Default::default()
        },
    );
    assert_eq!(
        (off.max_hit, task.max_hit, task.attack_roll),
        (35, 39, 30552)
    );
    assert_eq!(task.attack_ticks, 4);
    t.attributes.clear();
    assert_eq!(calc(&s, "Ranged", "", &t, &Settings::default()).max_hit, 27);
    let mut bad = s.clone();
    bad[13] = Some(9144);
    assert!(combat::calculate(&member(), &bad, "Ranged", &t, "").is_err());
    distribution(&task);
}

#[test]
fn new_powered_weapons_have_exact_charge_variants_levels_and_speed() {
    for (weapon, empty, required, max) in [(31113, 31115, 83, 27), (34725, 34727, 85, 26)] {
        let r = calc(
            &set(&[(3, weapon)]),
            "Magic",
            "",
            &target(),
            &Settings::default(),
        );
        assert_eq!((r.max_hit, r.attack_ticks), (max, 3));
        assert!(rules::weapon(empty).is_none());
        let mut low = member();
        low.level_overrides.insert("Magic".into(), required - 1);
        assert!(!planner::can_use(weapon, &low));
        low.level_overrides.insert("Magic".into(), required);
        assert!(planner::can_use(weapon, &low));
        distribution(&r);
    }
    let ayak = set(&[(3, 31113)]);
    let mut boosted = Settings::default();
    boosted.boosts.magic = "Saturated heart".into();
    assert_eq!(calc(&ayak, "Magic", "", &target(), &boosted).max_hit, 31);
}

#[test]
fn recent_melee_weapons_reuse_correct_styles_and_demonbane() {
    let t = target();
    let nox = calc(&set(&[(3, 29796)]), "Slash", "", &t, &Settings::default());
    assert_eq!(
        (nox.attack_roll, nox.max_hit, nox.attack_ticks),
        (21560, 34, 5)
    );
    assert_eq!(
        calc(&set(&[(3, 30955)]), "Slash", "", &t, &Settings::default()).max_hit,
        21
    );
    let mut demon = t.clone();
    demon.attributes.push("demon".into());
    let claws = calc(
        &set(&[(3, 29577)]),
        "Slash",
        "",
        &demon,
        &Settings::default(),
    );
    assert_eq!(claws.attack_roll, 13629);
    assert!(claws.dps > calc(&set(&[(3, 29577)]), "Slash", "", &t, &Settings::default()).dps);
    let kisten = calc(&set(&[(3, 33631)]), "Crush", "", &t, &Settings::default());
    assert_eq!(
        (kisten.attack_roll, kisten.max_hit, kisten.attack_ticks),
        (15840, 20, 4)
    );
}

#[test]
fn all_three_nagua_weapons_roll_two_independent_halves_with_per_splat_armour() {
    for (weapon, style) in [(29889, "Crush"), (29084, "Slash"), (30957, "Stab")] {
        let mut t = target();
        t.defensive.insert("flat_armour".into(), -5);
        let r = calc(&set(&[(3, weapon)]), style, "", &t, &Settings::default());
        assert_eq!(r.max_hit, 31); // base 21 -> 10+11, each accurate splat gains 5
        let p = combat::accuracy(110 * 136, 108 * 64);
        close(
            r.dps,
            p * (mean(10, |n| n.max(1) + 5) + mean(11, |n| n.max(1) + 5)) / 2.4,
        );
        distribution(&r);
    }
}

#[test]
fn charged_quiver_bonus_requires_compatible_arrows_or_bolts() {
    let t = target();
    for (weapon, ammo) in [(861, 892), (9185, 9144), (29591, 11212)] {
        let empty = calc(
            &set(&[(3, weapon), (13, ammo), (1, 28947)]),
            "Ranged",
            "",
            &t,
            &Settings::default(),
        );
        for cape in [28951, 28953, 28955, 28957, 28902, 28906] {
            let charged = calc(
                &set(&[(3, weapon), (13, ammo), (1, cape)]),
                "Ranged",
                "",
                &t,
                &Settings::default(),
            );
            assert_eq!(charged.attack_roll - empty.attack_roll, 1070);
            assert!(charged.max_hit >= empty.max_hit);
        }
    }
    for (weapon, ammo) in [(25865, None), (29000, Some(28991)), (4734, Some(4740))] {
        let mut s = set(&[(3, weapon), (1, 28947)]);
        s[13] = ammo;
        let empty = calc(&s, "Ranged", "", &t, &Settings::default());
        s[1] = Some(28951);
        let charged = calc(&s, "Ranged", "", &t, &Settings::default());
        assert_eq!(empty.attack_roll, charged.attack_roll);
        assert_eq!(empty.max_hit, charged.max_hit);
    }
}

#[test]
fn recent_armour_and_jewellery_requirements_have_real_boundaries() {
    for (ids, skill, required) in [
        (
            vec![30750, 30753, 30756, 30777, 30779, 30781],
            "Defence",
            78,
        ),
        (
            vec![26382, 26384, 26386, 28254, 28256, 28258],
            "Defence",
            80,
        ),
        (vec![27226, 27229, 27232], "Defence", 30),
        (vec![27235, 27238, 27241], "Defence", 80),
        (vec![27226, 27229, 27232, 27235, 27238, 27241], "Ranged", 80),
        (vec![29801, 29804, 33639], "Hitpoints", 90),
        (vec![30073, 30076], "Defence", 40),
        (vec![30073, 30076, 30079, 30082], "Ranged", 70),
        (vec![28933, 28936, 28939], "Prayer", 60),
        (vec![28933, 28936, 28939], "Defence", 40),
    ] {
        let mut m = member();
        m.level_overrides.insert(skill.into(), required - 1);
        for id in &ids {
            assert!(!planner::can_use(*id, &m), "{id} ignores {skill}");
        }
        m.level_overrides.insert(skill.into(), required);
        for id in ids {
            assert!(planner::can_use(id, &m), "{id} rejects {skill}={required}");
        }
    }
    for id in [28902, 28906] {
        let mut m = member();
        m.level_overrides.insert("Sailing".into(), 98);
        assert!(!planner::can_use(id, &m));
    }
}

#[test]
fn oathplate_cosmetics_keep_stats_and_do_not_create_an_invented_set_passive() {
    let base = set(&[(3, 4151), (0, 30750), (4, 30753), (7, 30756)]);
    let radiant = set(&[(3, 4151), (0, 30777), (4, 30779), (7, 30781)]);
    let normal = calc(&base, "Slash", "", &target(), &Settings::default());
    let cosmetic = calc(&radiant, "Slash", "", &target(), &Settings::default());
    assert_eq!(
        (normal.attack_roll, normal.max_hit),
        (cosmetic.attack_roll, cosmetic.max_hit)
    );
    assert_eq!(normal.attack_roll, 110 * (82 + 38 + 64));
    assert!(
        !rules::registry()
            .equipment_sets
            .iter()
            .any(|s| s.effect.contains("oathplate"))
    );
}

#[test]
fn recent_weapons_compete_with_moon_sets_without_borrowing_their_passives() {
    let snapshot = bank(&[
        (28997, 1),
        (29028, 1),
        (29022, 1),
        (29025, 1),
        (29796, 1),
        (8007, 1),
    ]);
    let mut o = options("Slash");
    o.gear_overrides.insert("Slash:3".into(), 29796);
    let chosen = planner::equipment_for_style(&snapshot, &o, "Slash");
    assert_eq!(chosen[3], Some(29796));
    assert!(!rules::set_effect(&chosen, "blood_moon"));
    close(
        calc(&chosen, "Slash", "", &target(), &Settings::default()).expected_attack_ticks,
        5.0,
    );
    let blue = set(&[(0, 29019), (4, 29013), (7, 29016), (3, 30634)]);
    let on = calc(
        &blue,
        "Magic",
        "Wind Wave",
        &target(),
        &Settings {
            blue_moon_melee: "Stab".into(),
            ..Default::default()
        },
    );
    let off = calc(&blue, "Magic", "Wind Wave", &target(), &Settings::default());
    close(on.dps, off.dps);
}

#[test]
fn new_multi_hit_weapons_respect_huey_tail_and_pillar_limits_per_splat() {
    for (id, earth_max, air_max) in [(14014, 18, 8), (100161, 22, 10)] {
        let t = planner::monster(id).unwrap();
        let s = set(&[(3, 30634)]);
        let earth = calc(&s, "Magic", "Earth Wave", t, &Settings::default());
        let air = calc(&s, "Magic", "Wind Wave", t, &Settings::default());
        assert_eq!(earth.max_hit, earth_max);
        assert_eq!(air.max_hit, air_max);
        distribution(&earth);
        distribution(&air);
        let glacial = calc(&set(&[(3, 29889)]), "Crush", "", t, &Settings::default());
        assert!(glacial.hit_distribution.iter().all(|(d, _)| *d >= 2));
    }
    let snapshot = bank(&[(29889, 1), (1434, 1), (8007, 1)]);
    let mut o = options("Crush");
    o.targets[0].id = 14014;
    assert_eq!(
        planner::equipment_for_style(&snapshot, &o, "Crush")[3],
        Some(29889)
    );
}
