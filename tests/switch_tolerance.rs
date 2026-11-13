use ironbound::{
    account, efficient,
    planner::{self, Options},
    setups, share,
};

#[test]
fn old_options_and_links_keep_the_three_percent_default() {
    let options: Options = serde_json::from_str("{}").unwrap();
    assert_eq!(options.switch_tolerance, 3);
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
            ..options.clone()
        };
        let setup = share::Setup::from_plan(&plan, &o);
        assert!(setup.constraints.is_none());
        let restored = share::Setup::parse(&setup.fragment().unwrap()).unwrap();
        assert_eq!(restored.options().switch_tolerance, tolerance);
        let text = serde_json::to_string(&setup).unwrap();
        for private in ["bank", "members", "level_overrides", "selected", "snapshot"] {
            assert!(!text.contains(&format!("\"{private}\"")));
        }
        let local: Options = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(local.switch_tolerance, tolerance);
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
    let after = planner::plan(&snapshot, &options);
    assert_eq!(before.equipment, after.equipment);
    assert_eq!(before.inventory, after.inventory);
    assert_eq!(token, ironbound::preparation::token(&snapshot, &options));
}
