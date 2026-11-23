use ironbound::{
    account::{self, Snapshot},
    defence,
    planner::{self, Boss, Options, Target},
    sailing::{Assignment, Boat, Settings},
    sailing_combat::{self, Model},
    sailing_owned::{self, BALL_IDS, KIT_IDS},
    sailing_recommendation as recommend, share,
};
fn snapshot() -> Snapshot {
    let mut s = account::demo();
    let m = &mut s.members[0];
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m.bank.as_mut().unwrap().extend([
        BALL_IDS[0] as i64,
        1000,
        BALL_IDS[5] as i64,
        1000,
        KIT_IDS[0] as i64,
        30,
        KIT_IDS[6] as i64,
        1,
    ]);
    s
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 15194,
            style: "Ranged".into(),
        }],
        sailing: Settings {
            boat: Boat::Sloop,
            hull: 6,
            keel: 6,
            cargo: Some(0),
            cannon: Some(5),
            cannon_count: 2,
            cannonball: 5,
            player_role: 2,
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
            cannonballs: 0,
            repair_kits: 0,
            trip_kills: 5,
            minutes: 20,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn cannon_formulas_distinguish_player_and_privateering_and_exclude_three_slots() {
    let s = snapshot();
    let m = &s.members[0];
    let t = planner::monster(15194).unwrap();
    let mut set = vec![None; 14];
    let settings = defence::Settings::default();
    let p = sailing_combat::cannon(m, &set, t, 0, 0, None, false, &settings).unwrap();
    assert_eq!(p.attack_roll, 107 * 136);
    assert_eq!(p.raw_max, 22);
    set[3] = Some(20997);
    set[5] = Some(12817);
    set[13] = Some(11212);
    let q = sailing_combat::cannon(m, &set, t, 0, 0, None, false, &settings).unwrap();
    assert_eq!(p.raw_max, q.raw_max);
    assert_eq!(p.attack_roll, q.attack_roll);
    let crew = sailing_combat::cannon(m, &set, t, 5, 5, Some(4), false, &settings).unwrap();
    assert_eq!(crew.attack_roll, 56 * 291);
    assert_eq!(crew.raw_max, 25);
    let mut low = m.clone();
    low.level_overrides.insert("Ranged".into(), 1);
    low.level_overrides.insert("Sailing".into(), 1);
    let q =
        sailing_combat::cannon(&low, &vec![None; 14], t, 5, 5, Some(4), true, &settings).unwrap();
    assert_eq!(q.attack_roll, crew.attack_roll);
    assert_eq!(q.raw_max, 26);
    assert!(sailing_combat::cannon(m, &set, t, 5, 5, Some(0), false, &settings).is_err());
}
#[test]
fn cannon_flat_armour_is_applied_to_each_roll_and_keel_can_prevent_all_damage() {
    let s = snapshot();
    let t = planner::monster(15576).unwrap();
    let r = sailing_combat::cannon(
        &s.members[0],
        &vec![None; 14],
        t,
        6,
        6,
        Some(4),
        false,
        &Default::default(),
    )
    .unwrap();
    assert_eq!(r.raw_max, 32);
    let armour = t.defensive["flat_armour"] as u32;
    assert_eq!(r.max_hit, 32 - armour);
    let expected = (0..=32u32)
        .map(|n| f64::from(n.saturating_sub(armour)))
        .sum::<f64>()
        / 33.0
        * r.accuracy
        / 4.2;
    assert!((r.dps - expected).abs() < 1e-12);
    assert!((r.distribution.iter().map(|(_, p)| p).sum::<f64>() - 1.0).abs() < 1e-12);
    let b = options().sailing;
    let h = sailing_combat::incoming(&b, planner::monster(15194).unwrap()).unwrap();
    assert_eq!(h.max_hit, 0);
    assert_eq!(h.dps, 0.0);
}
#[test]
fn two_guns_share_finite_stock_and_repeated_trials_are_deterministic() {
    let mut s = snapshot();
    s.members[0].bank = Some(vec![BALL_IDS[5] as i64, 1]);
    let o = options();
    let mut plan = planner::plan(&s, &o);
    plan.inventory.clear();
    let mut model = Model::build(
        &s,
        &o,
        &plan,
        &vec![None; 14],
        "Ranged",
        planner::monster(15194).unwrap(),
    )
    .unwrap();
    model.target_hp = 10000;
    let a = model.run(64);
    assert_eq!(a.depleted, 64);
    assert_eq!(a.mean_cannonballs, 1.0);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&model.run(64)).unwrap()
    );
    model.boat.attractor = true;
    let a = model.run(256);
    assert!(a.mean_cannonballs > 1.0);
    assert!(a.mean_cannonballs < 4.0);
    assert_eq!(a.depleted, 256);
}
#[test]
fn stock_loading_exclusions_and_cargo_do_not_create_supplies() {
    let mut s = snapshot();
    let mut o = options();
    s.members[0].sailing_owned.loaded.insert(BALL_IDS[5], 5);
    assert_eq!(sailing_owned::stock(&s, &o)[&BALL_IDS[5]], 1005);
    o.excluded.insert(BALL_IDS[5]);
    assert!(!sailing_owned::stock(&s, &o).contains_key(&BALL_IDS[5]));
    let mut plan = planner::plan(&s, &o);
    plan.inventory.clear();
    assert!(
        Model::build(
            &s,
            &o,
            &plan,
            &vec![None; 14],
            "Ranged",
            planner::monster(15194).unwrap()
        )
        .is_err()
    );
    o.excluded.clear();
    o.sailing.repair_tier = 0;
    o.sailing.repair_kits = 100;
    let model = Model::build(
        &s,
        &o,
        &plan,
        &vec![None; 14],
        "Ranged",
        planner::monster(15194).unwrap(),
    )
    .unwrap();
    assert_eq!(model.kit_count, 30);
    o.sailing.cargo_reserved = 10;
    assert!(
        Model::build(
            &s,
            &o,
            &plan,
            &vec![None; 14],
            "Ranged",
            planner::monster(15194).unwrap()
        )
        .is_err()
    );
    let mut unowned = vec![None; 14];
    unowned[0] = Some(11826);
    assert!(
        Model::build(
            &s,
            &options(),
            &plan,
            &unowned,
            "Ranged",
            planner::monster(15194).unwrap()
        )
        .unwrap_err()
        .contains("absent")
    );
}
#[test]
fn recommendation_uses_only_confirmed_parts_crew_gear_and_supply_and_applies() {
    let mut s = snapshot();
    let mut o = options();
    o.sailing.cannon_count = 1;
    o.sailing.crew[1] = Default::default();
    assert!(recommend::recommend(&s, &o).is_err());
    s.members[0].sailing_owned.confirm(&o.sailing);
    let r = recommend::recommend(&s, &o).unwrap();
    assert!(s.members[0].sailing_owned.permits(&r.boat));
    assert_eq!(r.boat.cannon_count, 1);
    assert!(r.boat.crew.iter().all(|c| c.person.is_none_or(|p| p == 4)));
    let stock = sailing_owned::stock(&s, &o);
    assert!(
        r.equipment
            .iter()
            .flatten()
            .all(|id| stock.get(id).copied().unwrap_or(0) > 0)
    );
    assert!(r.report.completed > 0);
    let next = recommend::apply(&s, &o, &r).unwrap();
    assert_eq!(next.sailing, r.boat);
    let p = planner::plan(&s, &next);
    assert_eq!(p.equipment, r.equipment);
    s.members[0].sailing_owned.boats = [false; 3];
    assert!(recommend::apply(&s, &o, &r).is_err());
}
#[test]
fn fleet_backups_validate_but_public_links_never_include_private_availability() {
    let mut s = snapshot();
    let o = options();
    let plan = planner::plan(&s, &o);
    let before = share::Setup::from_plan(&plan, &o);
    s.members[0].sailing_owned.confirm(&o.sailing);
    s.members[0].sailing_owned.loaded.insert(BALL_IDS[0], 12345);
    let after = share::Setup::from_plan(&plan, &o);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    let raw = serde_json::to_string(&s).unwrap();
    assert_eq!(
        Snapshot::parse(&raw, String::new()).unwrap().members[0].sailing_owned,
        s.members[0].sailing_owned
    );
    s.members[0].sailing_owned.loaded.insert(999, 1);
    assert!(Snapshot::parse(&serde_json::to_string(&s).unwrap(), String::new()).is_err());
    assert_eq!(
        serde_json::from_str::<Settings>("{}").unwrap(),
        Settings::default()
    );
}
#[test]
fn private_fleet_and_scenario_controls_use_icon_buttons_without_dropdowns() {
    let s = snapshot();
    let o = options();
    let html = ironbound::sailing_ui::controls(&s, &o, true)
        + &ironbound::sailing_ui::scenario(&o.sailing);
    assert!(!html.contains("<select"));
    assert!(html.contains("boat-add"));
    assert!(html.contains("Supplies already aboard"));
    assert!(html.contains("sailing-loaded-31914"));
    assert!(html.contains("sailing:kit:5"));
}
#[test]
fn player_boost_prayer_and_elite_void_follow_nested_floors_without_stance_bonus() {
    let s = snapshot();
    let mut set = vec![None; 14];
    for (slot, id) in [(0, 11664), (4, 13072), (7, 13073), (9, 8842)] {
        set[slot] = Some(id);
    }
    let mut settings = defence::Settings::default();
    settings.boosts.ranged = "Ranging potion".into();
    settings.offensive = "Rigour".into();
    let r = sailing_combat::cannon(
        &s.members[0],
        &set,
        planner::monster(15194).unwrap(),
        0,
        0,
        None,
        false,
        &settings,
    )
    .unwrap();
    // Visible 112: Rigour gives floor(112*1.20)+8=142 accuracy,
    // floor(112*1.23)+8=145 strength; Void floors to 156 / 163.
    assert_eq!(r.attack_roll, 156 * 136);
    assert_eq!(r.raw_max, 33);
    settings.ranged_stance = "Rapid".into();
    let q = sailing_combat::cannon(
        &s.members[0],
        &set,
        planner::monster(15194).unwrap(),
        0,
        0,
        None,
        false,
        &settings,
    )
    .unwrap();
    assert_eq!(q.attack_roll, r.attack_roll);
    assert_eq!(q.raw_max, r.raw_max);
}
#[test]
fn repairs_consume_only_loaded_kits_and_never_resurrect_a_sunk_hull() {
    let s = snapshot();
    let mut o = options();
    o.sailing.repair_kits = 3;
    o.sailing.repair_tier = 0;
    o.sailing.repair_workbench = true;
    o.sailing.crew[2] = Assignment {
        person: Some(0),
        role: 2,
    };
    let mut plan = planner::plan(&s, &o);
    plan.inventory.clear();
    let mut model = Model::build(
        &s,
        &o,
        &plan,
        &vec![None; 14],
        "Ranged",
        planner::monster(15194).unwrap(),
    )
    .unwrap();
    model.target_hp = 100000;
    model.incoming.distribution = vec![(10, 1.0)];
    model.incoming.ticks = 4.0;
    let r = model.run(64);
    assert_eq!(r.sunk, 64);
    assert_eq!(r.mean_kits, 3.0);
    assert_eq!(r.p95_kits, 3);
    model.incoming.distribution = vec![(model.boat.hp(), 1.0)];
    let r = model.run(64);
    assert_eq!(r.sunk, 64);
    assert_eq!(r.mean_kits, 0.0);
}
#[test]
fn charged_weapons_require_confirmed_loaded_stock_and_deplete() {
    let s = snapshot();
    let mut o = options();
    o.targets[0].style = "Magic".into();
    o.sailing.player_role = 0;
    o.sailing.cannon = None;
    o.sailing.cannon_count = 1;
    o.sailing.crew = Default::default();
    let mut plan = planner::plan(&s, &o);
    plan.inventory.clear();
    let mut set = vec![None; 14];
    set[3] = Some(11905);
    assert!(
        Model::build(
            &s,
            &o,
            &plan,
            &set,
            "Magic",
            planner::monster(15194).unwrap()
        )
        .unwrap_err()
        .contains("charges")
    );
    o.sailing.powered_charges = 1;
    let mut model = Model::build(
        &s,
        &o,
        &plan,
        &set,
        "Magic",
        planner::monster(15194).unwrap(),
    )
    .unwrap();
    model.target_hp = 100000;
    let r = model.run(64);
    assert_eq!(r.depleted, 64);
    assert_eq!(r.mean_kills, 0.0);
}

#[test]
fn choosing_a_loaded_dart_does_not_invent_unconfirmed_or_excluded_ammunition() {
    let mut s = snapshot();
    s.members[0].bank = Some(vec![12926, 1]);
    let mut o = options();
    o.sailing.player_role = 0;
    o.sailing.cannon = None;
    o.sailing.cannon_count = 1;
    o.sailing.crew = Default::default();
    o.sailing.powered_charges = 100;
    o.combat.loaded_dart = Some(11230);
    o.combat.ammo_shots = 100;
    let mut set = vec![None; 14];
    set[3] = Some(12926);
    let mut plan = planner::plan(&s, &o);
    plan.inventory.clear();
    let target = planner::monster(15194).unwrap();
    let model = Model::build(&s, &o, &plan, &set, "Ranged", target).unwrap();
    assert_eq!(model.stock.get(&11230).copied().unwrap_or(0), 0);
    assert_eq!(model.run(32).depleted, 32);
    o.combat.loaded_ammo_confirmed = true;
    let model = Model::build(&s, &o, &plan, &set, "Ranged", target).unwrap();
    assert_eq!(model.stock[&11230], 100);
    o.excluded.insert(11230);
    let model = Model::build(&s, &o, &plan, &set, "Ranged", target).unwrap();
    assert_eq!(model.stock.get(&11230).copied().unwrap_or(0), 0);
}

#[test]
fn saved_boats_enforce_five_unique_names_and_allow_upgrades_without_changing_other_boats() {
    let mut f = sailing_owned::Fleet::default();
    let b = Settings::default();
    for i in 0..5 {
        f.save_boat(None, &format!("Boat {i}"), &b).unwrap();
    }
    let mut snapshot = snapshot();
    snapshot.members[0].sailing_owned = f.clone();
    let mut options = options();
    options.sailing = b.clone();
    let html = ironbound::sailing_ui::controls(&snapshot, &options, false);
    assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1);
    let before = f.clone();
    assert!(f.save_boat(None, "Sixth", &b).is_err());
    assert_eq!(f, before);
    assert!(f.save_boat(Some(0), "boat 1", &b).is_err());
    assert!(f.save_boat(Some(0), "bad\nname", &b).is_err());
    let mut upgraded = b.clone();
    upgraded.hull = 6;
    f.save_boat(Some(2), "Upgraded", &upgraded).unwrap();
    assert_eq!(f.profiles.len(), 5);
    assert_eq!(f.profiles[2].settings.hull, 6);
    assert_eq!(&f.profiles[..2], &before.profiles[..2]);
    assert_eq!(&f.profiles[3..], &before.profiles[3..]);
    let mut invalid = upgraded;
    invalid.crew[0] = Assignment {
        person: Some(4),
        role: 3,
    };
    assert!(f.save_boat(Some(2), "Invalid", &invalid).is_err());
    let mut too_many = f.clone();
    too_many.profiles.push(f.profiles[0].clone());
    assert!(!too_many.valid());
    assert!(
        serde_json::from_str::<sailing_owned::Fleet>("{}")
            .unwrap()
            .profiles
            .is_empty()
    );
}
#[test]
fn switching_saved_boats_retains_trip_goal_and_does_not_mutate_the_profile() {
    let mut f = sailing_owned::Fleet::default();
    f.save_boat(None, "Fishing skiff", &Settings::default())
        .unwrap();
    let p = f.profiles[0].clone();
    let mut trip = options().sailing;
    trip.trip_kills = 42;
    trip.minutes = 31;
    trip.repair_kits = 7;
    trip.powered_charges = 123;
    let selected = p.for_trip(&trip);
    assert_eq!(selected.boat, Boat::Skiff);
    assert_eq!(
        (
            selected.trip_kills,
            selected.minutes,
            selected.repair_kits,
            selected.powered_charges
        ),
        (42, 31, 7, 123)
    );
    assert_eq!(f.profiles[0], p);
    assert!(p.matches(&selected));
}
#[test]
fn recommendations_compare_real_boats_without_combining_fittings_and_reject_stale_upgrades() {
    let mut s = snapshot();
    let o = options();
    let mut a = o.sailing.clone();
    a.keel = 0;
    let mut b = o.sailing.clone();
    b.hull = 0;
    let f = &mut s.members[0].sailing_owned;
    f.save_boat(None, "Strong hull", &a).unwrap();
    f.save_boat(None, "Strong keel", &b).unwrap();
    // Legacy loose-part confirmations cannot override the actual saved fittings.
    f.confirm(&o.sailing);
    let r = recommend::recommend(&s, &o).unwrap();
    assert!(
        s.members[0]
            .sailing_owned
            .profiles
            .iter()
            .any(|p| p.matches(&r.boat))
    );
    assert_ne!((r.boat.hull, r.boat.keel), (6, 6));
    assert!(r.saved_boat.is_some());
    recommend::apply(&s, &o, &r).unwrap();
    let chosen = s.members[0]
        .sailing_owned
        .profiles
        .iter()
        .position(|p| Some(&p.name) == r.saved_boat.as_ref())
        .unwrap();
    s.members[0].sailing_owned.profiles[chosen].settings.hull = 1;
    assert!(recommend::apply(&s, &o, &r).is_err());
}
#[test]
fn boat_fleet_survives_private_transfer_and_backup_but_stays_out_of_public_setup_links() {
    let mut s = snapshot();
    let o = options();
    for i in 0..5 {
        s.members[0]
            .sailing_owned
            .save_boat(None, &format!("Private boat {i}"), &o.sailing)
            .unwrap();
    }
    let mut future = o.sailing.clone();
    future.hull = 5;
    s.members[0].sailing_owned.save_upgrade(0, &future).unwrap();
    let saved = ironbound::setups::Saved {
        version: 1,
        snapshot: s.clone(),
        options: o.clone(),
        pins: Default::default(),
        last_setup_fragment: None,
        budget_caps: [2, 6],
        setups: Default::default(),
        death_risk: Default::default(),
    };
    let parsed = ironbound::setups::Saved::parse(&serde_json::to_string(&saved).unwrap()).unwrap();
    assert_eq!(
        parsed.snapshot.members[0].sailing_owned,
        s.members[0].sailing_owned
    );
    let restored =
        ironbound::account_transfer::parse(&ironbound::account_transfer::fragment(&saved).unwrap())
            .unwrap();
    assert_eq!(
        restored.snapshot.members[0].sailing_owned,
        s.members[0].sailing_owned
    );
    let public =
        serde_json::to_string(&share::Setup::from_plan(&planner::plan(&s, &o), &o)).unwrap();
    assert!(!public.contains("Private boat"));
    assert!(!public.contains("profiles"));
    let mut invalid = s;
    let extra = invalid.members[0].sailing_owned.profiles[0].clone();
    invalid.members[0].sailing_owned.profiles.push(extra);
    assert!(Snapshot::parse(&serde_json::to_string(&invalid).unwrap(), String::new()).is_err());
}
