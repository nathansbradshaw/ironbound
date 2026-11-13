use ironbound::{
    account, combat, encounter_summary,
    planner::{self, Options},
    tradeoffs,
};

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn comparison() -> tradeoffs::Comparison {
    let mut c = tradeoffs::compare(&account::demo(), &Options::default(), 4, 8);
    assert!(c.left.feasible && c.right.feasible);
    for side in [&mut c.left, &mut c.right] {
        side.windows.truncate(2);
        side.unmodelled_phases.clear();
        for w in &mut side.windows {
            w.metrics.incoming = vec![tradeoffs::Incoming {
                attack: "Normal".into(),
                dps: Some(1.0),
                reason: None,
                healing_per_second: Some(0.0),
            }];
            w.metrics.prayer_per_minute = 12.0;
            w.metrics.defence = ["stab", "slash", "crush", "magic", "ranged"]
                .into_iter()
                .map(|k| (k.into(), 10))
                .collect();
        }
    }
    c
}
fn dps(w: &mut tradeoffs::Window, dps: f64) {
    w.metrics.dps = Some(dps);
    w.metrics.attack.as_mut().unwrap().dps = dps;
}
#[test]
fn aggregate_uses_work_over_time_instead_of_averaging_dps() {
    let mut c = comparison();
    dps(&mut c.left.windows[0], 2.0);
    dps(&mut c.left.windows[1], 8.0);
    dps(&mut c.right.windows[0], 4.0);
    dps(&mut c.right.windows[1], 4.0);
    let result = encounter_summary::compare(&c);
    assert!(result.complete());
    let a = result.left.unwrap();
    let b = result.right.unwrap();
    near(result.workloads.iter().map(|w| w.hp).sum(), 200.0);
    near(a.seconds, 62.5);
    near(b.seconds, 50.0);
    near(a.effective_dps, 3.2);
    near(b.effective_dps, 4.0);
    near(a.prayer_points, 12.5);
    near(b.prayer_points, 10.0);
    near(a.prayer_per_minute, 12.0);
}
#[test]
fn dynamic_burn_comparison_uses_full_workload_for_both_sides() {
    let mut c = comparison();
    let mut snapshot = account::demo();
    let m = &mut snapshot.members[0];
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m.bank = Some(vec![
        29010, 1, 29004, 1, 29007, 1, 29000, 1, 28991, 1000, 9185, 1, 9144, 1000,
    ]);
    let mut eclipse = vec![None; 14];
    for (slot, id) in [(0, 29010), (4, 29004), (7, 29007), (3, 29000), (13, 28991)] {
        eclipse[slot] = Some(id);
    }
    let mut bow = vec![None; 14];
    bow[3] = Some(9185);
    bow[13] = Some(9144);
    let mut options = Options::default();
    options.combat.target_hp.insert(12223, 1);
    for (side, set) in [(&mut c.left, eclipse), (&mut c.right, bow)] {
        side.windows = vec![tradeoffs::Window {
            target_id: 12223,
            name: "Vardorvis".into(),
            style: "Ranged".into(),
            metrics: tradeoffs::measure(&snapshot, &options, &set, "Ranged", 12223),
            equipment: set,
        }];
    }
    let expected_left = c.left.windows[0]
        .metrics
        .dynamic_budget
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    let expected_right = c.right.windows[0]
        .metrics
        .dynamic_budget
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(
        (expected_left.start_hp, expected_right.start_hp),
        (700, 700)
    );
    assert!(expected_left.burn.is_some() && expected_right.burn.is_none());
    let report = encounter_summary::compare(&c);
    assert!(report.complete(), "{:?}", report.omissions);
    near(report.left.unwrap().seconds, expected_left.seconds);
    near(report.right.unwrap().seconds, expected_right.seconds);
    c.right.windows[0].metrics.dynamic_budget = Some(Err("Dynamic outcomes unavailable".into()));
    let report = encounter_summary::compare(&c);
    assert!(!report.complete());
    assert!(
        report
            .omissions
            .iter()
            .any(|s| s.contains("Dynamic outcomes unavailable"))
    );
}
#[test]
fn unknown_outgoing_excludes_the_same_workload_from_both_sides() {
    let mut c = comparison();
    c.right.windows[0].metrics.attack = None;
    c.right.windows[0].metrics.dps = None;
    c.right.windows[0].metrics.dps_reason = Some("Unsupported weapon effect".into());
    let result = encounter_summary::compare(&c);
    assert!(!result.complete());
    assert_eq!(result.covered_windows, 1);
    assert_eq!(result.total_windows, 2);
    assert!(
        result
            .omissions
            .iter()
            .any(|n| n.contains("Unsupported weapon effect"))
    );
    near(result.workloads.iter().map(|w| w.hp).sum(), 100.0);
    c.left.windows[1].metrics.attack = None;
    let none = encounter_summary::compare(&c);
    assert!(none.left.is_none() && none.right.is_none());
}
#[test]
fn unknown_incoming_is_not_a_zero_damage_total() {
    let mut c = comparison();
    c.left.windows[0]
        .metrics
        .incoming
        .push(tradeoffs::Incoming {
            attack: "Chip".into(),
            dps: None,
            reason: Some("unknown".into()),
            healing_per_second: None,
        });
    let result = encounter_summary::compare(&c);
    assert!(result.left.unwrap().incoming_damage.is_none());
    assert!(result.right.unwrap().incoming_damage.is_some());
}
#[test]
fn prayer_defence_and_incoming_ranges_follow_actual_time_in_each_set() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        dps(&mut side.windows[0], 2.0);
        dps(&mut side.windows[1], 8.0);
        side.windows[0].metrics.prayer_per_minute = 6.0;
        side.windows[1].metrics.prayer_per_minute = 18.0;
        side.windows[0].metrics.defence.insert("slash".into(), 20);
        side.windows[1].metrics.defence.insert("slash".into(), 100);
        side.windows[0].metrics.incoming.push(tradeoffs::Incoming {
            attack: "Other".into(),
            dps: Some(3.0),
            reason: None,
            healing_per_second: Some(0.0),
        });
    }
    let result = encounter_summary::compare(&c).left.unwrap();
    near(result.prayer_points, 8.75);
    near(result.prayer_per_minute, 8.4);
    near(result.defence["slash"], 36.0);
    assert_eq!(result.incoming_damage, Some((62.5, 162.5)));
}
#[test]
fn multiple_styles_share_one_boss_hp_budget_and_priorities_do_not_add_hp() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        side.windows[1].target_id = side.windows[0].target_id;
        side.windows[1].style = "Ranged".into();
        dps(&mut side.windows[0], 2.0);
        dps(&mut side.windows[1], 8.0);
        side.options
            .phase_priorities
            .insert(side.windows[0].target_id, 10);
    }
    let result = encounter_summary::compare(&c);
    near(result.workloads.iter().map(|w| w.hp).sum(), 100.0);
    near(result.left.unwrap().seconds, 31.25);
}
#[test]
fn alternative_yama_states_share_progress_without_recounting_the_main_boss() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        side.windows[0].target_id = 14176;
        side.windows[1].target_id = 100010;
        for w in &mut side.windows {
            w.style = "Slash".into();
            dps(w, 10.0);
        }
    }
    let result = encounter_summary::compare(&c);
    near(result.workloads.iter().map(|w| w.hp).sum(), 1667.0);
    near(result.left.unwrap().seconds, 166.7);
}
#[test]
fn invalid_caps_withhold_all_totals_and_unmapped_phases_mark_partial() {
    let mut c = comparison();
    c.left.unmodelled_phases.push("Wardens".into());
    let result = encounter_summary::compare(&c);
    assert!(!result.complete());
    assert!(result.left.is_some());
    assert!(result.omissions.iter().any(|n| n.contains("Wardens")));
    c.right.feasible = false;
    let result = encounter_summary::compare(&c);
    assert!(result.left.is_none() && result.right.is_none());
}
#[test]
fn blood_moon_healing_changes_time_and_cannot_be_dropped_when_unknown() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        side.windows.truncate(1);
        side.windows[0].target_id = 13011;
        dps(&mut side.windows[0], 10.0);
        side.windows[0].metrics.incoming[0].healing_per_second = Some(2.0);
    }
    let result = encounter_summary::compare(&c);
    let hp = planner::monster(13011).unwrap().skills["hp"] as f64;
    near(result.left.unwrap().seconds, hp / 8.0);
    c.left.windows[0].metrics.incoming[0].healing_per_second = None;
    let result = encounter_summary::compare(&c);
    assert!(result.left.is_none() && result.right.is_none());
}
#[test]
fn paired_judge_counts_two_intermission_cycles_once_with_style_specific_prayer() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        side.windows[0].target_id = 100012;
        side.windows[0].style = "Magic".into();
        side.windows[1].target_id = 100013;
        side.windows[1].style = "Slash".into();
        for (index, hit, ticks, drain) in [(0, 40, 5, 6.0), (1, 60, 4, 12.0)] {
            let w = &mut side.windows[index];
            let attack = w.metrics.attack.as_mut().unwrap();
            attack.guaranteed_hits = Some(vec![(hit, 1.0)]);
            attack.attack_ticks = ticks;
            w.metrics.prayer_per_minute = drain;
        }
    }
    let result = encounter_summary::compare(&c);
    assert!(result.complete());
    assert_eq!(result.covered_windows, 2);
    assert_eq!(result.workloads.len(), 1);
    near(result.workloads[0].hp, 800.0);
    let total = result.left.unwrap();
    // Four Magic and four melee attacks per 400-HP Judge, two Judges.
    near(total.seconds, 43.2);
    near(total.prayer_points, 6.24);
    c.right.windows.remove(1);
    assert!(encounter_summary::compare(&c).left.is_none());
}
#[test]
fn flare_probe_keeps_discrete_attack_commitments_and_declares_the_assumption() {
    let mut c = comparison();
    for side in [&mut c.left, &mut c.right] {
        side.windows.truncate(1);
        let w = &mut side.windows[0];
        w.target_id = 100014;
        w.style = "Magic".into();
        let attack = w.metrics.attack.as_mut().unwrap();
        attack.guaranteed_hits = Some(vec![(60, 1.0)]);
        attack.max_hit = 60;
        attack.attack_ticks = 5;
    }
    let result = encounter_summary::compare(&c);
    near(result.workloads[0].hp, 280.0);
    near(result.left.unwrap().seconds, 18.0);
    assert!(
        result
            .assumptions
            .iter()
            .any(|n| n.contains("not a prediction of spawn count"))
    );
}
#[test]
fn target_hp_uses_the_existing_toa_invocation_rounding() {
    let akkha = planner::monster(11789).unwrap();
    let base = akkha.skills["hp"];
    assert_eq!(combat::target_hitpoints(akkha, 0), Some(base));
    let scaled = base + base * 60 / 100;
    let rounded = (scaled + 5) / 10 * 10;
    assert_eq!(combat::target_hitpoints(akkha, 150), Some(rounded));
    assert_eq!(combat::target_hitpoints(akkha, 999), Some(rounded));
}
