use ironbound::ranking::{self, Metrics, Preferences, Weights};

fn weights(
    dps: impl Into<f64>,
    defence: impl Into<f64>,
    prayer: impl Into<f64>,
    weight: impl Into<f64>,
) -> Weights {
    Weights {
        dps: dps.into(),
        defence: defence.into(),
        prayer: prayer.into(),
        weight: weight.into(),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn pool() -> Vec<Metrics> {
    vec![
        Metrics {
            dps: Some(10.0),
            defence: Some(0.0),
            prayer: Some(0.0),
            weight: Some(12.0),
        },
        Metrics {
            dps: Some(5.0),
            defence: Some(100.0),
            prayer: Some(6.0),
            weight: Some(8.0),
        },
        Metrics {
            dps: Some(7.5),
            defence: Some(50.0),
            prayer: Some(3.0),
            weight: Some(10.0),
        },
    ]
}
#[test]
fn combined_weights_trade_damage_for_defence_without_mixing_units() {
    let damage = ranking::rank(&pool(), &weights(80, 20, 0, 0)).scores;
    close(damage[0].unwrap(), 80.0);
    close(damage[1].unwrap(), 20.0);
    close(damage[2].unwrap(), 50.0);
    let defence = ranking::rank(&pool(), &weights(20, 80, 0, 0)).scores;
    assert!(defence[1] > defence[2] && defence[2] > defence[0]);
    let mut rescaled = pool();
    for row in &mut rescaled {
        row.defence = row.defence.map(|n| n * 1000.0);
    }
    assert_eq!(
        ranking::rank(&rescaled, &weights(80, 20, 0, 0)).scores,
        damage
    );
}
#[test]
fn zero_importance_ignores_prayer_and_mass() {
    let mut changed = pool();
    changed[0].prayer = Some(1000.0);
    changed[0].weight = Some(-1000.0);
    assert_eq!(
        ranking::rank(&pool(), &weights(80, 20, 0, 0)).scores,
        ranking::rank(&changed, &weights(80, 20, 0, 0)).scores
    );
    assert_eq!(
        ranking::rank(&pool(), &weights(0, 0, 0, 0)).scores,
        vec![None; 3]
    );
}
#[test]
fn lighter_equipment_and_negative_weight_reduction_score_higher() {
    let mut rows = pool();
    rows[2].weight = Some(-4.0);
    let scores = ranking::rank(&rows, &weights(0, 0, 0, 100)).scores;
    assert!(scores[2] > scores[1] && scores[1] > scores[0]);
    close(scores[2].unwrap(), 100.0);
    let prayer = ranking::rank(&rows, &weights(0, 0, 100, 0)).scores;
    assert!(prayer[1] > prayer[2] && prayer[2] > prayer[0]);
}
#[test]
fn unknown_weight_never_wins_by_looking_weightless() {
    let mut rows = pool();
    rows[0].weight = None;
    let scores = ranking::rank(&rows, &weights(50, 0, 0, 50)).scores;
    assert_eq!(scores[0], None);
    assert!(scores[1].is_some() && scores[2].is_some());
    assert!(ranking::rank(&rows, &weights(100, 0, 0, 0)).scores[0].is_some());
}
#[test]
fn globally_unavailable_dps_is_omitted_without_zero_damage_claims() {
    let mut rows = pool();
    for row in &mut rows {
        row.dps = None;
    }
    let ranked = ranking::rank(&rows, &weights(80, 20, 0, 0));
    assert_eq!(ranked.unavailable, vec!["DPS"]);
    close(ranked.scores[1].unwrap(), 100.0);
    assert!(
        ranking::rank(&rows, &weights(100, 0, 0, 0))
            .scores
            .iter()
            .all(Option::is_none)
    );
}
#[test]
fn equal_metrics_and_nonfinite_values_are_handled_explicitly() {
    let equal = [Metrics {
        dps: Some(2.0),
        ..Default::default()
    }; 2];
    assert_eq!(
        ranking::rank(&equal, &weights(100, 0, 0, 0)).scores,
        [Some(100.0); 2]
    );
    let mut rows = pool();
    rows[0].dps = Some(f64::NAN);
    rows[1].dps = Some(f64::INFINITY);
    assert_eq!(
        ranking::rank(&rows, &weights(100, 0, 0, 0)).scores,
        [None, None, Some(100.0)]
    );
}
#[test]
fn preferences_roundtrip_and_old_backups_receive_combat_defaults() {
    let old: Preferences = serde_json::from_str("{}").unwrap();
    assert_eq!(old.weights, weights(99, 1, 0.01, 0));
    let prefs = Preferences {
        weights: weights(30, 40, 20, 10),
        defence: ranking::Defence::Magic,
    };
    assert_eq!(
        serde_json::from_str::<Preferences>(&serde_json::to_string(&prefs).unwrap()).unwrap(),
        prefs
    );
    assert_eq!(weights(999, 999, 999, 999).values(), [100.0; 4]);
}
#[test]
fn exact_item_weights_preserve_unknowns_and_setup_mass() {
    close(ranking::item_weight(4087).unwrap(), 9.071);
    close(
        ranking::equipment_weight(&[Some(4087), Some(4151)]).unwrap(),
        9.524,
    );
    assert_eq!(ranking::item_weight(999999), None);
    assert_eq!(ranking::equipment_weight(&[Some(4087), Some(999999)]), None);
    assert_eq!(ranking::equipment_weight(&[]), Some(0.0));
}

fn combat_bank() -> ironbound::account::Snapshot {
    let mut bank = ironbound::account::demo();
    bank.members[0].bank = Some(
        [4151, 10551, 9674, 544, 4087, 9676, 542, 1127]
            .into_iter()
            .flat_map(|id| [id, 1])
            .collect(),
    );
    for skill in ironbound::account::SKILLS {
        bank.members[0].level_overrides.insert(skill.into(), 99);
    }
    bank
}
fn combat_options() -> ironbound::planner::Options {
    ironbound::planner::Options {
        boss: ironbound::planner::Boss::Custom,
        targets: vec![ironbound::planner::Target {
            id: 1673,
            style: "Slash".into(),
        }],
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    }
}
#[test]
fn stale_long_trip_settings_cannot_force_prayer_gear_under_new_defaults() {
    let bank = combat_bank();
    let mut options = combat_options();
    options.combat.prayer_weight = 3;
    options.combat.safety = 3;
    options.combat.protection = "Melee".into();
    let plan = ironbound::planner::plan(&bank, &options);
    assert_eq!(
        plan.equipment[4],
        Some(10551),
        "DPS-first must keep the strength torso"
    );
    assert_eq!(plan.equipment[7], Some(4087));
}
#[test]
fn automatic_gear_honours_explicit_prayer_and_mass_priorities() {
    let bank = combat_bank();
    let mut options = combat_options();
    options.ranking.weights = weights(0, 0, 100, 0);
    let prayer = ironbound::planner::plan(&bank, &options);
    assert_eq!(prayer.equipment[4], Some(9674));
    options.ranking.weights = weights(0, 0, 0, 100);
    let light = ironbound::planner::plan(&bank, &options);
    assert_eq!(light.equipment[4], Some(544));
    assert!(
        ranking::equipment_weight(&light.equipment) < ranking::equipment_weight(&prayer.equipment)
    );
}
#[test]
fn preset_and_fractional_priorities_roundtrip_in_shared_setups() {
    let bank = combat_bank();
    let mut options = combat_options();
    for name in ["damage", "prayer", "defence", "equal"] {
        options.ranking.weights = Weights::preset(name).unwrap();
        let plan = ironbound::planner::plan(&bank, &options);
        let shared = ironbound::share::Setup::from_plan(&plan, &options);
        let parsed: ironbound::share::Setup =
            serde_json::from_str(&serde_json::to_string(&shared).unwrap()).unwrap();
        assert_eq!(parsed.options().ranking, options.ranking);
    }
    let equal = Weights::preset("equal").unwrap();
    assert_eq!(equal.dps, equal.defence);
    assert_eq!(equal.dps, equal.prayer);
}
#[test]
fn constrained_search_uses_the_same_priorities_and_preserves_hard_limits() {
    let bank = ironbound::account::demo();
    let mut options = ironbound::planner::Options {
        boss: ironbound::planner::Boss::Toa,
        max_switches: Some(4),
        switch_slots: 8,
        food: 4,
        ..Default::default()
    };
    options.ranking.weights = Weights::preset("prayer").unwrap();
    let plan = ironbound::planner::plan(&bank, &options);
    let report = plan.optimization.unwrap();
    assert!(report.max_changes <= 4);
    assert!(report.gear_slots <= 8);
    let reference = pool()[0];
    let mut lower_dps = reference;
    lower_dps.dps = Some(8.0);
    lower_dps.defence = Some(500.0);
    lower_dps.prayer = Some(20.0);
    assert!(
        ranking::relative_utility(reference, reference, &Weights::default())
            > ranking::relative_utility(lower_dps, reference, &Weights::default())
    );
    assert!(
        ranking::relative_utility(reference, reference, &weights(0, 0, 100, 0))
            < ranking::relative_utility(lower_dps, reference, &weights(0, 0, 100, 0))
    );
}
