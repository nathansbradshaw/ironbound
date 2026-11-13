use ironbound::{
    account,
    death_risk::{self, Input, Scenario, Settings},
    planner::{self, Options, Packed},
    setups::{Library, Saved},
};
use std::collections::BTreeMap;
fn plan() -> planner::Plan {
    let mut p = planner::plan(&account::demo(), &Options::default());
    p.equipment = vec![None; 14];
    p.inventory.clear();
    p.phases.clear();
    p.loaded_ammo = None;
    p
}
fn input(s: &mut Settings, id: i32, protection: u64, loss: u64) {
    s.items.insert(
        id,
        Input {
            protection: Some(protection),
            loss: Some(loss),
            ..Default::default()
        },
    );
}
fn unit(id: i32) -> u64 {
    death_risk::values().items[&id].market.unwrap()
}
#[test]
fn totals_supplies_runes_and_full_ammo_quantity_after_three_individual_keeps() {
    let mut p = plan();
    p.equipment[3] = Some(4151);
    p.equipment[4] = Some(1127);
    p.equipment[10] = Some(11840);
    p.equipment[13] = Some(9144);
    p.ammo_shots = 100;
    p.inventory = vec![
        Packed {
            id: 1215,
            quantity: 1,
            purpose: "spare".into(),
        },
        Packed {
            id: 385,
            quantity: 10,
            purpose: "food".into(),
        },
        Packed {
            id: 560,
            quantity: 200,
            purpose: "runes".into(),
        },
    ];
    let mut s = Settings::default();
    s.bind(&p);
    for (id, protect, loss) in [
        (4151, 1000, 999),
        (1127, 900, 999),
        (11840, 800, 999),
        (1215, 100, 40),
        (385, 1, 25),
        (560, 1, 3),
        (9144, 1, 2),
    ] {
        input(&mut s, id, protect, loss);
    }
    let r = death_risk::report(&p, &s);
    assert!(r.complete, "{:?}", r.issues);
    assert_eq!(r.known_gp, 40 + 250 + 600 + 200);
    assert_eq!(
        r.rows
            .iter()
            .filter(|r| r.kept > 0)
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![4151, 1127, 11840]
    );
}
#[test]
fn a_large_coin_stack_keeps_three_coins_not_the_whole_stack() {
    let mut p = plan();
    p.inventory = vec![Packed {
        id: 995,
        quantity: 100_000,
        purpose: "cash".into(),
    }];
    let r = death_risk::report(&p, &Settings::default());
    assert!(r.complete);
    assert_eq!(r.rows[0].kept, 3);
    assert_eq!(r.known_gp, 99_997);
}
#[test]
fn phases_do_not_duplicate_the_carried_pool_or_extra_copies() {
    let mut p = plan();
    p.equipment[3] = Some(4151);
    p.inventory = vec![Packed {
        id: 4151,
        quantity: 1,
        purpose: "spare copy".into(),
    }];
    p.phases = vec![planner::Phase {
        name: "preview".into(),
        style: "Melee".into(),
        equipment: p.equipment.clone(),
        note: String::new(),
    }];
    let mut s = Settings {
        scenario: Scenario::None,
        ..Default::default()
    };
    s.bind(&p);
    input(&mut s, 4151, 100, 123);
    let r = death_risk::report(&p, &s);
    assert_eq!(r.rows[0].quantity, 2);
    assert_eq!(r.known_gp, 246);
}
#[test]
fn unknown_values_are_not_a_zero_risk_total() {
    let mut p = plan();
    p.inventory = vec![Packed {
        id: 999999,
        quantity: 1,
        purpose: "unknown".into(),
    }];
    let r = death_risk::report(&p, &Settings::default());
    assert!(!r.complete);
    assert!(!r.kept_complete);
    let html = death_risk::render(&p, &Settings::default(), true, true);
    assert!(html.contains("Known subtotal · incomplete"));
    assert!(!html.contains("Total GP at risk"));
}
#[test]
fn ether_is_lost_even_when_the_weapon_is_kept_and_missing_counts_stay_unknown() {
    let mut p = plan();
    p.equipment[3] = Some(22545);
    let mut s = Settings::default();
    s.bind(&p);
    input(&mut s, 22545, 1_000_000, 1_000_000);
    assert!(!death_risk::report(&p, &s).complete);
    s.items.get_mut(&22545).unwrap().charges = Some(1500);
    let r = death_risk::report(&p, &s);
    assert!(r.complete, "{:?}", r.issues);
    assert_eq!(r.rows[0].kept, 1);
    assert_eq!(r.known_gp, unit(21820) * 1500);
}
#[test]
fn kept_trident_retains_charges_but_unkept_trident_loses_the_entered_contents() {
    let mut p = plan();
    p.equipment[3] = Some(11907);
    let mut s = Settings::default();
    s.bind(&p);
    input(&mut s, 11907, 1000, 500);
    let r = death_risk::report(&p, &s);
    assert!(r.complete);
    assert_eq!(r.known_gp, 0);
    s.scenario = Scenario::None;
    assert!(!death_risk::report(&p, &s).complete);
    s.items.get_mut(&11907).unwrap().charges = Some(10);
    let r = death_risk::report(&p, &s);
    assert!(r.complete, "{:?}", r.issues);
    assert_eq!(
        r.known_gp,
        500 + 10 * (unit(560) + unit(562) + 5 * unit(554) + 10)
    );
    // A partially depleted formerly-full trident cannot retain the full
    // tradeable variant's automatic protection value.
    p.equipment[3] = Some(11905);
    let mut partial = Settings::default();
    partial.bind(&p);
    partial.items.entry(11905).or_default().charges = Some(100);
    let r = death_risk::report(&p, &partial);
    assert!(!r.kept_complete);
    assert_eq!(r.rows[0].protection, None);
    partial.manual_kept = true;
    partial.kept.insert(11905, 1);
    assert!(death_risk::report(&p, &partial).complete);
    partial.scenario = Scenario::None;
    let r = death_risk::report(&p, &partial);
    assert!(r.complete);
    assert_eq!(
        r.known_gp,
        unit(11908) + 100 * (unit(560) + unit(562) + 5 * unit(554) + 10)
    );
}
#[test]
fn loaded_darts_use_actual_declared_count_and_do_not_double_count_planned_loading() {
    let mut p = plan();
    p.equipment[3] = Some(12926);
    p.loaded_ammo = Some(Packed {
        id: 11230,
        quantity: 9999,
        purpose: "planning budget, not actual contents".into(),
    });
    let mut s = Settings {
        scenario: Scenario::None,
        ..Default::default()
    };
    s.bind(&p);
    input(&mut s, 12926, 10_000, 1000);
    let v = s.items.get_mut(&12926).unwrap();
    v.charges = Some(100);
    v.loaded = Some(200);
    let r = death_risk::report(&p, &s);
    assert!(r.complete, "{:?}", r.issues);
    assert_eq!(r.known_gp, 1000 + unit(12934) * 100 + unit(11230) * 200);
    s.scenario = Scenario::Three;
    let r = death_risk::report(&p, &s);
    assert!(r.complete);
    assert_eq!(r.known_gp, 0);
}
#[test]
fn manual_keeps_resolve_unpriced_protection_and_ties_without_inventing_values() {
    let mut p = plan();
    p.equipment[3] = Some(4151);
    p.equipment[4] = Some(1127);
    p.equipment[10] = Some(11840);
    p.inventory = vec![Packed {
        id: 1215,
        quantity: 1,
        purpose: "spare".into(),
    }];
    let mut s = Settings::default();
    s.bind(&p);
    for id in [4151, 1127, 11840, 1215] {
        input(&mut s, id, 100, 5);
    }
    assert!(!death_risk::report(&p, &s).kept_complete);
    s.manual_kept = true;
    s.kept = [(4151, 1), (1127, 1), (11840, 1)].into_iter().collect();
    let r = death_risk::report(&p, &s);
    assert!(r.complete);
    assert_eq!(r.known_gp, 5);
    s.kept.remove(&11840);
    assert!(!death_risk::report(&p, &s).complete);
}
#[test]
fn changing_packed_quantity_expires_charge_and_kept_declarations() {
    let mut p = plan();
    p.equipment[3] = Some(22545);
    let mut s = Settings::default();
    s.bind(&p);
    input(&mut s, 22545, 1000, 1000);
    s.items.get_mut(&22545).unwrap().charges = Some(1500);
    assert!(death_risk::report(&p, &s).complete);
    p.inventory.push(Packed {
        id: 385,
        quantity: 1,
        purpose: "food".into(),
    });
    let r = death_risk::report(&p, &s);
    assert!(!r.complete);
    assert!(r.issues.iter().any(|s| s.contains("Packed items changed")));
    s.bind(&p);
    assert!(s.items.is_empty());
}
#[test]
fn unreviewed_charges_require_a_declared_loss_even_if_the_item_is_kept() {
    let mut p = plan();
    p.equipment[3] = Some(27275);
    let mut s = Settings::default();
    s.bind(&p);
    input(&mut s, 27275, 1000, 1000);
    assert!(!death_risk::report(&p, &s).complete);
    s.items.get_mut(&27275).unwrap().charge_loss = Some(0);
    let r = death_risk::report(&p, &s);
    assert!(r.complete, "{:?}", r.issues);
}
#[test]
fn barrows_protection_is_the_broken_value_and_replacement_is_separate() {
    let data = death_risk::values();
    assert_eq!(data.items[&4712].protect, data.items[&4872].protect);
    assert_eq!(data.items[&4869].protect, data.items[&4872].protect);
    assert!(data.items[&4869].market.is_some());
}
#[test]
fn risk_inputs_roundtrip_privately_and_do_not_enter_a_public_setup() {
    let p = planner::plan(&account::demo(), &Options::default());
    let mut risk = Settings::default();
    risk.bind(&p);
    risk.other_gp = 123456789;
    let saved = Saved {
        version: 1,
        snapshot: account::demo(),
        options: Options::default(),
        pins: BTreeMap::new(),
        last_setup_fragment: None,
        setups: Library::default(),
        budget_caps: [4, 8],
        death_risk: risk,
    };
    let raw = serde_json::to_string(&saved).unwrap();
    assert_eq!(Saved::parse(&raw).unwrap().death_risk.other_gp, 123456789);
    let public =
        serde_json::to_string(&ironbound::share::Setup::from_plan(&p, &saved.options)).unwrap();
    assert!(!public.contains("death_risk"));
    assert!(!public.contains("123456789"));
    let mut old: serde_json::Value = serde_json::from_str(&raw).unwrap();
    old.as_object_mut().unwrap().remove("death_risk");
    assert_eq!(
        Saved::parse(&old.to_string()).unwrap().death_risk.other_gp,
        0
    );
}
#[test]
fn scenario_changes_and_never_kept_containers_are_respected() {
    let mut p = plan();
    p.equipment[3] = Some(4151);
    p.inventory = vec![Packed {
        id: 12791,
        quantity: 1,
        purpose: "pouch".into(),
    }];
    let mut s = Settings::default();
    s.bind(&p);
    input(&mut s, 4151, 100, 500);
    input(&mut s, 12791, 10000, 1000);
    let r = death_risk::report(&p, &s);
    assert_eq!(r.rows.iter().find(|r| r.id == 12791).unwrap().kept, 0);
    assert_eq!(r.known_gp, 1000);
    s.scenario = Scenario::HighRisk;
    assert_eq!(death_risk::report(&p, &s).known_gp, 1500);
}
#[test]
fn empty_input_is_unknown_while_a_declared_zero_is_preserved() {
    let mut s = Settings::default();
    assert!(s.set("charge_loss", 22545, "0"));
    assert_eq!(s.items[&22545].charge_loss, Some(0));
    assert!(s.set("charge_loss", 22545, ""));
    assert_eq!(s.items[&22545].charge_loss, None);
    assert!(!s.set("charges", 22545, "-1"));
    assert!(!s.set("loss", 22545, "1000000000001"));
    assert!(s.valid());
}

#[test]
fn public_snapshot_does_not_interpret_absent_prices_as_free() {
    let data = death_risk::values();
    let zero: Vec<_> = data
        .items
        .iter()
        .filter(|(_, v)| v.market == Some(0) || v.protect == Some(0))
        .map(|(id, _)| *id)
        .collect();
    assert!(zero.is_empty(), "zero-valued rows: {zero:?}");
}
#[test]
fn charge_rules_have_valid_nonzero_components_and_no_duplicate_hosts() {
    let mut ids = std::collections::BTreeSet::new();
    for rule in &death_risk::death_rules().charges {
        assert!(rule.max > 0);
        for id in &rule.ids {
            assert!(ids.insert(*id));
        }
        assert!(
            rule.components
                .iter()
                .all(|(id, n, d)| *id > 0 && *n > 0 && *d > 0)
        );
    }
}
