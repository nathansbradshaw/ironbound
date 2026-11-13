//! Independent enumeration and arithmetic from docs/obligator-ascension.md.
use ironbound::{
    account::{self, Member, Snapshot},
    combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    rules,
};
use std::collections::BTreeMap;

fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn set(items: &[(usize, i32)]) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for &(slot, id) in items {
        s[slot] = Some(id);
    }
    s
}
fn target(id: i32) -> planner::Monster {
    let mut t = planner::monster(26).unwrap().clone();
    t.id = id;
    t.attributes.clear();
    t.weakness = None;
    t.defensive.clear();
    for style in [
        "stab", "slash", "crush", "heavy", "standard", "light", "magic",
    ] {
        t.defensive.insert(style.into(), 0);
    }
    t.skills.insert("def".into(), 99);
    t
}
fn result(
    s: &[Option<i32>],
    style: &str,
    t: &planner::Monster,
    settings: &Settings,
) -> combat::ResultStats {
    combat::calculate_configured(&member(), s, style, t, "", 0, settings).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(max: i32, f: impl Fn(i32) -> i32) -> f64 {
    (0..=max).map(|d| f(d) as f64).sum::<f64>() / (max + 1) as f64
}
fn valid(r: &combat::ResultStats) {
    close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    assert!(r.hit_distribution.iter().all(|(d, p)| *d >= 0 && *p >= 0.0));
    close(
        r.dps,
        r.hit_distribution
            .iter()
            .map(|(d, p)| *d as f64 * p)
            .sum::<f64>()
            / (r.expected_attack_ticks * 0.6),
    );
}
fn probability(r: &combat::ResultStats, damage: i32) -> f64 {
    r.hit_distribution
        .iter()
        .filter(|(d, _)| *d == damage)
        .map(|(_, p)| p)
        .sum()
}
fn brute(max: i32, rolls: u32, transform: impl Fn(i32) -> i32) -> BTreeMap<i32, f64> {
    fn enumerate(max: i32, left: u32, best: i32, counts: &mut BTreeMap<i32, u64>) {
        if left == 0 {
            *counts.entry(best).or_default() += 1;
        } else {
            for d in 0..=max {
                enumerate(max, left - 1, best.max(d), counts);
            }
        }
    }
    let mut counts = BTreeMap::new();
    enumerate(max, rolls, 0, &mut counts);
    let denominator = (max + 1).pow(rolls) as f64;
    let mut outcomes = BTreeMap::new();
    for (damage, count) in counts {
        *outcomes.entry(transform(damage)).or_default() += count as f64 / denominator;
    }
    outcomes
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
fn bank(items: &[(i32, i64)]) -> Snapshot {
    Snapshot {
        members: vec![Member {
            name: "Equipment QA".into(),
            bank: Some(items.iter().flat_map(|&(id, q)| [id as i64, q]).collect()),
            ..member()
        }],
        selected: "Equipment QA".into(),
        source: "Synthetic".into(),
        fetched_at: String::new(),
    }
}

#[test]
fn obligator_damage_rerolls_match_exhaustive_enumeration_and_keep_one_accuracy_roll() {
    for (stab, slash, rolls) in [(0, 0, 2), (1, 0, 3), (0, 1, 3), (1, 1, 4), (-1, -1, 2)] {
        let mut t = target(26);
        t.defensive.insert("stab".into(), stab);
        t.defensive.insert("slash".into(), slash);
        let r = result(&set(&[(3, 34715)]), "Crush", &t, &Settings::default());
        assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (20240, 17, 5));
        let accuracy = combat::accuracy(20240, 108 * 64);
        close(r.accuracy, accuracy);
        let mut expected = brute(17, rolls, |d| d.max(1));
        *expected.entry(0).or_default() += (1.0 - accuracy) / accuracy;
        for (d, p) in expected {
            close(probability(&r, d), p * accuracy);
        }
        valid(&r);
    }
}

#[test]
fn obligator_size_bonus_changes_the_roll_range_before_rolls_and_armour() {
    for (id, max) in [(26, 17), (2, 20), (8059, 23)] {
        let mut t = target(id);
        t.defensive.insert("flat_armour".into(), 4);
        let r = result(&set(&[(3, 34715)]), "Crush", &t, &Settings::default());
        assert_eq!(r.max_hit, max - 4);
        let expected = brute(max, 2, |d| (d.max(1) - 4).max(0));
        close(
            r.dps,
            r.accuracy * expected.iter().map(|(d, p)| *d as f64 * p).sum::<f64>() / 3.0,
        );
        valid(&r);
    }
}

#[test]
fn obligator_smash_and_uncharged_thrust_have_no_size_or_reroll_passive() {
    for id in [34715, 34717] {
        let smash = result(
            &set(&[(3, id)]),
            "Crush",
            &target(8059),
            &Settings {
                obligator_style: "Smash".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            (smash.attack_ticks, smash.max_hit, smash.attack_roll),
            (3, 18, 19688)
        );
        close(smash.dps, smash.accuracy * mean(18, |d| d.max(1)) / 1.8);
        valid(&smash);
    }
    let empty = result(
        &set(&[(3, 34717)]),
        "Crush",
        &target(8059),
        &Settings::default(),
    );
    assert_eq!((empty.attack_ticks, empty.max_hit), (5, 17));
    close(empty.dps, empty.accuracy * mean(17, |d| d.max(1)) / 3.0);
    let aggressive = result(
        &set(&[(3, 34715)]),
        "Crush",
        &target(26),
        &Settings {
            obligator_style: "Thrust aggressive".into(),
            ..Default::default()
        },
    );
    assert_eq!(
        (
            aggressive.max_hit,
            aggressive.attack_roll,
            aggressive.attack_ticks
        ),
        (18, 19688, 5)
    );
    valid(&aggressive);
}

#[test]
fn obligator_uncertain_inquisitor_and_missing_size_are_explained_not_scored() {
    let inquisitor = set(&[(3, 34715), (0, 24419)]);
    assert!(
        combat::calculate_configured(
            &member(),
            &inquisitor,
            "Crush",
            &target(26),
            "",
            0,
            &Settings::default()
        )
        .unwrap_err()
        .contains("Inquisitor")
    );
    result(
        &inquisitor,
        "Crush",
        &target(26),
        &Settings {
            obligator_style: "Smash".into(),
            ..Default::default()
        },
    );
    assert!(
        combat::calculate_configured(
            &member(),
            &set(&[(3, 34715)]),
            "Crush",
            &target(999999),
            "",
            0,
            &Settings::default()
        )
        .unwrap_err()
        .contains("size metadata")
    );
    assert!(
        combat::calculate_configured(
            &member(),
            &set(&[(3, 34715)]),
            "Crush",
            planner::monster(11753).unwrap(),
            "",
            0,
            &Settings::default()
        )
        .unwrap_err()
        .contains("blocked or not modelled")
    );
}

#[test]
fn obligator_npc_damage_reductions_follow_the_best_roll() {
    let r = result(
        &set(&[(3, 34715)]),
        "Crush",
        &target(319),
        &Settings::default(),
    );
    assert_eq!(r.max_hit, 11);
    let expected = brute(23, 2, |d| d.max(1) / 2);
    close(
        r.dps,
        r.accuracy * expected.iter().map(|(d, p)| *d as f64 * p).sum::<f64>() / 3.0,
    );
    valid(&r);
    let tail = result(
        &set(&[(3, 34715)]),
        "Crush",
        &target(14014),
        &Settings::default(),
    );
    assert_eq!(tail.max_hit, 9);
    assert!(tail.hit_distribution.iter().all(|(d, _)| *d >= 1));
    valid(&tail);
}

#[test]
fn ascension_accepts_only_its_five_bolt_variants_and_fires_one_shot_per_two_ticks() {
    for (id, max) in [(34684, 20), (34690, 20), (34702, 20)] {
        let r = result(
            &set(&[(3, 34681), (13, id)]),
            "Ranged",
            &target(26),
            &Settings::default(),
        );
        assert_eq!(
            (r.attack_ticks, r.attack_roll, r.max_hit),
            (2, 107 * 98, max)
        );
        close(r.dps, r.accuracy * mean(max, |d| d.max(1)) / 1.2);
        valid(&r);
    }
    for (weapon, ammo) in [
        (34681, 9144),
        (34681, 21944),
        (34681, 33595),
        (9185, 34696),
        (9185, 34684),
        (21902, 34708),
    ] {
        assert!(
            combat::calculate_configured(
                &member(),
                &set(&[(3, weapon), (13, ammo)]),
                "Ranged",
                &target(26),
                "",
                0,
                &Settings::default()
            )
            .is_err()
        );
    }
    let accurate = result(
        &set(&[(3, 34681), (13, 34684)]),
        "Ranged",
        &target(26),
        &Settings {
            ranged_stance: "Accurate".into(),
            ..Default::default()
        },
    );
    assert_eq!((accurate.attack_ticks, accurate.attack_roll), (3, 110 * 98));
    valid(&accurate);
}

#[test]
fn ascension_diamond_proc_halves_chance_bypasses_misses_and_applies_diary() {
    for (diary, chance) in [(false, 0.05), (true, 0.055)] {
        let r = result(
            &set(&[(3, 34681), (13, 34696)]),
            "Ranged",
            &target(26),
            &Settings {
                kandarin_diary: diary,
                ..Default::default()
            },
        );
        assert_eq!(r.max_hit, 23); // floor(20 * 1.15)
        let hit = combat::accuracy(107 * 98, 108 * 64);
        close(
            r.dps,
            ((1.0 - chance) * hit * mean(20, |d| d.max(1)) + chance * mean(23, |d| d.max(1))) / 1.2,
        );
        close(probability(&r, 0), (1.0 - hit) * (1.0 - chance));
        valid(&r);
    }
    close(rules::bolt_proc_chance(9243, false), 0.1); // ordinary crossbows unchanged
}

#[test]
fn ascension_onyx_requires_accuracy_and_undead_are_immune() {
    for (diary, chance) in [(false, 0.055), (true, 0.0605)] {
        let r = result(
            &set(&[(3, 34681), (13, 34708)]),
            "Ranged",
            &target(26),
            &Settings {
                kandarin_diary: diary,
                ..Default::default()
            },
        );
        assert_eq!(r.max_hit, 24);
        close(
            r.dps,
            r.accuracy
                * ((1.0 - chance) * mean(20, |d| d.max(1)) + chance * mean(24, |d| d.max(1)))
                / 1.2,
        );
        valid(&r);
        let mut undead = target(26);
        undead.attributes.push("undead".into());
        let immune = result(
            &set(&[(3, 34681), (13, 34708)]),
            "Ranged",
            &undead,
            &Settings {
                kandarin_diary: diary,
                ..Default::default()
            },
        );
        assert_eq!(immune.max_hit, 20);
        close(immune.dps, immune.accuracy * mean(20, |d| d.max(1)) / 1.2);
    }
}

#[test]
fn ascension_charged_quiver_adds_real_bonuses_and_npc_limiters_still_apply() {
    let r = result(
        &set(&[(3, 34681), (13, 34696), (1, 28951)]),
        "Ranged",
        &target(26),
        &Settings::default(),
    );
    assert_eq!(r.attack_roll, 107 * (98 + 18 + 10));
    valid(&r);
    let plain = result(
        &set(&[(3, 34681), (13, 34696), (1, 28947)]),
        "Ranged",
        &target(26),
        &Settings::default(),
    );
    assert_eq!(r.attack_roll - plain.attack_roll, 1070);
    let corp = result(
        &set(&[(3, 34681), (13, 34696)]),
        "Ranged",
        &target(319),
        &Settings::default(),
    );
    assert_eq!(corp.max_hit, 11);
    valid(&corp);
    let tail = result(
        &set(&[(3, 34681), (13, 34696)]),
        "Ranged",
        &target(14014),
        &Settings::default(),
    );
    assert_eq!(tail.max_hit, 4);
    valid(&tail);
}

#[test]
fn seeking_arrows_clamp_accurate_rolls_without_rerolls_or_turning_misses_into_hits() {
    let r = result(
        &set(&[(3, 29591), (13, 33595)]),
        "Ranged",
        &target(26),
        &Settings::default(),
    );
    assert_eq!(r.max_hit, 27); // floor((107 * (64 + 40 + 60) + 320) / 640)
    assert_eq!(r.attack_roll, 107 * (64 + 124 + 20));
    close(probability(&r, 0), 1.0 - r.accuracy);
    close(probability(&r, 3), r.accuracy * 4.0 / 28.0);
    close(probability(&r, 4), r.accuracy / 28.0);
    close(r.dps, r.accuracy * mean(27, |d| d.max(3)) / 2.4);
    valid(&r);
    let corp = result(
        &set(&[(3, 29591), (13, 33595)]),
        "Ranged",
        &target(319),
        &Settings::default(),
    );
    assert!(probability(&corp, 1) > 0.0); // minimum applies before halving, not after
    valid(&corp);
    assert!(!rules::ammo_ids(861).contains(&33595));
    for id in [33553, 33559, 33565, 33571, 33577, 33583, 33589, 33601] {
        assert!(rules::ammo_ids(861).contains(&id));
    }
}

#[test]
fn new_weapon_wear_requirements_and_settings_validate_real_boundaries() {
    for (id, skill, level) in [
        (34715, "Attack", 85),
        (34717, "Attack", 85),
        (34681, "Ranged", 80),
    ] {
        let mut m = member();
        m.level_overrides.insert(skill.into(), level - 1);
        assert!(!planner::can_use(id, &m));
        m.level_overrides.insert(skill.into(), level);
        assert!(planner::can_use(id, &m));
    }
    assert!(
        !Settings {
            obligator_style: "stabbity".into(),
            ..Default::default()
        }
        .valid()
    );
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(old.obligator_style, "Thrust accurate");
    let mut m = member();
    m.level_overrides.insert("Slayer".into(), 54);
    assert!(!planner::can_use(33601, &m));
    m.level_overrides.insert("Slayer".into(), 55);
    assert!(planner::can_use(33601, &m));
}

#[test]
fn new_damage_probabilities_survive_exact_json_round_trips() {
    for s in [
        set(&[(3, 34715)]),
        set(&[(3, 34681), (13, 34696)]),
        set(&[(3, 34681), (13, 34708)]),
    ] {
        let style = if s[3] == Some(34715) {
            "Crush"
        } else {
            "Ranged"
        };
        let r = result(&s, style, &target(26), &Settings::default());
        let parsed: combat::ResultStats =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(r.hit_distribution, parsed.hit_distribution);
        assert_eq!(r.dps, parsed.dps);
    }
}

#[test]
fn seeking_arrows_can_raise_a_small_successful_hit_above_the_base_max() {
    let mut m = member();
    m.level_overrides.insert("Ranged".into(), 1);
    let r = combat::calculate_configured(
        &m,
        &set(&[(3, 861), (13, 33553)]),
        "Ranged",
        &target(26),
        "",
        0,
        &Settings::default(),
    )
    .unwrap();
    assert_eq!(r.max_hit, 3);
    close(probability(&r, 3), r.accuracy);
    valid(&r);
}

#[test]
fn planner_packs_owned_ascension_shots_and_preserves_settings_in_share_and_export() {
    let snapshot = bank(&[(34681, 1), (34696, 37), (8007, 1)]);
    let mut o = options("Ranged");
    o.combat.ammo_shots = 37;
    let plan = planner::plan(&snapshot, &o);
    assert_eq!(plan.equipment[3], Some(34681));
    assert_eq!(plan.equipment[13], Some(34696));
    assert!(plan.exportable(), "{:?}", plan.missing);
    let exported: serde_json::Value =
        serde_json::from_str(&ironbound::export::inventory_setup(&plan, &o, "QA").unwrap())
            .unwrap();
    assert_eq!(exported["setup"]["eq"][13]["q"], 37);
    let snapshot = bank(&[(34715, 1), (8007, 1)]);
    o = options("Crush");
    o.combat.obligator_style = "Smash".into();
    let plan = planner::plan(&snapshot, &o);
    assert_eq!(plan.equipment[3], Some(34715));
    assert!(plan.exportable());
    let shared = ironbound::share::Setup::from_plan(&plan, &o);
    let restored = ironbound::share::Setup::parse(&shared.fragment().unwrap()).unwrap();
    assert_eq!(restored.options().combat.obligator_style, "Smash");
    assert!(
        ironbound::export::inventory_setup(&plan, &o, "QA")
            .unwrap()
            .contains("Obligator: Smash")
    );
    let r = result(&plan.equipment, "Crush", &target(26), &o.combat);
    let timing = ironbound::attack_budget::for_hp(&r, 100, 0).unwrap();
    close(timing.seconds, timing.attacks * 1.8);
}
