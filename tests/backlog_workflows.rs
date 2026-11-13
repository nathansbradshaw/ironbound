//! Cross-feature contracts, separate from independently captured combat goldens.
use ironbound::{
    account, coverage,
    defence::Settings,
    export,
    planner::{self, Boss, Options, Target},
    preparation, rules,
    setups::{Library, Saved},
    share,
};
use std::collections::BTreeMap;

fn saved(options: Options) -> Saved {
    Saved {
        death_risk: Default::default(),
        version: 1,
        snapshot: account::demo(),
        options,
        pins: BTreeMap::new(),
        last_setup_fragment: None,
        setups: Library::default(),
        budget_caps: ironbound::setups::default_budget_caps(),
    }
}

#[test]
fn old_account_backup_migrates_without_losing_bank_or_current_options() {
    let mut original = saved(Options::default());
    original.budget_caps = [3, 7];
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("setups");
    legacy["options"]
        .as_object_mut()
        .unwrap()
        .remove("confirmations");
    legacy["options"]["combat"]
        .as_object_mut()
        .unwrap()
        .remove("spell_casts");
    let restored = Saved::parse(&legacy.to_string()).unwrap();
    assert_eq!(
        restored.snapshot.available(false),
        original.snapshot.available(false)
    );
    assert_eq!(restored.options.combat.spell_casts, 150);
    assert!(restored.setups.0.is_empty());
    assert_eq!(restored.options.food, original.options.food);
    assert_eq!(restored.budget_caps, [3, 7]);
    legacy.as_object_mut().unwrap().remove("budget_caps");
    assert_eq!(
        Saved::parse(&legacy.to_string()).unwrap().budget_caps,
        [4, 8]
    );
}

#[test]
fn private_library_round_trip_preserves_packing_budget_and_public_export_excludes_library() {
    let mut state = saved(Options::default());
    state.options.combat.spell_casts = 30;
    state.options.inventory_overrides.insert(27, 560);
    state.options.inventory_quantities.insert(27, 73);
    let token = preparation::token(&state.snapshot, &state.options);
    state.options.confirmations.insert("charges".into(), token);
    state
        .setups
        .save("Learning with private name", &state.options)
        .unwrap();
    let restored = Saved::parse(&serde_json::to_string(&state).unwrap()).unwrap();
    let options = &restored.setups.0[0].options;
    let plan = planner::plan(&restored.snapshot, options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(
        (plan.inventory[27].id, plan.inventory[27].quantity),
        (560, 73)
    );
    assert!(preparation::confirmed(
        &restored.snapshot,
        options,
        "charges"
    ));
    let json: serde_json::Value =
        serde_json::from_str(&export::inventory_setup(&plan, options, "private").unwrap()).unwrap();
    assert_eq!(json["setup"]["inv"][27]["q"], 73);
    assert!(
        json["setup"]["notes"]
            .as_str()
            .unwrap()
            .contains("30 casts")
    );
    let public = share::Setup::from_plan(&plan, options);
    let raw = serde_json::to_string(&public).unwrap();
    for secret in [
        "Learning with private name",
        "Iron Adventurer",
        "confirmations",
        "bank",
        "setups",
    ] {
        assert!(!raw.contains(secret), "leaked {secret}");
    }
    let public = share::Setup::parse(&public.fragment().unwrap()).unwrap();
    assert_eq!(public.options().combat.spell_casts, 30);
    assert!(public.options().confirmations.is_empty());
    assert_eq!(public.plan().inventory[27].quantity, 73);
}

#[test]
fn library_rejects_duplicates_invalid_rows_and_overwrites_only_explicitly() {
    let mut library = Library::default();
    library.save("  Learning  ", &Options::default()).unwrap();
    assert!(library.save("learning", &Options::default()).is_err());
    assert!(library.rename(0, "\n").is_err());
    library.rename(0, "Fewer switches").unwrap();
    let mut next = Options::default();
    next.max_switches = Some(0);
    library.replace(0, &next).unwrap();
    assert_eq!(library.0[0].options.max_switches, Some(0));
    library.0[0].options.inventory_overrides.insert(28, 385);
    assert!(library.validate().is_err());
    assert!(Saved::parse("{broken").is_err());
    let mut corrupt = saved(Options::default());
    corrupt.setups = library;
    assert!(Saved::parse(&serde_json::to_string(&corrupt).unwrap()).is_err());
}

#[test]
fn loaded_configuration_rechecks_current_bank_instead_of_trusting_old_ownership() {
    let state = saved(Options::default());
    let mut library = Library::default();
    library.save("Before bank refresh", &state.options).unwrap();
    let mut empty = state.snapshot.clone();
    empty.members[0].bank = Some(vec![]);
    let plan = planner::plan(&empty, &library.0[0].options);
    assert!(!plan.exportable());
    assert!(!plan.missing.is_empty());
}

#[test]
fn confirmations_invalidate_on_real_changes_and_never_replace_known_failures() {
    let mut state = saved(Options::default());
    for (id, _) in preparation::CONFIRMATIONS {
        let token = preparation::token(&state.snapshot, &state.options);
        state.options.confirmations.insert(id.into(), token);
    }
    assert_eq!(preparation::pending(&state.snapshot, &state.options), 0);
    let mut changed = state.options.clone();
    changed.combat.spell_casts += 1;
    assert_eq!(preparation::pending(&state.snapshot, &changed), 4);
    changed = state.options.clone();
    changed.gear_overrides.insert("Magic:3".into(), 1381);
    assert_eq!(preparation::pending(&state.snapshot, &changed), 4);
    let mut refreshed = state.snapshot.clone();
    refreshed.members[0]
        .level_overrides
        .insert("Magic".into(), 1);
    assert_eq!(preparation::pending(&refreshed, &state.options), 4);
    let mut plan = planner::plan(&state.snapshot, &state.options);
    plan.missing.push("Required switch missing".into());
    assert_eq!(
        preparation::summary(&state.snapshot, &state.options, &plan),
        "Preparation needed"
    );
}

#[test]
fn short_spell_budget_changes_affordability_selection_and_packed_runes_consistently() {
    let mut state = saved(Options::default());
    for skill in account::SKILLS {
        state.snapshot.members[0]
            .level_overrides
            .insert(skill.into(), 99);
    }
    state.snapshot.members[0].bank =
        Some(vec![1381, 1, 565, 30, 555, 210, 385, 40, 2434, 12, 8007, 1]);
    state.options.boss = Boss::Custom;
    state.options.targets = vec![Target {
        id: 415,
        style: "Magic".into(),
    }];
    state.options.gear_overrides.insert("Magic:3".into(), 1381);
    state.options.combat.spell_casts = 30;
    let set = vec![
        None,
        None,
        None,
        Some(1381),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ];
    let member = state.snapshot.selected().unwrap();
    let stock = state.snapshot.available(false);
    assert_eq!(
        rules::elemental_candidates(member, &set, &stock, &state.options.combat).len(),
        2
    );
    assert!(rules::casting_supported_configured(
        member,
        &set,
        &stock,
        30,
        &state.options.combat
    ));
    let plan = planner::plan(&state.snapshot, &state.options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert!(
        plan.inventory
            .iter()
            .any(|p| p.id == 565 && p.quantity == 30)
    );
    assert!(
        ironbound::tradeoffs::measure(&state.snapshot, &state.options, &set, "Magic", 415)
            .dps
            .is_some()
    );
    state.options.combat.spell_casts = 150;
    assert!(
        ironbound::tradeoffs::measure(&state.snapshot, &state.options, &set, "Magic", 415)
            .dps
            .is_none()
    );
    assert!(rules::elemental_candidates(member, &set, &stock, &state.options.combat).is_empty());
    assert!(!planner::plan(&state.snapshot, &state.options).exportable());
    for casts in [0, 10001, u32::MAX] {
        assert!(
            !Settings {
                spell_casts: casts,
                ..Default::default()
            }
            .valid()
        );
    }
}

#[test]
fn gear_changes_preserve_manual_positions_and_quantities_while_conflicts_block_exports() {
    let mut state = saved(Options::default());
    state.options.inventory_overrides.insert(27, 560);
    state.options.inventory_quantities.insert(27, 73);
    state.options.gear_overrides.insert("Magic:3".into(), 1381);
    let plan = planner::plan(&state.snapshot, &state.options);
    assert_eq!(
        (plan.inventory[27].id, plan.inventory[27].quantity),
        (560, 73)
    );
    state.options.inventory_quantities.insert(27, 1_000_000);
    let conflict = planner::plan(&state.snapshot, &state.options);
    assert!(!conflict.exportable());
    assert!(conflict.missing.iter().any(|m| m.contains("Not enough")));
    assert_eq!(conflict.inventory[27].quantity, 1_000_000);
    let baseline = planner::plan(&state.snapshot, &Options::default());
    let switch = baseline
        .inventory
        .iter()
        .position(|p| {
            baseline
                .phases
                .iter()
                .any(|phase| phase.equipment.contains(&Some(p.id)))
                && planner::gear(p.id).is_some()
        })
        .unwrap();
    let mut options = Options::default();
    options.inventory_overrides.insert(switch, 0);
    assert!(!planner::plan(&state.snapshot, &options).exportable());
}

#[test]
fn coverage_keeps_wardens_explicit_and_only_enables_reviewed_archive_windows() {
    let rows = coverage::windows(&Options {
        boss: Boss::Toa,
        ..Default::default()
    });
    assert!(rows.iter().any(|r| r.name == "Wardens" && !r.supported));
    for entry in ironbound::encounters::catalog()
        .iter()
        .filter(|e| !e.targets.is_empty())
    {
        let options = Options {
            boss: Boss::Custom,
            targets: entry.targets.clone(),
            ..Default::default()
        };
        let rows = coverage::windows(&options);
        assert_eq!(rows.len(), options.targets.len());
        for (row, target) in rows.iter().zip(&options.targets) {
            assert_eq!(
                row.supported,
                ironbound::encounters::limitation(planner::monster(target.id).unwrap()).is_none()
            );
        }
    }
    let archive = ironbound::encounters::catalog()
        .iter()
        .find(|e| e.name == "The Fractured Archive")
        .unwrap();
    assert!(
        coverage::windows(&Options {
            boss: Boss::Custom,
            targets: archive.targets.clone(),
            ..Default::default()
        })
        .iter()
        .filter(|r| r.supported)
        .count()
            == 6
    );
}
