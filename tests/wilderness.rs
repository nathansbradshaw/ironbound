use ironbound::{
    account,
    bestiary::{self, Readiness},
    encounters,
    planner::{self, Options, Packed},
    wilderness::{self, Exposure, Kit},
};
fn entry(name: &str) -> &'static encounters::Entry {
    encounters::catalog()
        .iter()
        .find(|e| e.name == name)
        .unwrap()
}
#[test]
fn locations_include_dangerous_routes_and_do_not_confuse_the_archaeologists() {
    assert_eq!(wilderness::exposure("King Black Dragon"), Exposure::Route);
    assert_eq!(
        wilderness::exposure("Crazy Archaeologist"),
        Exposure::Combat
    );
    assert_eq!(
        wilderness::exposure("Deranged Archaeologist"),
        Exposure::None
    );
    assert_eq!(wilderness::exposure("Vorkath"), Exposure::None);
    for name in [
        "Artio",
        "Callisto",
        "Calvar'ion",
        "Vet'ion",
        "Spindel",
        "Venenatis",
        "Chaos Elemental",
        "Chaos Fanatic",
        "Scorpia",
    ] {
        assert_eq!(wilderness::exposure(name), Exposure::Combat);
    }
}
#[test]
fn unpriced_items_and_whole_ammo_stacks_consume_valuable_slots() {
    assert!(!wilderness::disposable(999999, 1, 10_000));
    let (&id, &value) = wilderness::prices()
        .prices
        .iter()
        .find(|(_, v)| **v > 0 && **v < 1000)
        .unwrap();
    assert!(wilderness::disposable(id, 1, 1000));
    assert!(!wilderness::disposable(id, 1001, 1000));
    assert!(value > 0);
}
#[test]
fn screened_kit_recalculates_damage_and_never_uses_four_valuable_items() {
    let snapshot = account::demo();
    let options = Options::default();
    let baseline = bestiary::assess_catalog(&snapshot, &options);
    let restricted = bestiary::assess_wilderness(&snapshot, &options, 10_000);
    let mut screened = 0;
    for ((entry, full), a) in encounters::catalog().iter().zip(&baseline).zip(&restricted) {
        if wilderness::exposure(&entry.name) == Exposure::None {
            assert_eq!(full.reason, a.reason);
            assert!(!a.wilderness);
            continue;
        }
        assert!(a.wilderness);
        if let Some(p) = &a.preview {
            screened += 1;
            assert!(p.kit.valid());
            assert_eq!(a.readiness, p.readiness);
            assert_eq!(p.full_readiness, full.readiness);
            assert!(a.reason.contains("3-item kit"));
            let count = p
                .equipment
                .iter()
                .flatten()
                .filter(|id| {
                    !wilderness::disposable(**id, wilderness::gear_quantity(**id, 100), 10_000)
                })
                .count();
            assert!(count <= 3, "{} has {count} valuables", entry.name);
            let mut constrained = options.clone();
            wilderness::apply(&mut constrained, entry, p);
            let plan = planner::plan(&snapshot, &constrained);
            assert!(
                wilderness::violations(&plan, &p.kit).is_empty(),
                "{}: {:?}",
                entry.name,
                plan.missing
            );
            assert_eq!(
                plan.equipment, p.equipment,
                "{} must use the screened gear",
                entry.name
            );
            let share = ironbound::share::Setup::from_plan(&plan, &constrained);
            if plan.exportable() {
                share.validate().unwrap();
                assert_eq!(share.options().wilderness, constrained.wilderness);
            }
        }
    }
    assert!(
        screened >= 5,
        "expected supported wilderness kits, got {screened}"
    );
}
#[test]
fn no_bank_or_levels_never_fabricates_a_limited_kit() {
    let mut snapshot = account::demo();
    snapshot.members[0].bank = None;
    snapshot.members[0].equipment = None;
    snapshot.members[0].inventory = None;
    assert!(
        wilderness::preview(
            &snapshot,
            &Options::default(),
            entry("Artio"),
            10_000,
            Readiness::Struggle
        )
        .is_none()
    );
    snapshot.members[0].skills = None;
    snapshot.members[0].level_overrides.clear();
    assert!(
        bestiary::assess_wilderness(&snapshot, &Options::default(), 10_000)
            .iter()
            .all(|a| a.preview.is_none())
    );
}
#[test]
fn manual_extra_gear_and_duplicate_valuables_are_caught_before_export() {
    let snapshot = account::demo();
    let mut plan = planner::plan(&snapshot, &Options::default());
    plan.equipment = vec![None; 14];
    plan.inventory.clear();
    let kit = Kit {
        allowed: [4151, 1127, 11840].into_iter().collect(),
        filler_cap: 1000,
        screened_shots: 100,
    };
    assert!(kit.valid());
    plan.equipment[3] = Some(4151);
    plan.equipment[4] = Some(1127);
    plan.equipment[10] = Some(11840);
    plan.inventory.push(Packed {
        id: 4151,
        quantity: 1,
        purpose: "extra weapon".into(),
    });
    assert!(
        wilderness::violations(&plan, &kit)
            .iter()
            .any(|s| s.contains("limit is 3"))
    );
    plan.inventory = vec![Packed {
        id: 11832,
        quantity: 1,
        purpose: "fourth valuable".into(),
    }];
    assert!(
        wilderness::violations(&plan, &kit)
            .iter()
            .any(|s| s.contains("outside the selected"))
    );
}
#[test]
fn invalid_kit_cannot_enter_saved_or_shared_setups() {
    let mut options = Options::default();
    options.wilderness = Some(Kit {
        allowed: [4151, 1127, 11840, 11832].into_iter().collect(),
        filler_cap: 1000,
        screened_shots: 100,
    });
    assert!(ironbound::setups::validate_options(&options).is_err());
    let mut options = Options::default();
    options.wilderness = Some(Kit {
        allowed: [4151].into_iter().collect(),
        filler_cap: 12345,
        screened_shots: 100,
    });
    assert!(ironbound::setups::validate_options(&options).is_err());
}
#[test]
fn old_saved_options_stay_unrestricted_and_price_missing_does_not_mean_free() {
    let options: Options = serde_json::from_str("{}").unwrap();
    assert!(options.wilderness.is_none());
    assert!(!wilderness::disposable(6570, 1, 50_000));
}

#[test]
fn changing_ammo_quantities_keeps_the_save_valid_but_rechecks_actual_risk() {
    let snapshot = account::demo();
    let mut plan = planner::plan(&snapshot, &Options::default());
    plan.equipment = vec![None; 14];
    plan.inventory.clear();
    let ammo = ironbound::rules::ammo_ids(9185)
        .iter()
        .copied()
        .find(|id| wilderness::disposable(*id, 100, 10_000))
        .expect("priced cheap bolts");
    let kit = Kit {
        allowed: [4151, 1127, 11840, ammo].into_iter().collect(),
        filler_cap: 10_000,
        screened_shots: 100,
    };
    assert!(kit.valid());
    let mut options = Options::default();
    options.wilderness = Some(kit.clone());
    options.combat.ammo_shots = 10_000;
    assert!(ironbound::setups::validate_options(&options).is_ok());
    plan.equipment[3] = Some(4151);
    plan.equipment[4] = Some(1127);
    plan.equipment[10] = Some(11840);
    plan.equipment[13] = Some(ammo);
    plan.ammo_shots = 10_000;
    assert!(
        wilderness::violations(&plan, &kit)
            .iter()
            .any(|s| s.contains("limit is 3"))
    );
}

#[test]
fn exporting_a_handcrafted_four_item_plan_is_rejected_even_without_cached_blockers() {
    let snapshot = account::demo();
    let mut plan = planner::plan(&snapshot, &Options::default());
    plan.missing.clear();
    plan.equipment = vec![None; 14];
    plan.equipment[3] = Some(4151);
    plan.equipment[4] = Some(1127);
    plan.equipment[10] = Some(11840);
    plan.inventory = vec![Packed {
        id: 4151,
        quantity: 1,
        purpose: "duplicate valuable".into(),
    }];
    let mut options = Options::default();
    options.wilderness = Some(Kit {
        allowed: [4151, 1127, 11840].into_iter().collect(),
        filler_cap: 1000,
        screened_shots: 100,
    });
    assert!(plan.exportable());
    assert!(
        ironbound::export::inventory_setup(&plan, &options, "")
            .unwrap_err()
            .contains("wilderness gear limit")
    );
    assert!(ironbound::export::bank_tags(&plan, &options).is_err());
}

#[test]
fn malformed_shared_equipment_is_rejected_before_risk_inspection() {
    let snapshot = account::demo();
    let plan = planner::plan(&snapshot, &Options::default());
    let mut shared = ironbound::share::Setup::from_plan(&plan, &Options::default());
    shared.wilderness = Some(Kit {
        allowed: [4151].into_iter().collect(),
        filler_cap: 1000,
        screened_shots: 100,
    });
    shared.equipment.clear();
    shared.combat.loaded_dart = Some(11230);
    assert!(shared.validate().is_err());
}
