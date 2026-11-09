use ironbound::{encounters, planner};
use std::collections::BTreeSet;

#[test]
fn every_public_hiscore_boss_has_an_explicit_catalog_entry() {
    let published: Vec<String> =
        serde_json::from_str(include_str!("../assets/bosses.json")).unwrap();
    assert!(
        published.len() >= 72,
        "The public boss snapshot is incomplete"
    );
    let catalog = encounters::catalog();
    let names: BTreeSet<_> = catalog.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names.len(), catalog.len(), "Duplicate encounter names");
    for name in published {
        let entry = catalog
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("Missing {name}"));
        assert!(!entry.note.is_empty());
        assert!(!entry.coverage().is_empty());
        if entry.targets.is_empty() {
            assert!(
                entry.coverage().contains("needed")
                    || entry.coverage().contains("missing")
                    || entry.coverage().contains("not supported")
            );
        }
    }
}

#[test]
fn authored_groups_only_reference_valid_recorded_targets() {
    for group in encounters::groups() {
        assert!(!group.targets.is_empty(), "{}", group.name);
        assert!(group.targets.len() <= ironbound::rules::MAX_TARGETS);
        let mut pairs = BTreeSet::new();
        for target in &group.targets {
            assert!(
                target.valid(),
                "{} has missing NPC {}",
                group.name,
                target.id
            );
            assert!(
                pairs.insert((target.id, &target.style)),
                "Duplicate target/style in {}",
                group.name
            );
        }
    }
}

#[test]
fn all_three_raids_have_editable_multi_style_room_templates() {
    for name in ["Chambers of Xeric", "Theatre of Blood", "Tombs of Amascut"] {
        let group = encounters::groups()
            .iter()
            .find(|g| g.name == name)
            .unwrap();
        let styles: BTreeSet<_> = group
            .targets
            .iter()
            .map(|t| ironbound::rules::base_style(&t.style))
            .collect();
        assert!(styles.contains("Melee") && styles.contains("Ranged") && styles.contains("Magic"));
        assert!(
            encounters::catalog()
                .iter()
                .find(|e| e.name == name)
                .unwrap()
                .coverage()
                .contains("partial")
        );
    }
    assert!(
        encounters::groups()
            .iter()
            .find(|g| g.name == "Chambers of Xeric")
            .unwrap()
            .targets
            .len()
            > 12
    );
}

#[test]
fn wave_encounters_are_visible_as_templates_not_simulated_waves() {
    for name in ["Fight Caves", "Inferno", "Fortis Colosseum"] {
        let group = encounters::groups()
            .iter()
            .find(|g| g.name == name)
            .unwrap();
        assert!(group.targets.len() > 6);
        assert!(group.note.contains("not fully modelled"));
        assert!(encounters::catalog().iter().any(|e| e.name == name));
    }
}

#[test]
fn every_bank_combat_boss_has_a_recorded_target_except_explicitly_unverified_modes() {
    let missing: Vec<_> = encounters::catalog()
        .iter()
        .filter(|e| {
            e.bank_equipment
                && !encounters::mechanics().unverified_modes.contains(&e.name)
                && e.targets.is_empty()
        })
        .map(|e| e.name.as_str())
        .collect();
    assert!(missing.is_empty(), "Unmapped combat bosses: {missing:?}");
}

#[test]
fn unverified_raid_modes_do_not_borrow_normal_mode_stats_silently() {
    for name in &encounters::mechanics().unverified_modes {
        let entry = encounters::catalog()
            .iter()
            .find(|e| &e.name == name)
            .unwrap();
        assert!(entry.targets.is_empty());
        assert_eq!(entry.coverage(), "Mode scaling not supported");
    }
}

#[test]
fn encounter_only_equipment_and_skilling_bosses_are_not_bank_combat_plans() {
    for name in &encounters::mechanics().no_bank_equipment {
        let entry = encounters::catalog()
            .iter()
            .find(|e| &e.name == name)
            .unwrap();
        assert!(!entry.bank_equipment);
        assert!(entry.targets.is_empty());
    }
}

#[test]
fn default_scurrius_and_mole_do_not_mix_solo_group_or_event_variants() {
    let scurrius = encounters::catalog()
        .iter()
        .find(|e| e.name == "Scurrius")
        .unwrap();
    assert_eq!(scurrius.targets.len(), 1);
    assert_eq!(scurrius.targets[0].id, 7222);
    let mole = encounters::catalog()
        .iter()
        .find(|e| e.name == "Giant Mole")
        .unwrap();
    assert_eq!(mole.targets.len(), 1);
    assert_eq!(mole.targets[0].id, 5779);
}

#[test]
fn moons_are_three_different_melee_attack_types() {
    let moons = encounters::groups()
        .iter()
        .find(|g| g.name == "Moons of Peril")
        .unwrap();
    assert_eq!(
        moons
            .targets
            .iter()
            .map(|t| (t.id, t.style.as_str()))
            .collect::<Vec<_>>(),
        vec![(13011, "Slash"), (13013, "Crush"), (13012, "Stab")]
    );
    let eclipse = planner::monster(13012).unwrap();
    assert_eq!(
        eclipse.defensive["flat_armour"], 6,
        "Use regular Eclipse, not the clone variant"
    );
}

#[test]
fn special_mechanic_fights_do_not_masquerade_as_standard_dps() {
    for id in [7550, 7540, 8369, 2042, 11278, 963, 14147, 16673, 11753] {
        let target = planner::monster(id).unwrap();
        assert!(
            encounters::limitation(target).is_some(),
            "{} needs explicit mechanic coverage",
            target.name
        );
    }
    assert!(
        encounters::limitation(planner::monster(13011).unwrap()).is_none(),
        "The supported Moon attack model should remain available"
    );
    assert!(encounters::limitation(planner::monster(5779).unwrap()).is_none());
}
