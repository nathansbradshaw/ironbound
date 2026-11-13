//! Primary references: pinned Wiki WardensP2.test.ts + ordered calculator
//! stages, documented in docs/combat-validation.md. No expected value is
//! generated from Ironbound itself.
use ironbound::{
    account::{self, Member},
    attack_budget, combat,
    defence::Settings,
    effects, encounters,
    planner::{self, Options},
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

#[test]
fn eighteen_pinned_warden_shield_observations_keep_their_maxima_and_minimum_bounds() {
    // MAR, base max, lowest observed, known maximum (-1 = not observed).
    for (roll, base, low, max) in [
        (57798, 50, 21, 30),
        (58539, 51, 21, -1),
        (8132, 19, 3, 6),
        (23562, 24, 6, 11),
        (39643, 34, 12, -1),
        (39504, 34, 11, -1),
        (19153, 28, 6, 12),
        (21576, 31, 7, 13),
        (24012, 32, 9, 14),
        (24360, 32, 8, 14),
        (22919, 29, 7, 13),
        (30270, 32, 9, 16),
        (29546, 30, 8, 14),
        (35004, 45, 14, 23),
        (36166, 46, 15, 24),
        (38237, 39, 13, 21),
        (37185, 41, 13, 22),
        (58333, 51, 20, 30),
    ] {
        let (a, b) = effects::wardens_p2_bounds(roll, 14606, base);
        assert!(a <= low, "{roll}, {base}");
        if max >= 0 {
            assert_eq!(b, max, "{roll}, {base}");
        }
    }
    assert_eq!(effects::wardens_p2_bounds(0, 0, 100), (15, 35));
    assert_eq!(effects::wardens_p2_bounds(i64::MAX, 0, 100), (40, 60));
}

#[test]
fn actual_warden_magic_window_has_guaranteed_accuracy_and_independent_damage_band() {
    // Trident: (99+11)*(15+64)=8690. Magic defence: (190+9)*34=6766.
    // delta=6435; modifier=18%; base28 becomes [5,10], mean7.5/2.4s.
    let r = combat::calculate(
        &member(),
        &set(11905, None),
        "Magic",
        planner::monster(11753).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!((r.attack_roll, r.defence_roll, r.max_hit), (8690, 6766, 10));
    assert_eq!(r.accuracy, 1.0);
    assert!((r.dps - 3.125).abs() < 1e-10);
    assert!(r.hit_distribution.iter().all(|(d, _)| (5..=10).contains(d)));
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Melee",
            planner::monster(11753).unwrap(),
            ""
        )
        .is_err()
    );
}

#[test]
fn twisted_bow_uses_double_p2_accuracy_but_only_one_damage_multiplier() {
    // 14338 base roll; target max(magic190,bonus225) gives136% accuracy
    // twice:19499 then26518. Base24*203%=48, then P2's27–47%=[12,22].
    let r = combat::calculate(
        &member(),
        &set(20997, Some(11212)),
        "Ranged",
        planner::monster(11753).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(r.attack_roll, 26518);
    assert_eq!(r.max_hit, 22);
    assert_eq!(r.accuracy, 1.0);
    assert!((r.dps - 17.0 / 3.0).abs() < 1e-10);
}

#[test]
fn core_and_vulnerable_p3_are_explicit_while_invulnerability_and_incoming_stay_blocked() {
    let core = planner::monster(11755).unwrap();
    let r = combat::calculate(&member(), &set(4151, None), "Melee", core, "").unwrap();
    assert_eq!(r.accuracy, 1.0);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d == r.max_hit));
    assert_eq!(combat::target_hitpoints(core, 0), Some(4500));
    assert_eq!(combat::target_hitpoints(core, 150), Some(5180));
    assert!(combat::calculate(&member(), &set(11905, None), "Magic", core, "").is_err());
    assert!(
        combat::calculate(
            &member(),
            &set(11905, None),
            "Magic",
            planner::monster(11761).unwrap(),
            ""
        )
        .is_ok()
    );
    assert!(encounters::limitation(planner::monster(11763).unwrap()).is_some());
    assert!(
        ironbound::defence::estimate(
            &member(),
            &set(4151, None),
            core,
            "Melee",
            &Settings::default()
        )
        .is_err()
    );
}

#[test]
fn target_hp_changes_ruby_damage_and_round_trips_as_a_public_assumption() {
    let target = planner::monster(8059).unwrap();
    let gear = set(9185, Some(9242));
    let full = combat::calculate(&member(), &gear, "Ranged", target, "").unwrap();
    let mut settings = Settings::default();
    settings.target_hp.insert(8059, 250);
    let low =
        combat::calculate_configured(&member(), &gear, "Ranged", target, "", 0, &settings).unwrap();
    assert_eq!((full.max_hit, low.max_hit), (100, 50));
    assert!((full.dps - low.dps - 1.0).abs() < 1e-10); // (100-50)*6%/3 seconds.
    settings.target_hp.insert(8059, 751);
    assert!(
        combat::calculate_configured(&member(), &gear, "Ranged", target, "", 0, &settings).is_err()
    );
    settings.target_hp.insert(8059, 250);
    let mut options = Options::default();
    options.combat = settings;
    let plan = planner::plan(&account::demo(), &options);
    let shared = ironbound::share::Setup::parse(
        &ironbound::share::Setup::from_plan(&plan, &options)
            .fragment()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(shared.options().combat.target_hp[&8059], 250);
}

#[test]
fn finite_hp_solves_miss_loops_and_finishing_overkill_without_simulation() {
    assert_eq!(
        attack_budget::expected_attacks(&[(2, 1.0)], 3, 0, None).unwrap(),
        2.0
    );
    assert_eq!(
        attack_budget::expected_attacks(&[(0, 0.5), (2, 0.5)], 3, 0, None).unwrap(),
        4.0
    );
    assert_eq!(
        attack_budget::expected_attacks(&[(1, 1.0)], 10, 7, None).unwrap(),
        3.0
    );
    assert!(attack_budget::expected_attacks(&[(0, 1.0)], 10, 0, None).is_err());
    assert!(attack_budget::expected_attacks(&[(1, 0.2)], 10, 0, None).is_err());
    let target = planner::monster(8059).unwrap();
    let r = combat::calculate(&member(), &set(9185, Some(9242)), "Ranged", target, "").unwrap();
    let dynamic =
        attack_budget::expected_attacks(&r.hit_distribution, 750, 0, r.hp_dependent).unwrap();
    let frozen = attack_budget::expected_attacks(&r.hit_distribution, 750, 0, None).unwrap();
    assert!(
        dynamic > frozen,
        "Full-health ruby DPS must not remain constant down to zero HP"
    );
}

#[test]
fn acid_reduction_rounds_each_outcome_after_bolt_effects() {
    let gear = set(9185, Some(9242));
    let normal = combat::calculate(
        &member(),
        &gear,
        "Ranged",
        planner::monster(8059).unwrap(),
        "",
    )
    .unwrap();
    let acid = combat::calculate(
        &member(),
        &gear,
        "Ranged",
        planner::monster(100020).unwrap(),
        "",
    )
    .unwrap();
    assert_eq!(normal.hit_distribution.len(), acid.hit_distribution.len());
    for ((d, p), (a, q)) in normal.hit_distribution.iter().zip(&acid.hit_distribution) {
        assert_eq!(*a, d / 2);
        assert_eq!(p, q);
    }
    assert!(
        acid.dps < normal.dps / 2.0,
        "Odd hits must be rounded down individually"
    );
    assert_eq!(acid.hp_dependent.unwrap().divisor, 2);
}

#[test]
fn yama_final_flare_and_boss_progress_use_distinct_hp_budgets() {
    let gear = set(11905, None);
    let options = Options::default();
    let final_flare = planner::monster(100030).unwrap();
    let r = combat::calculate(&member(), &gear, "Magic", final_flare, "").unwrap();
    let b = attack_budget::for_window(final_flare, &r, &options, &gear).unwrap();
    assert_eq!(
        (b.start_hp, b.stop_hp, b.entities, b.attacks),
        (71, 0, 1, 3.0)
    );
    assert!((b.seconds - 7.2).abs() < 1e-10);
    let pair = planner::monster(100014).unwrap();
    let r = combat::calculate(&member(), &gear, "Magic", pair, "").unwrap();
    assert_eq!(
        attack_budget::for_window(pair, &r, &options, &gear)
            .unwrap()
            .attacks,
        10.0
    );
    let boss = planner::monster(14176).unwrap();
    let r = combat::calculate(&member(), &gear, "Magic", boss, "").unwrap();
    let b = attack_budget::for_window(boss, &r, &options, &gear).unwrap();
    assert_eq!((b.start_hp, b.stop_hp), (2500, 833));
    let final_phase = planner::monster(100011).unwrap();
    let r = combat::calculate(&member(), &gear, "Magic", final_phase, "").unwrap();
    let b = attack_budget::for_window(final_phase, &r, &options, &gear).unwrap();
    assert_eq!((b.start_hp, b.stop_hp), (833, 0));
}
