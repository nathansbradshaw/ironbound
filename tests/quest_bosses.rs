use ironbound::{encounters, planner};
use std::collections::BTreeSet;

#[test]
fn notable_quest_bosses_are_selectable_in_the_bestiary() {
    for name in [
        "Lowerniel Drakan",
        "Ranis Drakan",
        "Vanstrom Klause",
        "Fragment of Seren",
        "Galvek",
        "Sliske's wights",
        "Elvarg",
        "Jungle Demon",
        "Kruk",
        "Glough",
        "Robert the Strong",
        "Vorkath (Quest)",
        "Xamphur",
        "Surok Magis",
        "Barrelchest",
        "Damis",
        "Fareed",
        "Kamil",
        "Dessous",
        "Duke Sucellus (Quest)",
        "Vardorvis (Quest)",
        "The Leviathan (Quest)",
        "The Whisperer (Quest)",
    ] {
        let entry = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("Missing quest boss: {name}"));
        assert_eq!(entry.category(), "Quest bosses", "{name}");
        assert!(entry.bank_equipment && !entry.targets.is_empty(), "{name}");
        assert!(entry.targets.iter().all(planner::Target::valid), "{name}");
        assert!(entry.note.contains("Quest boss"), "{name}");
        assert!(entry.matches(&entry.targets), "{name}");
        assert!(!entry.guide_url().contains("(Quest)"), "{name}");
        assert!(!entry.guide_url().ends_with("/Strategies"), "{name}");
    }
}

#[test]
fn quest_search_accepts_familiar_names_and_quest_titles() {
    for (query, name) in [
        ("Drakan", "Lowerniel Drakan"),
        ("Seren", "Fragment of Seren"),
        ("galvak", "Galvek"),
        ("Sliske", "Sliske's wights"),
        ("Song of the Elves", "Fragment of Seren"),
        ("The Blood Moon Rises", "Lowerniel Drakan"),
        ("Sins of the Father", "Vanstrom Klause"),
        ("DT2 final battle", "Sliske's wights"),
    ] {
        let group = encounters::groups()
            .iter()
            .find(|g| g.name == name)
            .unwrap();
        let query = query.to_lowercase();
        assert!(
            group.name.to_lowercase().contains(&query)
                || group
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().contains(&query)),
            "{name}"
        );
    }
}

#[test]
fn quest_variants_do_not_borrow_post_quest_or_nightmare_zone_stats() {
    for (name, id, hp) in [
        ("Vorkath (Quest)", 8058, 460),
        ("Duke Sucellus (Quest)", 12195, 330),
        ("Vardorvis (Quest)", 12224, 500),
        ("The Leviathan (Quest)", 12215, 720),
        ("The Whisperer (Quest)", 12206, 660),
        ("Elvarg", 817, 80),
        ("Barrelchest", 600, 134),
        ("Vanstrom Klause", 9567, 750),
        ("Lowerniel Drakan", 16204, 3000),
    ] {
        let entry = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert_eq!(entry.targets.len(), 1, "{name}");
        assert_eq!(entry.targets[0].id, id, "{name}");
        let monster = planner::monster(id).unwrap();
        assert_eq!(monster.skills["hp"], hp, "{name}");
        assert!(!monster.name.contains("Nightmare Zone"));
    }
    for (name, id) in [
        ("Vorkath", 8059),
        ("Vardorvis", 12223),
        ("Duke Sucellus", 12191),
        ("The Leviathan", 12214),
        ("The Whisperer", 12204),
    ] {
        let entry = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert_eq!(entry.targets[0].id, id, "Existing post-quest boss: {name}");
    }
}

#[test]
fn galvek_phases_share_one_1200_hp_fight_in_the_correct_order() {
    let entry = encounters::catalog()
        .iter()
        .find(|e| e.name == "Galvek")
        .unwrap();
    assert_eq!(
        entry.targets.iter().map(|t| t.id).collect::<Vec<_>>(),
        [8094, 8097, 8096, 8098]
    );
    assert_eq!(
        entry
            .targets
            .iter()
            .map(|t| planner::monster(t.id).unwrap().skills["hp"])
            .sum::<i32>(),
        1200
    );
    for target in &entry.targets {
        assert_eq!(planner::monster(target.id).unwrap().skills["hp"], 300);
    }
}

#[test]
fn sliske_finale_uses_four_distinct_wights_without_ketlas_clone() {
    let entry = encounters::catalog()
        .iter()
        .find(|e| e.name == "Sliske's wights")
        .unwrap();
    assert_eq!(
        entry.targets.iter().map(|t| t.id).collect::<Vec<_>>(),
        [12328, 12329, 12331, 12333]
    );
    assert_eq!(
        entry
            .targets
            .iter()
            .map(|t| t.id)
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
    assert_eq!(
        entry
            .targets
            .iter()
            .map(|t| planner::monster(t.id).unwrap().skills["hp"])
            .sum::<i32>(),
        800
    );
    assert!(
        entry
            .note
            .contains("Sliske himself is not the combat target")
    );
}

#[test]
fn reviewed_quest_bodies_have_damage_without_claiming_complete_fights() {
    for name in [
        "Lowerniel Drakan",
        "Ranis Drakan",
        "Vanstrom Klause",
        "Galvek",
        "Sliske's wights",
        "Glough",
        "Vorkath (Quest)",
        "Xamphur",
        "Surok Magis",
        "Damis",
        "Fareed",
        "Kamil",
        "Dessous",
        "Duke Sucellus (Quest)",
        "Vardorvis (Quest)",
        "The Leviathan (Quest)",
        "The Whisperer (Quest)",
    ] {
        let entry = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert_eq!(
            entry.coverage(),
            "Basic attack estimates · mechanics partial",
            "{name}"
        );
        for target in &entry.targets {
            assert!(
                encounters::limitation(planner::monster(target.id).unwrap()).is_none(),
                "{name}"
            );
        }
    }
}

#[test]
fn reviewed_seren_body_gains_gear_readiness_without_claiming_the_full_quest_fight() {
    let index = encounters::catalog()
        .iter()
        .position(|e| e.name == "Fragment of Seren")
        .unwrap();
    let entry = &encounters::catalog()[index];
    assert!(encounters::limitation(planner::monster(8917).unwrap()).is_none());
    let assessment =
        ironbound::bestiary::assess_catalog(&ironbound::account::demo(), &Default::default())
            [index]
            .clone();
    assert_ne!(
        assessment.readiness,
        ironbound::bestiary::Readiness::Unknown
    );
    assert!(entry.note.contains("near-max-HP"));
    assert!(
        assessment
            .detail
            .contains("full-fight supplies are not scored")
    );
    assert_eq!(
        assessment.requirements.status,
        ironbound::bestiary::RequirementStatus::NoLevelGate
    );
}
