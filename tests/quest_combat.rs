use ironbound::{
    account::{self, Member},
    bestiary::{self, Readiness},
    combat, encounters,
    monster_rules::{self, Transform},
    planner::{self, Boss, Options},
    quest_bosses, rules,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn quest_ammunition_candidates_respect_the_configured_shot_budget() {
    let owned = BTreeMap::from([(9185, 1), (9144, 50), (1580, 1)]);
    let base = vec![("Ranged", set(9185))];
    let p = quest_bosses::profile("Fareed").unwrap();
    let short = quest_bosses::candidate_sets(p, &base, &member(), &owned, 50);
    assert!(short.iter().any(|(_, s)| s[13] == Some(9144)));
    let long = quest_bosses::candidate_sets(p, &base, &member(), &owned, 100);
    assert!(long.iter().all(|(_, s)| s[13].is_none()));
}
fn member() -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m
}
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn calc(
    id: i32,
    s: &[Option<i32>],
    style: &str,
    spell: &str,
) -> Result<combat::ResultStats, String> {
    combat::calculate(&member(), s, style, planner::monster(id).unwrap(), spell)
}
fn strong() -> account::Snapshot {
    let mut a = account::demo();
    a.members[0] = member();
    a.members[0].name = a.selected.clone();
    // Public synthetic fixture: real weapons, four armour slots and sufficient
    // ordinary runes/ammunition; includes stronger generic weapons as decoys.
    a.members[0].bank = Some(vec![
        33718, 1, 22398, 1, 24699, 1, 4151, 1, 1381, 1, 1383, 1, 1387, 1, 1401, 1, 11905, 1, 9185,
        1, 9144, 1000, 9242, 1000, 892, 1000, 861, 1, 1580, 1, 21140, 1, 10828, 1, 6570, 1, 1725,
        1, 6585, 1, 11832, 1, 11834, 1, 7462, 1, 11840, 1, 4708, 1, 4712, 1, 4714, 1, 2503, 1,
        2497, 1, 4091, 1, 4093, 1, 556, 10000, 554, 10000, 555, 10000, 557, 10000, 558, 10000, 560,
        10000, 565, 10000, 562, 10000,
    ]);
    a
}
#[test]
fn quest_weapon_fallback_never_recommends_ordinary_weapons_or_later_rewards() {
    for (id, required) in [(16204, 33718), (8242, 22398), (9567, 24699)] {
        let mut a = strong();
        // Reproduce opening the encounter's generic Melee route, with stronger
        // ordinary weapons and even the later Sunspear reward available.
        a.members[0].bank.as_mut().unwrap().extend([29577, 1, 33722, 1]);
        let mut options = Options {
            boss: Boss::Custom,
            targets: vec![planner::Target { id, style: "Melee".into() }],
            ..Default::default()
        };
        options.gear_overrides.insert("Melee:9".into(), 7462);
        let plan = planner::plan(&a, &options);
        assert_eq!(plan.equipment[3], Some(required));
        assert!(plan.phases.iter().all(|p| p.equipment[3] == Some(required)));
        assert!(!plan.missing.iter().any(|s| s.contains("requires")), "{:?}", plan.missing);

        for case in ["absent", "excluded", "level"] {
            let mut missing = a.clone();
            let mut trial = options.clone();
            match case {
                "absent" => { missing.members[0].bank = Some(vec![29577, 1, 33722, 1, 4151, 1, 7462, 1]); }
                "excluded" => { trial.excluded.insert(required); }
                _ => { missing.members[0].level_overrides.insert("Attack".into(), 1); }
            }
            let p = planner::plan(&missing, &trial);
            if id == 16204 && case == "absent" {
                assert_eq!(p.equipment[3], Some(33718));
                assert!(p.phases.iter().all(|p| p.equipment[3] == Some(33718)));
                assert!(p.warnings.iter().any(|s| s.contains("Quest-provided Hallowed flail")));
                assert!(!p.missing.iter().any(|s| s.contains("Hallowed flail")));
                continue;
            }
            assert_eq!(p.equipment[3], None, "{id}: {case}");
            assert!(p.phases.iter().all(|p| p.equipment[3].is_none()), "{id}: {case}");
            assert!(p.missing.iter().any(|s| s.contains(&planner::item_name(required))), "{id}: {case}: {:?}", p.missing);
            assert!(!p.exportable(), "{id}: {case}");
        }
        // User pins remain reviewable, but cannot bypass quest requirements.
        options.gear_overrides.insert("Melee:3".into(), 29577);
        let pinned = planner::plan(&a, &options);
        assert_eq!(pinned.equipment[3], Some(29577));
        assert!(pinned.missing.iter().any(|s| s.contains(&planner::item_name(required))));
        assert!(!pinned.exportable());
    }
    let issue = quest_bosses::weapon_requirement(16204, &set(33722), "Melee").unwrap();
    assert!(issue.contains("Hallowed flail"));
    assert!(issue.contains("Sunspear is awarded after defeating Drakan"));
}
#[test]
fn quest_provided_flail_matches_owned_damage_and_readiness_without_changing_bank() {
    let owned = strong();
    let mut future = owned.clone();
    future.members[0].bank = Some(owned.members[0].bank.as_ref().unwrap()
        .chunks_exact(2).filter(|pair| pair[0] != 33718).flatten().copied().collect());
    assert!(!future.available(false).contains_key(&33718));
    let options = Options {
        boss: Boss::Custom,
        targets: vec![planner::Target { id: 16204, style: "Slash".into() }],
        ..Default::default()
    };
    let actual = planner::plan(&owned, &options);
    let projected = planner::plan(&future, &options);
    assert_eq!(projected.equipment, actual.equipment);
    assert_eq!(projected.equipment[3], Some(33718));
    assert_eq!(projected.missing, actual.missing);
    let m = planner::monster(16204).unwrap();
    let damage = combat::calculate_configured(future.selected().unwrap(), &projected.equipment,
        "Slash", m, "", 0, &options.combat).unwrap();
    let baseline = combat::calculate_configured(owned.selected().unwrap(), &actual.equipment,
        "Slash", m, "", 0, &options.combat).unwrap();
    assert!(damage.dps > 0.0);
    assert_eq!(damage.dps, baseline.dps);
    assert_eq!(damage.hit_distribution, baseline.hit_distribution);
    let index = encounters::catalog().iter().position(|e| e.name == "Lowerniel Drakan").unwrap();
    let actual = bestiary::assess_catalog(&owned, &options);
    let projected = bestiary::assess_catalog(&future, &options);
    assert_eq!(projected[index].readiness, actual[index].readiness);
    assert!(projected[index].reason.contains("Hallowed flail (quest-provided)"));
    assert!(!projected[index].reason.contains("Needs owned"));
    assert!(!future.available(false).contains_key(&33718));
    assert_eq!(planner::planning_stock(&future, &options).get(&33718), Some(&1));
    assert!(!planner::planning_stock(&future, &options).contains_key(&33722));
    assert!(!planner::planning_stock(&future, &Options::default()).contains_key(&33718));
    for id in [8242, 9567, 1672] {
        let mut other = options.clone();
        other.targets[0].id = id;
        assert!(!planner::planning_stock(&future, &other).contains_key(&33718));
    }
    for level in [49, 50] {
        let mut account = future.clone();
        account.members[0].level_overrides.insert("Attack".into(), level);
        assert_eq!(planner::planning_stock(&account, &options).contains_key(&33718), level >= 50);
    }
    let mut excluded = options.clone();
    excluded.excluded.insert(33718);
    assert!(!planner::planning_stock(&future, &excluded).contains_key(&33718));
    let blocked = bestiary::assess_catalog(&future, &excluded);
    assert!(blocked[index].reason.contains("Needs owned"));
}
#[test]
fn all_quest_profiles_are_tagged_filterable_and_rating_is_available() {
    let mut ids = BTreeSet::new();
    let a = strong();
    let rows = bestiary::assess_catalog(&a, &Options::default());
    assert_eq!(quest_bosses::profiles().len(), 23);
    for p in quest_bosses::profiles() {
        let i = encounters::catalog()
            .iter()
            .position(|e| e.name == p.name)
            .unwrap();
        let e = &encounters::catalog()[i];
        assert_eq!(e.quest(), Some(p.quest.as_str()));
        assert_eq!(e.category(), "Quest bosses");
        assert!(!quest_bosses::visible(e, true));
        assert!(quest_bosses::visible(e, false));
        for id in &p.ids {
            assert!(ids.insert(*id));
            assert!(
                encounters::limitation(planner::monster(*id).unwrap()).is_none(),
                "{id}"
            );
        }
        assert_ne!(
            rows[i].readiness,
            Readiness::Unknown,
            "{}: {}",
            p.name,
            rows[i].reason
        );
        assert!(
            !rows[i].reason.contains("No usable"),
            "{}: {}",
            p.name,
            rows[i].reason
        );
        let html = bestiary::card(e, i, &rows[i]);
        assert!(html.contains("Quest boss ·"));
        assert!(html.contains(&p.quest));
    }
    for name in ["Vorkath", "Duke Sucellus", "Vardorvis", "Scurrius"] {
        let e = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert_eq!(e.quest(), None);
        assert!(quest_bosses::visible(e, true));
        assert_eq!(e.category(), "Bosses");
    }
}
#[test]
fn required_flails_apply_independent_roll_and_splat_arithmetic() {
    for (id, weapon, style, n, d) in [
        (8242, 22398, "Crush", 1, 1),
        (9567, 24699, "Crush", 21, 20),
        (16204, 33718, "Slash", 5, 4),
    ] {
        let result = calc(id, &set(weapon), style, "").unwrap();
        let bonus = combat::stats(weapon).unwrap().offensive[&style.to_lowercase()];
        assert_eq!(
            result.attack_roll,
            (99 + 8 + 3) * i64::from(bonus + 64) * n / d
        );
        assert!(result.dps > 0.0);
        assert!((result.hit_distribution.iter().map(|(_, p)| p).sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(calc(id, &set(4151), "Slash", "").is_err());
        let mut ring = set(weapon);
        ring[12] = Some(21140);
        let r = calc(id, &ring, style, "").unwrap();
        assert_eq!(r.attack_roll, result.attack_roll * 23 / 20);
    }
    let flail = Transform::QuestVampyre {
        numerator: 5,
        denominator: 4,
        efaritay: false,
        drakan_cap: false,
    };
    assert_eq!(flail.outcomes(18, 1.0), vec![(22, 1.0)]);
    let ring = Transform::QuestVampyre {
        numerator: 5,
        denominator: 4,
        efaritay: true,
        drakan_cap: false,
    };
    assert_eq!(ring.outcomes(18, 1.0), vec![(23, 1.0)]);
}
#[test]
fn drakan_cap_is_after_vampyre_damage_and_preserves_misses_and_mass() {
    let t = Transform::QuestVampyre {
        numerator: 5,
        denominator: 4,
        efaritay: false,
        drakan_cap: true,
    };
    assert_eq!(t.outcomes(0, 1.0), vec![(0, 1.0)]);
    assert_eq!(t.outcomes(23, 1.0), vec![(28, 1.0)]);
    assert_eq!(
        t.outcomes(24, 1.0),
        vec![(26, 1.0 / 3.0), (27, 1.0 / 3.0), (28, 1.0 / 3.0)]
    );
    let mut s = set(33718);
    for (slot, id) in [
        (0, 10828),
        (1, 6570),
        (2, 1725),
        (4, 11832),
        (7, 11834),
        (9, 7462),
        (10, 11840),
        (12, 21140),
    ] {
        s[slot] = Some(id);
    }
    let r = calc(16204, &s, "Slash", "").unwrap();
    assert_eq!(r.max_hit, 28);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d <= 28));
}
#[test]
fn magic_immunities_gloves_and_flying_phases_are_enforced() {
    let water = set(1383);
    let mut gloves = water.clone();
    gloves[9] = Some(1580);
    assert!(
        calc(3456, &water, "Magic", "Water Wave")
            .unwrap_err()
            .contains("ice gloves")
    );
    assert!(calc(3456, &gloves, "Magic", "Water Wave").unwrap().dps > 0.0);
    assert!(calc(3456, &gloves, "Magic", "Fire Wave").is_err());
    assert!(calc(3458, &set(1387), "Magic", "Fire Wave").unwrap().dps > 0.0);
    assert!(calc(3458, &water, "Magic", "Water Wave").is_err());
    assert!(calc(3458, &set(11905), "Magic", "").is_err());
    assert!(calc(13482, &set(11905), "Magic", "").is_err());
    assert!(calc(13482, &set(1387), "Magic", "Fire Wave").unwrap().dps > 0.0);
    for id in [12215, 8097, 8096] {
        assert!(calc(id, &set(4151), "Slash", "").is_err());
    }
}
#[test]
fn quest_vardorvis_uses_its_own_hp_curve_and_duke_has_partial_demonbane() {
    let m = planner::monster(12224).unwrap();
    assert_eq!(m.skills["def"], 180);
    assert_eq!(m.skills["str"], 210);
    for (hp, def, strength) in [(500, 180, 210), (250, 155, 245), (0, 130, 280)] {
        let scaled = monster_rules::hp_scaled(m, hp).unwrap().unwrap();
        assert_eq!(scaled.skills["def"], def);
        assert_eq!(scaled.skills["str"], strength);
    }
    assert!(monster_rules::hp_scaled(m, 501).is_err());
    assert_eq!(encounters::demonbane_vulnerability(12195), 70);
}
#[test]
fn planner_and_readiness_select_required_owned_gear_and_respect_exclusions() {
    let a = strong();
    let mut options = Options {
        boss: Boss::Custom,
        ..Default::default()
    };
    for (name, id, style, weapon) in [
        ("Lowerniel Drakan", 16204, "Slash", 33718),
        ("Ranis Drakan", 8242, "Crush", 22398),
        ("Vanstrom Klause", 9567, "Crush", 24699),
    ] {
        options.targets = vec![planner::Target {
            id,
            style: style.into(),
        }];
        let s = planner::equipment_for_style(&a, &options, style);
        assert_eq!(s[3], Some(weapon), "{name}");
        let i = encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap();
        assert!(
            bestiary::assess_catalog(&a, &options)[i]
                .reason
                .contains(&planner::item_name(weapon)),
            "{name}"
        );
        let mut excluded = options.clone();
        excluded.excluded.insert(weapon);
        assert_eq!(
            bestiary::assess_catalog(&a, &excluded)[i].readiness,
            Readiness::Struggle,
            "{name}"
        );
        assert_ne!(
            planner::equipment_for_style(&a, &excluded, style)[3],
            Some(weapon)
        );
    }
    options.targets = vec![planner::Target {
        id: 3456,
        style: "Magic".into(),
    }];
    let s = planner::equipment_for_style(&a, &options, "Magic");
    assert_eq!(s[9], Some(1580));
    let owned = a.available(false);
    let spell = rules::spell_name_for_target(
        a.selected().unwrap(),
        &s,
        &owned,
        &options.combat,
        planner::monster(3456).unwrap(),
        0,
    );
    assert!(spell.starts_with("Water"));
    options.targets = vec![planner::Target {
        id: 13482,
        style: "Magic".into(),
    }];
    let s = planner::equipment_for_style(&a, &options, "Magic");
    assert_ne!(s[3], Some(11905));
    assert_eq!(
        quest_bosses::profile("Galvek").unwrap().ids,
        [8094, 8097, 8096, 8098]
    );
}
