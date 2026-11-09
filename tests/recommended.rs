//! Public, synthetic midgame banks: no imported account or private bank data.
use ironbound::{
    account, combat, encounters, optimization,
    planner::{self, Boss, Options, Plan},
    rules,
};
use std::collections::{BTreeMap, BTreeSet};

fn item(name: &str) -> i32 {
    planner::equipment()
        .iter()
        .find(|g| {
            g.name == name && matches!(g.version.as_str(), "" | "Undamaged" | "Normal" | "Worn")
        })
        .unwrap_or_else(|| panic!("Public catalog has no ordinary usable {name}"))
        .id
}

fn bank() -> account::Snapshot {
    let mut snapshot = account::demo();
    let mut stock = snapshot.available(false);
    for name in [
        "Fighter torso",
        "Torag's platebody",
        "Torag's platelegs",
        "Ahrim's robetop",
        "Ahrim's robeskirt",
        "Obsidian platebody",
        "Obsidian platelegs",
        "Amulet of fury",
        "Dragon boots",
        "Fire cape",
        "Berserker ring (i)",
        "Zombie axe",
        "Morytania legs 3",
        "Graceful top",
        "Graceful legs",
    ] {
        stock.insert(item(name), 1);
    }
    let member = &mut snapshot.members[0];
    member.name = "Public recommendation fixture".into();
    snapshot.selected = member.name.clone();
    member.bank = Some(
        stock
            .into_iter()
            .flat_map(|(id, n)| [id as i64, n as i64])
            .collect(),
    );
    member.inventory = None;
    member.equipment = None;
    member.rune_pouch = None;
    member.shared_bank = None;
    member.potion_storage = None;
    member.quests = None; // Possession needs no additional quest checks.
    for (skill, level) in [
        ("Attack", 75),
        ("Strength", 85),
        ("Defence", 75),
        ("Ranged", 80),
        ("Magic", 80),
        ("Prayer", 70),
        ("Hitpoints", 85),
        ("Mining", 72),
    ] {
        member.level_overrides.insert(skill.into(), level);
    }
    snapshot
}

fn toa() -> Options {
    Options {
        boss: Boss::Toa,
        recommended: Some(true),
        max_switches: Some(4),
        switch_slots: 8,
        food: 5,
        prayer_potions: 3,
        raid_level: 150,
        ..Default::default()
    }
}

fn sets(plan: &Plan) -> impl Iterator<Item = &Vec<Option<i32>>> {
    std::iter::once(&plan.equipment).chain(plan.phases.iter().map(|p| &p.equipment))
}

fn assert_owned_and_usable(snapshot: &account::Snapshot, plan: &Plan) {
    let stock = snapshot.available(false);
    let member = snapshot.selected().unwrap();
    for equipment in sets(plan) {
        for id in equipment.iter().flatten() {
            assert!(
                stock.contains_key(id),
                "Unowned {}",
                planner::item_name(*id)
            );
            assert!(
                planner::can_use(*id, member),
                "Unusable {}",
                planner::item_name(*id)
            );
        }
    }
    let mut packed = BTreeMap::<i32, u32>::new();
    for (slot, id) in plan.equipment.iter().enumerate() {
        if let Some(id) = id {
            *packed.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
        }
    }
    for entry in &plan.inventory {
        if entry.id > 0 && entry.quantity > 0 {
            *packed.entry(entry.id).or_default() += entry.quantity;
        }
    }
    for (id, quantity) in packed {
        assert!(
            quantity <= stock.get(&id).copied().unwrap_or(0),
            "Packed {quantity} {}, owned {:?}",
            planner::item_name(id),
            stock.get(&id)
        );
    }
}

#[test]
fn recommended_toa_keeps_real_combat_gear_and_all_three_styles_usable() {
    // Reviewed starter jobs, rather than a particular numerical DPS answer:
    // https://oldschool.runescape.wiki/w/Tombs_of_Amascut/Strategies
    let snapshot = bank();
    let mut options = toa();
    options.recommended = None; // Default ranking weights must choose Recommended automatically.
    let plan = planner::plan(&snapshot, &options);
    assert!(
        plan.exportable(),
        "Missing {:?}; warnings {:?}",
        plan.missing,
        plan.warnings
    );
    assert_owned_and_usable(&snapshot, &plan);
    let utility: BTreeSet<_> = ["Morytania legs 3", "Graceful top", "Graceful legs"]
        .into_iter()
        .map(item)
        .collect();
    for set in sets(&plan) {
        assert!(
            !set.iter().flatten().any(|id| utility.contains(id)),
            "Utility clothing displaced owned encounter armour: {:?}",
            set.iter()
                .flatten()
                .map(|id| planner::item_name(*id))
                .collect::<Vec<_>>()
        );
    }
    for (phase_name, style, target_id) in [
        ("Ba-ba", "Melee", 11778),
        ("Kephri", "Melee", 11719),
        ("Zebak", "Ranged", 11730),
        ("Akkha", "Magic", 11789),
    ] {
        let phase = plan.phases.iter().find(|p| p.name == phase_name).unwrap();
        let weapon = phase.equipment[3].expect("Each combat job needs a weapon");
        let rule = rules::weapon(weapon).expect("Recommended weapon must have supported rules");
        if style == "Ranged" {
            assert_eq!(rule.family.style, "Ranged");
            let allowed = rules::ammo_ids(weapon);
            assert!(
                allowed.contains(&phase.equipment[13].expect("Crossbow needs bolts")),
                "Incompatible ammunition for {}",
                planner::item_name(weapon)
            );
        }
        if style == "Magic" {
            assert_eq!(
                rule.family.style, "Magic",
                "Akkha needs its actual magic switch"
            );
        }
        let result = combat::calculate_configured(
            snapshot.selected().unwrap(),
            &phase.equipment,
            style,
            planner::monster(target_id).unwrap(),
            &plan.spell,
            options.raid_level,
            &options.combat,
        )
        .unwrap_or_else(|why| panic!("{phase_name}: {why}"));
        assert!(
            result.dps.is_finite() && result.dps > 0.0,
            "{phase_name} has no supported damage"
        );
    }
}

#[test]
fn recommended_toa_honours_four_switches_bag_space_and_stock() {
    let snapshot = bank();
    let options = toa();
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_owned_and_usable(&snapshot, &plan);
    assert!(plan.inventory_slots() <= 28);
    let equipment: Vec<_> = sets(&plan).collect();
    for a in &equipment {
        for b in &equipment {
            assert!(
                optimization::changes(a, b) <= 4,
                "A phase transition exceeds the chosen four-item switch limit"
            );
        }
    }
    let starting: BTreeSet<_> = plan.equipment.iter().flatten().copied().collect();
    let extra: BTreeSet<_> = plan
        .phases
        .iter()
        .flat_map(|p| p.equipment.iter().flatten())
        .filter(|id| !starting.contains(id))
        .copied()
        .collect();
    assert!(
        extra.len() <= options.switch_slots,
        "Too many carried switches: {extra:?}"
    );
}

#[test]
fn plain_owned_keris_does_not_become_an_upgrade() {
    // Upgraded partisans are distinct owned items, not free encounter bonuses:
    // https://oldschool.runescape.wiki/w/Keris_partisan_of_corruption
    let snapshot = bank();
    let plan = planner::plan(&snapshot, &toa());
    let base = item("Keris partisan");
    let kephri = plan.phases.iter().find(|p| p.name == "Kephri").unwrap();
    assert_eq!(
        kephri.equipment[3],
        Some(base),
        "Use the actual owned base partisan at Kephri"
    );
    for id in sets(&plan).flat_map(|s| s.iter().flatten()) {
        let name = planner::item_name(*id);
        assert!(
            !name.starts_with("Keris partisan of"),
            "Invented unowned upgrade: {name}"
        );
    }
}

#[test]
fn recommended_moons_prefers_barrows_tank_armour_over_obsidian_strength() {
    // Explicit guide advice: avoid obsidian's low defence/small strength gain,
    // prefer tankier Barrows. This tests that encounter decision, not a DPS constant.
    // https://oldschool.runescape.wiki/w/Moons_of_Peril/Strategies
    let snapshot = bank();
    let options = Options {
        boss: Boss::Custom,
        targets: encounters::groups()
            .iter()
            .find(|g| g.name == "Moons of Peril")
            .unwrap()
            .targets
            .clone(),
        recommended: Some(true),
        max_switches: Some(4),
        switch_slots: 8,
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_owned_and_usable(&snapshot, &plan);
    assert_eq!(plan.phases.len(), 3);
    for set in sets(&plan) {
        assert_eq!(
            set[4],
            Some(item("Torag's platebody")),
            "Moons needs defensive body armour"
        );
        assert_eq!(
            set[7],
            Some(item("Torag's platelegs")),
            "Moons needs defensive leg armour"
        );
    }
}

#[test]
fn unknown_core_skills_block_a_recommendation_instead_of_assuming_levels() {
    let mut snapshot = bank();
    snapshot.members[0].skills = None;
    snapshot.members[0].level_overrides.clear();
    assert!(!planner::missing_combat_levels(&snapshot).is_empty());
    let plan = planner::plan(&snapshot, &toa());
    assert!(
        !plan.exportable(),
        "Unknown combat levels must not produce an exportable best setup"
    );
    assert!(
        !plan.missing.is_empty(),
        "The missing source input needs an actionable blocker"
    );
}

#[test]
fn recommended_mode_and_legacy_custom_weights_survive_share_roundtrips() {
    use ironbound::{ranking::Weights, recommendation, share::Setup};
    let options = toa();
    let plan = planner::plan(&bank(), &options);
    let custom = Weights {
        dps: 10.0,
        defence: 80.0,
        prayer: 10.0,
        weight: 0.0,
    };
    for (mode, weights, enabled) in [
        (None, Weights::default(), true),
        (None, custom.clone(), false),
        (Some(true), custom.clone(), true),
        (Some(false), Weights::default(), false),
    ] {
        let mut settings = options.clone();
        settings.recommended = mode;
        settings.ranking.weights = weights.clone();
        assert_eq!(recommendation::enabled(&settings), enabled);
        let shared = Setup::from_plan(&plan, &settings);
        let loaded = Setup::parse(&shared.fragment().unwrap()).unwrap().options();
        assert_eq!(loaded.recommended, mode);
        assert_eq!(loaded.ranking.weights, weights);
        assert_eq!(recommendation::enabled(&loaded), enabled);
    }
    // Existing links have no recommended field. Their custom priorities stay custom.
    let mut old = Setup::from_plan(&plan, &options);
    old.ranking.weights = custom;
    let mut json = serde_json::to_value(old).unwrap();
    json.as_object_mut().unwrap().remove("recommended");
    let legacy: Setup = serde_json::from_value(json).unwrap();
    assert!(!recommendation::enabled(&legacy.options()));
}

#[test]
fn weapon_trials_never_reintroduce_unowned_shared_gear_overrides() {
    let snapshot = bank();
    let unowned = item("Ancestral robe top");
    assert!(!snapshot.available(false).contains_key(&unowned));
    let mut options = toa();
    options.gear_overrides.insert("Magic:4".into(), unowned);
    let set = planner::equipment_for_style(&snapshot, &options, "Magic");
    assert_ne!(
        set[4],
        Some(unowned),
        "An ignored unowned override must remain ignored during weapon trials"
    );
    let plan = planner::plan(&snapshot, &options);
    assert!(
        plan.exportable(),
        "Rejected unowned gear must not make switch optimization infeasible: {:?}",
        plan.missing
    );
    assert_owned_and_usable(&snapshot, &plan);
}

#[test]
fn recommended_ranged_search_discovers_the_owned_bowfa_crystal_combination() {
    // The passive requires an eligible bow; independent armour swaps can miss
    // the combination. This tests discovery rather than a hardcoded DPS value.
    // https://oldschool.runescape.wiki/w/Crystal_equipment
    let mut snapshot = bank();
    snapshot.members[0]
        .level_overrides
        .insert("Agility".into(), 70);
    let catalog_item = |name: &str, version: &str| {
        planner::equipment()
            .iter()
            .find(|g| g.name == name && g.version == version)
            .unwrap()
            .id
    };
    let bow = catalog_item("Bow of Faerdhinen", "Charged");
    let pieces: Vec<_> = ["Crystal helm", "Crystal body", "Crystal legs"]
        .into_iter()
        .map(|name| catalog_item(name, "Active"))
        .collect();
    for id in std::iter::once(&bow).chain(pieces.iter()) {
        snapshot.members[0]
            .bank
            .as_mut()
            .unwrap()
            .extend([*id as i64, 1]);
    }
    let mut options = toa();
    options.max_switches = None; // Discovery before the player's switch trade-off.
    let set = planner::equipment_for_style(&snapshot, &options, "Ranged");
    assert_eq!(
        set[3],
        Some(bow),
        "Discover the owned bow's complete competitive setup"
    );
    for (slot, id) in [0, 4, 7].into_iter().zip(pieces) {
        assert_eq!(
            set[slot],
            Some(id),
            "Compatible owned crystal armour at slot {slot}"
        );
    }
    assert!(
        set[5].is_none(),
        "The two-handed bow cannot retain a shield"
    );
    assert!(
        set[13].is_none(),
        "Bowfa does not require crossbow ammunition"
    );
}
