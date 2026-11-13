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
    assert!(html.contains("sailing-own:cannon:5"));
    assert!(html.contains("Already aboard (not in import)"));
    assert!(html.contains("sailing-loaded-31914"));
    assert!(html.contains("sailing:cannon2:5"));
}
