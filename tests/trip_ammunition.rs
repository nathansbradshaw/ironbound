use ironbound::{
    account,
    defence::Settings,
    export,
    planner::{self, Boss, Options, Target},
    rules, share,
};

fn ranged(shots: u32, weapon: i32, ammo: i32, owned: i64) -> (account::Snapshot, Options) {
    let mut snapshot = account::demo();
    snapshot.members[0].bank = Some(vec![weapon as i64, 1, ammo as i64, owned, 8007, 1]);
    let mut options = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 415,
            style: "Ranged".into(),
        }],
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    options.combat.ammo_shots = shots;
    (snapshot, options)
}

#[test]
fn small_budget_controls_owned_selection_worn_quantity_and_all_exports() {
    let (snapshot, mut options) = ranged(30, 9185, 9242, 30);
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!((plan.ammo_shots, plan.equipment[13]), (30, Some(9242)));
    let json: serde_json::Value =
        serde_json::from_str(&export::inventory_setup(&plan, &options, "private").unwrap())
            .unwrap();
    assert_eq!(json["setup"]["eq"][13]["q"], 30);
    assert!(
        export::bank_tags(&plan, &options)
            .unwrap()
            .split(',')
            .any(|id| id == "9242")
    );
    let shared =
        share::Setup::parse(&share::Setup::from_plan(&plan, &options).fragment().unwrap()).unwrap();
    assert_eq!(shared.plan().ammo_shots, 30);
    assert_eq!(shared.options().combat.ammo_shots, 30);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &export::inventory_setup(&shared.plan(), &shared.options(), "private").unwrap()
        )
        .unwrap()["setup"]["eq"][13]["q"],
        30
    );
    options.combat.ammo_shots = 31;
    let missing = planner::plan(&snapshot, &options);
    assert!(!missing.exportable());
    assert!(missing.missing.iter().any(|m| m.contains("31 compatible")));
}

#[test]
fn phase_previews_conserve_nondefault_ammunition_and_do_not_change_starting_exports() {
    let snapshot = account::demo();
    let mut options = Options::default();
    options.combat.ammo_shots = 37;
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    let ranged = plan
        .phases
        .iter()
        .position(|phase| phase.style == "Ranged")
        .unwrap();
    let ammo = plan.phases[ranged].equipment[13].unwrap();
    assert_eq!(
        plan.inventory
            .iter()
            .filter(|p| p.id == ammo)
            .map(|p| p.quantity)
            .sum::<u32>(),
        37
    );
    let before = export::inventory_setup(&plan, &options, "private").unwrap();
    let preview = plan.preview_inventory(Some(ranged));
    assert_eq!(
        preview
            .iter()
            .filter(|p| p.id == ammo)
            .map(|p| p.quantity)
            .sum::<u32>(),
        0
    );
    assert_eq!(
        before,
        export::inventory_setup(&plan, &options, "private").unwrap()
    );
    let shared =
        share::Setup::parse(&share::Setup::from_plan(&plan, &options).fragment().unwrap()).unwrap();
    let shared_preview = shared.plan().preview_inventory(Some(ranged));
    assert_eq!(
        shared_preview
            .iter()
            .map(|p| (p.id, p.quantity))
            .collect::<Vec<_>>(),
        preview
            .iter()
            .map(|p| (p.id, p.quantity))
            .collect::<Vec<_>>()
    );
}

#[test]
fn blowpipe_loading_and_stock_reservations_use_the_chosen_budget() {
    let (snapshot, mut options) = ranged(25, 12926, 11230, 25);
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(plan.loaded_ammo.as_ref().unwrap().quantity, 25);
    assert!(!plan.inventory.iter().any(|p| p.id == 11230));
    let shared =
        share::Setup::parse(&share::Setup::from_plan(&plan, &options).fragment().unwrap()).unwrap();
    assert_eq!(shared.plan().loaded_ammo.unwrap().quantity, 25);
    options.inventory_overrides.insert(27, 11230);
    options.inventory_quantities.insert(27, 1);
    let conflict = planner::plan(&snapshot, &options);
    assert!(
        !conflict.exportable(),
        "Loaded darts cannot also be packed from the same stock"
    );
    assert!(
        conflict
            .missing
            .iter()
            .any(|m| m.contains("25 needed to load"))
    );
}

#[test]
fn legacy_settings_keep_one_hundred_and_invalid_budgets_are_rejected() {
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(old.ammo_shots, rules::DEFAULT_LOADED_SHOTS);
    for shots in [0, 10_001, u32::MAX] {
        assert!(
            !Settings {
                ammo_shots: shots,
                ..Default::default()
            }
            .valid()
        );
    }
    assert_eq!(planner::equipped_quantity(13, 9242, 500), 500);
    assert_eq!(planner::equipped_quantity(13, 20220, 500), 1); // Blessing/non-projectile item.
}

#[test]
fn increasing_a_shared_budget_cannot_invent_missing_switch_ammunition() {
    let mut options = Options::default();
    options.combat.ammo_shots = 37;
    let plan = planner::plan(&account::demo(), &options);
    let mut shared = share::Setup::from_plan(&plan, &options);
    shared.validate().unwrap();
    shared.combat.ammo_shots = 38;
    assert!(shared.validate().is_err());
}
