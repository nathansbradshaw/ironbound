//! Synthetic data tests the comparison machinery, never gameplay accuracy.
use ironbound::{
    account,
    calibration::{self, Dataset, Evidence, EvidenceKind, Observation, Outcome, Split},
    combat, planner,
    simulation::{self, Attack, Encounter, Model, Settings},
};
use std::collections::BTreeMap;

fn model(encounter: Encounter) -> Model {
    let mut member = account::Member::default();
    for skill in account::SKILLS {
        member.level_overrides.insert(skill.into(), 99);
    }
    let mut gear = vec![None; 14];
    gear[3] = Some(4151);
    let mut hits =
        combat::calculate(&member, &gear, "Melee", planner::monster(8059).unwrap(), "").unwrap();
    hits.hit_distribution = vec![(500, 1.0)];
    hits.hp_dependent = None;
    hits.max_hit = 500;
    hits.attack_ticks = 1;
    hits.expected_attack_ticks = 1.0;
    let attack = Attack {
        hits,
        weapon: 4151,
        runes: vec![],
        projectile: None,
        projectile_loss: None,
        charge: false,
        charge_on_damage: false,
        scales: false,
        prayer: 0.0,
    };
    let n = if encounter == Encounter::Vorkath {
        2
    } else {
        6
    };
    Model {
        encounter,
        settings: Settings {
            trials: 16,
            trip_limit: 1,
            uptime: 100,
            incoming: 0,
            mistakes: 0,
            transition: 0,
            judge_travel: 0,
            glyph_hp: 0,
            glyph_prayer: 0,
            skull_ticks: 1,
            ..Default::default()
        },
        attacks: vec![attack; n],
        max_hp: 99,
        max_prayer: 99,
        hp: if encounter == Encounter::Yama {
            vec![2500, 2500, 400, 400, 140, 71]
        } else {
            vec![100; n]
        },
        available: BTreeMap::new(),
        foods: vec![],
        potions: vec![],
        notes: vec![],
        mark_ticks: 0,
    }
}
fn observation(id: &str, split: Split, ticks: u32) -> Observation {
    Observation {
        id: id.into(),
        split,
        evidence: Evidence {
            kind: EvidenceKind::ClientLog,
            reference: format!("synthetic-test-only-{id}.log"),
            recorded_on: "2026-11-13".into(),
            start_seconds: 0.0,
            end_seconds: 100.0,
            uncertainty_ticks: 0,
            notes: "Synthetic fixture; not gameplay evidence".into(),
        },
        setup_verified: true,
        exclusions: vec![],
        outcome: Outcome::Completed,
        ticks,
        metrics: BTreeMap::new(),
        timings: BTreeMap::new(),
        exposed_ticks: None,
        residual_damage: None,
        eligible_opportunities: None,
        used_opportunities: None,
    }
}
#[test]
fn all_three_start_uncalibrated_and_round_trip_the_exact_numerical_setup() {
    for e in [Encounter::Vorkath, Encounter::Yama, Encounter::Wardens] {
        let data = calibration::worksheet(&model(e));
        let raw = serde_json::to_string(&data).unwrap();
        let parsed = calibration::parse(&raw).unwrap();
        assert!(calibration::same_inputs(&data.inputs, &parsed.inputs));
        let report = calibration::compare(&parsed).unwrap();
        assert_eq!(report.accepted, 0);
        assert!(report.comparisons.is_empty());
        assert!(report.status.starts_with("Awaiting"));
        assert!(
            report
                .completion
                .iter()
                .all(|c| c.observed_rate.is_none() && c.wilson_95.is_none())
        );
    }
}
#[test]
fn independent_vorkath_one_attack_fixture_has_exact_bias_and_interval_coverage() {
    let mut data = calibration::worksheet(&model(Encounter::Vorkath));
    // 100 HP / a guaranteed 500-damage attack => one tick at tick-one.
    data.observations = vec![
        observation("a", Split::Training, 3),
        observation("b", Split::Holdout, 1),
    ];
    let report = calibration::compare(&data).unwrap();
    assert_eq!(report.comparisons[0].predicted_median, 1.0);
    assert_eq!(report.comparisons[0].error, -2.0);
    assert!(!report.comparisons[0].inside_predictive_interval);
    assert!(report.comparisons[1].inside_predictive_interval);
    assert_eq!(report.summaries[0].mean_absolute_error, 2.0);
    assert_eq!(report.summaries[1].samples, 1);
    assert!(report.status.contains("requires review"));
    assert!(!report.status.contains("certified"));
}
#[test]
fn failed_attempts_are_retained_without_treating_censored_times_as_completions() {
    let mut data = calibration::worksheet(&model(Encounter::Vorkath));
    data.observations = (0..4)
        .map(|n| observation(&n.to_string(), Split::Holdout, 4))
        .collect();
    data.observations[0].outcome = Outcome::Died;
    data.observations[1].outcome = Outcome::Teleported;
    data.observations[2].outcome = Outcome::Timeout;
    let r = calibration::compare(&data).unwrap();
    let c = &r.completion[1];
    assert_eq!(
        (c.attempts, c.completed, c.deaths, c.teleports, c.timeouts),
        (4, 1, 1, 1, 1)
    );
    assert_eq!(c.observed_rate, Some(0.25));
    let (low, high) = c.wilson_95.unwrap();
    assert!((low - 0.04558726).abs() < 1e-6 && (high - 0.69935816).abs() < 1e-6);
    assert_eq!(r.comparisons.len(), 1);
}
#[test]
fn missing_metrics_are_not_zeros_and_resource_loss_is_distinct_from_shots() {
    let mut data = calibration::worksheet(&model(Encounter::Vorkath));
    let mut o = observation("a", Split::Training, 1);
    o.metrics.insert("shots".into(), 10.0);
    o.metrics.insert("resource:9242".into(), 2.0);
    data.observations.push(o);
    let r = calibration::compare(&data).unwrap();
    assert_eq!(r.comparisons.len(), 3);
    assert!(r.comparisons.iter().all(|c| c.metric != "food_bites"));
    assert_eq!(
        r.comparisons
            .iter()
            .find(|c| c.metric == "shots")
            .unwrap()
            .observed,
        10.0
    );
    assert_eq!(
        r.comparisons
            .iter()
            .find(|c| c.metric == "resource:9242")
            .unwrap()
            .observed,
        2.0
    );
}
#[test]
fn training_proposals_use_weighted_exposure_and_never_peek_at_holdouts() {
    let mut data = calibration::worksheet(&model(Encounter::Vorkath));
    let mut a = observation("a", Split::Training, 100);
    a.exposed_ticks = Some(100);
    a.residual_damage = Some(10.0);
    a.eligible_opportunities = Some(10);
    a.used_opportunities = Some(5);
    a.timings.insert("acid_ticks".into(), vec![28, 30, 32]);
    let mut b = observation("b", Split::Training, 300);
    b.exposed_ticks = Some(300);
    b.residual_damage = Some(90.0);
    b.eligible_opportunities = Some(30);
    b.used_opportunities = Some(30);
    let mut h = observation("h", Split::Holdout, 500);
    h.exposed_ticks = Some(500);
    h.residual_damage = Some(1000.0);
    h.timings.insert("acid_ticks".into(), vec![99]);
    data.observations = vec![a, b, h];
    let r = calibration::compare(&data).unwrap();
    let value = |key| r.proposals.iter().find(|p| p.setting == key).unwrap();
    assert_eq!(value("incoming").proposed, 25.0);
    assert_eq!(value("uptime").proposed, 87.5);
    assert_eq!(value("acid_ticks").proposed, 30.0);
    assert_eq!(value("acid_ticks").measurements, 3);
    assert_eq!(data.inputs.settings.incoming, 0); // no automatic mutation
}
#[test]
fn unknown_setup_and_excluded_effects_never_become_calibration_evidence() {
    let mut data = calibration::worksheet(&model(Encounter::Yama));
    let mut a = observation("a", Split::Training, 100);
    a.setup_verified = false;
    a.timings.insert("flare_charge".into(), vec![50]);
    let mut b = observation("b", Split::Holdout, 100);
    b.exclusions.push("Thralls".into());
    data.observations = vec![a, b];
    let r = calibration::compare(&data).unwrap();
    assert_eq!(r.skipped.len(), 2);
    assert_eq!(r.accepted, 0);
    assert!(r.proposals.is_empty());
}
#[test]
fn wrong_encounter_phase_or_timing_key_is_not_silently_accepted() {
    let mut data = calibration::worksheet(&model(Encounter::Yama));
    let mut o = observation("a", Split::Training, 20);
    o.metrics.insert("phase:Yama P1/P2".into(), 10.0);
    data.observations.push(o);
    assert!(calibration::compare(&data).is_ok());
    data.observations[0]
        .metrics
        .insert("phase:Acid".into(), 10.0);
    assert!(calibration::compare(&data).is_err());
    data.observations[0].metrics.clear();
    data.observations[0]
        .timings
        .insert("acid_ticks".into(), vec![30]);
    assert!(calibration::compare(&data).is_err());
}
#[test]
fn outside_bounds_measurements_are_reported_without_clamping_or_applying() {
    let mut data = calibration::worksheet(&model(Encounter::Wardens));
    let mut o = observation("a", Split::Training, 100);
    o.timings.insert("prayer_rotation".into(), vec![200]);
    data.observations.push(o);
    let r = calibration::compare(&data).unwrap();
    let p = &r.proposals[0];
    assert_eq!(p.proposed, 200.0);
    assert!(!p.within_bounds);
    assert_eq!(p.current, 21);
}
#[test]
fn duplicates_and_overlapping_training_holdout_spans_are_rejected() {
    let mut data = calibration::worksheet(&model(Encounter::Wardens));
    let a = observation("a", Split::Training, 10);
    let mut b = a.clone();
    b.id = "b".into();
    b.split = Split::Holdout;
    data.observations = vec![a, b];
    assert!(calibration::compare(&data).is_err());
    data.observations[1].evidence.start_seconds = 100.0;
    data.observations[1].evidence.end_seconds = 200.0;
    assert!(calibration::compare(&data).is_ok());
    data.observations[1].id = "a".into();
    assert!(calibration::compare(&data).is_err());
    data.observations[1].id = "b".into();
    data.observations[1].evidence.start_seconds = 0.0;
    data.observations[0].evidence.reference = "https://www.youtube.com/watch?v=fixture&t=0".into();
    data.observations[1].evidence.reference = "https://youtu.be/fixture?si=share".into();
    assert!(calibration::compare(&data).is_err());
}
#[test]
fn bounded_inputs_and_partial_measurement_pairs_are_rejected() {
    let mut data = calibration::worksheet(&model(Encounter::Vorkath));
    data.observations
        .push(observation("a", Split::Training, 10));
    data.observations[0].residual_damage = Some(5.0);
    assert!(calibration::compare(&data).is_err());
    data.observations[0].residual_damage = None;
    data.observations[0].eligible_opportunities = Some(10);
    data.observations[0].used_opportunities = Some(11);
    assert!(calibration::compare(&data).is_err());
    assert!(calibration::parse(&" ".repeat(calibration::MAX_BYTES + 1)).is_err());
    let raw = serde_json::to_string(&data).unwrap().replacen(
        "\"version\":1",
        "\"unknown\":1,\"version\":1",
        1,
    );
    assert!(calibration::parse(&raw).is_err());
    data.observations.clear();
    data.model_revision += 1;
    assert!(calibration::compare(&data).is_err());
}
#[test]
fn stale_setup_checks_include_supplies_route_prayer_and_distribution() {
    let a = model(Encounter::Wardens);
    let mut b = a.clone();
    b.notes.push("Descriptive note".into());
    assert!(calibration::same_inputs(&a, &b));
    b.settings.warden = 1;
    assert!(!calibration::same_inputs(&a, &b));
    b = a.clone();
    b.attacks[0].prayer = 1.0;
    assert!(!calibration::same_inputs(&a, &b));
    b = a.clone();
    b.available.insert(385, 1);
    assert!(!calibration::same_inputs(&a, &b));
    b = a.clone();
    b.attacks[0].hits.hit_distribution = vec![(499, 1.0)];
    assert!(!calibration::same_inputs(&a, &b));
}
#[test]
fn timing_uncertainty_changes_coverage_but_not_raw_error() {
    let mut data = Dataset {
        version: 1,
        model_revision: simulation::MODEL_REVISION,
        inputs: model(Encounter::Vorkath),
        observations: vec![observation("a", Split::Training, 3)],
    };
    data.observations[0].evidence.uncertainty_ticks = 2;
    let r = calibration::compare(&data).unwrap();
    assert!(r.comparisons[0].inside_predictive_interval);
    assert_eq!(r.comparisons[0].error, -2.0);
}
