use ironbound::{
    account, combat,
    planner::{self, Boss, Options},
    simulation::{self, Attack, Encounter, Model, Settings},
};
use std::collections::BTreeMap;

fn hit(damage: i32) -> Attack {
    Attack {
        hits: combat::ResultStats {
            hit_distribution: vec![(damage, 1.0)],
            hp_dependent: None,
            dps: damage as f64 / 0.6,
            potential_healing_per_second: 0.0,
            accuracy: 1.0,
            max_hit: damage,
            attack_ticks: 1,
            expected_attack_ticks: 1.0,
            attack_roll: 1,
            defence_roll: 0,
            attack_type: "stab".into(),
            spell: String::new(),
            elemental: None,
            guaranteed_hits: Some(vec![(damage, 1.0)]),
        },
        weapon: 4151,
        runes: vec![],
        projectile: None,
        projectile_loss: None,
        charge: false,
        charge_on_damage: false,
        scales: false,
        prayer: 0.0,
    }
}
fn model(encounter: Encounter, damage: i32) -> Model {
    let count = if encounter == Encounter::Vorkath {
        2
    } else {
        6
    };
    Model {
        encounter,
        settings: Settings {
            trials: 16,
            trip_limit: 1,
            uptime: 100,
            incoming: 0,
            mistakes: 0,
            transition: 0,
            judge_travel: 0,
            flare_interval: 300,
            skull_ticks: 1,
            ..Default::default()
        },
        attacks: vec![hit(damage); count],
        max_hp: 99,
        max_prayer: 99,
        hp: if encounter == Encounter::Yama {
            vec![2500, 2500, 400, 400, 140, 71]
        } else {
            vec![100; count]
        },
        available: BTreeMap::new(),
        foods: vec![],
        potions: vec![],
        notes: vec![],
        mark_ticks: 0,
    }
}
#[test]
fn vorkath_six_attack_blocks_alternate_specials_and_never_damage_during_freeze() {
    let mut m = model(Encounter::Vorkath, 1);
    m.hp[0] = 75;
    let result = simulation::simulate(&m).unwrap();
    for run in result.runs {
        assert!(run.completed);
        let u = run.usage;
        // 75 one-tick attacks + one 30-tick acid window + one 12-tick freeze.
        assert_eq!(
            (u.ticks, u.attacks, u.acid, u.freeze, u.crumble),
            (117, 75, 1, 1, 1)
        );
        assert_eq!(u.phase_ticks["Freeze / spawn"], 12);
        assert_eq!(u.resources[&556], 2);
        assert_eq!(u.resources[&557], 2);
        assert_eq!(u.resources[&562], 1);
        assert!(!run.packed, "Crumble runes must be in the bag");
    }
}
#[test]
fn yama_has_two_complete_magic_first_judges_and_preserves_boss_overkill() {
    let mut m = model(Encounter::Yama, 500);
    m.settings.glyph_hp = 0;
    m.settings.glyph_prayer = 0;
    m.attacks[2] = hit(80);
    m.attacks[2].runes = vec![(565, 1)];
    m.attacks[3] = hit(80);
    m.attacks[3].charge = true;
    for r in simulation::simulate(&m).unwrap().runs {
        assert!(r.completed);
        assert_eq!(r.usage.judges, 2);
        assert_eq!(r.usage.attacks, 15);
        assert_eq!(r.usage.resources[&565], 6);
        assert_eq!(r.usage.charges[&4151], 4);
        assert_eq!(r.usage.phase_ticks["Judge alternation"], 10);
    }
}
#[test]
fn wardens_repeats_shields_uses_fivefold_core_damage_and_enters_enrage_once() {
    let mut m = model(Encounter::Wardens, 5);
    m.hp = vec![10, 10, 10, 4000, 100, 100];
    m.attacks[3] = hit(10);
    for r in simulation::simulate(&m).unwrap().runs {
        assert!(r.completed);
        let u = r.usage;
        assert_eq!((u.core_downs, u.skulls, u.enrage, u.healing), (3, 4, 1, 20));
        // 21*50 + 29*50 + 30*50 = 4000; third exposure allows 37 ticks.
        assert_eq!(u.phase_ticks["Core exposure"], 80);
        assert_eq!(u.phase_ticks["P2 shield"], 6);
        assert_eq!(u.ticks, 112);
    }
    assert_eq!(simulation::core_exposure(80, 100), 21);
    assert_eq!(simulation::core_exposure(79, 100), 29);
    assert_eq!(simulation::core_exposure(60, 100), 29);
    assert_eq!(simulation::core_exposure(59, 100), 37);
}
#[test]
fn cooldown_survives_phase_changes_food_delays_and_utility_waits() {
    let mut m = model(Encounter::Yama, 500);
    for a in &mut m.attacks {
        a.hits.attack_ticks = 5;
    }
    m.settings.glyph_hp = 0;
    m.settings.glyph_prayer = 0;
    // 5 boss attacks + 2 alternating one-hit Judges = seven committed hits.
    let a = simulation::simulate(&m).unwrap();
    let u = &a.runs[0].usage;
    assert_eq!(u.attacks, 7);
    assert_eq!(u.ticks, 31); // hits at 1,6,...31
    m.settings.start_hp = 10;
    m.foods = vec![(385, 20, 10)];
    let b = simulation::simulate(&m).unwrap();
    assert!(b.runs[0].usage.ticks > u.ticks);
    assert!(b.runs[0].usage.food_bites > 0);
}
#[test]
fn trip_reuses_stock_and_never_turns_bank_items_into_carried_ammunition() {
    let mut m = model(Encounter::Vorkath, 10);
    m.hp[0] = 10;
    m.settings.trip_limit = 5;
    m.settings.ammo_loss = 100;
    for a in &mut m.attacks {
        a.projectile = Some(9242);
    }
    m.available.insert(9242, 2);
    let a = simulation::simulate(&m).unwrap();
    assert!(a.runs.iter().all(|r| r.trip_kills == 2));
    m.settings.ammo_loss = 0;
    let b = simulation::simulate(&m).unwrap();
    assert!(b.runs.iter().all(|r| r.trip_kills == 5));
    m.available.clear();
    let c = simulation::simulate(&m).unwrap();
    assert_eq!(c.covered(), 0);
    assert!(c.runs.iter().all(|r| r.usage.ammo_shortage));
    m.available.insert(9242, 2);
    for a in &mut m.attacks {
        a.projectile_loss = Some(100);
    }
    let racks = simulation::simulate(&m).unwrap();
    assert!(racks.runs.iter().all(|r| r.trip_kills == 2));
}
#[test]
fn food_prayer_charges_and_death_are_independent_of_completion_percentiles() {
    let mut m = model(Encounter::Vorkath, 1);
    m.hp[0] = 20;
    m.settings.start_hp = 10;
    m.attacks[0].prayer = 1.0;
    m.settings.start_prayer = 0;
    m.attacks[0].charge = true;
    m.mark_ticks = 10;
    let report = simulation::simulate(&m).unwrap();
    assert_eq!(report.completed(), 16);
    assert_eq!(report.covered(), 0);
    let u = &report.runs[0].usage;
    assert!(u.extra_food > 0 && u.extra_doses > 0);
    assert_eq!(u.mark, u.ticks.div_ceil(10));
    assert_eq!(u.resources[&564], u.mark);
    assert_eq!(u.resources[&566], u.mark);
    assert_eq!(u.charges[&4151], 20);
    m.settings.incoming = 600;
    m.settings.start_hp = 10;
    m.settings.eat_at = 10;
    let fail = simulation::simulate(&m).unwrap();
    assert!(fail.completed() < 16);
    assert!(
        fail.runs
            .iter()
            .any(|r| !r.completed && r.reason == "HP depleted")
    );
}
#[test]
fn flare_deadlines_regeneration_and_damage_heal_the_same_boss_hp_state() {
    let mut m = model(Encounter::Yama, 10);
    m.settings.flare_interval = 20;
    m.settings.flare_charge = 10;
    m.settings.glyph_hp = 0;
    m.settings.glyph_prayer = 0;
    m.attacks[4] = hit(1);
    m.attacks[5] = hit(1);
    let report = simulation::simulate(&m).unwrap();
    assert!(report.runs.iter().all(|r| r.usage.flares > 0));
    assert!(
        report
            .runs
            .iter()
            .any(|r| r.usage.exploded > 0 && r.usage.healing > 0)
    );
    let mut safe = m.clone();
    safe.attacks[4] = hit(200);
    safe.attacks[5] = hit(200);
    safe.settings.flare_heal = 0;
    let a = simulation::simulate(&safe).unwrap();
    assert_eq!(a.completed(), 16);
    assert!(a.runs.iter().all(|r| r.usage.exploded == 0));
}
#[test]
fn successful_hit_charges_are_not_spent_on_misses_or_per_splat() {
    let mut m = model(Encounter::Vorkath, 5);
    m.hp[0] = 10;
    m.attacks[0].charge = true;
    m.attacks[0].charge_on_damage = true;
    m.attacks[0].hits.hit_distribution = vec![(0, 0.5), (5, 0.5)];
    let report = simulation::simulate(&m).unwrap();
    assert!(
        report
            .runs
            .iter()
            .all(|r| r.completed && r.usage.charges[&4151] == 2)
    );
    assert!(report.runs.iter().any(|r| r.usage.attacks > 2));
}
#[test]
fn final_melee_range_autos_add_flares_and_attack_cost_instead_of_free_uptime() {
    let mut m = model(Encounter::Yama, 50);
    m.settings.glyph_hp = 0;
    m.settings.glyph_prayer = 0;
    m.attacks[4] = hit(200);
    m.attacks[5] = hit(200);
    let clean = simulation::simulate(&m).unwrap();
    m.settings.melee_autos = 100;
    let exposed = simulation::simulate(&m).unwrap();
    assert!(
        exposed
            .runs
            .iter()
            .all(|r| r.completed && r.usage.flares >= 2)
    );
    assert!(exposed.runs[0].usage.attacks > clean.runs[0].usage.attacks);
}
#[test]
fn seeds_are_reproducible_and_invalid_or_stalled_models_are_bounded() {
    let m = model(Encounter::Vorkath, 1);
    assert_eq!(
        serde_json::to_string(&simulation::simulate(&m).unwrap()).unwrap(),
        serde_json::to_string(&simulation::simulate(&m).unwrap()).unwrap()
    );
    let mut bad = m.clone();
    bad.attacks.clear();
    assert!(simulation::simulate(&bad).is_err());
    bad = m.clone();
    bad.attacks[0].hits.hp_dependent = Some(combat::HpDependentHit {
        chance: 0.5,
        percent: 20,
        cap: 100,
        divisor: 0,
        transform: Default::default(),
        distribution_entries: 1,
    });
    assert!(simulation::simulate(&bad).is_err());
    let report = simulation::simulate(&m).unwrap();
    let raw = serde_json::to_value(&report).unwrap();
    let replay: Model = serde_json::from_value(raw["inputs"].clone()).unwrap();
    assert_eq!(
        serde_json::to_string(&simulation::simulate(&replay).unwrap()).unwrap(),
        serde_json::to_string(&report).unwrap()
    );
    bad = m.clone();
    bad.settings.trials = 1000;
    assert!(simulation::simulate(&bad).is_err());
    let mut stalled = m;
    stalled.attacks = vec![hit(0); 2];
    let a = simulation::simulate(&stalled).unwrap();
    assert_eq!(a.completed(), 0);
    assert!(a.runs.iter().all(|r| r.usage.ticks <= 6001));
}
#[test]
fn settings_migrate_validate_and_round_trip_without_changing_the_packing_plan() {
    let mut snapshot = account::demo();
    for skill in account::SKILLS {
        snapshot.members[0].level_overrides.insert(skill.into(), 99);
    }
    let mut o = Options::default();
    let original = planner::plan(&snapshot, &o);
    o.simulation.incoming = 120;
    o.simulation.seed = 456;
    let plan = planner::plan(&snapshot, &o);
    assert_eq!(original.equipment, plan.equipment);
    assert_eq!(original.inventory, plan.inventory);
    let setup = ironbound::share::Setup::parse(
        &ironbound::share::Setup::from_plan(&plan, &o)
            .fragment()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(setup.options().simulation, o.simulation);
    let legacy: Options = serde_json::from_str("{}").unwrap();
    assert!(legacy.simulation.valid());
    o.simulation.uptime = 0;
    assert!(ironbound::setups::validate_options(&o).is_err());
}
#[test]
fn all_three_build_only_from_real_carried_phase_sets() {
    let mut snapshot = account::demo();
    for skill in account::SKILLS {
        snapshot.members[0].level_overrides.insert(skill.into(), 99);
    }
    let mut options = Options::default();
    options.boss = Boss::Toa;
    let toa = planner::plan(&snapshot, &options);
    let m = Model::from_plan(&snapshot, &options, &toa).unwrap();
    assert_eq!(m.encounter, Encounter::Wardens);
    for name in ["Vorkath", "Yama"] {
        let entry = ironbound::encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        let mut o = Options {
            boss: Boss::Custom,
            targets: entry.targets.clone(),
            ..Default::default()
        };
        o.targets = planner::recommend_encounter(&snapshot, &o, &o.targets);
        let plan = planner::plan(&snapshot, &o);
        let model = Model::from_plan(&snapshot, &o, &plan).unwrap();
        assert!(!model.attacks.is_empty());
        assert!(model.max_hp > 0);
    }
}
