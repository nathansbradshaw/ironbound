//! Arithmetic fixtures independently derived from the pinned calculator sources
//! listed in docs/equipment-backlog.md; synthetic accounts, no private banks.
use ironbound::{
    account::{self, Member},
    attack_budget, combat,
    defence::{self, Settings},
    equipment_effects,
    planner::{self, Options},
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
    let mut set = vec![None; 14];
    for (slot, id) in items {
        set[*slot] = Some(*id);
    }
    set
}
fn target(id: i32) -> planner::Monster {
    let mut target = planner::monster(id).unwrap().clone();
    target.attributes.clear();
    target.defensive.clear();
    for kind in [
        "stab", "slash", "crush", "light", "standard", "heavy", "magic",
    ] {
        target.defensive.insert(kind.into(), 0);
    }
    target.skills.insert("def".into(), 99);
    target
}
fn calc(
    set: &[Option<i32>],
    style: &str,
    target: &planner::Monster,
    settings: &Settings,
) -> combat::ResultStats {
    combat::calculate_configured(&member(), set, style, target, "Wind Wave", 0, settings).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn accurate_mean(max: i32) -> f64 {
    (0..=max).map(|n| n.max(1) as f64).sum::<f64>() / (max + 1) as f64
}
fn valid_distribution(r: &combat::ResultStats) {
    close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    close(
        r.hit_distribution
            .iter()
            .map(|(d, p)| *d as f64 * p)
            .sum::<f64>()
            / (r.expected_attack_ticks * 0.6),
        r.dps,
    );
    assert!(r.hit_distribution.iter().all(|(d, p)| *d >= 0 && *p >= 0.0));
}

#[test]
fn all_imbued_berserker_variants_calculate_identically() {
    let base = calc(
        &set(&[(3, 4151), (12, 11773)]),
        "Slash",
        &target(26),
        &Settings::default(),
    );
    for id in [25264, 26770] {
        assert!(rules::unsupported_equipment(id).is_none());
        let variant = calc(
            &set(&[(3, 4151), (12, id)]),
            "Slash",
            &target(26),
            &Settings::default(),
        );
        assert_eq!(variant.max_hit, base.max_hit);
        assert_eq!(variant.attack_roll, base.attack_roll);
        close(variant.dps, base.dps);
    }
}

#[test]
fn scythe_size_selects_independent_splats_and_preserves_finite_hp_distribution() {
    for id in [26, 1, 8059] {
        let target = target(id);
        let size = equipment_effects::target_size(id).unwrap().min(3);
        let r = calc(&set(&[(3, 22325)]), "Slash", &target, &Settings::default());
        assert_eq!(r.attack_roll, 110 * 189);
        let single_accuracy = combat::accuracy(20790, 108 * 64);
        let maxima: Vec<_> = (0..size).map(|i| 23 / (1 << i)).collect();
        assert_eq!(r.max_hit, maxima.iter().sum::<i32>());
        close(
            r.dps,
            single_accuracy * maxima.iter().map(|max| accurate_mean(*max)).sum::<f64>() / 3.0,
        );
        close(r.accuracy, 1.0 - (1.0 - single_accuracy).powi(size as i32));
        valid_distribution(&r);
        assert!(
            attack_budget::expected_attacks(&r.hit_distribution, 100, 0, None)
                .unwrap()
                .is_finite()
        );
    }
}

#[test]
fn scythe_applies_armour_to_each_splat_and_rejects_missing_size_and_uncharged_variants() {
    let mut t = target(8059);
    t.defensive.insert("flat_armour".into(), 4);
    let r = calc(&set(&[(3, 22325)]), "Slash", &t, &Settings::default());
    assert_eq!(r.max_hit, 19 + 7 + 1);
    for charged in [25736, 25739] {
        assert_eq!(
            calc(&set(&[(3, charged)]), "Slash", &t, &Settings::default()).max_hit,
            r.max_hit
        );
    }
    for empty in [22486, 25738, 25741] {
        assert!(rules::weapon(empty).is_none());
    }
    t.id = 999999;
    assert!(
        combat::calculate(&member(), &set(&[(3, 22325)]), "Slash", &t, "")
            .unwrap_err()
            .contains("target size")
    );
}

#[test]
fn soulreaper_stacks_add_to_strength_after_prayer_and_round_before_stance() {
    let s = set(&[(3, 28338)]);
    assert_eq!(
        calc(&s, "Slash", &target(26), &Settings::default()).max_hit,
        32
    );
    let mut settings = Settings {
        soulreaper_stacks: 5,
        ..Default::default()
    };
    assert_eq!(calc(&s, "Slash", &target(26), &settings).max_hit, 40);
    settings.offensive = "Piety".into();
    // floor(99*1.23) + floor(99*.30) + 8 = 158; (158*189+320)/640 = 47.
    assert_eq!(calc(&s, "Slash", &target(26), &settings).max_hit, 47);
    settings.soulreaper_stacks = 6;
    assert!(!settings.valid());
    let legacy: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(legacy.soulreaper_stacks, 0);
    let copy: Settings = serde_json::from_str(
        &serde_json::to_string(&Settings {
            soulreaper_stacks: 4,
            bulwark_block: true,
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(copy.soulreaper_stacks, 4);
    assert!(copy.bulwark_block);
}

#[test]
fn macuahuitl_second_hit_depends_on_first_accuracy_and_armour_is_per_splat() {
    let mut t = target(26);
    let s = set(&[(3, 28997)]);
    let base = calc(&s, "Crush", &t, &Settings::default());
    let p = combat::accuracy(110 * 185, 108 * 64);
    assert_eq!(base.max_hit, 24);
    close(
        base.dps,
        p * (accurate_mean(12) + p * accurate_mean(12)) / 2.4,
    );
    valid_distribution(&base);
    t.defensive.insert("flat_armour".into(), 5);
    let armoured = calc(&s, "Crush", &t, &Settings::default());
    assert_eq!(armoured.max_hit, 14);
    let mean = (0..=12).map(|n| (n.max(1) - 5).max(0) as f64).sum::<f64>() / 13.0;
    close(armoured.dps, p * (mean + p * mean) / 2.4);
}

#[test]
fn blood_moon_uses_accuracy_dependent_expected_timing_and_requires_all_active_pieces() {
    let s = set(&[(3, 28997), (0, 29028), (4, 29022), (7, 29025)]);
    let r = calc(&s, "Crush", &target(26), &Settings::default());
    close(
        r.expected_attack_ticks,
        4.0 - r.accuracy / 3.0 - r.accuracy * r.accuracy * 2.0 / 9.0,
    );
    valid_distribution(&r);
    for slot in [0, 4, 7] {
        let mut partial = s.clone();
        partial[slot] = None;
        close(
            calc(&partial, "Crush", &target(26), &Settings::default()).expected_attack_ticks,
            4.0,
        );
    }
    let mut broken = s.clone();
    broken[0] = Some(29073);
    assert!(!rules::set_effect(&broken, "blood_moon"));
}

#[test]
fn atlatl_uses_strength_not_ranged_damage_and_blue_spear_support_is_explicit() {
    let mut m = member();
    m.level_overrides.insert("Strength".into(), 50);
    let s = set(&[(3, 29000), (13, 28991)]);
    let r = combat::calculate(&m, &s, "Ranged", &target(26), "").unwrap();
    assert_eq!(r.max_hit, 9); // floor((58 * (40+64)+320)/640)
    assert_eq!(r.attack_roll, 107 * 151);
    assert_eq!(r.attack_ticks, 3);
    let mut stronger = m.clone();
    stronger.level_overrides.insert("Strength".into(), 99);
    assert_eq!(
        combat::calculate(&stronger, &s, "Ranged", &target(26), "")
            .unwrap()
            .max_hit,
        17
    );
    assert_eq!(
        calc(
            &set(&[(3, 28988)]),
            "Magic",
            &target(26),
            &Settings::default()
        )
        .max_hit,
        21
    );
    assert!(
        equipment_effects::notes(&set(&[(3, 28988)]), &Settings::default())[0].contains("binding")
    );
}

#[test]
fn eclipse_burn_is_gated_instead_of_silently_scoring_a_full_set_without_its_passive() {
    let s = set(&[(3, 29000), (13, 28991), (0, 29010), (4, 29004), (7, 29007)]);
    assert!(
        combat::calculate(&member(), &s, "Ranged", &target(26), "")
            .unwrap_err()
            .contains("burn")
    );
}

#[test]
fn opal_and_pearl_can_proc_on_misses_and_diary_and_zaryte_modifiers_are_separate() {
    for (weapon, bolt, chance, extra) in [
        (9185, 9236, 0.05, 9),
        (26374, 21932, 0.05, 11),
        (9185, 9238, 0.06, 4),
        (26374, 21936, 0.06, 5),
    ] {
        for diary in [false, true] {
            let settings = Settings {
                kandarin_diary: diary,
                ..Default::default()
            };
            let s = set(&[(3, weapon), (13, bolt)]);
            let r = calc(&s, "Ranged", &target(26), &settings);
            let base_max = r.max_hit - extra;
            let c = chance * if diary { 1.1 } else { 1.0 };
            close(
                r.dps,
                (r.accuracy * (accurate_mean(base_max) - c / f64::from(base_max + 1))
                    + c * extra as f64)
                    / (r.attack_ticks as f64 * 0.6),
            );
            valid_distribution(&r);
        }
    }
}

#[test]
fn pearl_uses_fiery_divisor_and_dragonstone_requires_accuracy_and_respects_immunity() {
    let mut t = target(26);
    t.attributes.push("fiery".into());
    let pearl = calc(
        &set(&[(3, 9185), (13, 9238)]),
        "Ranged",
        &t,
        &Settings::default(),
    );
    let ordinary = calc(
        &set(&[(3, 9185), (13, 9238)]),
        "Ranged",
        &target(26),
        &Settings::default(),
    );
    assert_eq!(pearl.max_hit, ordinary.max_hit + 2);
    let s = set(&[(3, 9185), (13, 9244)]);
    let r = calc(&s, "Ranged", &target(26), &Settings::default());
    close(
        r.dps,
        r.accuracy
            * (accurate_mean(r.max_hit - 19) + 0.06 * (19.0 - 1.0 / f64::from(r.max_hit - 19 + 1)))
            / 3.0,
    );
    let immune = calc(&s, "Ranged", &t, &Settings::default());
    close(
        immune.dps,
        immune.accuracy * accurate_mean(immune.max_hit) / 3.0,
    );
    valid_distribution(&r);
}

#[test]
fn onyx_requires_accuracy_and_undead_are_immune() {
    for weapon in [9185, 26374] {
        let r = calc(
            &set(&[(3, weapon), (13, 9245)]),
            "Ranged",
            &target(26),
            &Settings::default(),
        );
        let ordinary = calc(
            &set(&[(3, weapon), (13, 9342)]),
            "Ranged",
            &target(26),
            &Settings::default(),
        );
        let proc_max = ordinary.max_hit * if weapon == 26374 { 132 } else { 120 } / 100;
        close(
            r.dps,
            r.accuracy * (0.89 * accurate_mean(ordinary.max_hit) + 0.11 * accurate_mean(proc_max))
                / 3.0,
        );
        valid_distribution(&r);
        let mut undead = target(26);
        undead.attributes.push("undead".into());
        close(
            calc(
                &set(&[(3, weapon), (13, 9245)]),
                "Ranged",
                &undead,
                &Settings::default(),
            )
            .dps,
            ordinary.dps,
        );
    }
}

#[test]
fn elysian_and_justiciar_reduce_each_hit_before_averaging() {
    let s = set(&[(5, 12817)]);
    close(
        equipment_effects::incoming_hit(1, 0, &s, &Settings::default()),
        0.3,
    );
    close(
        equipment_effects::incoming_hit(7, 0, &s, &Settings::default()),
        6.3,
    );
    close(
        equipment_effects::incoming_hit(8, 0, &s, &Settings::default()),
        6.6,
    );
    let justiciar = set(&[(0, 22326), (4, 22327), (7, 22328), (5, 12817)]);
    // Crush bonus includes shield; use an explicit 300 bonus for this arithmetic fixture.
    close(
        equipment_effects::incoming_hit(8, 300, &justiciar, &Settings::default()),
        0.3 * 7.0 + 0.7 * 5.0,
    );
    let mut partial = justiciar.clone();
    partial[0] = None;
    close(
        equipment_effects::incoming_hit(8, 300, &partial, &Settings::default()),
        6.6,
    );
    close(
        equipment_effects::incoming_hit(0, 300, &justiciar, &Settings::default()),
        0.0,
    );
}

#[test]
fn defensive_passives_enable_basic_hits_and_keep_chains_chip_and_healing_gated() {
    let s = set(&[(5, 12817)]);
    let r = defence::estimate(
        &member(),
        &s,
        planner::monster(2215).unwrap(),
        "Crush",
        &Settings::default(),
    )
    .unwrap();
    let mean = (0..=60)
        .map(|d| 0.3 * d as f64 + 0.7 * (d - (d / 4).max(1)).max(0) as f64)
        .sum::<f64>()
        / 61.0;
    close(
        r.damage_per_second,
        r.hit_chance * mean / (planner::monster(2215).unwrap().speed as f64 * 0.6),
    );
    for (id, attack) in [
        (13011, "Chained slash"),
        (11278, "Magic (Smoke / Ice)"),
        (1674, "Crush"),
    ] {
        let settings = Settings {
            protection: if id == 11278 { "Magic" } else { "None" }.into(),
            ..Default::default()
        };
        assert!(
            defence::estimate(
                &member(),
                &s,
                planner::monster(id).unwrap(),
                attack,
                &settings
            )
            .is_err()
        );
    }
}

#[test]
fn bulwark_block_is_explicit_applies_reduction_and_prevents_attacks() {
    let s = set(&[(3, 21015)]);
    close(
        equipment_effects::incoming_hit(9, 0, &s, &Settings::default()),
        9.0,
    );
    let block = Settings {
        bulwark_block: true,
        ..Default::default()
    };
    close(equipment_effects::incoming_hit(9, 0, &s, &block), 7.0);
    let boss = planner::monster(2215).unwrap();
    assert!(
        defence::estimate(&member(), &s, boss, "Crush", &block)
            .unwrap()
            .damage_per_second
            < defence::estimate(&member(), &s, boss, "Crush", &Settings::default())
                .unwrap()
                .damage_per_second
    );
    assert!(
        combat::calculate_configured(&member(), &s, "Crush", boss, "", 0, &block)
            .unwrap_err()
            .contains("prevents outgoing")
    );
    close(
        equipment_effects::incoming_hit(9, 0, &set(&[]), &block),
        9.0,
    );
    let rondache = set(&[(5, 34719)]);
    close(
        equipment_effects::incoming_hit(1, 0, &rondache, &Settings::default()),
        0.0,
    );
    close(
        equipment_effects::incoming_hit(9, 0, &rondache, &Settings::default()),
        7.0,
    );
}

#[test]
fn new_settings_survive_public_share_validation_without_overriding_missing_mechanics() {
    let mut o = Options::default();
    o.combat.soulreaper_stacks = 5;
    o.combat.bulwark_block = true;
    let snapshot = account::Snapshot {
        members: vec![Member {
            name: "Synthetic equipment".into(),
            bank: Some(vec![28338, 1]),
            ..member()
        }],
        selected: "Synthetic equipment".into(),
        source: "Synthetic".into(),
        fetched_at: String::new(),
    };
    let p = planner::plan(&snapshot, &o);
    let setup = ironbound::share::Setup::from_plan(&p, &o);
    let copy = ironbound::share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(copy.combat.soulreaper_stacks, 5);
    assert!(copy.combat.bulwark_block);
}
