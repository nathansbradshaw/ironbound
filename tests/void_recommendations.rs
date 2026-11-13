//! Synthetic owned banks. These regress search behavior, not new combat formulas.
use ironbound::{
    account, combat,
    planner::{self, Boss, Options, Target},
    ranking, rules,
};

fn bank(ids: &[i32]) -> account::Snapshot {
    let mut s = account::demo();
    let m = &mut s.members[0];
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m.bank = Some(ids.iter().flat_map(|id| [i64::from(*id), 1000]).collect());
    m.inventory = None;
    m.equipment = None;
    m.shared_bank = None;
    m.quests = None;
    s
}
fn options(id: i32, style: &str, recommended: bool) -> Options {
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id,
            style: style.into(),
        }],
        recommended: Some(recommended),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    o.ranking.weights = ranking::Weights {
        dps: 100.0,
        defence: 0.0,
        prayer: 0.0,
        weight: 0.0,
    };
    o
}
fn dps(s: &account::Snapshot, o: &Options, set: &[Option<i32>], style: &str) -> f64 {
    combat::calculate_configured(
        s.selected().unwrap(),
        set,
        style,
        planner::monster(o.targets[0].id).unwrap(),
        "",
        o.raid_level,
        &o.combat,
    )
    .unwrap()
    .dps
}
const RANGED: &[i32] = &[
    21012, 21905, 11664, 13072, 13073, 8842, 3749, 2497, 2491, 7462,
];

#[test]
fn ahrim_regression_finds_the_complete_set_in_both_ranking_modes() {
    let s = bank(RANGED);
    for recommended in [true, false] {
        let o = options(1672, "Ranged", recommended);
        let set = planner::equipment_for_style(&s, &o, "Ranged");
        assert!(rules::set_effect(&set, "elite_void_ranged"), "{set:?}");
        // Audit reproducer previously returned 4.306962, despite owning a set
        // that scores 4.746871 with the same crossbow and dragon bolts.
        assert!((dps(&s, &o, &set, "Ranged") - 4.746871).abs() < 0.000001);
    }
}

#[test]
fn normal_elite_and_mixed_void_work_without_an_encounter_guide() {
    for (top, legs, elite) in [
        (8839, 8840, false),
        (13072, 13073, true),
        (13072, 8840, false),
    ] {
        let s = bank(&[21012, 21905, 11664, top, legs, 8842, 3749, 2497, 2491, 7462]);
        let o = options(26, "Ranged", true);
        let set = planner::equipment_for_style(&s, &o, "Ranged");
        assert!(rules::set_effect(&set, "void_ranged"), "{set:?}");
        assert_eq!(rules::set_effect(&set, "elite_void_ranged"), elite);
    }
}

#[test]
fn all_melee_subtypes_and_magic_compare_full_void() {
    for style in ["Melee", "Stab", "Slash", "Crush"] {
        let s = bank(&[11889, 11665, 13072, 13073, 8842, 1127, 1079, 1169, 1065]);
        let o = options(26, style, true);
        let set = planner::equipment_for_style(&s, &o, style);
        assert!(rules::set_effect(&set, "void_melee"), "{style}: {set:?}");
    }
    let s = bank(&[11905, 11663, 13072, 13073, 8842, 4089, 4091, 4093, 4095]);
    for recommended in [true, false] {
        let o = options(1673, "Magic", recommended);
        let set = planner::equipment_for_style(&s, &o, "Magic");
        assert!(rules::set_effect(&set, "elite_void_magic"), "{set:?}");
    }
}

#[test]
fn raid_and_barrows_templates_also_compare_full_sets() {
    let s = bank(RANGED);
    for boss in [Boss::Barrows, Boss::Toa] {
        let mut o = options(1672, "Ranged", false);
        o.boss = boss;
        // A template must match the result of its corresponding single target.
        let id = if boss == Boss::Barrows { 1672 } else { 11730 };
        let single = options(id, "Ranged", false);
        let expected = planner::equipment_for_style(&s, &single, "Ranged");
        let set = planner::equipment_for_style(&s, &o, "Ranged");
        assert!(rules::set_effect(&expected, "void_ranged"));
        assert_eq!(set, expected);
    }
}

#[test]
fn higher_dps_conventional_armour_can_beat_an_owned_full_set() {
    let s = bank(&[
        21012, 21905, 11664, 13072, 13073, 8842, 27226, 27229, 27232, 26235,
    ]);
    for recommended in [true, false] {
        let o = options(1672, "Ranged", recommended);
        let set = planner::equipment_for_style(&s, &o, "Ranged");
        assert!(!rules::set_effect(&set, "void_ranged"), "{set:?}");
        let mut full = set.clone();
        for (slot, id) in [(0, 11664), (4, 13072), (7, 13073), (9, 8842)] {
            full[slot] = Some(id);
        }
        assert!(dps(&s, &o, &set, "Ranged") > dps(&s, &o, &full, "Ranged"));
    }
}

#[test]
fn target_defence_changes_the_winner_with_the_same_owned_bank() {
    let mut ids = RANGED.to_vec();
    ids.push(2503); // Include the conventional body's accuracy as well as legs.
    let s = bank(&ids);
    for recommended in [true, false] {
        for (id, expect_void) in [(1672, true), (1673, false)] {
            let o = options(id, "Ranged", recommended);
            let set = planner::equipment_for_style(&s, &o, "Ranged");
            assert_eq!(rules::set_effect(&set, "void_ranged"), expect_void);
        }
    }
}

#[test]
fn complete_sets_still_obey_the_shared_switch_and_inventory_budget() {
    let s = bank(&[
        21012, 21905, 11905, 11663, 11664, 13072, 13073, 8842, 3749, 2497, 2491, 7462, 4089, 4091,
        4093, 4095, 8007,
    ]);
    let mut o = options(1673, "Magic", true);
    o.targets.push(Target {
        id: 1672,
        style: "Ranged".into(),
    });
    o.max_switches = Some(3);
    o.switch_slots = 3;
    let plan = planner::plan(&s, &o);
    assert!(plan.exportable(), "{:?}", plan.missing);
    let report = plan.optimization.as_ref().unwrap();
    assert!(report.max_changes <= 3 && report.gear_slots <= 3);
    for phase in &plan.phases {
        let effect = if phase.style == "Magic" {
            "elite_void_magic"
        } else {
            "elite_void_ranged"
        };
        assert!(
            rules::set_effect(&phase.equipment, effect),
            "{:?}",
            phase.equipment
        );
    }
}

#[test]
fn ownership_levels_exclusions_and_pins_remain_binding() {
    let s = bank(RANGED);
    let base = options(1672, "Ranged", true);
    for slot in [0, 4, 7, 9] {
        let mut o = base.clone();
        o.gear_overrides.insert(format!("Ranged:{slot}"), 0);
        let set = planner::equipment_for_style(&s, &o, "Ranged");
        assert_eq!(set[slot], None);
        assert!(!rules::set_effect(&set, "void_ranged"));
    }
    let mut o = base.clone();
    o.gear_overrides.insert("Ranged:0".into(), 3749);
    assert_eq!(
        planner::equipment_for_style(&s, &o, "Ranged")[0],
        Some(3749)
    );
    o.gear_overrides.insert("Ranged:0".into(), 11664);
    assert!(rules::set_effect(
        &planner::equipment_for_style(&s, &o, "Ranged"),
        "void_ranged"
    ));
    o = base.clone();
    o.excluded.insert(8842);
    let set = planner::equipment_for_style(&s, &o, "Ranged");
    assert!(!set.contains(&Some(8842)));
    assert!(!rules::set_effect(&set, "void_ranged"));
    let mut low = s.clone();
    low.members[0].level_overrides.insert("Attack".into(), 41);
    assert!(!rules::set_effect(
        &planner::equipment_for_style(&low, &base, "Ranged"),
        "void_ranged"
    ));
    let partial = bank(&[21012, 21905, 11664, 13072, 13073, 3749, 2497, 2491, 7462]);
    assert!(!rules::set_effect(
        &planner::equipment_for_style(&partial, &base, "Ranged"),
        "void_ranged"
    ));
    let broken = bank(&[
        21012, 21905, 11664, 13072, 13073, 20475, 3749, 2497, 2491, 7462,
    ]);
    assert!(!rules::set_effect(
        &planner::equipment_for_style(&broken, &base, "Ranged"),
        "void_ranged"
    ));
}

#[test]
fn owned_cosmetic_variants_and_pins_preserve_the_actual_item_ids() {
    let s = bank(&[
        21012, 21905, 26475, 27003, 26471, 26467, 3749, 2497, 2491, 7462,
    ]);
    let mut o = options(1672, "Ranged", true);
    o.gear_overrides.insert("Ranged:4".into(), 27003);
    let set = planner::equipment_for_style(&s, &o, "Ranged");
    assert!(rules::set_effect(&set, "elite_void_ranged"));
    for (slot, id) in [(0, 26475), (4, 27003), (7, 26471), (9, 26467)] {
        assert_eq!(set[slot], Some(id));
    }
}

#[test]
fn unsupported_encounters_still_withhold_damage() {
    let s = bank(RANGED);
    let id = planner::monsters()
        .iter()
        .find(|m| m.name.contains("Archive Guardian"))
        .unwrap()
        .id;
    let o = options(id, "Ranged", true);
    let set = planner::equipment_for_style(&s, &o, "Ranged");
    assert!(
        combat::calculate(
            s.selected().unwrap(),
            &set,
            "Ranged",
            planner::monster(id).unwrap(),
            ""
        )
        .is_err()
    );
}
