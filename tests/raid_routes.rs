use ironbound::{
    encounters,
    planner::{self, Target},
    raids,
};

#[test]
fn catalog_defaults_are_complete_public_routes_and_match_encounter_groups() {
    for raid in raids::raids() {
        let defaults = raid.default_targets();
        assert_eq!(raids::for_targets(&defaults).unwrap().id, raid.id);
        assert_eq!(
            encounters::groups()
                .iter()
                .find(|g| g.name == raid.name)
                .unwrap()
                .targets,
            defaults
        );
        for room in &raid.rooms {
            assert!(!room.targets.is_empty());
            for target in &room.targets {
                assert!(
                    planner::monster(target.id).is_some(),
                    "{}: {}",
                    room.name,
                    target.id
                );
            }
        }
    }
    assert_eq!(raids::raid("cox").unwrap().default_targets().len(), 17);
    let archive = raids::raid("tfa").unwrap();
    assert_eq!(
        archive
            .rooms
            .iter()
            .filter(|r| r.selected(&archive.default_targets()))
            .count(),
        5
    );
    assert!(raids::raid("unknown").is_none());
}

#[test]
fn cox_toggles_complete_rooms_and_keeps_olm_and_edited_styles() {
    let raid = raids::raid("cox").unwrap();
    let mut targets = raid.default_targets();
    targets[0].style = "Stab".into();
    let without = raid.toggle(&targets, "vanguards").unwrap();
    assert_eq!(without.len(), 14);
    assert!(!without.iter().any(|t| [7527, 7528, 7529].contains(&t.id)));
    assert_eq!(without.iter().find(|t| t.id == 7540).unwrap().style, "Stab");
    assert_eq!(raid.toggle(&without, "vanguards").unwrap(), targets);
    assert!(raid.toggle(&targets, "olm").is_err());
    assert!(raid.toggle(&targets, "unknown").is_err());
    for room in raid.rooms.iter().filter(|r| !r.required) {
        targets = raid.toggle(&targets, &room.id).unwrap();
    }
    assert_eq!(
        targets.iter().map(|t| t.id).collect::<Vec<_>>(),
        [7550, 7552, 7551]
    );
    assert_eq!(raids::for_targets(&targets).unwrap().id, "cox");
}

#[test]
fn archive_forks_replace_rooms_and_preserve_all_final_phases() {
    let raid = raids::raid("tfa").unwrap();
    let mut targets = raid.default_targets();
    for (choice, removed, added) in [
        ("bork-smorks", vec![16652], vec![16640, 16641]),
        ("dreadnoth", vec![16667], vec![16665]),
        ("dagonhai", vec![16684], vec![16643, 16645, 16647]),
    ] {
        targets = raid.toggle(&targets, choice).unwrap();
        assert!(
            removed
                .iter()
                .all(|id| !targets.iter().any(|t| t.id == *id))
        );
        assert!(added.iter().all(|id| targets.iter().any(|t| t.id == *id)));
        assert_eq!(
            raid.toggle(&targets, choice).unwrap(),
            targets,
            "selected fork cannot be emptied"
        );
        assert_eq!(raids::for_targets(&targets).unwrap().id, "tfa");
    }
    assert!(raid.toggle(&targets, "archive-guardian").is_err());
    assert!(raid.toggle(&targets, "ocellus").is_err());
    let setup = ironbound::share::Setup {
        sailing: Default::default(),
        wilderness: None,
        exclude_void: false,
        switch_tolerance: ironbound::efficient::DEFAULT_TOLERANCE_PERCENT,
        damage_tolerance: 0,
        prayer_tolerance: 0,
        simulation: Default::default(),
        recommended: None,
        ranking: Default::default(),
        combat: Default::default(),
        constraints: None,
        version: 1,
        boss: planner::Boss::Custom,
        raid_level: 0,
        targets: targets.clone(),
        equipment: vec![None; planner::SLOTS.len()],
        inventory: vec![],
        switches: targets
            .iter()
            .map(|target| ironbound::share::Switch {
                style: target.style.clone(),
                equipment: vec![None; planner::SLOTS.len()],
            })
            .collect(),
    };
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
        .unwrap()
        .targets;
    assert_eq!(restored, targets);
    for id in [16673, 16675, 16677] {
        assert!(restored.iter().any(|t| t.id == id));
    }
    assert_eq!(raids::for_targets(&restored).unwrap().id, "tfa");
}

#[test]
fn every_archive_fork_combination_is_a_five_boss_route() {
    let raid = raids::raid("tfa").unwrap();
    for first in ["ezekiel", "bork-smorks"] {
        for second in ["moia", "dreadnoth"] {
            for third in ["xarth", "dagonhai"] {
                let targets = [first, second, third]
                    .iter()
                    .fold(raid.default_targets(), |targets, id| {
                        raid.toggle(&targets, id).unwrap()
                    });
                assert_eq!(
                    raid.rooms.iter().filter(|r| r.selected(&targets)).count(),
                    5
                );
                assert_eq!(raids::for_targets(&targets).unwrap().id, "tfa");
            }
        }
    }
}

#[test]
fn incomplete_conflicting_duplicate_or_unrelated_targets_are_not_routes() {
    for raid in raids::raids() {
        let defaults = raid.default_targets();
        let mut missing_final = defaults.clone();
        missing_final.pop();
        assert!(raids::for_targets(&missing_final).is_none());
        assert!(raid.toggle(&missing_final, &raid.rooms[0].id).is_err());
        let mut extra = defaults.clone();
        extra.push(Target {
            id: 5779,
            style: "Melee".into(),
        });
        assert!(raids::for_targets(&extra).is_none());
        let mut duplicate = defaults.clone();
        duplicate.push(defaults[0].clone());
        assert!(raids::for_targets(&duplicate).is_none());
    }
    let archive = raids::raid("tfa").unwrap();
    let mut conflict = archive.default_targets();
    conflict.extend(
        archive
            .rooms
            .iter()
            .find(|r| r.id == "dagonhai")
            .unwrap()
            .targets
            .clone(),
    );
    assert!(raids::for_targets(&conflict).is_none());
    let mut partial_room = raids::raid("cox").unwrap().default_targets();
    partial_room.retain(|t| t.id != 7527);
    assert!(raids::for_targets(&partial_room).is_none());
    let mut no_fork = archive.default_targets();
    no_fork.retain(|t| t.id != 16667);
    assert!(raids::for_targets(&no_fork).is_none());
    assert!(raids::for_targets(&[]).is_none());
    assert!(
        raids::for_targets(&[Target {
            id: -1,
            style: "Melee".into()
        }])
        .is_none()
    );
}

#[test]
fn all_archive_room_targets_have_recorded_outgoing_probes() {
    for target in raids::raid("tfa")
        .unwrap()
        .rooms
        .iter()
        .flat_map(|r| &r.targets)
    {
        assert!(
            encounters::limitation(planner::monster(target.id).unwrap()).is_none() && encounters::window(target.id).is_some_and(|w|w.comparison_probe&&w.hp_probe.is_some()),
            "{}",
            target.id
        );
    }
}

#[test]
fn selected_routes_roundtrip_the_public_shared_setup_from_a_real_plan() {
    let demo = ironbound::account::demo();
    for (id, choices) in [
        ("cox", vec!["vasa"]),
        ("tfa", vec!["bork-smorks", "dreadnoth", "dagonhai"]),
    ] {
        let raid = raids::raid(id).unwrap();
        let targets = choices
            .iter()
            .fold(raid.default_targets(), |targets, choice| {
                raid.toggle(&targets, choice).unwrap()
            });
        let options = planner::Options {
            boss: planner::Boss::Custom,
            targets: targets.clone(),
            food: 0,
            prayer_potions: 0,
            max_switches: Some(4),
            switch_slots: 8,
            ..Default::default()
        };
        let plan = planner::plan(&demo, &options);
        let setup = ironbound::share::Setup::from_plan(&plan, &options);
        let fragment = setup
            .fragment()
            .expect("selected route should be shareable");
        let restored = ironbound::share::Setup::parse(&fragment).unwrap();
        assert_eq!(restored.targets, targets);
        assert_eq!(raids::for_targets(&restored.targets).unwrap().id, id);
    }
}

#[test]
fn invalid_attack_styles_cannot_be_recognized_as_a_raid() {
    let raid = raids::raid("cox").unwrap();
    let mut targets = raid.default_targets();
    targets[0].style = "Invalid style".into();
    assert!(raids::for_targets(&targets).is_none());
    assert!(raid.toggle(&targets, "vasa").is_err());
}
