//! Independent per-roll means and fixed-state limits from SOURCES.md.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::{self, Settings},
    encounters,
    monster_rules::{self, Transform},
    planner::{self, Boss, Options, Target},
};
const SHIELDED: i32 = 13599;
const DEFENCELESS: i32 = 100192;
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
    combat::calculate(&member(), set, style, planner::monster(id).unwrap(), spell)
        .unwrap_or_else(|e| panic!("target {id}, weapon {:?}, {style}, {spell}: {e}", set[3]))
}
fn exempt(weapon: i32, kind: &str, spell: &str) -> bool {
    monster_rules::transform_for(SHIELDED, weapon, kind, &combat::Bonuses::default(), spell)
        == Transform::Identity
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
fn ordinary(set: &[Option<i32>], style: &str, spell: &str) -> combat::ResultStats {
    let mut t = planner::monster(SHIELDED).unwrap().clone();
    t.id = 1;
    t.name = "Reference demon".into();
    combat::calculate(&member(), set, style, &t, spell).unwrap()
}
#[test]
fn shield_rounds_each_positive_splat_and_preserves_misses() {
    for (d, want) in [(0, 0), (1, 1), (2, 1), (4, 3), (5, 4), (9, 7), (100, 80)] {
        assert_eq!(
            Transform::TormentedShield.outcomes(d, 0.3),
            vec![(want, 0.3)]
        );
    }
    assert_eq!(
        monster_rules::transform_splats(&[2, 3], 1.0, Transform::TormentedShield),
        vec![(3, 1.0)]
    );
    let encoded = serde_json::to_string(&Transform::TormentedShield).unwrap();
    assert_eq!(
        serde_json::from_str::<Transform>(&encoded).unwrap(),
        Transform::TormentedShield
    );
}
#[test]
fn defenceless_keeps_the_fire_shield_but_replaces_accuracy() {
    let s = gear(1434, None);
    let shield = calc(SHIELDED, &s, "Crush", "");
    let free = calc(DEFENCELESS, &s, "Crush", "");
    assert_eq!(
        (shield.attack_roll, shield.defence_roll, shield.max_hit),
        (13640, 20988, 16)
    );
    let p = 13640.0 / 41978.0;
    close(shield.accuracy, p);
    close(free.accuracy, 1.0);
    // Raw 0..20; accurate zero becomes one before floor(4d/5), retaining one.
    close(mean(&shield), p * 162.0 / 21.0);
    close(mean(&free), 162.0 / 21.0);
    close(free.dps, 162.0 / 21.0 / 2.4);
    assert_eq!((free.attack_ticks, free.max_hit), (4, 16));
    assert!(
        shield
            .hit_distribution
            .iter()
            .any(|(d, p)| *d == 0 && *p > 0.0)
    );
    assert!(free.hit_distribution.iter().all(|(d, _)| *d >= 1));
}
#[test]
fn weapon_and_spell_exemptions_are_style_specific() {
    for (weapon, style, spell) in [
        (4151, "Slash", ""),
        (26482, "Slash", ""),
        (12006, "Slash", ""),
        (13265, "Stab", ""),
        (13263, "Crush", ""),
        (2402, "Slash", ""),
        (6746, "Slash", ""),
        (19675, "Slash", ""),
        (29589, "Slash", ""),
        (29577, "Slash", ""),
        (29591, "Ranged", ""),
        (29594, "Magic", "Dark Demonbane"),
    ] {
        let s = gear(weapon, if weapon == 29591 { Some(11212) } else { None });
        let kind = match style {
            "Slash" => "slash",
            "Stab" => "stab",
            "Crush" => "crush",
            "Ranged" => "standard",
            _ => "magic",
        };
        assert!(exempt(weapon, kind, spell));
        if ironbound::rules::weapon_for_style(weapon, style).is_none() {
            assert!(
                combat::calculate(
                    &member(),
                    &s,
                    style,
                    planner::monster(SHIELDED).unwrap(),
                    spell
                )
                .is_err()
            );
            continue;
        }
        let source = if style == "Magic" { 100190 } else { SHIELDED };
        let free_source = if style == "Magic" {
            100200
        } else {
            DEFENCELESS
        };
        let actual = calc(source, &s, style, spell);
        let reference = ordinary(&s, style, spell);
        close(mean(&actual), mean(&reference));
        assert_eq!(actual.max_hit, reference.max_hit, "{weapon}");
        let free = calc(free_source, &s, style, spell);
        close(free.accuracy, 1.0);
        assert_eq!(free.max_hit, reference.max_hit);
    }
    assert!(!exempt(4151, "magic", "Water Bolt"));
    assert!(!exempt(29591, "crush", ""));
    assert!(!exempt(11103, "slash", "")); // abyssal bracelet isn't a weapon exception
    let s = gear(1381, None);
    let water = calc(100200, &s, "Magic", "Water Bolt");
    assert_eq!(water.max_hit, 12); // elemental 30% weakness still applies before shield
    close(water.accuracy, 1.0);
}
#[test]
fn ruby_procs_and_finite_hp_recomputation_keep_the_shield() {
    let s = gear(9185, Some(9242));
    let r = calc(SHIELDED, &s, "Ranged", "");
    let ruby = r.hp_dependent.unwrap();
    assert_eq!(ruby.outcomes(600), vec![(80, 0.06)]);
    assert_eq!(ruby.outcomes(10), vec![(1, 0.06)]);
    let free = calc(DEFENCELESS, &s, "Ranged", "");
    close(free.accuracy, 1.0);
    assert_eq!(free.hp_dependent.unwrap().outcomes(600), vec![(80, 0.06)]);
    let mut o = Options::default();
    o.combat.target_hp.insert(SHIELDED, 10);
    assert!(
        attack_budget::for_window(planner::monster(SHIELDED).unwrap(), &r, &o, &s)
            .unwrap_err()
            .contains("no complete encounter workload")
    );
}
#[test]
fn independent_and_chained_hits_apply_reduction_before_summing() {
    let s = gear(4747, None);
    let r = calc(DEFENCELESS, &s, "Crush", "");
    assert_eq!(r.max_hit, 17);
    close(mean(&r), 50.0 / 12.0 + 59.0 / 13.0);
    let s = gear(28997, None);
    let r = calc(DEFENCELESS, &s, "Crush", "");
    assert_eq!(r.max_hit, 18);
    close(mean(&r), 2.0 * 59.0 / 13.0);
    close(r.accuracy, 1.0);
    let mut blood = s;
    for (slot, id) in [(0, 29073), (4, 29067), (7, 29070)] {
        blood[slot] = Some(id);
    }
    let r = calc(DEFENCELESS, &blood, "Crush", "");
    close(r.expected_attack_ticks, 4.0 - 0.33 - 0.33 * 0.67);
}
#[test]
fn comparisons_share_one_pool_and_exclude_unreviewed_fights() {
    assert_eq!(
        encounters::window(SHIELDED).unwrap().progress_group,
        encounters::window(DEFENCELESS).unwrap().progress_group
    );
    for id in [SHIELDED, 100190, 100191, DEFENCELESS, 100200, 100201] {
        assert_eq!(
            combat::target_hp_bounds(planner::monster(id).unwrap(), 0).unwrap(),
            (1, 600)
        );
        assert!(encounters::window(id).unwrap().comparison_probe);
        assert!(
            defence::estimate(
                &member(),
                &gear(1434, None),
                planner::monster(id).unwrap(),
                "Magic",
                &Settings::default()
            )
            .is_err()
        );
    }
    for id in [100193, 13600, 13601, 13602] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
    }
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: SHIELDED,
                style: "Slash".into(),
            },
            Target {
                id: DEFENCELESS,
                style: "Slash".into(),
            },
        ],
        ..Default::default()
    };
    o.combat.target_hp.insert(DEFENCELESS, 300);
    let plan = planner::plan(&account::demo(), &o);
    let setup = ironbound::share::Setup::from_plan(&plan, &o);
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
        .unwrap()
        .options();
    assert_eq!(restored.targets, o.targets);
    assert_eq!(restored.combat.target_hp[&DEFENCELESS], 300);
    let note = coverage::export_note(&restored);
    assert!(
        note.contains("defenceless") && note.contains("300 HP"),
        "{note}"
    );
    let mut eclipse = gear(29000, Some(28991));
    for (slot, id) in [(0, 29055), (4, 29049), (7, 29052)] {
        eclipse[slot] = Some(id);
    }
    assert!(
        combat::calculate(
            &member(),
            &eclipse,
            "Ranged",
            planner::monster(SHIELDED).unwrap(),
            ""
        )
        .unwrap_err()
        .contains("burn")
    );
}
#[test]
fn dharok_player_health_scaling_precedes_the_npc_shield() {
    let mut s = gear(4718, None);
    for (slot, id) in [(0, 4716), (4, 4720), (7, 4722)] {
        s[slot] = Some(id);
    }
    let settings = Settings {
        current_hitpoints: Some(1),
        ..Default::default()
    };
    let r = combat::calculate_configured(
        &member(),
        &s,
        "Slash",
        planner::monster(DEFENCELESS).unwrap(),
        "",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!(r.max_hit, 44);
    close(r.accuracy, 1.0);
    let expected = (0..=28)
        .map(|d| {
            let scaled = (d * 19702 / 10000).max(1);
            f64::from((scaled * 4 / 5).max(1))
        })
        .sum::<f64>()
        / 29.0;
    close(mean(&r), expected);
    close(r.dps, expected / 4.2);
}
#[test]
fn scythe_reduces_three_separate_hits_in_the_guaranteed_accuracy_state() {
    let r = calc(DEFENCELESS, &gear(22325, None), "Crush", "");
    let expected = [24, 12, 6]
        .iter()
        .map(|max| {
            (0..=*max)
                .map(|d| f64::from((d.max(1) * 4 / 5).max(1)))
                .sum::<f64>()
                / f64::from(max + 1)
        })
        .sum();
    assert_eq!(r.max_hit, 32);
    close(mean(&r), expected);
    close(r.accuracy, 1.0);
}
#[test]
fn bestiary_screen_requires_two_owned_styles_but_does_not_check_quest_access() {
    let mut snapshot = account::demo();
    snapshot.members[0].level_overrides = member().level_overrides;
    snapshot.members[0]
        .bank
        .get_or_insert_with(Vec::new)
        .extend([4151, 1]);
    let keep = gear(4151, None);
    let excluded = snapshot
        .available(false)
        .keys()
        .copied()
        .filter(|id| !keep.contains(&Some(*id)))
        .collect();
    let o = Options {
        excluded,
        ..Default::default()
    };
    let index = encounters::catalog()
        .iter()
        .position(|e| e.name == "Tormented Demons")
        .unwrap();
    let result = &ironbound::bestiary::assess_catalog(&snapshot, &o)[index];
    assert_eq!(result.readiness, ironbound::bestiary::Readiness::Struggle);
    assert!(
        result.reason.contains("Two supported owned attack styles"),
        "{}",
        result.reason
    );
    // A melee/ranged hybrid is valid; Magic is not a mandatory second style.
    snapshot.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([861, 1, 892, 100]);
    let excluded = snapshot
        .available(false)
        .keys()
        .copied()
        .filter(|id| ![4151, 861, 892].contains(id))
        .collect();
    let two = Options {
        excluded,
        ..Default::default()
    };
    let pair = &ironbound::bestiary::assess_catalog(&snapshot, &two)[index];
    assert!(
        pair.reason.contains("· Slash ·")
            && pair.reason.contains("· Ranged ·")
            && !pair.reason.contains("· Magic ·"),
        "{}",
        pair.reason
    );

    let result = &ironbound::bestiary::assess_catalog(&snapshot, &Options::default())[index];
    assert!(
        result
            .detail
            .contains("While Guthix Sleeps access is not checked")
    );
    assert_eq!(
        result.requirements.status,
        ironbound::bestiary::RequirementStatus::NoLevelGate
    );
}
