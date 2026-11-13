use ironbound::{
    account,
    bestiary::{
        self, Assessment, GearFilter, Readiness, RequirementStatus, Requirements,
        RequirementsFilter,
    },
    planner::Options,
};
fn a(gear: Readiness, requirements: RequirementStatus) -> Assessment {
    Assessment {
        readiness: gear,
        requirements: Requirements {
            status: requirements,
            reason: String::new(),
        },
        reason: String::new(),
        detail: String::new(),
        wilderness: false,
        preview: None,
    }
}
#[test]
fn every_gear_and_requirement_combination_is_an_intersection() {
    for gear in [
        Readiness::WellEquipped,
        Readiness::Doable,
        Readiness::Struggle,
        Readiness::Unknown,
    ] {
        for status in [
            RequirementStatus::Missing,
            RequirementStatus::LevelMet,
            RequirementStatus::NoLevelGate,
            RequirementStatus::Unverified,
        ] {
            let row = a(gear, status);
            for gf in GearFilter::ALL {
                for rf in RequirementsFilter::ALL {
                    let gear_ok = match gf {
                        GearFilter::Any => true,
                        GearFilter::WellEquipped => gear == Readiness::WellEquipped,
                        GearFilter::Doable => gear == Readiness::Doable,
                        GearFilter::Struggle => gear == Readiness::Struggle,
                        GearFilter::Unknown => gear == Readiness::Unknown,
                    };
                    let req_ok = match rf {
                        RequirementsFilter::Any => true,
                        RequirementsFilter::LevelsMet => matches!(
                            status,
                            RequirementStatus::LevelMet | RequirementStatus::NoLevelGate
                        ),
                        RequirementsFilter::Missing => status == RequirementStatus::Missing,
                        RequirementsFilter::Unknown => status == RequirementStatus::Unverified,
                    };
                    assert_eq!(
                        bestiary::matches_filters(&row, gf, rf),
                        gear_ok && req_ok,
                        "{gear:?}/{status:?}/{gf:?}/{rf:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn locked_well_equipped_kraken_is_excluded_until_the_level_is_met() {
    let mut account = account::demo();
    for skill in account::SKILLS {
        account.members[0].level_overrides.insert(skill.into(), 99);
    }
    account.members[0]
        .level_overrides
        .insert("Slayer".into(), 81);
    let index = ironbound::encounters::catalog()
        .iter()
        .position(|e| e.name == "Kraken")
        .unwrap();
    let before = bestiary::assess_catalog(&account, &Options::default())[index].clone();
    assert!(bestiary::matches_filters(
        &before,
        GearFilter::WellEquipped,
        RequirementsFilter::Missing
    ));
    assert!(!bestiary::matches_filters(
        &before,
        GearFilter::WellEquipped,
        RequirementsFilter::LevelsMet
    ));
    account.members[0]
        .level_overrides
        .insert("Slayer".into(), 87);
    let after = bestiary::assess_catalog(&account, &Options::default())[index].clone();
    assert!(bestiary::matches_filters(
        &after,
        GearFilter::WellEquipped,
        RequirementsFilter::LevelsMet
    ));
    assert_eq!(before.readiness, after.readiness);
    assert!(
        after
            .requirements
            .reason
            .contains("access checks remain unverified")
    );
}
#[test]
fn no_recorded_gate_and_unknown_level_data_are_different() {
    let mut account = account::demo();
    account.members[0].skills = None;
    account.members[0].level_overrides.clear();
    let rows = bestiary::assess_catalog(&account, &Options::default());
    let index = |name: &str| {
        ironbound::encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap()
    };
    assert!(RequirementsFilter::LevelsMet.matches(&rows[index("Scurrius")]));
    assert!(
        rows[index("Scurrius")]
            .requirements
            .reason
            .contains("No recorded Slayer")
    );
    assert!(!RequirementsFilter::LevelsMet.matches(&rows[index("Kraken")]));
    assert!(RequirementsFilter::Unknown.matches(&rows[index("Kraken")]));
    assert!(
        !RequirementsFilter::LevelsMet
            .matches(&a(Readiness::Unknown, RequirementStatus::Unverified))
    );
}
#[test]
fn filter_parsers_reject_unknown_values() {
    for f in GearFilter::ALL {
        assert_eq!(GearFilter::parse(f.label()), Some(f));
    }
    for f in RequirementsFilter::ALL {
        assert_eq!(RequirementsFilter::parse(f.label()), Some(f));
    }
    assert!(GearFilter::parse("Unlocked").is_none());
    assert!(RequirementsFilter::parse("Unlocked").is_none());
}
