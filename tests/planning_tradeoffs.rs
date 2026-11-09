use ironbound::{
    account, defence, encounters,
    planner::{self, Boss, Options},
    prayer,
    share::Setup,
};
fn stocked() -> account::Snapshot {
    let mut bank = account::demo();
    bank.members[0].bank.as_mut().unwrap().extend([
        10551, 1, 4749, 1, 4751, 1, 9674, 1, 9676, 1, 28810, 1, 1434, 1, 11840, 1, 4087, 1,
    ]);
    bank.members[0].level_overrides.insert("Prayer".into(), 80);
    bank
}
#[test]
fn moons_export_preserves_three_melee_styles_and_private_free_preferences() {
    let bank = stocked();
    let o = Options {
        boss: Boss::Custom,
        targets: encounters::groups()
            .iter()
            .find(|g| g.name == "Moons of Peril")
            .unwrap()
            .targets
            .clone(),
        max_switches: Some(4),
        switch_slots: 8,
        food: 4,
        combat: defence::Settings {
            offensive: "Piety".into(),
            safety: 2,
            prayer_weight: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let plan = planner::plan(&bank, &o);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(
        plan.phases
            .iter()
            .map(|p| p.style.as_str())
            .collect::<Vec<_>>(),
        ["Slash", "Crush", "Stab"]
    );
    assert!(plan.optimization.as_ref().unwrap().max_changes <= 4);
    let shared = Setup::from_plan(&plan, &o);
    shared.validate().unwrap();
    let loaded = Setup::parse(&shared.fragment().unwrap()).unwrap();
    assert_eq!(loaded.options().combat, o.combat);
    assert_eq!(loaded.targets, o.targets);
    let raw = serde_json::to_string(&shared).unwrap();
    assert!(!raw.contains("Iron Adventurer") && !raw.contains("bank") && !raw.contains("skills"));
}
#[test]
fn prayer_preference_changes_real_planner_gear_and_reduces_drain() {
    let bank = stocked();
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![planner::Target {
            id: 1673,
            style: "Slash".into(),
        }],
        combat: defence::Settings {
            offensive: "Piety".into(),
            protection: "Melee".into(),
            safety: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let damage = planner::plan(&bank, &o);
    o.ranking.weights = ironbound::ranking::Weights {
        dps: 10.0,
        defence: 0.0,
        prayer: 90.0,
        weight: 0.0,
    };
    let efficient = planner::plan(&bank, &o);
    let drain = |set: &[Option<i32>]| prayer::drain_per_minute(prayer::bonus(set), &o.combat);
    assert!(drain(&efficient.equipment) < drain(&damage.equipment));
    assert!(efficient.exportable(), "{:?}", efficient.missing);
}
#[test]
fn malformed_public_combat_settings_are_rejected() {
    let o = Options::default();
    let p = planner::plan(&account::demo(), &o);
    let mut shared = Setup::from_plan(&p, &o);
    shared.combat.protection = "private text".into();
    assert!(shared.validate().is_err());
    shared.combat = Default::default();
    shared.combat.safety = 99;
    assert!(shared.validate().is_err());
}
