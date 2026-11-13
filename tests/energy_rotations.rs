use ironbound::{
    account::{self, Member},
    combat,
    defence::Settings,
    planner,
    rotation::{self, EnergyClock, Policy, Report},
};

fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn gear(weapon: i32) -> Vec<Option<i32>> {
    let mut set = vec![None; 14];
    set[3] = Some(weapon);
    set
}
fn target() -> planner::Monster {
    let mut t = planner::monster(26).unwrap().clone();
    t.attributes.clear();
    t.weakness = None;
    t.defensive.clear();
    for key in [
        "stab", "slash", "crush", "magic", "heavy", "standard", "light",
    ] {
        t.defensive.insert(key.into(), 0);
    }
    t.skills.insert("def".into(), 99);
    t.skills.insert("magic".into(), 99);
    t.skills.insert("hp".into(), 10000);
    t
}
fn settings(window: u32, energy: u32) -> Settings {
    let mut s = Settings {
        special_energy: energy,
        ..Default::default()
    };
    s.rotation.window_ticks = window;
    s.rotation.trials = 1;
    s
}
fn run(set: &[Option<i32>], s: &Settings) -> Report {
    rotation::evaluate(
        &member(),
        set,
        if set[3] == Some(34725) {
            "Magic"
        } else {
            "Crush"
        },
        &target(),
        0,
        s,
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn conserved(r: &Report, initial_energy: u32, hp: u32) {
    close(
        initial_energy as f64 + r.energy_restored + r.external_energy,
        r.energy_spent + r.energy_remaining,
    );
    close(r.effective_damage + r.remaining_hp, hp as f64);
    assert!(r.damage >= r.effective_damage);
    assert!(r.kill_interval_95.0 <= r.kill_chance + 1e-12);
    assert!(r.kill_interval_95.1 + 1e-12 >= r.kill_chance);
}

#[test]
fn pulses_are_discrete_and_spending_preserves_partial_progress() {
    for (lb, interval) in [(false, 50), (true, 25)] {
        let mut c = EnergyClock::new(100, 1, lb).unwrap();
        assert_eq!(c.advance(1000), 0);
        assert_eq!(c.next_pulse, interval);
        assert!(c.spend(50));
        assert_eq!(c.advance(interval - 1), 0);
        assert_eq!((c.energy, c.next_pulse), (50, 1));
        assert!(c.spend(50));
        assert_eq!(c.next_pulse, 1);
        assert_eq!(c.advance(1), 10);
        assert_eq!((c.energy, c.next_pulse), (10, interval));
        assert_eq!(c.advance(2 * interval), 20);
        assert_eq!(c.energy, 30);
        let before = c;
        assert!(!c.spend(31));
        assert_eq!(c, before);
    }
}

#[test]
fn full_energy_caps_actual_recovery_and_cannot_bank_a_pulse() {
    let mut c = EnergyClock::new(95, 1, false).unwrap();
    assert_eq!(c.advance(1234), 5);
    assert_eq!((c.energy, c.next_pulse), (100, 50));
    assert!(c.spend(50));
    assert_eq!(c.advance(49), 0);
    assert_eq!(c.advance(1), 10);
    let mut c = EnergyClock::new(100, 1, true).unwrap();
    assert_eq!(c.next_pulse, 25);
    c.equip_lightbearer(false);
    assert_eq!(c.next_pulse, 50);
    c.equip_lightbearer(true);
    assert_eq!(c.next_pulse, 25);
}

#[test]
fn lightbearer_equipping_preserves_near_pulses_but_removal_resets() {
    for remaining in [1, 10, 25, 26, 40, 50] {
        let mut c = EnergyClock::new(20, remaining, false).unwrap();
        c.equip_lightbearer(true);
        assert_eq!(c.next_pulse, remaining.min(25));
        assert_eq!(c.energy, 20);
        c.advance(1);
        let before = c;
        c.equip_lightbearer(true);
        assert_eq!(c, before);
        c.equip_lightbearer(false);
        assert_eq!(c.next_pulse, 50);
    }
    assert!(EnergyClock::new(101, 0, false).is_err());
    assert!(EnergyClock::new(0, 51, false).is_err());
    assert!(EnergyClock::new(0, 26, true).is_err());
    assert_eq!(EnergyClock::new(0, 0, true).unwrap().next_pulse, 25);
    assert_eq!(EnergyClock::new(100, 50, true).unwrap().next_pulse, 25);
}

#[test]
fn five_minute_kisten_timeline_matches_independent_energy_arithmetic() {
    let mut set = gear(33631);
    let s = settings(500, 100);
    for (ring, expected, restored) in [
        (None, vec![0, 4, 252], 100.0),
        (Some(25975), vec![0, 4, 128, 252, 376], 200.0),
    ] {
        set[12] = ring;
        let r = run(&set, &s);
        assert_eq!(
            r.trace
                .iter()
                .filter(|e| e.attack == "Special")
                .map(|e| e.tick)
                .collect::<Vec<_>>(),
            expected
        );
        close(r.special_activations, expected.len() as f64);
        close(r.normal_attacks, 125.0 - expected.len() as f64);
        close(r.energy_restored, restored);
        close(r.energy_remaining, 50.0);
        assert_eq!(r.trace.len(), 125);
        conserved(&r, 100, 10000);
    }
}

#[test]
fn deadline_pulses_do_not_fund_an_action_outside_the_window() {
    let mut set = gear(33631);
    let at_end = run(&set, &settings(250, 0));
    close(at_end.special_activations, 0.0);
    close(at_end.energy_remaining, 50.0);
    let after = run(&set, &settings(253, 0));
    assert_eq!(after.trace.last().unwrap().tick, 252);
    close(after.special_activations, 1.0);
    set[12] = Some(25975);
    close(run(&set, &settings(128, 0)).special_activations, 0.0);
    let after = run(&set, &settings(129, 0));
    close(after.special_activations, 1.0);
    assert_eq!(after.trace.last().unwrap().tick, 128);
    let mut s = settings(5, 40);
    s.rotation.next_pulse = 4;
    let r = run(&set, &s);
    assert_eq!(r.trace[1].tick, 4);
    assert_eq!(r.trace[1].energy_before, 50);
    assert_eq!(r.trace[1].attack, "Special");
    conserved(&r, 40, 10000);
}

#[test]
fn normal_policy_keeps_energy_full_and_existing_empowerment_is_consumed_first() {
    let mut s = settings(500, 100);
    s.rotation.policy = Policy::NormalOnly;
    let r = run(&gear(33631), &s);
    close(r.normal_attacks, 125.0);
    close(r.special_activations, 0.0);
    close(r.energy_restored, 0.0);
    close(r.energy_remaining, 100.0);
    s.rotation.window_ticks = 7;
    s.zorya_empowered_hits = 3;
    s.special_energy = 0;
    let r = run(&gear(34725), &s);
    assert_eq!(
        r.trace.iter().map(|e| e.tick).collect::<Vec<_>>(),
        vec![0, 2, 4, 6]
    );
    assert_eq!(
        r.trace
            .iter()
            .map(|e| e.attack.as_str())
            .collect::<Vec<_>>(),
        vec!["Empowered", "Empowered", "Empowered", "Normal"]
    );
    close(r.empowered_remaining, 0.0);
    close(r.energy_spent, 0.0);
    conserved(&r, 0, 10000);
    s.special_energy = 100;
    s.rotation.policy = Policy::SpecialsWhenReady;
    let r = run(&gear(34725), &s);
    assert_eq!(r.trace[0].attack, "Empowered");
    assert_eq!(r.trace[1].attack, "Empowered");
    assert_eq!(r.trace[2].attack, "Empowered");
    assert_eq!(
        (r.trace[3].tick, r.trace[3].attack.as_str()),
        (6, "Special")
    );
    close(r.energy_spent, 50.0);
    conserved(&r, 100, 10000);
}

#[test]
fn death_stops_rotation_and_recovery_at_the_killing_action() {
    let mut s = settings(500, 100);
    s.rotation.trials = 128;
    s.target_hp.insert(9466, 1);
    let r = rotation::evaluate(
        &member(),
        &gear(33631),
        "Crush",
        planner::monster(9466).unwrap(),
        0,
        &s,
    )
    .unwrap();
    close(r.kill_chance, 1.0);
    close(r.effective_damage, 1.0);
    close(r.special_activations, 1.0);
    close(r.energy_remaining, 50.0);
    close(r.energy_restored, 0.0);
    assert_eq!(r.trace.len(), 1);
    conserved(&r, 100, 1);
}

#[test]
fn zorya_accurate_zero_still_grants_buffs_and_sampling_matches_exact_burst() {
    let set = gear(34725);
    let mut s = settings(3, 100);
    s.rotation.trials = 4096;
    let mut t = target();
    t.defensive.insert("magic_resistance".into(), 100);
    let r = rotation::evaluate(&member(), &set, "Magic", &t, 0, &s).unwrap();
    let p = combat::accuracy(29975, 108 * 64);
    close(r.damage, 0.0);
    close(r.special_activations, 1.0);
    assert!((r.empowered_remaining / 3.0 - p).abs() < 6.0 * (p * (1.0 - p) / 4096.0).sqrt());
    conserved(&r, 100, 10000);
    s.target_hp.insert(26, 30);
    let exact =
        ironbound::special_attacks::evaluate(&member(), &set, "Magic", &target(), 0, &s).unwrap();
    // One opener only: independently enumerate its 42 equally likely accurate rolls.
    let expected = p * (0..=41).map(|d| d.max(1).min(30) as f64).sum::<f64>() / 42.0;
    let r = run(&set, &s);
    assert!((r.effective_damage - expected).abs() < 6.0 * r.damage_standard_error);
    assert!(exact.effective_damage >= r.effective_damage);
    conserved(&r, 100, 30);
    assert_eq!(
        serde_json::to_value(&r).unwrap(),
        serde_json::to_value(run(&set, &s)).unwrap()
    );
}

#[test]
fn ring_counterfactual_changes_real_stats_and_preserves_initial_phase() {
    let mut set = gear(33631);
    set[12] = Some(11773);
    let mut s = settings(100, 40);
    s.rotation.next_pulse = 10;
    let (with_lb, comparison) = rotation::equip_at_start(&s, &set, 25975).unwrap();
    assert_eq!(set[12], Some(11773));
    assert_eq!(with_lb[12], Some(25975));
    assert_eq!(comparison.rotation.next_pulse, 10);
    assert!(!rotation::lightbearer(&set));
    assert!(rotation::lightbearer(&with_lb));
    let normal = |gear: &[Option<i32>]| {
        combat::calculate_configured(&member(), gear, "Crush", &target(), "", 0, &s).unwrap()
    };
    assert!(normal(&set).max_hit > normal(&with_lb).max_hit);
    let mut wrong_slot = gear(33631);
    wrong_slot[0] = Some(25975);
    assert!(!rotation::lightbearer(&wrong_slot));
    assert!(rotation::equip_at_start(&s, &set, 11773).is_err());
}

#[test]
fn rotation_is_opt_in_serializable_and_does_not_change_normal_calculations() {
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert!(!old.rotation.enabled);
    let mut s = settings(500, 0);
    s.rotation.enabled = true;
    s.rotation.next_pulse = 7;
    s.rotation.policy = Policy::NormalOnly;
    let restored: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(restored.rotation, s.rotation);
    let set = gear(34725);
    let normal = |settings: &Settings| {
        combat::calculate_configured(&member(), &set, "Magic", &target(), "", 0, settings).unwrap()
    };
    assert_eq!(
        serde_json::to_value(normal(&old)).unwrap(),
        serde_json::to_value(normal(&s)).unwrap()
    );
    for (ticks, trials, pulse) in [(0, 1, 0), (501, 1, 0), (1, 0, 0), (1, 4097, 0), (1, 1, 51)] {
        s.rotation.window_ticks = ticks;
        s.rotation.trials = trials;
        s.rotation.next_pulse = pulse;
        assert!(!s.valid());
        assert!(rotation::evaluate(&member(), &set, "Magic", &target(), 0, &s).is_err());
    }
    for id in [100180, 100181, 100185] {
        assert!(
            rotation::evaluate(
                &member(),
                &set,
                "Magic",
                planner::monster(id).unwrap(),
                0,
                &old
            )
            .unwrap_err()
            .contains("not verified")
        );
    }
}

#[test]
fn swapping_preserves_previous_cooldowns_and_charges_each_real_setup_change() {
    let set = gear(4151);
    let mut s = settings(17, 100);
    s.rotation.swap_weapon = Some(33631);
    s.rotation.swap_style = "Crush".into();
    s.rotation.swap_delay = 2;
    let r = rotation::evaluate(&member(), &set, "Slash", &target(), 0, &s).unwrap();
    let actions: Vec<_> = r
        .trace
        .iter()
        .filter(|e| e.attack != "Swap")
        .map(|e| (e.tick, e.attack.as_str(), e.weapon))
        .collect();
    assert_eq!(
        actions,
        vec![
            (2, "Special", Some(33631)),
            (6, "Special", Some(33631)),
            (12, "Normal", Some(4151)),
            (16, "Normal", Some(4151))
        ]
    );
    assert_eq!(
        r.trace
            .iter()
            .filter(|e| e.attack == "Swap")
            .map(|e| e.tick)
            .collect::<Vec<_>>(),
        vec![0, 10]
    );
    close(r.switches, 2.0);
    close(r.energy_spent, 100.0);
    conserved(&r, 100, 10000);
    s.rotation.window_ticks = 2;
    let r = rotation::evaluate(&member(), &set, "Slash", &target(), 0, &s).unwrap();
    close(r.special_activations, 0.0);
    close(r.energy_spent, 0.0);
    close(r.switches, 1.0);
}

#[test]
fn zorya_swap_removes_offhand_and_finishes_buffs_before_returning_to_filler() {
    let mut set = gear(4151);
    set[5] = Some(22322);
    let mut s = settings(10, 50);
    s.rotation.swap_weapon = Some(34725);
    let (special, style) = rotation::special_loadout(&set, "Slash", &s.rotation).unwrap();
    assert_eq!(special[5], None);
    assert_eq!(set[5], Some(22322));
    assert_eq!(style, "Magic");
    let mut t = target();
    t.defensive.insert("magic".into(), -64);
    let r = rotation::evaluate(&member(), &set, "Slash", &t, 0, &s).unwrap();
    assert_eq!(
        r.trace
            .iter()
            .filter(|e| e.attack != "Swap")
            .map(|e| (e.tick, e.attack.as_str(), e.weapon))
            .collect::<Vec<_>>(),
        vec![
            (0, "Special", Some(34725)),
            (3, "Empowered", Some(34725)),
            (5, "Empowered", Some(34725)),
            (7, "Empowered", Some(34725)),
            (9, "Normal", Some(4151))
        ]
    );
    close(r.switches, 2.0);
    close(r.empowered_remaining, 0.0);
    conserved(&r, 50, 10000);
    let normal = combat::calculate_configured(&member(), &set, "Slash", &t, "", 0, &s).unwrap();
    let mut unshielded = set.clone();
    unshielded[5] = None;
    assert!(
        normal.attack_roll
            > combat::calculate_configured(&member(), &unshielded, "Slash", &t, "", 0, &s)
                .unwrap()
                .attack_roll
    );
}

#[test]
fn surge_during_cooldown_funds_the_next_action_without_resetting_natural_progress() {
    let mut s = settings(51, 40);
    s.rotation.surge_tick = Some(3);
    let r = run(&gear(33631), &s);
    let dose = r.trace.iter().find(|e| e.attack == "Surge dose").unwrap();
    assert_eq!(
        (dose.tick, dose.energy_before, dose.energy_after),
        (3, 40, 65)
    );
    assert_eq!(
        (r.trace[2].tick, r.trace[2].attack.as_str()),
        (4, "Special")
    );
    close(r.surge_doses, 1.0);
    close(r.external_energy, 25.0);
    close(r.energy_restored, 10.0);
    close(r.energy_remaining, 25.0);
    close(r.surge_cooldown_remaining, 452.0);
    conserved(&r, 40, 10000);
}

#[test]
fn restore_at_deadline_updates_energy_without_starting_an_extra_attack() {
    let mut s = settings(4, 40);
    s.rotation.surge_tick = Some(4);
    let r = run(&gear(33631), &s);
    close(r.special_activations, 0.0);
    close(r.normal_attacks, 1.0);
    close(r.energy_remaining, 65.0);
    assert_eq!(r.trace.last().unwrap().attack, "Surge dose");
    s.rotation.surge_tick = Some(5);
    let r = run(&gear(33631), &s);
    close(r.external_energy, 0.0);
    close(r.energy_remaining, 40.0);
    s.rotation.surge_tick = Some(0);
    s.special_energy = 100;
    let r = run(&gear(33631), &s);
    close(r.surge_doses, 0.0);
    close(r.external_energy, 0.0);
    assert_eq!(r.trace[0].attack, "Surge skipped · full energy");
}

#[test]
fn simultaneous_restores_follow_natural_surge_transfer_order_and_cap_actual_energy() {
    let mut s = settings(53, 40);
    s.rotation.surge_tick = Some(50);
    s.rotation.transfer_tick = Some(50);
    let r = run(&gear(33631), &s);
    let arrivals: Vec<_> = r
        .trace
        .iter()
        .filter(|e| e.tick == 50)
        .map(|e| (e.attack.as_str(), e.energy_before, e.energy_after))
        .collect();
    assert_eq!(
        arrivals,
        vec![("Surge dose", 50, 75), ("Energy Transfer arrival", 75, 100)]
    );
    close(r.energy_restored, 10.0);
    close(r.external_energy, 50.0);
    close(r.special_activations, 1.0);
    close(r.energy_remaining, 50.0);
    conserved(&r, 40, 10000);
    let mut clock = EnergyClock::new(90, 1, false).unwrap();
    assert_eq!(clock.restore(5), 5);
    assert_eq!(clock.next_pulse, 1);
    assert_eq!(clock.restore(25), 5);
    assert_eq!(clock.next_pulse, 50);
    clock.advance(49);
    clock.spend(50);
    assert_eq!(clock.advance(49), 0);
    assert_eq!(clock.advance(1), 10);
}

#[test]
fn external_arrivals_run_during_declared_swap_overhead() {
    let mut s = settings(10, 100);
    s.rotation.swap_weapon = Some(34725);
    s.rotation.swap_delay = 5;
    s.rotation.transfer_tick = Some(2);
    s.rotation.surge_tick = Some(3);
    let r = run(&gear(33631), &s);
    assert_eq!(
        r.trace
            .iter()
            .take(4)
            .map(|e| (e.tick, e.attack.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (0, "Swap"),
            (2, "Energy Transfer arrival"),
            (3, "Surge skipped · full energy"),
            (5, "Special")
        ]
    );
    close(r.external_energy, 0.0);
    close(r.surge_doses, 0.0);
    close(r.transfer_events, 1.0);
    conserved(&r, 100, 10000);
}

#[test]
fn death_charge_applies_only_at_an_eligible_kill_before_expiry() {
    let set = gear(33631);
    let t = planner::monster(9466).unwrap();
    let mut s = settings(500, 100);
    s.target_hp.insert(9466, 1);
    s.rotation.death_charge = Some(100);
    s.rotation.surge_tick = Some(2);
    s.rotation.transfer_tick = Some(3);
    let r = rotation::evaluate(&member(), &set, "Crush", t, 0, &s).unwrap();
    close(r.kill_chance, 1.0);
    close(r.death_charge_triggers, 1.0);
    close(r.external_energy, 15.0);
    close(r.energy_remaining, 65.0);
    close(r.surge_doses, 0.0);
    close(r.transfer_events, 0.0);
    assert_eq!(
        r.trace
            .iter()
            .map(|e| e.attack.as_str())
            .collect::<Vec<_>>(),
        vec!["Special", "Death Charge"]
    );
    conserved(&r, 100, 1);
    s.rotation.policy = Policy::NormalOnly;
    let r = rotation::evaluate(&member(), &set, "Crush", t, 0, &s).unwrap();
    close(r.death_charge_triggers, 1.0);
    close(r.external_energy, 0.0);
    close(r.energy_remaining, 100.0);
    // A weapon/stance change delays the guaranteed killing special until expiry.
    s.rotation.policy = Policy::SpecialsWhenReady;
    s.rotation.swap_weapon = Some(33631);
    s.rotation.swap_style = "Stab".into();
    s.rotation.swap_delay = 1;
    s.rotation.death_charge = Some(1);
    let r = rotation::evaluate(&member(), &set, "Crush", t, 0, &s).unwrap();
    assert_eq!(r.trace.last().unwrap().tick, 1);
    close(r.death_charge_triggers, 0.0);
    let mut low = member();
    low.level_overrides.insert("Magic".into(), 79);
    assert!(
        rotation::evaluate(&low, &set, "Crush", t, 0, &s)
            .unwrap_err()
            .contains("80 Magic")
    );
    for id in [13011, 13012, 13013, 7550, 7552] {
        assert!(!rotation::death_charge_eligible(
            planner::monster(id).unwrap()
        ));
    }
}

#[test]
fn resource_availability_rejects_unowned_excluded_and_noted_comparison_items() {
    let mut options = planner::Options::default();
    options.combat.rotation.swap_weapon = Some(34725);
    let mut stock = std::collections::BTreeMap::new();
    assert!(rotation::validate_available(&stock, &options).is_err());
    stock.insert(34725, 1);
    assert!(rotation::validate_available(&stock, &options).is_ok());
    options.combat.rotation.surge_tick = Some(1);
    stock.insert(30876, 10);
    assert!(rotation::validate_available(&stock, &options).is_err());
    stock.insert(30884, 1);
    assert!(rotation::validate_available(&stock, &options).is_ok());
    options.excluded.insert(30884);
    assert!(rotation::validate_available(&stock, &options).is_err());
}

#[test]
fn cooldown_and_swap_validation_preserve_old_defaults_and_normal_gear_scores() {
    let mut s = settings(100, 50);
    s.rotation.surge_tick = Some(10);
    s.rotation.surge_ready = 11;
    assert!(
        rotation::evaluate(&member(), &gear(33631), "Crush", &target(), 0, &s)
            .unwrap_err()
            .contains("cooldown")
    );
    for id in [34727, 4151, -1] {
        s.rotation.swap_weapon = Some(id);
        assert!(!s.valid());
    }
    s.rotation.swap_weapon = Some(34725);
    s.rotation.surge_ready = 0;
    s.rotation.swap_style = "Slash".into();
    assert!(rotation::evaluate(&member(), &gear(33631), "Crush", &target(), 0, &s).is_err());
    let old: rotation::Settings = serde_json::from_str("{}").unwrap();
    assert!(
        old.swap_weapon.is_none()
            && old.surge_tick.is_none()
            && old.transfer_tick.is_none()
            && old.death_charge.is_none()
    );
    s.rotation.swap_style.clear();
    let normal = |cfg: &Settings| {
        combat::calculate_configured(&member(), &gear(33631), "Crush", &target(), "", 0, cfg)
            .unwrap()
    };
    assert_eq!(
        serde_json::to_value(normal(&s)).unwrap(),
        serde_json::to_value(normal(&Settings::default())).unwrap()
    );
}

#[test]
fn standard_spell_fillers_keep_the_selected_spell_and_its_cooldown_after_a_swap() {
    let set = gear(1381); // Standard casting air staff, not a powered weapon.
    let mut s = settings(10, 50);
    s.rotation.swap_weapon = Some(33631);
    let r = rotation::evaluate_with_spell(&member(), &set, "Magic", &target(), 0, &s, "Fire Wave")
        .unwrap();
    assert_eq!(
        r.trace
            .iter()
            .filter(|e| e.attack != "Swap")
            .map(|e| (e.tick, e.attack.as_str(), e.weapon))
            .collect::<Vec<_>>(),
        vec![
            (0, "Special", Some(33631)),
            (4, "Normal", Some(1381)),
            (9, "Normal", Some(1381))
        ]
    );
    close(r.normal_attacks, 2.0);
    close(r.special_activations, 1.0);
    conserved(&r, 50, 10000);
    assert!(
        rotation::evaluate_with_spell(
            &member(),
            &set,
            "Magic",
            &target(),
            0,
            &s,
            "Unavailable spell"
        )
        .is_err()
    );
}

#[test]
fn stateful_fillers_are_withheld_instead_of_crediting_stationary_effects_across_swaps() {
    let mut s = settings(100, 50);
    s.rotation.swap_weapon = Some(33631);
    let mut blood = gear(28997);
    blood[0] = Some(29028);
    blood[4] = Some(29022);
    blood[7] = Some(29025);
    assert!(
        rotation::evaluate(&member(), &blood, "Crush", &target(), 0, &s)
            .unwrap_err()
            .contains("variable cooldown")
    );
    let mut eclipse = gear(29000);
    eclipse[13] = Some(28991);
    eclipse[0] = Some(29010);
    eclipse[4] = Some(29004);
    eclipse[7] = Some(29007);
    assert!(
        rotation::evaluate(&member(), &eclipse, "Ranged", &target(), 0, &s)
            .unwrap_err()
            .contains("delayed damage")
    );
}
