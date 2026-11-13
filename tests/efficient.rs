use ironbound::{
    account,
    efficient::{self, Candidate},
    planner::Options,
    tradeoffs,
};
fn candidate(cap: usize, gear: usize, bag: usize, time: f64, prayer: f64) -> Candidate {
    let mut row = Candidate::new(tradeoffs::budget(&account::demo(), &Options::default(), 11));
    row.budget.cap = cap;
    row.budget.max_pair = cap;
    row.budget.carried_gear = gear;
    row.budget.max_inventory_space = bag;
    row.budget.feasible = true;
    row.budget.prep_blockers.clear();
    let total = row.totals.as_mut().unwrap();
    total.seconds = time;
    total.prayer_points = prayer;
    total.incoming_damage = None;
    row
}
#[test]
fn chooses_compact_near_best_instead_of_filling_the_largest_ceiling() {
    let result = efficient::choose(
        vec![
            candidate(1, 2, 17, 105.0, 100.0),
            candidate(2, 3, 18, 102.0, 102.0),
            candidate(8, 8, 23, 100.0, 100.0),
        ],
        3,
        11,
    );
    assert_eq!(result.best, Some(2));
    assert_eq!(result.compact, Some(1));
}
#[test]
fn a_fast_but_fragile_benchmark_cannot_erase_the_full_budget_defensive_gain() {
    let mut compact = candidate(2, 3, 18, 101.0, 101.0);
    let mut fastest = candidate(5, 5, 20, 100.0, 100.0);
    let mut defensive = candidate(11, 8, 23, 102.0, 100.0);
    for row in [&mut compact, &mut fastest] {
        row.budget.windows[0].metrics.incoming[0].dps = Some(1.0);
    }
    defensive.budget.windows[0].metrics.incoming[0].dps = Some(0.5);
    let result = efficient::choose(vec![compact, fastest, defensive], 3, 11);
    assert_eq!(result.best, Some(1));
    assert_eq!(result.compact, Some(2));
}

#[test]
fn prayer_cost_is_guarded_alongside_attack_time() {
    let result = efficient::choose(
        vec![
            candidate(2, 3, 18, 101.0, 104.0),
            candidate(8, 8, 23, 100.0, 100.0),
        ],
        3,
        11,
    );
    assert_eq!(result.compact, Some(1));
}
#[test]
fn known_incoming_attacks_remain_guarded_when_the_total_is_unknown() {
    let mut light = candidate(2, 3, 18, 101.0, 101.0);
    let mut best = candidate(8, 8, 23, 100.0, 100.0);
    light.budget.windows[0].metrics.incoming[0].dps = Some(1.04);
    best.budget.windows[0].metrics.incoming[0].dps = Some(1.0);
    let result = efficient::choose(vec![light, best], 3, 11);
    assert_eq!(result.compact, Some(1));
}
#[test]
fn unknown_candidate_incoming_cannot_escape_a_known_reference() {
    let mut light = candidate(2, 3, 18, 101.0, 101.0);
    let mut best = candidate(8, 8, 23, 100.0, 100.0);
    light.budget.windows[0].metrics.incoming[0].dps = None;
    best.budget.windows[0].metrics.incoming[0].dps = Some(1.0);
    assert!(!efficient::qualifies(&light, &best, 3));
}
#[test]
fn protected_zero_damage_is_not_allowed_to_become_positive() {
    let mut light = candidate(2, 3, 18, 101.0, 101.0);
    let mut best = candidate(8, 8, 23, 100.0, 100.0);
    light.budget.windows[0].metrics.incoming[0].dps = Some(0.01);
    best.budget.windows[0].metrics.incoming[0].dps = Some(0.0);
    assert!(!efficient::qualifies(&light, &best, 3));
}
#[test]
fn normal_damage_exposure_cannot_grow_beyond_the_tolerance() {
    let mut light = candidate(2, 3, 18, 102.0, 101.0);
    let mut best = candidate(8, 8, 23, 100.0, 100.0);
    light.totals.as_mut().unwrap().incoming_damage = Some((50.0, 104.0));
    best.totals.as_mut().unwrap().incoming_damage = Some((50.0, 100.0));
    assert!(!efficient::qualifies(&light, &best, 3));
}
#[test]
fn losing_a_difficult_window_cannot_make_a_plan_the_fastest() {
    let mut partial = candidate(1, 1, 16, 10.0, 10.0);
    partial.covered -= 1;
    partial.signature.pop();
    let result = efficient::choose(vec![partial, candidate(8, 8, 23, 100.0, 100.0)], 3, 11);
    assert_eq!(result.best, Some(1));
    assert_eq!(result.compact, Some(1));
}
#[test]
fn missing_supplies_and_infeasible_fallbacks_are_excluded() {
    let mut missing = candidate(1, 1, 16, 90.0, 90.0);
    missing
        .budget
        .prep_blockers
        .push("Required ammunition missing".into());
    let mut fallback = candidate(2, 1, 16, 90.0, 90.0);
    fallback.budget.feasible = false;
    let result = efficient::choose(vec![missing, fallback], 3, 11);
    assert!(result.compact.is_none() && result.best.is_none());
    assert!(result.reason.is_some());
}
#[test]
fn actual_bag_space_wins_before_the_numeric_cap() {
    let result = efficient::choose(
        vec![
            candidate(2, 4, 21, 101.0, 101.0),
            candidate(4, 3, 20, 102.0, 102.0),
            candidate(8, 8, 23, 100.0, 100.0),
        ],
        3,
        11,
    );
    assert_eq!(result.compact, Some(1));
}
#[test]
fn user_ceiling_does_not_move_the_fastest_reference_or_get_raised() {
    let rows = vec![
        candidate(2, 3, 18, 110.0, 100.0),
        candidate(8, 8, 23, 100.0, 100.0),
    ];
    let result = efficient::choose(rows, 3, 2);
    assert_eq!(result.best, Some(1));
    assert!(result.compact.is_none());
    assert!(result.reason.unwrap().contains("2-change limit"));
}
#[test]
fn cache_reuses_caps_but_invalidates_actual_combat_bank_and_supply_changes() {
    let mut s = account::demo();
    let mut o = Options::default();
    let key = efficient::cache_key(&s, &o);
    o.max_switches = Some(2);
    o.inventory_overrides.insert(0, 385);
    o.inventory_quantities.insert(0, 4);
    assert_eq!(efficient::cache_key(&s, &o), key);
    o.food += 1;
    assert_ne!(efficient::cache_key(&s, &o), key);
    o.food -= 1;
    o.combat.protection = "Melee".into();
    assert_ne!(efficient::cache_key(&s, &o), key);
    o.combat.protection = "None".into();
    o.gear_overrides.insert("Magic:0".into(), 10828);
    assert_ne!(efficient::cache_key(&s, &o), key);
    o.gear_overrides.clear();
    let selected = s.selected.clone();
    s.members
        .iter_mut()
        .find(|m| m.name == selected)
        .unwrap()
        .level_overrides
        .insert("Magic".into(), 99);
    assert_ne!(efficient::cache_key(&s, &o), key);
}
#[test]
fn public_demo_end_to_end_keeps_identical_work_and_respects_stock_and_limits() {
    let snapshot = account::demo();
    let options = Options::default();
    let rows = (0..=11)
        .map(|cap| Candidate::new(tradeoffs::budget(&snapshot, &options, cap)))
        .collect();
    let result = efficient::choose(rows, 3, 11);
    let chosen = &result.candidates[result.compact.expect("prepared demo suggestion")];
    let best = &result.candidates[result.best.unwrap()];
    assert!(chosen.budget.feasible && chosen.budget.prep_blockers.is_empty());
    assert!(chosen.budget.max_pair <= chosen.budget.cap && chosen.budget.max_inventory_space <= 28);
    assert_eq!(chosen.signature, best.signature);
    assert!(efficient::qualifies(chosen, best, 3));
    assert!(chosen.budget.max_inventory_space <= best.budget.max_inventory_space);
}
