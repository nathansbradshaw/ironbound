use ironbound::{
    account::{self, Member},
    bestiary, combat, defence, encounters, monster_rules,
    planner::{self, Boss, Options, Target},
    sailing::{self, Assignment, Boat, Settings},
    setups, share,
};
fn member(level: u32) -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m.level_overrides.insert("Sailing".into(), level);
    m
}
fn gunboat() -> Settings {
    Settings {
        boat: Boat::Sloop,
        hull: 5,
        keel: 5,
        cannon: Some(5),
        cannon_count: 2,
        cannonball: 5,
        crew: [
            Assignment {
                person: Some(4),
                role: 3,
            },
            Assignment {
                person: Some(5),
                role: 4,
            },
            Default::default(),
            Default::default(),
            Default::default(),
        ],
        cannonballs: 1000,
        ..Default::default()
    }
}
#[test]
fn all_twenty_one_sea_creatures_are_selectable_and_distinct_from_land_kraken() {
    let entries: Vec<_> = encounters::catalog()
        .iter()
        .filter(|e| e.category() == "Sailing")
        .collect();
    assert_eq!(entries.len(), 21);
    for id in sailing::SEA_IDS {
        let e = entries.iter().find(|e| e.targets[0].id == id).unwrap();
        assert!(e.targets[0].valid());
        assert_ne!(e.targets[0].style, "Melee");
        assert!(
            e.guide_url()
                .starts_with("https://oldschool.runescape.wiki/w/")
        );
    }
    assert!(!sailing::is_sea(494));
    assert!(bestiary::CATEGORY_FILTERS.contains(&"Sailing"));
}
#[test]
fn hull_and_keel_health_and_flat_reduction_use_boat_specific_tables() {
    let mut s = Settings::default();
    assert_eq!((s.hp(), s.armour(), s.cargo_capacity()), (80, 1, 30));
    s.boat = Boat::Sloop;
    s.hull = 6;
    s.keel = 6;
    s.cargo = Some(6);
    assert_eq!((s.hp(), s.armour(), s.cargo_capacity()), (360, 10, 240));
    s.boat = Boat::Raft;
    assert_eq!((s.hp(), s.armour()), (120, 0));
    let dolphin = planner::monster(15234).unwrap();
    let shark = planner::monster(15200).unwrap();
    assert_eq!(sailing::reduced_max(&s, dolphin), Some(2));
    s.boat = Boat::Skiff;
    s.keel = 1;
    assert_eq!(sailing::reduced_max(&s, dolphin), Some(0));
    s.keel = 5;
    assert_eq!(sailing::reduced_max(&s, shark), Some(7));
    assert_eq!(
        sailing::reduced_max(&s, planner::monster(494).unwrap()),
        None
    );
}
#[test]
fn crew_max_hits_use_sailing_privateering_and_rounding_not_worn_gear() {
    let mut boat = gunboat();
    // floor((99*(26+201+64)+320)/640)=45; floor(45*17/20)=38.
    assert_eq!(boat.crew_max(&member(99), 0).unwrap(), 38);
    boat.perildance = true; // floor(46*.85)=39.
    assert_eq!(boat.crew_max(&member(99), 0).unwrap(), 39);
    boat.visible_sailing = Some(112); // floor((112*291+320)/640)=51, +1; floor52*.85=44.
    assert_eq!(boat.crew_max(&member(99), 1).unwrap(), 44);
    let mut low_ranged = member(99);
    low_ranged.level_overrides.insert("Ranged".into(), 1);
    assert_eq!(boat.crew_max(&low_ranged, 1).unwrap(), 44);
    assert!(
        boat.crew_max(&member(79), 1)
            .unwrap_err()
            .contains("80 Sailing")
    );
}
#[test]
fn crew_and_player_cannot_double_operate_cannons_and_small_boats_have_real_limits() {
    let mut boat = gunboat();
    assert!(boat.conflicts().is_empty());
    boat.player_role = 1;
    assert!(
        boat.conflicts()
            .iter()
            .any(|s| s.contains("already has an operator"))
    );
    boat.player_role = 0;
    boat.crew[1].person = Some(4);
    assert!(
        boat.conflicts()
            .iter()
            .any(|s| s.contains("more than once"))
    );
    boat = gunboat();
    boat.boat = Boat::Skiff;
    assert!(boat.conflicts().iter().any(|s| s.contains("two cannons")));
    boat = gunboat();
    boat.crew[0].person = Some(0);
    assert!(
        boat.crew_max(&member(99), 0)
            .unwrap_err()
            .contains("Privateering")
    );
    boat = gunboat();
    boat.cargo = None;
    assert!(boat.conflicts().iter().any(|s| s.contains("cargo hold")));
    boat = gunboat();
    boat.cannonball = 6;
    assert!(!boat.ammo_compatible());
    assert!(boat.crew_max(&member(99), 0).is_err());
    let raft = Settings {
        boat: Boat::Raft,
        cannon: Some(0),
        ..Default::default()
    };
    assert!(raft.conflicts().iter().any(|s| s.contains("one facility")));
}
#[test]
fn boosted_sailing_does_not_unlock_extra_crew_capacity() {
    let boat = Settings {
        boat: Boat::Sloop,
        visible_sailing: Some(99),
        ..Default::default()
    };
    assert_eq!(boat.crew_limit(39), 0);
    assert_eq!(boat.crew_limit(40), 1);
    assert_eq!(boat.crew_limit(55), 2);
    assert_eq!(boat.crew_limit(70), 3);
    assert_eq!(boat.crew_limit(85), 4);
    assert_eq!(boat.crew_limit(95), 5);
    assert_eq!(Settings::default().crew_limit(99), 2);
}
#[test]
fn ammo_consumption_counts_operators_uptime_and_random_recovery_as_an_expectation() {
    let mut b = gunboat();
    b.minutes = 7;
    b.uptime = 50;
    // 420 seconds / 4.2 seconds * 2 guns * 50% = 100 shots.
    assert_eq!(b.ammo_budget(), (100.0, 100.0));
    b.attractor = true;
    assert_eq!(b.ammo_budget(), (100.0, 40.0));
    b.crew[1].role = 1;
    assert_eq!(b.ammo_budget(), (50.0, 20.0));
}
#[test]
fn sea_creatures_reject_ordinary_melee_and_personal_incoming_defence() {
    let m = member(99);
    let target = planner::monster(15200).unwrap();
    let mut set = vec![None; 14];
    set[3] = Some(4151);
    assert!(
        monster_rules::style_limitation(target.id, "Slash")
            .unwrap()
            .contains("Sea creatures")
    );
    assert!(combat::calculate(&m, &set, "Melee", target, "").is_err());
    let mut settings = defence::Settings::default();
    settings.protection = "Melee".into();
    assert!(
        defence::estimate(&m, &set, target, "Slash", &settings)
            .unwrap_err()
            .contains("boat")
    );
    set[3] = Some(9185);
    set[13] = Some(9144);
    let r = combat::calculate(&m, &set, "Ranged", target, "").unwrap();
    assert!(r.dps > 0.0);
    assert!(r.max_hit > 10, "No obsolete sea-creature damage penalty");
}
#[test]
fn invalid_boat_imports_are_rejected_and_browser_updates_are_atomic() {
    let mut b = Settings::default();
    let original = b.clone();
    for (key, value) in [
        ("hull", "9999"),
        ("person-0", "9999"),
        ("role-9", "1"),
        ("uptime", "101"),
        ("visible", "999"),
        ("minutes", "0"),
    ] {
        assert!(!b.update(key, value));
        assert_eq!(b, original);
    }
    assert!(b.update("boat", "Sloop"));
    assert!(b.update("keel", "6"));
    assert!(b.update("person-0", "4"));
    let bad: Settings = serde_json::from_value(serde_json::json!({"hull":99})).unwrap();
    assert!(!bad.valid());
    let o = Options {
        sailing: bad,
        ..Default::default()
    };
    assert!(setups::validate_options(&o).is_err());
    assert_eq!(
        serde_json::from_str::<Settings>("{}").unwrap(),
        Settings::default()
    );
}
#[test]
fn private_backups_named_setups_and_public_sailing_links_roundtrip_without_bank_data() {
    let snapshot = account::demo();
    let o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 15200,
            style: "Ranged".into(),
        }],
        sailing: gunboat(),
        ..Default::default()
    };
    let mut library = setups::Library::default();
    library.save("Sea hunt", &o).unwrap();
    library.validate().unwrap();
    let saved = setups::Saved {
        version: 1,
        snapshot: snapshot.clone(),
        options: o.clone(),
        pins: Default::default(),
        last_setup_fragment: None,
        setups: library,
        budget_caps: [4, 8],
        death_risk: Default::default(),
    };
    let restored = setups::Saved::parse(&serde_json::to_string(&saved).unwrap()).unwrap();
    assert_eq!(restored.options.sailing, o.sailing);
    let p = planner::plan(&snapshot, &o);
    let setup = share::Setup::from_plan(&p, &o);
    let text = serde_json::to_string(&setup).unwrap();
    assert!(text.contains("sailing"));
    assert!(!text.contains("members"));
    assert!(!text.contains("level_overrides"));
    let parsed = share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(parsed.options().sailing, o.sailing);
    let nonsea = share::Setup::from_plan(
        &p,
        &Options {
            sailing: o.sailing.clone(),
            ..Default::default()
        },
    );
    assert!(!serde_json::to_string(&nonsea).unwrap().contains("sailing"));
    let mut legacy = serde_json::to_value(setup).unwrap();
    legacy.as_object_mut().unwrap().remove("sailing");
    assert_eq!(
        serde_json::from_value::<share::Setup>(legacy)
            .unwrap()
            .sailing,
        Settings::default()
    );
}
#[test]
fn missing_sailing_does_not_borrow_combat_levels_or_rate_boat_readiness() {
    let mut m = member(99);
    m.level_overrides.remove("Sailing");
    assert!(
        Settings::default()
            .checks(&m)
            .iter()
            .any(|s| s.contains("Enter your Sailing"))
    );
    let snapshot = account::demo();
    let ratings = bestiary::assess_catalog(&snapshot, &Options::default());
    for (e, a) in encounters::catalog().iter().zip(ratings) {
        if e.category() == "Sailing" {
            assert_eq!(a.readiness, bestiary::Readiness::Unknown);
        }
    }
}

#[test]
fn shipyard_uses_osrs_parts_crew_boxes_and_no_dropdowns() {
    let b = gunboat();
    let targets = [Target {
        id: 15200,
        style: "Ranged".into(),
    }];
    for slot in [
        "hull", "keel", "cannon", "cargo", "ammo", "player", "person-0", "role-0",
    ] {
        let html = ironbound::sailing_view::render(&b, Some(&member(99)), &targets, slot, false);
        assert!(!html.contains("<select"), "{slot} reintroduced a dropdown");
        assert!(html.contains("ship-model ship-art-sloop"));
        assert!(html.contains("data-action=\"sailing-slot:hull\""));
        assert!(html.contains("Great white shark"));
        assert!(html.contains("ship-face-4"));
        assert!(html.contains("sailing:boat:Skiff"));
    }
    let html = ironbound::sailing_view::render(&b, None, &targets, "cannon", true);
    assert!(html.contains("<fieldset class=\"ship-controls\" disabled>"));
    for slot in ["person-5", "role-900", "<script>", "arbitrary"] {
        assert!(!ironbound::sailing_view::valid_slot(slot));
    }
}
