//! Comparisons use public synthetic banks; no private account fixtures.
use ironbound::{
    account, combat, defence,
    planner::{self, Boss, Options, Target},
    tradeoffs,
};
fn bank() -> account::Snapshot {
    let mut s = account::demo();
    s.members.truncate(1);
    s.members[0].inventory = None;
    s.members[0].equipment = None;
    s.members[0].rune_pouch = None;
    for skill in account::SKILLS {
        s.members[0].level_overrides.insert(skill.into(), 99);
    }
    s.members[0].bank = Some(
        [
            4151, 11905, 10551, 10462, 10466, 10454, 10448, 10472, 3844, 12601, 8007,
        ]
        .into_iter()
        .flat_map(|id| [id, 1])
        .collect(),
    );
    s
}
fn options() -> Options {
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 100013,
                style: "Melee".into(),
            },
            Target {
                id: 100012,
                style: "Magic".into(),
            },
        ],
        food: 0,
        prayer_potions: 0,
        focus_style: "Melee".into(),
        switch_slots: 12,
        ..Default::default()
    };
    o.combat.protection = "Melee".into();
    for slot in [0, 1, 2, 5, 7, 12] {
        o.gear_overrides.insert(format!("Melee:{slot}"), 0);
    }
    o.gear_overrides.insert("Melee:4".into(), 10551);
    o
}
#[test]
fn four_vs_eight_reports_actual_prayer_defence_and_inventory_tradeoffs() {
    let s = bank();
    let c = tradeoffs::compare(&s, &options(), 4, 8);
    assert!(c.left.feasible, "{:?}", c.left.prep_blockers);
    assert!(c.right.feasible, "{:?}", c.right.prep_blockers);
    assert!(c.left.max_pair <= 4 && c.right.max_pair <= 8);
    let left = c.left.windows.iter().find(|w| w.style == "Magic").unwrap();
    let right = c.right.windows.iter().find(|w| w.style == "Magic").unwrap();
    assert_eq!(left.metrics.dps, right.metrics.dps);
    assert!(right.metrics.prayer_per_minute < left.metrics.prayer_per_minute);
    assert_eq!(left.metrics.defence.len(), 5);
    assert_eq!(right.metrics.defence.len(), 5);
    assert_ne!(left.metrics.defence, right.metrics.defence);
    assert!(c.right.carried_gear > c.left.carried_gear);
    assert!(c.right.inventory_space > c.left.inventory_space);
    assert!(!c.added_gear.is_empty());
    assert_eq!(c.right.inventory_space, c.right.plan.inventory_slots());
    assert!(c.right.max_inventory_space >= c.right.inventory_space);
    assert!(
        right
            .metrics
            .incoming
            .iter()
            .all(|i| i.dps.is_none() && i.reason.is_some())
    );
}
#[test]
fn impossible_zero_budget_withholds_unconstrained_fallback_measurements() {
    let c = tradeoffs::compare(&bank(), &options(), 0, 8);
    assert!(!c.left.feasible);
    assert!(c.right.feasible);
    assert!(!c.left.prep_blockers.is_empty());
    assert!(c.added_gear.is_empty() && c.removed_gear.is_empty());
    assert!(
        c.left.windows.iter().all(|w| w.metrics.dps.is_none()
            && w.metrics.dps_reason.as_deref().unwrap().contains("budget"))
    );
    assert!(
        c.left
            .windows
            .iter()
            .flat_map(|w| &w.metrics.incoming)
            .all(|i| i.dps.is_none() && i.reason.is_some())
    );
}
#[test]
fn comparisons_hold_manual_combat_and_supply_settings_fixed_and_repack_both_sides() {
    let mut o = options();
    o.food = 2;
    o.prayer_potions = 1;
    o.combat.boosts.magic = "Magic potion".into();
    o.combat.reductions.insert(
        14176,
        ironbound::modifiers::Reductions {
            dwh: 1,
            ..Default::default()
        },
    );
    o.phase_priorities.insert(100012, 3);
    o.inventory_overrides.insert(0, 385);
    o.inventory_quantities.insert(0, 5);
    let original = serde_json::to_value(&o).unwrap();
    let c = tradeoffs::compare(&bank(), &o, 4, 8);
    assert_eq!(serde_json::to_value(&o).unwrap(), original);
    for b in [&c.left, &c.right] {
        assert_eq!(b.options.gear_overrides, o.gear_overrides);
        assert_eq!(b.options.combat, o.combat);
        assert_eq!(b.options.phase_priorities, o.phase_priorities);
        assert_eq!(b.options.switch_slots, o.switch_slots);
        assert_eq!(b.options.food, 2);
        assert_eq!(b.options.prayer_potions, 1);
        assert!(
            b.options.inventory_overrides.is_empty() && b.options.inventory_quantities.is_empty()
        );
        assert_eq!(b.prep_blockers, b.plan.missing);
    }
}
#[test]
fn measure_uses_shared_filtered_stock_and_base_equipment_levels() {
    let mut s = bank();
    s.members[0].bank = Some(vec![12926, 1, 811, 100]);
    let mut shared = s.members[0].clone();
    shared.name = "@SHARED".into();
    shared.bank = Some(vec![11230, 100]);
    s.members.push(shared);
    let mole = planner::monsters()
        .iter()
        .find(|m| m.name == "Giant Mole")
        .unwrap();
    let mut set = vec![None; 14];
    set[3] = Some(12926);
    let mut o = options();
    o.combat.protection = "None".into();
    let own = tradeoffs::measure(&s, &o, &set, "Ranged", mole.id);
    o.shared = true;
    let group = tradeoffs::measure(&s, &o, &set, "Ranged", mole.id);
    assert!(group.dps.unwrap() > own.dps.unwrap());
    o.excluded.insert(11230);
    assert_eq!(
        tradeoffs::measure(&s, &o, &set, "Ranged", mole.id).dps,
        own.dps
    );
    s.members[0].level_overrides.insert("Ranged".into(), 74);
    o.combat.boosts.ranged = "Ranging potion".into();
    let locked = tradeoffs::measure(&s, &o, &set, "Ranged", mole.id);
    assert!(locked.dps.is_none());
    assert!(locked.defence.is_empty());
    assert!(
        locked
            .incoming
            .iter()
            .all(|i| i.dps.is_none() && i.reason.is_some())
    );
    assert!(locked.dps_reason.unwrap().contains("Base equipment"));
}
#[test]
fn every_incoming_scenario_is_measured_or_explicitly_unknown() {
    let s = bank();
    let o = options();
    let mut set = vec![None; 14];
    set[3] = Some(4151);
    let yama = planner::monster(14176).unwrap();
    let metrics = tradeoffs::measure(&s, &o, &set, "Slash", yama.id);
    assert_eq!(metrics.incoming.len(), defence::attacks(yama).len());
    assert!(
        metrics
            .incoming
            .iter()
            .all(|i| i.dps.is_none() && i.reason.as_ref().is_some_and(|r| !r.is_empty()))
    );
    let unknown = tradeoffs::measure(&s, &o, &set, "Slash", 999999);
    assert!(unknown.dps.is_none());
    assert!(unknown.incoming[0].dps.is_none());
    set[4] = Some(999999);
    let unverified = tradeoffs::measure(&s, &o, &set, "Slash", yama.id);
    assert!(unverified.defence.is_empty());
    let empty = tradeoffs::measure(&s, &o, &vec![None; 14], "Slash", yama.id);
    assert_eq!(empty.defence.len(), 5);
    assert!(empty.defence.values().all(|b| *b == 0));
}
#[test]
fn toa_composite_akkha_uses_the_actual_carried_set_for_each_style() {
    let s = account::demo();
    let o = Options {
        boss: Boss::Toa,
        max_switches: Some(8),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    let p = planner::plan(&s, &o);
    for style in ["Magic", "Ranged", "Melee"] {
        let set = tradeoffs::set_for_window(&p, &o, 11789, style).expect("carried Akkha style");
        assert!(
            set[3]
                .and_then(ironbound::rules::weapon)
                .unwrap()
                .permits(style)
        );
        let measured = tradeoffs::measure(&s, &o, set, style, 11789);
        let m = s.selected().unwrap();
        let owned = s.available(false);
        let expected = combat::calculate_configured(
            m,
            set,
            style,
            planner::monster(11789).unwrap(),
            ironbound::rules::spell_name_configured(m, set, &owned, &o.combat),
            o.raid_level,
            &o.combat,
        )
        .unwrap();
        assert_eq!(measured.dps, Some(expected.dps));
    }
}
#[test]
fn switch_caps_are_bounded_without_mutating_the_requested_options() {
    let o = options();
    let c = tradeoffs::compare(&bank(), &o, 4, 99);
    assert_eq!(c.right.cap, 11);
    assert!(!c.right.notes.is_empty());
    assert_eq!(o.max_switches, None);
}

#[test]
fn teleport_rings_use_inventory_space_without_becoming_switch_pieces() {
    let mut s = bank();
    let stock = s.members[0].bank.as_mut().unwrap();
    for pair in stock.chunks_exact_mut(2) {
        if pair[0] == 8007 {
            pair[0] = 2552;
        }
    }
    let c = tradeoffs::compare(&s, &options(), 4, 8);
    for budget in [&c.left, &c.right] {
        assert!(budget.plan.inventory.iter().any(|item| item.id == 2552));
        let switches = budget
            .plan
            .phases
            .iter()
            .flat_map(|phase| {
                phase.equipment.iter().enumerate().filter_map(|(slot, id)| {
                    id.filter(|id| budget.plan.equipment[slot] != Some(*id))
                })
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(budget.carried_gear, switches.len());
        assert!(!switches.contains(&2552));
        assert!(budget.inventory_space > budget.carried_gear);
    }
    assert!(!c.added_gear.contains(&2552) && !c.removed_gear.contains(&2552));
}

#[test]
fn raid_phases_without_registered_combat_windows_are_named_explicitly() {
    let s = account::demo();
    let o = Options {
        boss: Boss::Toa,
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    let c = tradeoffs::compare(&s, &o, 4, 8);
    assert!(
        c.right
            .notes
            .iter()
            .any(|note| note.contains("Wardens") && note.contains("not estimated"))
    );
}

#[test]
fn measured_damage_selects_each_targets_elemental_weakness() {
    // Same public air-staff/rune fixture as elemental_ranged::best_owned_spell.
    let mut s = bank();
    s.members[0].bank = Some(vec![1381, 1, 565, 150, 555, 1050, 557, 1050, 554, 1050]);
    let mut set = vec![None; 14];
    set[3] = Some(1381);
    let o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 5779,
                style: "Magic".into(),
            },
            Target {
                id: 8059,
                style: "Magic".into(),
            },
        ],
        ..Default::default()
    };
    let member = s.selected().unwrap();
    for (id, expected_spell) in [(5779, "Earth Wave"), (8059, "Fire Wave")] {
        let target = planner::monster(id).unwrap();
        let measured = tradeoffs::measure(&s, &o, &set, "Magic", id);
        let matching = combat::calculate_configured(
            member,
            &set,
            "Magic",
            target,
            expected_spell,
            0,
            &o.combat,
        )
        .unwrap();
        let generic_air =
            combat::calculate_configured(member, &set, "Magic", target, "Wind Wave", 0, &o.combat)
                .unwrap();
        assert_eq!(measured.dps, Some(matching.dps));
        assert!(
            measured.dps.unwrap() > generic_air.dps,
            "{expected_spell} should use the target's matching weakness"
        );
    }
}
