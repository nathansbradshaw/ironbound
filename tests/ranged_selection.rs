use ironbound::{
    account, combat,
    defence::Settings,
    export,
    planner::{self, Boss, Options},
    rules::{self, RangedType},
    setups::{Library, Saved},
    share::Setup,
};
fn stock() -> account::Snapshot {
    let mut bank = account::demo();
    bank.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([12926, 1, 811, 500, 861, 1, 892, 500]);
    bank
}
fn options(kind: RangedType) -> Options {
    let mut o = Options {
        boss: Boss::Barrows,
        ..Default::default()
    };
    o.combat.ranged_type = kind;
    o.combat.ammo_shots = 250;
    o
}
fn ranged(p: &planner::Plan) -> &[Option<i32>] {
    &p.phases
        .iter()
        .find(|s| s.style == "Ranged")
        .unwrap()
        .equipment
}
fn quantity(p: &planner::Plan, id: i32) -> u32 {
    p.inventory
        .iter()
        .filter(|p| p.id == id)
        .map(|p| p.quantity)
        .sum()
}
#[test]
fn each_category_rebuilds_the_weapon_and_compatible_ammunition() {
    for (kind, expected) in [
        (RangedType::Light, 12926),
        (RangedType::Standard, 861),
        (RangedType::Heavy, 9185),
    ] {
        let o = options(kind);
        let p = planner::plan(&stock(), &o);
        let set = ranged(&p);
        assert!(p.exportable(), "{kind:?}: {:?}", p.missing);
        assert_eq!(set[3], Some(expected));
        assert!(kind.permits_weapon(expected));
        if kind == RangedType::Light {
            assert_eq!(set[13], None);
            let loaded = p.loaded_ammo.as_ref().unwrap();
            assert_eq!((loaded.id, loaded.quantity), (811, 250));
            assert_eq!(quantity(&p, 811), 0, "loaded darts are outside the bag");
            assert_eq!(quantity(&p, 9242), 0, "old bolts must not remain packed");
        } else {
            let ammo = set[13].unwrap();
            assert!(rules::ammo_ids(expected).contains(&ammo));
            assert_eq!(quantity(&p, ammo), 250);
            assert!(p.loaded_ammo.is_none());
        }
        let json = export::inventory_setup(&p, &o, "Test").unwrap();
        assert!(json.contains(&format!("Ranged ammunition type: {}", kind.label())));
    }
}
#[test]
fn category_is_a_constraint_for_recommended_custom_and_budgeted_searches() {
    let bank = stock();
    for kind in [RangedType::Light, RangedType::Standard, RangedType::Heavy] {
        for recommended in [true, false] {
            let mut o = options(kind);
            o.recommended = Some(recommended);
            o.max_switches = Some(8);
            o.switch_slots = 12;
            let p = planner::plan(&bank, &o);
            assert!(
                p.exportable(),
                "{kind:?}, recommended={recommended}: {:?}",
                p.missing
            );
            assert!(kind.permits_weapon(ranged(&p)[3].unwrap()));
            assert!(p.optimization.as_ref().is_some_and(|r| r.max_changes <= 8));
        }
    }
}
#[test]
fn missing_weapons_ammo_or_levels_cannot_fall_back_to_another_category() {
    let mut o = options(RangedType::Light);
    let p = planner::plan(&account::demo(), &o);
    assert_eq!(ranged(&p)[3], None);
    assert!(!p.exportable());
    assert!(
        p.missing.iter().any(|m| m.contains("Light ranged weapon")),
        "{:?}",
        p.missing
    );
    let mut bank = stock();
    o.combat.ammo_shots = 501;
    let p = planner::plan(&bank, &o);
    assert_eq!(ranged(&p)[3], Some(12926));
    assert!(!p.exportable());
    assert!(
        p.missing
            .iter()
            .any(|m| m.contains("501") && m.contains("darts")),
        "{:?}",
        p.missing
    );
    o.combat.ammo_shots = 250;
    bank.members[0].level_overrides.insert("Ranged".into(), 74);
    let p = planner::plan(&bank, &o);
    assert_eq!(ranged(&p)[3], None);
    assert!(!p.exportable());
    bank.members[0].level_overrides.insert("Ranged".into(), 76);
    o.excluded.insert(12926);
    assert_eq!(ranged(&planner::plan(&bank, &o))[3], None);
    o = options(RangedType::Standard);
    o.combat.ammo_shots = 501;
    let p = planner::plan(&bank, &o);
    assert_eq!(ranged(&p)[3], Some(861));
    assert_eq!(ranged(&p)[13], None);
    assert!(!p.exportable());
    assert!(export::inventory_setup(&p, &o, "Test").is_err());
}
#[test]
fn conflicting_pinned_weapon_stays_visible_and_blocks_export() {
    let bank = stock();
    let mut o = options(RangedType::Standard);
    o.gear_overrides.insert("Ranged:3".into(), 9185);
    let p = planner::plan(&bank, &o);
    assert_eq!(ranged(&p)[3], Some(9185));
    assert!(!p.exportable());
    assert!(
        p.missing
            .iter()
            .any(|m| m.contains("does not match Standard"))
    );
    assert!(
        combat::calculate_configured(
            bank.selected().unwrap(),
            ranged(&p),
            "Ranged",
            planner::monster(1672).unwrap(),
            "",
            0,
            &o.combat
        )
        .unwrap_err()
        .contains("Standard")
    );
    assert!(!rules::casting_supported_configured(
        bank.selected().unwrap(),
        ranged(&p),
        &bank.available(false),
        150,
        &o.combat
    ));
}
#[test]
fn auto_restores_an_unrestricted_owned_choice() {
    let bank = stock();
    let mut o = options(RangedType::Standard);
    assert_eq!(ranged(&planner::plan(&bank, &o))[3], Some(861));
    o.combat.ranged_type = RangedType::Auto;
    let p = planner::plan(&bank, &o);
    let automatic = planner::plan(&bank, &options(RangedType::Auto));
    assert_eq!(ranged(&p), ranged(&automatic));
    assert!(p.exportable());
    assert!(
        RangedType::Auto.permits_weapon(12926)
            && RangedType::Auto.permits_weapon(861)
            && RangedType::Auto.permits_weapon(9185)
    );
}
#[test]
fn choice_and_real_ammo_round_trip_saved_and_shared_setups() {
    for kind in [RangedType::Light, RangedType::Standard, RangedType::Heavy] {
        let snapshot = stock();
        let o = options(kind);
        let p = planner::plan(&snapshot, &o);
        let mut saved = Saved {
            version: 1,
            snapshot,
            options: o.clone(),
            pins: Default::default(),
            last_setup_fragment: None,
            setups: Library::default(),
            budget_caps: [4, 8],
        };
        saved.setups.save("Ranged plan", &o).unwrap();
        let loaded = Saved::parse(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(loaded.options.combat.ranged_type, kind);
        assert_eq!(loaded.setups.0[0].options.combat.ranged_type, kind);
        assert_eq!(
            ranged(&planner::plan(&loaded.snapshot, &loaded.options)),
            ranged(&p)
        );
        let setup = Setup::from_plan(&p, &o);
        let decoded = Setup::parse(&setup.fragment().unwrap()).unwrap();
        let mut conflicting = setup.clone();
        conflicting.combat.ranged_type = if kind == RangedType::Heavy {
            RangedType::Light
        } else {
            RangedType::Heavy
        };
        assert!(conflicting.validate().is_err());
        assert_eq!(decoded.combat.ranged_type, kind);
        assert_eq!(ranged(&decoded.plan()), ranged(&p));
        if kind == RangedType::Light {
            assert_eq!(decoded.plan().loaded_ammo.unwrap().quantity, 250);
        } else {
            assert_eq!(quantity(&decoded.plan(), ranged(&p)[13].unwrap()), 250);
        }
    }
    assert_eq!(
        serde_json::from_str::<Settings>("{}").unwrap().ranged_type,
        RangedType::Auto
    );
    assert!(serde_json::from_str::<Settings>(r#"{"ranged_type":"unknown"}"#).is_err());
}
#[test]
fn charged_crystal_bow_does_not_pack_unrelated_arrows() {
    let mut bank = stock();
    bank.members[0].bank.as_mut().unwrap().extend([23983, 1]);
    let mut o = options(RangedType::Standard);
    o.gear_overrides.insert("Ranged:3".into(), 23983);
    let p = planner::plan(&bank, &o);
    assert!(p.exportable(), "{:?}", p.missing);
    assert_eq!(ranged(&p)[13], None);
    assert_eq!(quantity(&p, 892), 0);
    assert!(p.loaded_ammo.is_none());
}
