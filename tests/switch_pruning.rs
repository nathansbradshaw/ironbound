use ironbound::{
    account, defence,
    efficient::{self, Tolerances},
    encounters,
    planner::{self, Boss, Options},
    setups, switch_pruning, tradeoffs,
};
use std::collections::BTreeSet;
fn fixture() -> (account::Snapshot, Options) {
    let melee = [
        Some(11865),
        Some(6570),
        Some(6585),
        Some(19675),
        Some(29043),
        Some(12954),
        None,
        Some(4996),
        None,
        Some(7462),
        Some(11840),
        None,
        Some(11773),
        None,
    ];
    let mage = [
        Some(11865),
        Some(21795),
        Some(6585),
        Some(4864),
        Some(4869),
        Some(20714),
        None,
        Some(4876),
        None,
        Some(7462),
        Some(4097),
        None,
        Some(28327),
        None,
    ];
    let mut s = account::demo();
    let m = &mut s.members[0];
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m.level_overrides.insert("Magic".into(), 86);
    let ids: BTreeSet<_> = melee.iter().chain(&mage).flatten().copied().collect();
    m.bank = Some(
        ids.into_iter()
            .flat_map(|id| [i64::from(id), 1])
            .chain([
                554, 2000, 566, 1000, 564, 100, 385, 8, 2434, 3, 12625, 1, 2552, 1,
            ])
            .collect(),
    );
    m.inventory = None;
    m.equipment = None;
    m.rune_pouch = None;
    let mut o = Options {
        boss: Boss::Custom,
        targets: encounters::groups()
            .iter()
            .find(|g| g.name == "Tormented Demons")
            .unwrap()
            .targets
            .clone(),
        switch_tolerance: 10,
        combat: defence::Settings {
            mark_darkness: true,
            protection: "Melee".into(),
            slayer_targets: [13599].into(),
            ..Default::default()
        },
        ..Default::default()
    };
    for (style, set) in [("Melee", melee), ("Magic", mage)] {
        for (slot, id) in set.into_iter().enumerate() {
            if slot != 10 {
                o.gear_overrides
                    .insert(format!("{style}:{slot}"), id.unwrap_or(0));
            }
        }
    }
    (s, o)
}
#[test]
fn a_small_boot_switch_can_be_removed_without_undressing_the_magic_phase() {
    let (s, o) = fixture();
    let limits = Tolerances {
        dps_loss: 10,
        damage_taken: 0,
        prayer_drain: 0,
    };
    let result = switch_pruning::search(&s, &o, limits);
    let full = &result.candidates[result.best.unwrap()];
    let reduced = &result.candidates[result.compact.unwrap()];
    assert!(result.removed.contains(&4097), "{:?}", result.removed);
    assert!(reduced.budget.carried_gear < full.budget.carried_gear);
    assert!(reduced.budget.max_pair < full.budget.max_pair);
    for phase in &reduced.budget.plan.phases {
        assert_eq!(phase.equipment[10], Some(11840));
    }
    assert!(efficient::qualifies(reduced, full, limits));
    assert!(reduced.budget.prep_blockers.is_empty());
    assert!(reduced.budget.plan.inventory.iter().any(|p| p.id == 2552));
    assert!(
        result
            .trials
            .iter()
            .any(|t| t.item == 4097 && t.within_limits)
    );
}
#[test]
fn applying_reproduces_the_measured_pool_pins_and_original_benchmark() {
    let (s, mut o) = fixture();
    o.max_switches = Some(0);
    let original_key = efficient::cache_key(&s, &o);
    let result = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    let reduced = &result.candidates[result.compact.unwrap()];
    let applied = reduced.budget.options.clone();
    assert_eq!(efficient::cache_key(&s, &applied), original_key);
    let p = planner::plan(&s, &applied);
    assert_eq!(p.equipment, reduced.budget.plan.equipment);
    assert_eq!(p.inventory, reduced.budget.plan.inventory);
    assert_eq!(
        p.phases.iter().map(|p| &p.equipment).collect::<Vec<_>>(),
        reduced
            .budget
            .plan
            .phases
            .iter()
            .map(|p| &p.equipment)
            .collect::<Vec<_>>()
    );
    let again = switch_pruning::search(&s, &applied, Tolerances::from_options(&o));
    assert_eq!(
        again.candidates[again.best.unwrap()].budget.plan.equipment,
        result.candidates[result.best.unwrap()]
            .budget
            .plan
            .equipment
    );
}
#[test]
fn manual_pins_survive_removal_and_invalid_saved_pools_are_rejected() {
    let (s, mut o) = fixture();
    o.gear_overrides.insert("Magic:10".into(), 4097);
    o.gear_overrides.insert("Melee:10".into(), 11840);
    let result = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    assert!(result.removed.is_empty());
    assert!(setups::validate_options(&o).is_ok());
    o.switch_pool = Some([999999].into());
    assert!(setups::validate_options(&o).is_err());
    o.switch_pool = Some([4097, 11840].into());
    o.switch_pins.insert("Magic:100".into(), 4097);
    assert!(setups::validate_options(&o).is_err());
}
#[test]
fn successive_removals_keep_the_original_loss_budget_and_real_supplies() {
    let s = account::demo();
    let o = Options {
        switch_tolerance: 10,
        damage_tolerance: 10,
        prayer_tolerance: 10,
        max_switches: Some(0),
        ..Default::default()
    };
    let limits = Tolerances::from_options(&o);
    let result = switch_pruning::search(&s, &o, limits);
    let full = &result.candidates[result.best.unwrap()];
    let reduced = &result.candidates[result.compact.unwrap()];
    assert!(efficient::qualifies(reduced, full, limits));
    assert!(reduced.budget.max_inventory_space <= 28);
    assert!(reduced.budget.prep_blockers.is_empty());
    let rebuilt = tradeoffs::budget(&s, &reduced.budget.options, reduced.budget.cap);
    assert_eq!(rebuilt.plan.inventory, reduced.budget.plan.inventory);
    assert!(
        reduced
            .budget
            .plan
            .inventory
            .iter()
            .all(|p| p.quantity <= s.available(false).get(&p.id).copied().unwrap_or(0))
    );
}

#[test]
fn private_transfer_and_named_setup_preserve_the_reduced_pool() {
    let (s, mut o) = fixture();
    o.switch_pool = Some(
        s.available(false)
            .keys()
            .copied()
            .filter(|id| planner::gear(*id).is_some() && *id != 4097)
            .collect(),
    );
    o.switch_pins.insert("Magic:10".into(), 11840);
    let mut library = setups::Library::default();
    library.save("Reduced TD", &o).unwrap();
    let saved = setups::Saved {
        version: 1,
        snapshot: s,
        options: o.clone(),
        pins: Default::default(),
        last_setup_fragment: None,
        setups: library,
        budget_caps: [2, 7],
        death_risk: Default::default(),
    };
    let loaded =
        ironbound::account_transfer::parse(&ironbound::account_transfer::fragment(&saved).unwrap())
            .unwrap();
    assert_eq!(loaded.options.switch_pool, o.switch_pool);
    assert_eq!(loaded.options.switch_pins, o.switch_pins);
    assert_eq!(loaded.setups.0[0].options.switch_pool, o.switch_pool);
}

#[test]
fn an_owned_shared_piece_outside_the_full_outfit_is_considered() {
    let (mut s, mut o) = fixture();
    o.gear_overrides.remove("Melee:7");
    o.gear_overrides.remove("Magic:7");
    s.members[0].bank.as_mut().unwrap().extend([542, 1]);
    let result = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    assert!(
        result
            .trials
            .iter()
            .any(|t| t.label == "Shared Legs" && t.replacement.contains("Monk")),
        "{:?}",
        result.trials
    );
    let chosen = &result.candidates[result.compact.unwrap()];
    assert!(
        result.added.contains(&542),
        "added {:?}, removed {:?}",
        result.added,
        result.removed
    );
    assert!(
        chosen
            .budget
            .plan
            .phases
            .iter()
            .all(|p| p.equipment[7] == Some(542))
    );
    assert!(efficient::qualifies(
        chosen,
        &result.candidates[result.best.unwrap()],
        Tolerances::from_options(&o)
    ));
}
#[test]
fn complete_owned_void_is_compared_jointly_and_exclusion_is_respected() {
    let (mut s, mut o) = fixture();
    o.combat.slayer_targets.clear();
    for style in ["Melee", "Magic"] {
        for slot in [0, 4, 7, 9] {
            o.gear_overrides.remove(&format!("{style}:{slot}"));
        }
    }
    s.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([11665, 1, 11663, 1, 8839, 1, 8840, 1, 8842, 1]);
    let result = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    assert!(
        result
            .trials
            .iter()
            .any(|t| t.label.contains("Void armour")),
        "{:?}",
        result.trials
    );
    o.exclude_void = true;
    let excluded = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    assert!(
        excluded
            .trials
            .iter()
            .all(|t| !t.label.contains("Void armour"))
    );
    let selected = &excluded.candidates[excluded.compact.unwrap()].budget.plan;
    assert!(
        selected
            .equipment
            .iter()
            .flatten()
            .all(|id| !ironbound::rules::void_piece(*id))
    );
}

#[test]
fn rejected_weapon_trials_do_not_display_incomparable_dps() {
    let (s, mut o) = fixture();
    o.gear_overrides.retain(|key, _| !key.ends_with(":3"));
    let result = switch_pruning::search(&s, &o, Tolerances::from_options(&o));
    // Cross-style weapon substitutions cannot complete the same workload.
    // Their rejected rows must not advertise a spurious negative DPS loss.
    let invalid_weapons: Vec<_> = result
        .trials
        .iter()
        .filter(|t| t.round == 0 && [19675, 4864].contains(&t.item))
        .collect();
    assert!(!invalid_weapons.is_empty());
    assert!(
        invalid_weapons
            .iter()
            .all(|t| !t.within_limits && t.dps_loss.is_none() && t.changes.is_none())
    );
}
