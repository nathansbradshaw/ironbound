//! Public synthetic regression bank; no user bank or character data.
use ironbound::{
    account, combat, encounters, guidance,
    planner::{self, Boss, Options, Target},
    rules,
};
use std::collections::BTreeMap;
fn bank() -> account::Snapshot {
    let mut s = account::demo();
    let mut stock = s.available(false);
    for id in [
        26219, 29280, 29283, 21012, 11664, 13072, 13073, 8842, 12018, 10588, 1540, 22109, 6570,
        7462, 6585, 11840, 11773,
    ] {
        stock.insert(id, 1);
    }
    stock.insert(21944, 1000);
    stock.insert(21946, 1000);
    s.members[0].bank = Some(
        stock
            .into_iter()
            .flat_map(|(id, q)| [i64::from(id), i64::from(q)])
            .collect(),
    );
    s.members[0].inventory = None;
    s.members[0].equipment = None;
    s.members[0].quests = None;
    for skill in account::SKILLS {
        s.members[0].level_overrides.insert(skill.into(), 99);
    }
    s
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 8059,
            style: "Stab".into(),
        }],
        recommended: Some(true),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    }
}
#[test]
fn ranged_salve_seed_requires_an_imbued_owned_variant() {
    // Unimbued Salve(e) gives no ranged bonus: Vorkath/Strategies.
    let s = bank();
    let m = s.selected().unwrap();
    let mut set = vec![None; 14];
    set[3] = Some(21012);
    set[2] = Some(6585);
    guidance::seed(
        &mut set,
        "Ranged",
        m,
        &BTreeMap::from([(10588, 1), (6585, 1)]),
        &options(),
    );
    assert_eq!(set[2], Some(6585));
    guidance::seed(
        &mut set,
        "Ranged",
        m,
        &BTreeMap::from([(10588, 1), (12018, 1), (6585, 1)]),
        &options(),
    );
    assert_eq!(set[2], Some(12018));
}
#[test]
fn partial_void_is_not_a_reviewed_full_set_or_a_joint_candidate() {
    let s = bank();
    let m = s.selected().unwrap();
    let mut seed = vec![None; 14];
    seed[3] = Some(21012);
    seed[0] = Some(11664);
    let partial = BTreeMap::from([(11664, 1), (13072, 1), (13073, 1)]);
    assert!(guidance::joint_sets(&seed, "Ranged", m, &partial, &options()).is_empty());
    assert!(guidance::item_tier("Ranged", 0, 11664, &options(), &seed).is_none());
}
#[test]
fn complete_void_candidates_preserve_owned_weapon_and_eligible_full_set() {
    let s = bank();
    let m = s.selected().unwrap();
    let mut seed = vec![None; 14];
    seed[3] = Some(21012);
    seed[13] = Some(21944);
    seed[2] = Some(12018);
    let rows = guidance::joint_sets(&seed, "Ranged", m, &s.available(false), &options());
    assert_eq!(rows.len(), 1);
    let set = &rows[0];
    for (slot, id) in [(0, 11664), (4, 13072), (7, 13073), (9, 8842)] {
        assert_eq!(set[slot], Some(id));
        assert!(guidance::item_tier("Ranged", slot, id, &options(), set).is_some());
    }
    assert_eq!(set[3], seed[3]);
    assert_eq!(set[13], seed[13]);
    assert_eq!(set[2], seed[2]);
    let mut excluded = options();
    excluded.excluded.insert(8842);
    assert!(guidance::joint_sets(&seed, "Ranged", m, &s.available(false), &excluded).is_empty());
}
#[test]
fn quest_vorkath_does_not_silently_borrow_the_postquest_profile() {
    let mut o = options();
    assert_eq!(guidance::profile(&o).unwrap().id, "vorkath");
    o.targets[0].id = 8058;
    assert!(guidance::profile(&o).is_none());
}
#[test]
fn whole_owned_vorkath_comparison_discovers_dhcb_void_and_salve() {
    // The Wiki's post-quest crossbow progression compares full sets, not isolated
    // zero-stat Void pieces: https://oldschool.runescape.wiki/w/Vorkath/Strategies
    let s = bank();
    let mut o = options();
    let target = encounters::catalog()
        .iter()
        .find(|e| e.name == "Vorkath")
        .unwrap();
    o.targets = planner::recommend_encounter(&s, &o, &target.targets);
    assert_eq!(o.targets.len(), 1);
    assert_eq!(rules::base_style(&o.targets[0].style), "Ranged");
    let set = planner::equipment_for_style(&s, &o, "Ranged");
    assert_eq!(set[3], Some(21012));
    assert_eq!(set[2], Some(12018));
    for (slot, id) in [(0, 11664), (4, 13072), (7, 13073), (9, 8842)] {
        assert_eq!(set[slot], Some(id));
    }
    assert_eq!(set[13], Some(21944));
    let result = combat::calculate_configured(
        s.selected().unwrap(),
        &set,
        "Ranged",
        planner::monster(8059).unwrap(),
        "",
        0,
        &o.combat,
    )
    .unwrap();
    assert!(result.dps > 0.0);
}
#[test]
fn yama_guidance_does_not_leak_into_contract_or_unrelated_groups() {
    let mut o = options();
    o.targets = vec![Target {
        id: 14176,
        style: "Slash".into(),
    }];
    assert_eq!(guidance::profile(&o).unwrap().id, "yama");
    o.targets.push(Target {
        id: 100001,
        style: "Slash".into(),
    });
    assert!(guidance::profile(&o).is_none());
}

#[test]
fn comparing_styles_preserves_the_boss_identity_and_guide_link() {
    let entry = encounters::catalog()
        .iter()
        .find(|e| e.name == "Vorkath")
        .unwrap();
    let mut selected = entry.targets.clone();
    selected[0].style = "Ranged".into();
    assert!(entry.matches(&selected));
    selected[0].style = "not a combat style".into();
    assert!(!entry.matches(&selected));
    selected[0].style = "Ranged".into();
    selected[0].id = 11778;
    assert!(!entry.matches(&selected));
}
