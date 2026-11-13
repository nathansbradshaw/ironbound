use ironbound::{
    account::{self, Member},
    combat,
    defence::{self, Settings},
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
fn set(ids: &[(usize, i32)]) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for (slot, id) in ids {
        s[*slot] = Some(*id);
    }
    s
}
fn blood() -> Vec<Option<i32>> {
    set(&[(0, 29028), (4, 29022), (7, 29025), (3, 28997)])
}
fn blue() -> Vec<Option<i32>> {
    set(&[(0, 29019), (4, 29013), (7, 29016), (3, 28988)])
}
fn eclipse() -> Vec<Option<i32>> {
    set(&[(0, 29010), (4, 29004), (7, 29007), (3, 29000), (13, 28991)])
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
    spell: &str,
    settings: &Settings,
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
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(max: i32) -> f64 {
    (0..=max).map(|n| f64::from(n.max(1))).sum::<f64>() / f64::from(max + 1)
}
fn snapshot(s: &[Option<i32>]) -> account::Snapshot {
    account::Snapshot {
        members: vec![Member {
            name: "Set tests".into(),
            bank: Some(
                s.iter()
                    .flatten()
                    .flat_map(|id| [i64::from(*id), 1000])
                    .chain([8007, 1])
                    .collect(),
            ),
            ..member()
        }],
        selected: "Set tests".into(),
        source: "Synthetic".into(),
        fetched_at: String::new(),
    }
}
#[test]
fn macuahuitl_second_accuracy_depends_on_first_and_splits_maximum() {
    let r = calculate(&set(&[(3, 28997)]), "Crush", "", &Settings::default());
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit, r.attack_ticks),
        (20350, 1216, 24, 4)
    );
    let a = 1.0 - 1218.0 / (2.0 * 20351.0);
    close(r.accuracy, a);
    close(r.dps, (a * mean(12) + a * a * mean(12)) / 2.4);
    close(
        r.hit_distribution
            .iter()
            .filter(|(d, _)| *d == 0)
            .map(|(_, p)| p)
            .sum(),
        1.0 - a,
    );
    assert!(calculate(&set(&[(3, 28997)]), "Stab", "", &Settings::default()).dps > 0.0);
}
#[test]
fn bloodrager_uses_exact_33_percent_per_accurate_splat_and_finite_hp_cooldown() {
    let s = blood();
    let r = calculate(&s, "Crush", "", &Settings::default());
    let a = r.accuracy;
    close(r.expected_attack_ticks, 4.0 - (a * 0.33 + a * a * 0.2211));
    let mut missing = s.clone();
    missing[0] = None;
    assert_eq!(
        calculate(&missing, "Crush", "", &Settings::default()).expected_attack_ticks,
        4.0
    );
    let budget = ironbound::attack_budget::for_window(
        planner::monster(26).unwrap(),
        &r,
        &options("Crush"),
        &s,
    )
    .unwrap();
    close(
        budget.seconds,
        budget.attacks * r.expected_attack_ticks * 0.6,
    );
    let target = planner::monster(13013).unwrap();
    let acid =
        combat::calculate_configured(&member(), &s, "Crush", target, "", 0, &Settings::default())
            .unwrap();
    assert_eq!(
        acid.max_hit,
        r.max_hit - 2 * target.defensive["flat_armour"]
    );
}
#[test]
fn full_blood_loadout_is_discovered_and_pins_exclusions_levels_and_variants_apply() {
    let s = blood();
    let bank = snapshot(&s);
    let mut o = options("Crush");
    let chosen = planner::equipment_for_style(&bank, &o, "Crush");
    assert!(rules::set_effect(&chosen, "blood_moon"), "{chosen:?}");
    o.excluded.insert(29028);
    assert!(!rules::set_effect(
        &planner::equipment_for_style(&bank, &o, "Crush"),
        "blood_moon"
    ));
    o.excluded.clear();
    o.gear_overrides.insert("Crush:0".into(), 0);
    assert!(!rules::set_effect(
        &planner::equipment_for_style(&bank, &o, "Crush"),
        "blood_moon"
    ));
    let mut m = member();
    m.level_overrides.insert("Strength".into(), 74);
    assert!(!planner::can_use(28997, &m));
    assert!(!planner::can_use(29073, &m));
    let broken = set(&[(0, 29073), (4, 29043), (7, 29045), (3, 28997)]);
    assert!(rules::set_effect(&broken, "blood_moon"));
    assert!(calculate(&broken, "Crush", "", &Settings::default()).expected_attack_ticks < 4.0);
    assert!(rules::set_effect(
        &planner::equipment_for_style(&snapshot(&broken), &options("Crush"), "Crush"),
        "blood_moon"
    ));
}
#[test]
fn atlatl_uses_strength_level_melee_strength_and_ranged_prayers() {
    let s = set(&[(3, 29000), (13, 28991)]);
    let mut m = member();
    m.level_overrides.insert("Ranged".into(), 80);
    let target = planner::monster(26).unwrap();
    let settings = Settings::default();
    let r = combat::calculate_configured(&m, &s, "Ranged", target, "", 0, &settings).unwrap();
    assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (13288, 17, 3));
    m.level_overrides.insert("Strength".into(), 50);
    assert_eq!(
        combat::calculate_configured(&m, &s, "Ranged", target, "", 0, &settings)
            .unwrap()
            .max_hit,
        9
    );
    let e = calculate(
        &eclipse(),
        "Ranged",
        "",
        &Settings {
            offensive: "Deadeye".into(),
            ..Default::default()
        },
    );
    let p = calculate(
        &eclipse(),
        "Ranged",
        "",
        &Settings {
            offensive: "Piety".into(),
            ..Default::default()
        },
    );
    assert!(e.max_hit > p.max_hit);
    let mut wrong = s.clone();
    wrong[13] = Some(892);
    assert!(combat::calculate(&member(), &wrong, "Ranged", target, "").is_err());
    assert!(!rules::RangedType::Heavy.permits_weapon(29000));
}
#[test]
fn eclipse_burn_is_delayed_capped_and_respects_npc_immunity() {
    let s = eclipse();
    let r = calculate(&s, "Ranged", "", &Settings::default());
    assert!(r.burn_dps > 0.0);
    let direct = r
        .hit_distribution
        .iter()
        .map(|(d, p)| f64::from(*d) * p)
        .sum::<f64>()
        / 1.8;
    close(r.dps, direct + r.burn_dps);
    assert!(r.burn_dps < r.accuracy * 2.0 / 1.8);
    assert!(
        ironbound::attack_budget::for_window(
            planner::monster(26).unwrap(),
            &r,
            &options("Ranged"),
            &s
        )
        .unwrap_err()
        .contains("not treated as instant")
    );
    for id in [
        13011, 13012, 13013, 7706, 4797, 14779, 16305, 7851, 7540, 2137,
    ] {
        assert!(ironbound::moon::burn_immune(id), "{id}");
    }
    let mut immune = planner::monster(13011).unwrap().clone();
    immune.defensive.insert("flat_armour".into(), 0);
    let moon = combat::calculate(&member(), &s, "Ranged", &immune, "").unwrap();
    assert_eq!(moon.burn_dps, 0.0);
    let mut missing = s.clone();
    missing[7] = None;
    assert_eq!(
        calculate(&missing, "Ranged", "", &Settings::default()).burn_dps,
        0.0
    );
    assert!(rules::set_effect(
        &planner::equipment_for_style(&snapshot(&s), &options("Ranged"), "Ranged"),
        "eclipse_moon"
    ));
}
#[test]
fn burn_loss_formula_matches_independent_finite_state_stationary_enumeration() {
    for n in 1..=8usize {
        for cap in 1..=3usize {
            for p in [0.01, 0.2, 0.65] {
                let mut states = vec![0.0; 1 << n];
                states[0] = 1.0;
                for _ in 0..2000 {
                    let mut next = vec![0.0; 1 << n];
                    for (mask, weight) in states.iter().enumerate() {
                        let shifted = (mask << 1) & ((1 << n) - 1);
                        let chance = if mask.count_ones() < (cap as u32) {
                            p
                        } else {
                            0.0
                        };
                        next[shifted] += weight * (1.0 - chance);
                        next[shifted | 1] += weight * chance;
                    }
                    states = next;
                }
                let accepted = p * states
                    .iter()
                    .enumerate()
                    .filter(|(mask, _)| mask.count_ones() < (cap as u32))
                    .map(|(_, v)| v)
                    .sum::<f64>();
                close(
                    ironbound::moon::accepted_burns_per_attack(p, 1, (n + 1) as i32, cap),
                    accepted,
                );
            }
        }
    }
}
#[test]
fn frostweaver_combines_conditional_magic_and_independent_melee_only_when_enabled() {
    let s = blue();
    let off = calculate(&s, "Magic", "Undead Grasp", &Settings::default());
    let enabled = Settings {
        blue_moon_melee: "Stab".into(),
        ..Default::default()
    };
    let melee = calculate(&s, "Stab", "", &enabled);
    let on = calculate(&s, "Magic", "Undead Grasp", &enabled);
    close(on.dps, off.dps + off.accuracy * 0.5 * melee.dps); // Both have a five-tick cycle.
    close(on.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    assert_eq!(on.max_hit, off.max_hit + melee.max_hit);
    let wave_off = calculate(&s, "Magic", "Wind Wave", &Settings::default());
    close(
        calculate(&s, "Magic", "Wind Wave", &enabled).dps,
        wave_off.dps,
    );
    let ice_off = calculate(&s, "Magic", "Ice Barrage", &Settings::default());
    close(
        calculate(&s, "Magic", "Ice Barrage", &enabled).dps,
        ice_off.dps + ice_off.accuracy * 0.2 * melee.dps,
    );
    let mut broken = s.clone();
    broken[4] = Some(29058);
    assert!(rules::set_effect(&broken, "blue_moon"));
    let mut incomplete = s.clone();
    incomplete[0] = None;
    close(
        calculate(&incomplete, "Magic", "Undead Grasp", &enabled).dps,
        calculate(&incomplete, "Magic", "Undead Grasp", &Settings::default()).dps,
    );
}
#[test]
fn blue_spear_melee_needs_no_runes_but_magic_respects_exact_supplies_and_selection() {
    let s = blue();
    let m = member();
    let empty = std::collections::BTreeMap::new();
    let settings = Settings::default();
    assert!(rules::casting_supported_for_style(
        &m, &s, &empty, &settings, "Stab"
    ));
    assert!(!rules::casting_supported_for_style(
        &m, &s, &empty, &settings, "Magic"
    ));
    let plan = planner::plan(&snapshot(&s), &options("Stab"));
    assert!(plan.missing.is_empty(), "{:?}", plan.missing);
    assert!(
        plan.inventory
            .iter()
            .all(|p| ![555, 560, 565, 554].contains(&p.id))
    );
    let mut o = options("Magic");
    o.combat.spell_selection = rules::SpellSelection::Spell("Undead Grasp".into());
    let mut bank = snapshot(&s);
    bank.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([554, 12000, 565, 1000]);
    let chosen = planner::equipment_for_style(&bank, &o, "Magic");
    assert!(rules::set_effect(&chosen, "blue_moon"));
    assert!(rules::casting_supported_for_style(
        &m,
        &chosen,
        &bank.available(false),
        &o.combat,
        "Magic"
    ));
    assert_eq!(
        rules::rune_supplies_for_set("Undead Grasp", 100, &s),
        vec![(554, 1200), (565, 100)]
    );
}
#[test]
fn virtus_bonus_applies_per_piece_only_to_ancient_spells_and_ahrim_needs_full_amulet_set() {
    let mut s = set(&[(3, 4675)]);
    assert_eq!(
        calculate(&s, "Magic", "Ice Barrage", &Settings::default()).max_hit,
        30
    );
    for (slot, id) in [(0, 26241), (4, 26243), (7, 26245)] {
        s[slot] = Some(id);
        // Base two percent plus three Ancient-only percent per piece.
        let pieces = s
            .iter()
            .flatten()
            .filter(|id| [26241, 26243, 26245].contains(id))
            .count() as i32;
        assert_eq!(
            calculate(&s, "Magic", "Ice Barrage", &Settings::default()).max_hit,
            30 + 30 * pieces * 5 / 100
        );
    }
    assert_eq!(
        calculate(&s, "Magic", "Wind Wave", &Settings::default()).max_hit,
        21
    );
    let ahrim = set(&[(0, 4708), (3, 4710), (4, 4712), (7, 4714), (2, 12851)]);
    assert!(calculate(&ahrim, "Magic", "Ice Barrage", &Settings::default()).dps > 0.0);
    let mut incomplete = ahrim.clone();
    incomplete[2] = None;
    assert!(
        combat::calculate(
            &member(),
            &incomplete,
            "Magic",
            planner::monster(26).unwrap(),
            "Ice Barrage"
        )
        .is_err()
    );
}
#[test]
fn justiciar_uses_matching_defence_per_hit_and_never_increases_negative_magic_damage() {
    for (hit, bonus, expected) in [
        (0, 400, 0),
        (1, 400, 0),
        (20, 300, 18),
        (11, 301, 9),
        (20, -100, 19),
    ] {
        assert_eq!(defence::justiciar_hit(hit, bonus), expected);
    }
    let s = set(&[(0, 22326), (4, 22327), (7, 22328)]);
    let m = member();
    let target = planner::monster(2215).unwrap();
    let r = defence::estimate(&m, &s, target, "Crush", &Settings::default()).unwrap();
    let bonus = combat::bonuses(&s).defensive["crush"];
    let expected = (0..=60)
        .map(|hit| {
            ((f64::from(hit) * (1.0 - f64::from(bonus) / 3000.0)).floor() as i32).max(0) as f64
        })
        .sum::<f64>()
        / 61.0;
    close(r.damage_per_second, r.hit_chance * expected / 3.6);
    let protected = Settings {
        protection: "Melee".into(),
        ..Default::default()
    };
    assert_eq!(
        defence::estimate(&m, &s, target, "Crush", &protected)
            .unwrap()
            .damage_per_second,
        0.0
    );
    assert!(rules::set_effect(&s, "justiciar"));
    let mut partial = s.clone();
    partial[0] = None;
    assert!(!rules::set_effect(&partial, "justiciar"));
    let moon = planner::monster(13011).unwrap();
    let r = defence::estimate(&m, &s, moon, "Chained slash", &Settings::default()).unwrap();
    close(
        r.damage_per_second,
        (r.hit_chance * 2.0 + r.hit_chance.powi(2) * 4.0 + r.hit_chance.powi(3) * 10.0) / 3.6,
    );
}
#[test]
fn torag_amulet_missing_health_defence_follows_prayer_before_magic_weighting() {
    let s = set(&[(0, 4745), (4, 4749), (7, 4751), (3, 4747), (2, 12851)]);
    let target = planner::monster(2205).unwrap();
    let full = defence::estimate(&member(), &s, target, "Crush", &Settings::default()).unwrap();
    let settings = Settings {
        current_hitpoints: Some(1),
        offensive: "Piety".into(),
        ..Default::default()
    };
    let low = defence::estimate(&member(), &s, target, "Crush", &settings).unwrap();
    let bonus = combat::bonuses(&s).defensive["crush"];
    let player = (99 * 125 / 100 * 199 / 100 + 8) * (bonus + 64);
    let npc = (280 + 9) * (195 + 64);
    close(low.hit_chance, combat::accuracy(npc.into(), player.into()));
    assert!(low.damage_per_second < full.damage_per_second);
    let mut noamulet = s.clone();
    noamulet[2] = None;
    let a = defence::estimate(&member(), &noamulet, target, "Magic", &settings).unwrap();
    assert!(
        defence::estimate(&member(), &s, target, "Magic", &settings)
            .unwrap()
            .hit_chance
            < a.hit_chance
    );
}

#[test]
fn complete_moon_sets_compete_against_stronger_individual_armour() {
    for (equipment, style, effect, spell, bait) in [
        (
            blood(),
            "Crush",
            "blood_moon",
            "",
            vec![26382, 26384, 26386],
        ),
        (
            eclipse(),
            "Ranged",
            "eclipse_moon",
            "",
            vec![26382, 26384, 26386],
        ),
        (
            blue(),
            "Magic",
            "blue_moon",
            "Undead Grasp",
            vec![21018, 21021, 21024],
        ),
    ] {
        let mut bank = snapshot(&equipment);
        bank.members[0].bank.as_mut().unwrap().extend(
            bait.iter()
                .flat_map(|id| [i64::from(*id), 1])
                .chain([554, 12000, 565, 1000]),
        );
        for recommended in [false, true] {
            let mut o = options(style);
            o.recommended = Some(recommended);
            if !spell.is_empty() {
                o.combat.spell_selection = rules::SpellSelection::Spell(spell.into());
                o.combat.blue_moon_melee = "Stab".into();
            }
            let chosen = planner::equipment_for_style(&bank, &o, style);
            assert!(rules::set_effect(&chosen, effect), "{style}: {chosen:?}");
        }
    }
}

#[test]
fn frostweaver_applies_debuffs_and_acid_reduction_once_to_each_style() {
    let s = blue();
    for id in [2215, 100020, 319] {
        let mut settings = Settings {
            blue_moon_melee: "Stab".into(),
            ..Default::default()
        };
        settings.reductions.insert(
            id,
            ironbound::modifiers::Reductions {
                bgs: 20,
                ..Default::default()
            },
        );
        let target = planner::monster(id).unwrap();
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
fn frostweaver_does_not_block_magic_or_add_melee_damage_when_the_spear_cannot_reach() {
    let s = blue();
    let target = planner::monster(2042).unwrap();
    let settings = Settings {
        blue_moon_melee: "Stab".into(),
        ..Default::default()
    };
    let on =
        combat::calculate_configured(&member(), &s, "Magic", target, "Undead Grasp", 0, &settings)
            .unwrap();
    let off = combat::calculate_configured(
        &member(),
        &s,
        "Magic",
        target,
        "Undead Grasp",
        0,
        &Settings::default(),
    )
    .unwrap();
    close(on.dps, off.dps);
    assert_eq!(on.max_hit, off.max_hit);
}

#[test]
fn moon_assumptions_survive_sharing_and_export_and_old_setups_default_to_melee_off() {
    let mut bank = snapshot(&blue());
    bank.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([554, 12000, 565, 1000]);
    let mut o = options("Magic");
    o.combat.spell_selection = rules::SpellSelection::Spell("Undead Grasp".into());
    o.combat.blue_moon_melee = "Stab".into();
    let plan = planner::plan(&bank, &o);
    let shared = ironbound::share::Setup::from_plan(&plan, &o);
    let restored = ironbound::share::Setup::parse(&shared.fragment().unwrap()).unwrap();
    assert_eq!(restored.combat.blue_moon_melee, "Stab");
    let export = ironbound::export::inventory_setup(&plan, &o, "Moon regression").unwrap();
    assert!(export.contains("Blue Moon free melee: Stab"));
    assert_eq!(
        serde_json::from_str::<Settings>("{}")
            .unwrap()
            .blue_moon_melee,
        "Off"
    );
    assert!(
        !Settings {
            blue_moon_melee: "Magic".into(),
            ..Default::default()
        }
        .valid()
    );
}
