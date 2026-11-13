use ironbound::{
    account, efficient,
    planner::{self, Options},
    setups, share,
};

#[test]
fn old_options_and_links_keep_the_three_percent_default() {
    let options: Options = serde_json::from_str("{}").unwrap();
    assert_eq!(options.switch_tolerance, 3);
    assert_eq!((options.damage_tolerance, options.prayer_tolerance), (0, 0));
    let snapshot = account::demo();
    let plan = planner::plan(&snapshot, &options);
    let setup = share::Setup::from_plan(&plan, &options);
    let value = serde_json::to_value(&setup).unwrap();
    assert!(value.get("switch_tolerance").is_none());
    let restored = share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(restored.options().switch_tolerance, 3);
}
#[test]
fn tolerance_survives_sharing_without_a_manual_switch_ceiling_or_bank() {
    let snapshot = account::demo();
    let options = Options::default();
    let plan = planner::plan(&snapshot, &options);
    for tolerance in [0, 5, 10] {
        let o = Options {
            switch_tolerance: tolerance,
            damage_tolerance: 4,
            prayer_tolerance: 7,
            ..options.clone()
        };
        let setup = share::Setup::from_plan(&plan, &o);
        assert!(setup.constraints.is_none());
        let restored = share::Setup::parse(&setup.fragment().unwrap()).unwrap();
        assert_eq!(restored.options().switch_tolerance, tolerance);
        assert_eq!(
            (
                restored.options().damage_tolerance,
                restored.options().prayer_tolerance
            ),
            (4, 7)
        );
        let text = serde_json::to_string(&setup).unwrap();
        for private in ["bank", "members", "level_overrides", "selected", "snapshot"] {
            assert!(!text.contains(&format!("\"{private}\"")));
        }
        let local: Options = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(local.switch_tolerance, tolerance);
        assert_eq!((local.damage_tolerance, local.prayer_tolerance), (4, 7));
    }
}
#[test]
fn malformed_tolerances_are_rejected_by_backup_and_share_validation() {
    let snapshot = account::demo();
    let mut options = Options::default();
    let plan = planner::plan(&snapshot, &options);
    options.switch_tolerance = efficient::MAX_TOLERANCE_PERCENT + 1;
    assert!(setups::validate_options(&options).is_err());
    assert!(share::Setup::from_plan(&plan, &options).validate().is_err());
    assert!(serde_json::from_str::<Options>(r#"{"switch_tolerance":-1}"#).is_err());
}

#[test]
fn changing_tolerance_alone_preserves_the_loadout_and_preparation_checks() {
    let snapshot = account::demo();
    let mut options = Options::default();
    let before = planner::plan(&snapshot, &options);
    let token = ironbound::preparation::token(&snapshot, &options);
    options.switch_tolerance = 10;
    options.damage_tolerance = 5;
    options.prayer_tolerance = 7;
    let after = planner::plan(&snapshot, &options);
    assert_eq!(before.equipment, after.equipment);
    assert_eq!(before.inventory, after.inventory);
    assert_eq!(token, ironbound::preparation::token(&snapshot, &options));
}

#[test]
fn legacy_shared_allowance_is_dps_only_not_permission_for_more_damage_or_drain() {
    let o: Options = serde_json::from_str(r#"{"switch_tolerance":10}"#).unwrap();
    assert_eq!(
        (o.switch_tolerance, o.damage_tolerance, o.prayer_tolerance),
        (10, 0, 0)
    );
    let snapshot = account::demo();
    let plan = planner::plan(&snapshot, &o);
    let mut json = serde_json::to_value(share::Setup::from_plan(&plan, &o)).unwrap();
    json.as_object_mut().unwrap().remove("damage_tolerance");
    json.as_object_mut().unwrap().remove("prayer_tolerance");
    let restored: share::Setup = serde_json::from_value(json).unwrap();
    assert_eq!(
        (
            restored.options().switch_tolerance,
            restored.options().damage_tolerance,
            restored.options().prayer_tolerance
        ),
        (10, 0, 0)
    );
    for field in ["damage_tolerance", "prayer_tolerance"] {
        let mut bad = serde_json::to_value(&o).unwrap();
        bad[field] = 11.into();
        assert!(setups::validate_options(&serde_json::from_value(bad).unwrap()).is_err());
        let mut bad = serde_json::to_value(&restored).unwrap();
        bad[field] = 11.into();
        assert!(
            serde_json::from_value::<share::Setup>(bad)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
