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
fn bankless_gear_and_slayer_requirements_are_separate() {
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
    assert_eq!(
        hydra.requirements.status,
        bestiary::RequirementStatus::Missing
    );
    assert!(hydra.requirements.reason.contains("95 Slayer"));
    assert_ne!(hydra.readiness, Readiness::Unknown);
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
            requirements: Default::default(),
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
        if entry.category() == "Sailing" {
            assert_eq!(row.readiness, Readiness::Unknown);
            assert!(row.reason.contains("boat"));
            continue;
        }
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
        (
            "Ranged",
            vec!["Dagannoth Kings", "Tombs of Amascut", "The Royal Titans"],
        ),
        (
            "Magic",
            vec![
                "Dagannoth Kings",
                "Tombs of Amascut",
                "Yama",
                "The Royal Titans",
            ],
        ),
        (
            "Melee",
            vec![
                "Dagannoth Kings",
                "Tombs of Amascut",
                "Yama",
                "The Royal Titans",
            ],
        ),
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
fn kraken_at_81_slayer_shows_both_good_gear_and_the_unmet_entry_requirement() {
    let mut account = strong_account();
    account.members[0]
        .level_overrides
        .insert("Slayer".into(), 81);
    let a = result("Kraken", &account, &Options::default());
    assert_eq!(a.readiness, Readiness::WellEquipped);
    assert_eq!(a.requirements.status, bestiary::RequirementStatus::Missing);
    assert!(a.reason.contains("basic DPS"));
    assert!(
        a.requirements
            .reason
            .contains("Needs 87 Slayer (yours: 81).")
    );
    assert!(a.matches_filter("Well equipped"));
    assert!(a.matches_filter("Requirements missing"));
    assert!(!a.matches_filter("A struggle"));
    let index = encounters::catalog()
        .iter()
        .position(|e| e.name == "Kraken")
        .unwrap();
    let html = bestiary::card(&encounters::catalog()[index], index, &a);
    assert!(html.contains("Gear: Well equipped"));
    assert!(html.contains("requirement-missing"));
    assert!(html.contains("Requirements missing"));
    assert!(!html.contains("A struggle"));
    assert!(!html.contains("readiness-struggle"));
    assert!(bestiary::FILTERS.contains(&"Requirements missing"));
}

#[test]
fn changing_a_slayer_gate_does_not_change_gear_readiness_for_any_reviewed_boss() {
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
        assert_eq!(
            below.requirements.status,
            bestiary::RequirementStatus::Missing,
            "{name}"
        );
        assert!(
            below
                .requirements
                .reason
                .contains(&format!("Needs {minimum} Slayer"))
        );
        account.members[0]
            .level_overrides
            .insert("Slayer".into(), minimum);
        let unlocked = result(name, &account, &Options::default());
        assert_eq!(
            unlocked.requirements.status,
            bestiary::RequirementStatus::LevelMet,
            "{name}"
        );
        assert_eq!(below.readiness, unlocked.readiness, "{name}");
        assert_eq!(below.reason, unlocked.reason, "{name}");
        assert!(!unlocked.matches_filter("Requirements missing"));
        assert!(
            unlocked
                .requirements
                .reason
                .contains("access checks remain unverified")
        );
        // Sire still has an unreviewed opening phase; unlocking Slayer does
        // not certify those calculations or its task/access requirements.
        if name == "Abyssal Sire" {
            assert_eq!(unlocked.readiness, Readiness::Unknown);
        }
    }
}

#[test]
fn unknown_slayer_does_not_hide_a_supported_gear_rating() {
    let mut account = strong_account();
    account.members[0].skills = None;
    account.members[0].level_overrides.remove("Slayer");
    let kraken = result("Kraken", &account, &Options::default());
    assert_eq!(kraken.readiness, Readiness::WellEquipped);
    assert_eq!(
        kraken.requirements.status,
        bestiary::RequirementStatus::Unverified
    );
    assert!(kraken.requirements.reason.contains("Add your Slayer level"));
    assert!(!kraken.matches_filter("Requirements missing"));
    account.members[0]
        .level_overrides
        .insert("Prayer".into(), 42);
    let jad = result("TzTok-Jad", &account, &Options::default());
    assert_eq!(jad.readiness, Readiness::Struggle);
    assert!(jad.reason.contains("recommended"));
    assert_eq!(
        jad.requirements.status,
        bestiary::RequirementStatus::NoLevelGate
    );
}

#[test]
fn weak_or_unknown_gear_and_entry_locks_can_coexist() {
    let mut account = strong_account();
    account.members[0]
        .level_overrides
        .insert("Slayer".into(), 81);
    for member in &mut account.members {
        member.bank = None;
        member.inventory = None;
        member.equipment = None;
        member.shared_bank = None;
    }
    let weak = result("Kraken", &account, &Options::default());
    assert_eq!(weak.readiness, Readiness::Struggle);
    assert_eq!(
        weak.requirements.status,
        bestiary::RequirementStatus::Missing
    );
    assert!(weak.matches_filter("A struggle") && weak.matches_filter("Requirements missing"));
    account.members[0].skills = None;
    account.members[0].level_overrides.remove("Magic");
    let unknown = result("Kraken", &account, &Options::default());
    assert_eq!(unknown.readiness, Readiness::Unknown);
    assert_eq!(
        unknown.requirements.status,
        bestiary::RequirementStatus::Missing
    );
    assert!(
        unknown.matches_filter("Not assessed") && unknown.matches_filter("Requirements missing")
    );
}

#[test]
fn huey_and_titans_have_scoped_gear_screens_separate_from_access() {
    let account = strong_account();
    for name in ["The Hueycoatl", "The Royal Titans"] {
        let row = result(name, &account, &Options::default());
        assert_ne!(row.readiness, Readiness::Unknown);
        assert_eq!(
            row.requirements.status,
            bestiary::RequirementStatus::NoLevelGate
        );
        assert!(
            row.reason.contains("basic DPS") && !row.reason.contains("No usable"),
            "{name}: {}",
            row.reason
        );
        assert!(row.detail.contains("incoming") && row.detail.contains("not scored"));
    }
}
