//! Archive ordinary attacks: independent arithmetic plus the reported weapon comparison.
use ironbound::{
    account::{self, Member},
    alternatives, combat, encounters, modifiers,
    planner::{self, Boss, Options, Target},
    tooltip,
};

fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn calc(id: i32, weapon: i32, style: &str) -> Result<combat::ResultStats, String> {
    combat::calculate(
        &member(),
        &set(weapon),
        style,
        planner::monster(id).unwrap(),
        "",
    )
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn ezekiel_mace_and_trident_match_independent_standard_rolls() {
    // Pinned NPC: Defence 200, crush +35, Magic 200, magic defence +200.
    // Mace Accurate: effective Attack 110, Strength 107; crush +60, strength +55.
    let mace = calc(16652, 1434, "Crush").unwrap();
    assert_eq!(
        (mace.attack_roll, mace.defence_roll, mace.max_hit),
        (13640, 20691, 20)
    );
    let hit = 13640.0 / (2.0 * 20692.0);
    near(mace.accuracy, hit);
    near(mace.dps, hit * 211.0 / 21.0 / 2.4);
    // Trident: effective Magic 110, attack +15; max floor(99/3)-5=28.
    let trident = calc(16652, 11905, "Magic").unwrap();
    assert_eq!(
        (trident.attack_roll, trident.defence_roll, trident.max_hit),
        (8690, 55176, 28)
    );
    let hit = 8690.0 / (2.0 * 55177.0);
    near(trident.accuracy, hit);
    near(trident.dps, hit * 407.0 / 29.0 / 2.4);
    assert!(mace.dps > calc(16652, 11920, "Crush").unwrap().dps);
}

#[test]
fn reviewed_windows_calculate_without_enabling_unknown_phases_or_reach() {
    for id in [
        16652, 16667, 16643, 16645, 16647, 16665, 16673, 16675, 16677,
    ] {
        let r = calc(id, 11905, "Magic").unwrap();
        assert!(r.dps > 0.0 && r.dps.is_finite(), "{id}");
        assert!(
            encounters::window(id)
                .unwrap()
                .description
                .contains("excluded")
        );
    }
    assert!(calc(16659, 11905, "Magic").unwrap().dps > 0.0);
    assert!(calc(16659, 1434, "Crush").unwrap_err().contains("reach"));
    assert!(ironbound::monster_rules::check_weapon(16659, "Melee", 3204).is_ok());
    for id in [16640, 16641, 16684] {
        let error = calc(id, 11905, "Magic").unwrap_err();
        assert!(error.contains("needs"), "{id}: {error}");
    }
}

#[test]
fn archive_defence_floors_and_demonbane_effectiveness_remain_target_specific() {
    for (id, floor) in [
        (16652, 120),
        (16659, 156),
        (16667, 117),
        (16673, 129),
        (16677, 150),
    ] {
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &modifiers::Reductions {
                bgs: 10_000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], floor);
    }
    for (id, percent) in [(16665, 25), (16673, 80), (16677, 80), (16675, 100)] {
        assert_eq!(encounters::demonbane_vulnerability(id), percent);
    }
    assert!(
        !planner::monster(16675)
            .unwrap()
            .attributes
            .iter()
            .any(|a| a == "demon")
    );
}

#[test]
fn reported_pickaxe_alternative_has_numeric_damage_and_automatic_plan_uses_mace() {
    let mut bank = account::demo();
    let m = &mut bank.members[0];
    m.bank = Some(vec![11920, 1, 1434, 1]);
    m.inventory = None;
    m.equipment = None;
    m.shared_bank = None;
    m.rune_pouch = None;
    m.potion_storage = None;
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 16652,
            style: "Crush".into(),
        }],
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    let automatic = planner::plan(&bank, &o);
    assert_eq!(automatic.equipment[3], Some(1434));
    o.gear_overrides.insert("Crush:3".into(), 11920);
    let pinned = planner::plan(&bank, &o);
    assert_eq!(pinned.equipment[3], Some(11920));
    let rows = alternatives::equipment(&bank, &o, &pinned, None, 3, 4);
    let mace = rows.iter().find(|r| r.id == 1434).unwrap();
    assert!(mace.dps_delta.is_some_and(|d| d > 0.0));
    assert!(mace.dps_reason.is_none());
    let html = tooltip::render("Dragon pickaxe", Some(11920), &rows, None);
    assert!(!html.contains("Unverified") && !html.contains("DPS comparison unavailable"));
}
