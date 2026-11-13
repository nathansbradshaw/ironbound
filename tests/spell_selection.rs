use ironbound::{
    account, combat,
    defence::Settings,
    planner,
    rules::{self, SpellSelection},
};
use std::collections::BTreeMap;
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
#[test]
fn manual_element_and_spell_preserve_choices_with_missing_runes() {
    let bank = account::demo();
    let member = bank.selected().unwrap();
    let owned = bank.available(false);
    let target = planner::monster(1673).unwrap();
    let settings = Settings::default();
    let staff = set(1381);
    let choose = |choice: &SpellSelection| -> &'static str {
        let mut settings = settings.clone();
        settings.spell_selection = choice.clone();
        rules::spell_name_for_target(member, &staff, &owned, &settings, target, 0)
    };
    assert_eq!(choose(&SpellSelection::Auto), "Wind Wave");
    assert_eq!(
        choose(&SpellSelection::Element("water".into())),
        "Water Wave"
    );
    let wave = choose(&SpellSelection::Spell("Water Wave".into()));
    let strike = choose(&SpellSelection::Spell("Water Strike".into()));
    let a = combat::calculate(member, &staff, "Magic", target, wave).unwrap();
    let b = combat::calculate(member, &staff, "Magic", target, strike).unwrap();
    assert_eq!((a.max_hit, b.max_hit), (20, 8));
    assert!(a.dps > b.dps);
    let reason = rules::spell_requirement(member, &staff, &owned, &settings, wave).unwrap();
    assert!(
        reason.contains("Water rune ×1050") && reason.contains("Prepare these runes"),
        "{reason}"
    );
    assert_eq!(choose(&SpellSelection::Auto), "Wind Wave");
}
#[test]
fn selection_respects_weapon_families_levels_boosts_and_staff_runes() {
    let mut member = account::demo().selected().unwrap().clone();
    let target = planner::monster(1673).unwrap();
    let owned = BTreeMap::from([(565, 150), (555, 1050)]);
    let staff = set(1381);
    let mut settings = Settings::default();
    assert!(rules::spell_requirement(&member, &staff, &owned, &settings, "Water Wave").is_none());
    assert!(
        rules::spell_requirement(&member, &staff, &owned, &settings, "Fire Surge")
            .unwrap()
            .contains("95 Magic")
    );
    let powered = set(11907);
    settings.spell_selection = SpellSelection::Spell("Water Wave".into());
    assert_eq!(
        rules::spell_name_for_target(&member, &powered, &owned, &settings, target, 0),
        "Water Wave"
    );
    assert!(
        combat::calculate_configured(
            &member,
            &powered,
            "Magic",
            target,
            "Water Wave",
            0,
            &settings
        )
        .unwrap_err()
        .contains("cannot cast")
    );
    assert!(
        rules::spell_requirement(&member, &powered, &owned, &settings, "Water Wave")
            .unwrap()
            .contains("compatible")
    );
    member.level_overrides.insert("Magic".into(), 61);
    settings.boosts.magic = "Magic potion".into();
    settings.spell_selection = SpellSelection::Element("water".into());
    let name = rules::spell_name_for_target(&member, &staff, &owned, &settings, target, 0);
    assert_eq!(name, "Water Wave");
    assert_eq!(
        combat::calculate_configured(&member, &staff, "Magic", target, name, 0, &settings)
            .unwrap()
            .max_hit,
        17
    );
    assert!(rules::spell_requirement(&member, &staff, &owned, &settings, name).is_none());
}
#[test]
fn every_supported_spell_and_prayer_has_a_bundled_game_icon() {
    let icons: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../assets/book-icons.json")).unwrap();
    for name in rules::spells()
        .iter()
        .map(|s| s.name.as_str())
        .chain(
            ironbound::prayer::prayers()
                .iter()
                .filter(|p| p.name != "None")
                .map(|p| p.name.as_str()),
        )
        .chain([
            "Preserve",
            "Protect from Magic",
            "Protect from Missiles",
            "Protect from Melee",
        ])
    {
        assert!(
            icons
                .get(name)
                .is_some_and(|data| data.starts_with("data:image/png;base64,iVBORw0KGgo")),
            "missing PNG for {name}"
        );
    }
}

fn manual_plan(spell: &str) -> (account::Snapshot, planner::Options) {
    let mut bank = account::demo();
    bank.members[0]
        .bank
        .as_mut()
        .unwrap()
        .extend([555, 1050, 558, 150]);
    let mut options = planner::Options {
        boss: planner::Boss::Barrows,
        ..Default::default()
    };
    options.gear_overrides.insert("Magic:3".into(), 1381);
    options.combat.spell_selection = SpellSelection::Spell(spell.into());
    (bank, options)
}
fn quantity(plan: &planner::Plan, id: i32) -> u32 {
    plan.inventory
        .iter()
        .filter(|p| p.id == id)
        .map(|p| p.quantity)
        .sum()
}
#[test]
fn exact_spell_drives_plan_runes_and_cast_budget() {
    let (bank, mut options) = manual_plan("Water Wave");
    let wave = planner::plan(&bank, &options);
    assert_eq!(wave.spell, "Water Wave");
    assert!(wave.exportable(), "{:?}", wave.missing);
    assert_eq!((quantity(&wave, 555), quantity(&wave, 565)), (1050, 150));
    assert_eq!((quantity(&wave, 556), quantity(&wave, 560)), (0, 0));
    options.combat.spell_casts = 100;
    let smaller = planner::plan(&bank, &options);
    assert_eq!(
        (quantity(&smaller, 555), quantity(&smaller, 565)),
        (700, 100)
    );
    options.combat.spell_casts = 150;
    options.combat.spell_selection = SpellSelection::Spell("Water Strike".into());
    let strike = planner::plan(&bank, &options);
    assert_eq!(strike.spell, "Water Strike");
    assert_eq!((quantity(&strike, 555), quantity(&strike, 558)), (150, 150));
    assert_eq!(quantity(&strike, 565), 0);
    options.combat.spell_selection = SpellSelection::Auto;
    assert_eq!(planner::plan(&bank, &options).spell, "Wind Wave");
}
#[test]
fn missing_manual_runes_never_silently_switch_to_an_owned_spell() {
    let (_, options) = manual_plan("Water Wave");
    let plan = planner::plan(&account::demo(), &options);
    assert_eq!(plan.spell, "Water Wave");
    assert!(!plan.exportable());
    assert!(
        plan.missing
            .iter()
            .any(|m| m.contains("Water rune") && m.contains("1050")),
        "{:?}",
        plan.missing
    );
    assert_eq!(quantity(&plan, 555), 0);
    assert!(ironbound::export::inventory_setup(&plan, &options, "Test").is_err());
}
#[test]
fn tome_and_staff_free_runes_are_used_by_the_manual_plan() {
    let (_, mut options) = manual_plan("Water Wave");
    let mut bank = account::demo();
    bank.members[0].bank.as_mut().unwrap().extend([25574, 1]);
    options.gear_overrides.insert("Magic:5".into(), 25574);
    let plan = planner::plan(&bank, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(plan.spell, "Water Wave");
    assert_eq!(
        (
            quantity(&plan, 555),
            quantity(&plan, 556),
            quantity(&plan, 565)
        ),
        (0, 0, 150)
    );
}
#[test]
fn manual_selection_finds_compatible_owned_weapons_but_preserves_explicit_overrides() {
    let (mut bank, mut options) = manual_plan("Water Wave");
    bank.members[0].bank.as_mut().unwrap().extend([11907, 1]);
    options.gear_overrides.remove("Magic:3");
    let plan = planner::plan(&bank, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert!(
        options
            .combat
            .spell_selection
            .permits_weapon(plan.equipment[3].unwrap())
    );
    assert_ne!(plan.equipment[3], Some(11907));
    options.gear_overrides.insert("Magic:3".into(), 11907);
    let plan = planner::plan(&bank, &options);
    assert_eq!(plan.equipment[3], Some(11907));
    assert_eq!(plan.spell, "Water Wave");
    assert!(!plan.exportable());
}
#[test]
fn persisted_and_shared_spell_matches_exported_runes_and_notes() {
    use ironbound::{export, share::Setup};
    let (bank, options) = manual_plan("Water Wave");
    let encoded = serde_json::to_string(&options).unwrap();
    let restored: planner::Options = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        restored.combat.spell_selection,
        options.combat.spell_selection
    );
    let plan = planner::plan(&bank, &restored);
    let setup = Setup::from_plan(&plan, &restored);
    let decoded = Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(
        decoded.combat.spell_selection,
        options.combat.spell_selection
    );
    assert_eq!(decoded.plan().spell, "Water Wave");
    assert_eq!(quantity(&decoded.plan(), 555), 1050);
    let raw = export::inventory_setup(&plan, &restored, "Test").unwrap();
    let exported: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(raw.contains("Water Wave"));
    assert!(exported.to_string().contains("1050"));
    assert!(
        serde_json::from_str::<Settings>("{}")
            .unwrap()
            .spell_selection
            .is_auto()
    );
    let mut invalid = Settings::default();
    invalid.spell_selection = SpellSelection::Spell("Invented spell".into());
    assert!(!invalid.valid());
    invalid.spell_selection = SpellSelection::Element("ice".into());
    assert!(!invalid.valid());
}

#[test]
fn element_selection_and_high_level_spell_keep_their_actual_requirements() {
    let (bank, mut options) = manual_plan("Water Wave");
    options.combat.spell_selection = SpellSelection::Element("water".into());
    let plan = planner::plan(&bank, &options);
    assert_eq!(plan.spell, "Water Wave");
    assert_eq!(quantity(&plan, 555), 1050);
    options.combat.spell_selection = SpellSelection::Spell("Fire Surge".into());
    let plan = planner::plan(&bank, &options);
    assert_eq!(plan.spell, "Fire Surge");
    assert!(!plan.exportable());
    assert!(
        plan.missing.iter().any(|m| m.contains("95 Magic")),
        "{:?}",
        plan.missing
    );
}
#[test]
fn device_backup_and_named_setup_preserve_manual_choice_and_validate_it() {
    use ironbound::setups::{Library, NamedSetup, Saved};
    let (snapshot, options) = manual_plan("Water Strike");
    let saved = Saved {
        death_risk: Default::default(),
        version: 1,
        snapshot,
        options: options.clone(),
        pins: Default::default(),
        last_setup_fragment: None,
        setups: Library(vec![NamedSetup {
            name: "Water spells".into(),
            options,
        }]),
        budget_caps: [4, 8],
    };
    let raw = serde_json::to_string(&saved).unwrap();
    let loaded = Saved::parse(&raw).unwrap();
    assert_eq!(
        loaded.options.combat.spell_selection,
        SpellSelection::Spell("Water Strike".into())
    );
    assert_eq!(
        loaded.setups.0[0].options.combat.spell_selection,
        loaded.options.combat.spell_selection
    );
    let plan = planner::plan(&loaded.snapshot, &loaded.options);
    assert_eq!(plan.spell, "Water Strike");
    assert_eq!(quantity(&plan, 558), 150);
    let invalid = raw.replace("Water Strike", "Made-up spell");
    assert!(Saved::parse(&invalid).is_err());
}
