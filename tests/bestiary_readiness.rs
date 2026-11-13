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
fn bankless_accounts_struggle_but_slayer_locks_are_requirements_missing() {
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
    assert_eq!(hydra.readiness, Readiness::RequirementsMissing);
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
            wilderness: false,
            preview: None,
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

fn strong_account() -> account::Snapshot {
    let mut account = account::demo();
    for skill in account::SKILLS {
        account.members[0].level_overrides.insert(skill.into(), 99);
    }
    account
}

#[test]
fn all_reviewed_bank_combat_entries_have_a_readiness_guideline() {
    let rows = bestiary::assess_catalog(&strong_account(), &Options::default());
    for (entry, row) in encounters::catalog().iter().zip(&rows) {
        if entry.bank_equipment
            && !entry.targets.is_empty()
            && entry.targets.iter().all(|t| {
                ironbound::planner::monster(t.id)
                    .is_some_and(|m| encounters::limitation(m).is_none())
            })
        {
            assert_ne!(
                row.readiness,
                Readiness::Unknown,
                "{}: {}",
                entry.name,
                row.reason
            );
        }
    }
    for name in [
        "Brutus",
        "Corporeal Beast",
        "Dagannoth Kings",
        "Dagannoth Prime",
        "Dagannoth Supreme",
        "Fight Caves",
        "Lunar Chests",
        "Mimic",
        "Skotizo",
        "Tombs of Amascut",
        "TzTok-Jad",
        "Yama",
    ] {
        let index = encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap();
        assert_ne!(rows[index].readiness, Readiness::Unknown, "{name}");
        assert!(!rows[index].detail.is_empty());
    }
}

#[test]
fn nightmare_screens_require_both_melee_and_magic_and_keep_encounter_scope() {
    let account = strong_account();
    for name in ["Nightmare", "Phosani's Nightmare"] {
        let row = result(name, &account, &Options::default());
        assert_ne!(row.readiness, Readiness::Unknown, "{name}: {}", row.reason);
        assert!(
            ["Melee", "Stab", "Slash", "Crush"]
                .iter()
                .any(|style| row.reason.contains(style))
                && row.reason.contains("Magic"),
            "{}",
            row.reason
        );
        assert!(row.detail.contains("full-trip supplies"));
    }
    let mut options = Options::default();
    for id in account.available(false).keys() {
        if ironbound::rules::weapon(*id).is_some_and(|w| w.family.style == "Magic") {
            options.excluded.insert(*id);
        }
    }
    for name in ["Nightmare", "Phosani's Nightmare"] {
        let row = result(name, &account, &options);
        assert_eq!(row.readiness, Readiness::Struggle);
        assert!(
            row.reason.contains("No usable owned Magic"),
            "{}",
            row.reason
        );
    }
}

#[test]
fn tribrid_and_yama_ratings_require_every_mandatory_style() {
    let account = strong_account();
    for (style, bosses) in [
        ("Ranged", vec!["Dagannoth Kings", "Tombs of Amascut"]),
        ("Magic", vec!["Dagannoth Kings", "Tombs of Amascut", "Yama"]),
        ("Melee", vec!["Dagannoth Kings", "Tombs of Amascut", "Yama"]),
    ] {
        let mut options = Options::default();
        for id in account.available(false).keys() {
            if ironbound::rules::weapon(*id).is_some_and(|w| w.family.style == style) {
                options.excluded.insert(*id);
            }
        }
        let rows = bestiary::assess_catalog(&account, &options);
        for name in bosses {
            let index = encounters::catalog()
                .iter()
                .position(|e| e.name == name)
                .unwrap();
            assert_eq!(rows[index].readiness, Readiness::Struggle, "{name}/{style}");
            assert!(
                rows[index]
                    .reason
                    .contains(&format!("No usable owned {style}")),
                "{name}: {}",
                rows[index].reason
            );
        }
    }
}

#[test]
fn jad_and_waves_check_prayer_and_real_ammunition() {
    let mut account = strong_account();
    account.members[0]
        .level_overrides
        .insert("Prayer".into(), 42);
    let rows = bestiary::assess_catalog(&account, &Options::default());
    for name in ["TzTok-Jad", "Fight Caves"] {
        let index = encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap();
        assert_eq!(rows[index].readiness, Readiness::Struggle);
        assert!(rows[index].reason.contains("43 Prayer"));
    }
    let account = strong_account();
    let options = Options {
        excluded: account
            .available(false)
            .keys()
            .filter(|id| {
                ironbound::inventory::category(**id, &ironbound::planner::item_name(**id))
                    == ironbound::inventory::Category::Ammunition
            })
            .copied()
            .collect(),
        ..Default::default()
    };
    let jad = result("TzTok-Jad", &account, &options);
    assert_eq!(jad.readiness, Readiness::Struggle);
    assert!(jad.reason.contains("No usable owned setup"));
}

#[test]
fn excluded_runes_are_unavailable_to_new_magic_roles() {
    let account = strong_account();
    let mut options = Options::default();
    for id in account.available(false).keys() {
        if ironbound::inventory::category(*id, &ironbound::planner::item_name(*id))
            == ironbound::inventory::Category::Runes
            || ironbound::rules::weapon(*id)
                .is_some_and(|w| w.family.formula == ironbound::rules::Formula::Powered)
        {
            options.excluded.insert(*id);
        }
    }
    let toa = result("Tombs of Amascut", &account, &options);
    assert_eq!(toa.readiness, Readiness::Struggle);
    assert!(
        toa.reason.contains("No usable owned Magic"),
        "{}",
        toa.reason
    );
}

#[test]
fn aliases_agree_and_unreviewed_modes_explain_the_blocker() {
    let account = strong_account();
    let rows = bestiary::assess_catalog(&account, &Options::default());
    let row = |name| {
        &rows[encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap()]
    };
    assert_eq!(
        row("Lunar Chests").readiness,
        row("Moons of Peril").readiness
    );
    assert_eq!(row("Lunar Chests").reason, row("Moons of Peril").reason);
    for name in [
        "The Royal Titans",
        "Nex",
        "Theatre of Blood",
        "The Fractured Archive",
        "Phantom Muspah",
    ] {
        assert_eq!(row(name).readiness, Readiness::Unknown, "{name}");
        assert!(row(name).reason.contains("Unreviewed phases"));
        assert!(
            row(name)
                .detail
                .contains("Readiness needs reviewed calculations")
        );
    }
    assert!(row("The Gauntlet").reason.contains("equipment or skilling"));
    assert!(
        row("Tombs of Amascut: Expert Mode")
            .reason
            .contains("scaling")
    );
    assert!(row("Tombs of Amascut").detail.contains("raid-level-0"));
    assert!(
        row("Tombs of Amascut")
            .reason
            .contains("Solo · raid level 0")
    );
    assert!(row("Fight Caves").detail.contains("63 waves"));
    assert!(row("Yama").detail.contains("Hybrid baseline"));
    assert!(row("Yama").reason.contains("Melee-boss hybrid"));
    assert!(bestiary::FILTERS.contains(&"Not assessed"));
}

#[test]
fn kraken_at_81_slayer_is_locked_without_a_combat_difficulty_rating() {
    let mut account = strong_account();
    account.members[0]
        .level_overrides
        .insert("Slayer".into(), 81);
    let a = result("Kraken", &account, &Options::default());
    assert_eq!(a.readiness, Readiness::RequirementsMissing);
    assert_eq!(a.reason, "Needs 87 Slayer (yours: 81).");
    assert!(
        a.detail
            .contains("Reach 87 Slayer to clear this entry requirement")
    );
    let index = encounters::catalog()
        .iter()
        .position(|e| e.name == "Kraken")
        .unwrap();
    let html = bestiary::card(&encounters::catalog()[index], index, &a);
    assert!(html.contains("readiness-requirements"));
    assert!(html.contains("Requirements missing"));
    assert!(!html.contains("A struggle"));
    assert!(!html.contains("readiness-struggle"));
    assert!(bestiary::FILTERS.contains(&"Requirements missing"));
}

#[test]
fn every_reviewed_slayer_gate_changes_to_requirements_missing_below_its_level() {
    for (name, minimum) in [
        ("Grotesque Guardians", 75),
        ("Abyssal Sire", 85),
        ("Kraken", 87),
        ("Cerberus", 91),
        ("Araxxor", 92),
        ("Thermonuclear Smoke Devil", 93),
        ("Alchemical Hydra", 95),
    ] {
        let mut account = strong_account();
        account.members[0]
            .level_overrides
            .insert("Slayer".into(), minimum - 1);
        let below = result(name, &account, &Options::default());
        assert_eq!(below.readiness, Readiness::RequirementsMissing, "{name}");
        assert!(below.reason.contains(&format!("Needs {minimum} Slayer")));
        account.members[0]
            .level_overrides
            .insert("Slayer".into(), minimum);
        let unlocked = result(name, &account, &Options::default());
        assert_ne!(unlocked.readiness, Readiness::RequirementsMissing, "{name}");
        // Sire still has an unreviewed opening phase; unlocking Slayer does
        // not certify those calculations or its task/access requirements.
        if name == "Abyssal Sire" {
            assert_eq!(unlocked.readiness, Readiness::Unknown);
        }
    }
}

#[test]
fn unknown_slayer_is_unassessed_and_recommendations_are_separate_from_entry_gates() {
    let mut account = strong_account();
    account.members[0].skills = None;
    account.members[0].level_overrides.remove("Slayer");
    let kraken = result("Kraken", &account, &Options::default());
    assert_eq!(kraken.readiness, Readiness::Unknown);
    assert!(kraken.reason.contains("Add your Slayer level"));
    account.members[0]
        .level_overrides
        .insert("Prayer".into(), 42);
    let jad = result("TzTok-Jad", &account, &Options::default());
    assert_eq!(jad.readiness, Readiness::Struggle);
    assert!(jad.reason.contains("recommended"));
    assert!(Readiness::Struggle < Readiness::RequirementsMissing);
    assert!(Readiness::RequirementsMissing < Readiness::Unknown);
}
