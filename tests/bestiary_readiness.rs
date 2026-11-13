use ironbound::{
    account,
    bestiary::{self, Readiness},
    encounters,
    planner::Options,
};
fn result(name: &str, account: &account::Snapshot, options: &Options) -> bestiary::Assessment {
    let index = encounters::catalog()
        .iter()
        .position(|e| e.name == name)
        .unwrap();
    bestiary::assess_catalog(account, options)[index].clone()
}
#[test]
fn missing_levels_and_unsupported_raids_are_not_called_ready() {
    let mut account = account::demo();
    account.members[0].skills = None;
    account.members[0].level_overrides.clear();
    assert!(
        bestiary::assess_catalog(&account, &Options::default())
            .iter()
            .all(|a| a.readiness == Readiness::Unknown)
    );
    assert_eq!(
        result("Theatre of Blood", &account::demo(), &Options::default()).readiness,
        Readiness::Unknown
    );
}
#[test]
fn bankless_and_level_locked_accounts_struggle() {
    let mut account = account::demo();
    for member in &mut account.members {
        member.bank = None;
        member.equipment = None;
        member.inventory = None;
        member.shared_bank = None;
    }
    assert_eq!(
        result("Scurrius", &account, &Options::default()).readiness,
        Readiness::Struggle
    );
    let hydra = result("Alchemical Hydra", &account::demo(), &Options::default());
    assert_eq!(hydra.readiness, Readiness::Struggle);
    assert!(hydra.reason.contains("95 Slayer"));
}
#[test]
fn lower_levels_and_exclusions_cannot_earn_the_top_band() {
    let mut account = account::demo();
    for skill in [
        "Attack",
        "Strength",
        "Defence",
        "Magic",
        "Ranged",
        "Hitpoints",
        "Prayer",
    ] {
        account.members[0].level_overrides.insert(skill.into(), 1);
    }
    assert_eq!(
        result("Scurrius", &account, &Options::default()).readiness,
        Readiness::Struggle
    );
    let account = account::demo();
    let options = Options {
        excluded: account.available(false).keys().copied().collect(),
        ..Default::default()
    };
    assert_eq!(
        result("Scurrius", &account, &options).readiness,
        Readiness::Struggle
    );
}
#[test]
fn assessments_are_independent_of_active_boss_and_boost_settings() {
    let account = account::demo();
    let base = bestiary::assess_catalog(&account, &Options::default());
    let mut options = Options {
        boss: ironbound::planner::Boss::Toa,
        raid_level: 150,
        ..Default::default()
    };
    options.gear_overrides.insert("Magic:3".into(), 0);
    options.combat.mark_darkness = true;
    let other = bestiary::assess_catalog(&account, &options);
    assert_eq!(
        base.iter()
            .map(|a| (&a.reason, a.readiness))
            .collect::<Vec<_>>(),
        other
            .iter()
            .map(|a| (&a.reason, a.readiness))
            .collect::<Vec<_>>()
    );
}
#[test]
fn real_portraits_stats_and_escaped_account_text_render() {
    let entry = encounters::catalog()
        .iter()
        .find(|e| e.name == "Scurrius")
        .unwrap();
    let html = bestiary::card(
        entry,
        0,
        &bestiary::Assessment {
            readiness: Readiness::Doable,
            reason: "<script>".into(),
            detail: "A & B".into(),
        },
    );
    assert!(html.contains("data:image/png;base64,"));
    assert!(html.contains("HP</span>"));
    assert!(html.contains("readiness-doable"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("A &amp; B"));
}
#[test]
fn print_demo_screening_for_review() {
    let results = bestiary::assess_catalog(&account::demo(), &Options::default());
    for (entry, result) in encounters::catalog().iter().zip(results) {
        if result.readiness != Readiness::Unknown {
            println!("{}: {:?}: {}", entry.name, result.readiness, result.reason);
        }
    }
}

#[test]
fn a_strong_owned_account_can_be_ready_and_mixed_styles_need_both_sets() {
    let mut account = account::demo();
    for skill in [
        "Attack",
        "Strength",
        "Defence",
        "Magic",
        "Ranged",
        "Hitpoints",
        "Prayer",
    ] {
        account.members[0].level_overrides.insert(skill.into(), 99);
    }
    assert_eq!(
        result("Scurrius", &account, &Options::default()).readiness,
        Readiness::WellEquipped
    );
    let mut options = Options::default();
    for id in account.available(false).keys() {
        if ironbound::rules::weapon(*id).is_some_and(|w| w.family.style == "Ranged") {
            options.excluded.insert(*id);
        }
    }
    let barrows = result("Barrows Chests", &account, &options);
    assert_eq!(barrows.readiness, Readiness::Struggle);
    assert!(barrows.reason.contains("Ranged"));
}
#[test]
fn every_catalog_entry_has_a_real_portrait() {
    for entry in encounters::catalog() {
        assert!(
            bestiary::portrait(entry).contains("data:image/png;base64,"),
            "{} has no portrait",
            entry.name
        );
    }
}
