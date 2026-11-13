use ironbound::{
    account::{self, Member, Snapshot},
    export,
    planner::{self, Boss, Options, Target},
    share::Setup,
};
fn demo(boss: Boss) -> (Options, planner::Plan) {
    let o = Options {
        boss,
        ..Default::default()
    };
    let p = planner::plan(&account::demo(), &o);
    (o, p)
}
#[test]
fn bank_table_is_private_and_combines_duplicates() {
    let s = Snapshot::from_bank_tsv(
        "Item id\tItem name\tItem quantity\n385\tShark\t2\n385\tShark\t3\n99999\tUnreviewed\t1\n",
        "Test",
        "now".into(),
    )
    .unwrap();
    assert_eq!(s.available(false)[&385], 5);
    assert_eq!(s.members[0].item_names[&99999], "Unreviewed");
    assert_eq!(s.members[0].level("Magic"), None);
}
#[test]
fn malformed_bank_does_not_silently_import() {
    for raw in [
        "Item id\tItem name\tItem quantity\n385\tShark\t-1",
        "Item id\tItem name\tItem quantity\n385\tShark\t2147483648",
        "Item id\tItem name\tItem quantity\n385\tShark",
        "id,name,quantity\n385,Shark,1",
    ] {
        assert!(Snapshot::from_bank_tsv(raw, "Test", "now".into()).is_err());
    }
}
#[test]
fn public_levels_map_by_name_not_position() {
    let mut m = Member::default();
    let skills = [
        "Magic",
        "Prayer",
        "Hitpoints",
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
    ]
    .iter()
    .enumerate()
    .map(|(i, n)| serde_json::json!({"name":n,"level":70+i}))
    .collect::<Vec<_>>();
    m.apply_hiscores(&serde_json::json!({"skills":skills}).to_string())
        .unwrap();
    assert_eq!(m.level("Magic"), Some(70));
    assert_eq!(m.level("Attack"), Some(73));
    assert_eq!(m.level("Mining"), None);
}
#[test]
fn incomplete_lookup_is_atomic() {
    let mut m = Member::default();
    m.level_overrides.insert("Magic".into(), 99);
    assert!(
        m.apply_hiscores(r#"{"skills":[{"name":"Magic","level":1}]}"#)
            .is_err()
    );
    assert_eq!(m.level("Magic"), Some(99));
}
#[test]
fn shared_setup_roundtrips_all_carried_items() {
    for boss in [Boss::Barrows, Boss::Toa] {
        let (o, p) = demo(boss);
        let setup = Setup::from_plan(&p, &o);
        let link = setup.fragment().unwrap();
        let decoded = Setup::parse(&link).unwrap();
        assert_eq!(decoded.equipment, p.equipment);
        assert_eq!(decoded.inventory.len(), p.inventory.len());
        assert_eq!(decoded.switches.len(), p.phases.len());
        assert!(decoded.plan().exportable());
    }
}
#[test]
fn mark_assumption_roundtrips_and_old_links_default_inactive() {
    let (mut options, plan) = demo(Boss::Barrows);
    options.combat.mark_darkness = true;
    let setup = Setup::from_plan(&plan, &options);
    let decoded = Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert!(decoded.combat.mark_darkness);
    let mut legacy = serde_json::to_value(setup).unwrap();
    legacy["combat"]
        .as_object_mut()
        .unwrap()
        .remove("mark_darkness");
    let old: Setup = serde_json::from_value(legacy).unwrap();
    assert!(!old.combat.mark_darkness);
}
#[test]
fn sharing_only_contains_an_explicit_public_allowlist() {
    let (o, p) = demo(Boss::Toa);
    let value = serde_json::to_value(Setup::from_plan(&p, &o)).unwrap();
    let keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        vec![
            "boss",
            "combat",
            "constraints",
            "equipment",
            "inventory",
            "raid_level",
            "ranking",
            "switches",
            "targets",
            "version"
        ]
    );
    let raw = value.to_string();
    for private in [
        "bank",
        "skills",
        "account",
        "Iron Adventurer",
        "level_overrides",
        "quests",
    ] {
        assert!(!raw.contains(private));
    }
}
#[test]
fn portable_exports_do_not_include_the_account_name() {
    let (o, p) = demo(Boss::Barrows);
    let raw = export::inventory_setup(&p, &o, "Private player name").unwrap();
    assert!(!raw.contains("Private player name"));
}
#[test]
fn malformed_or_unbounded_urls_are_rejected() {
    for hash in ["#setup=0", "#setup=zz", "#setup=éé", "#setup=7b7d"] {
        assert!(Setup::parse(hash).is_err());
    }
    assert!(Setup::parse(&format!("#setup={}", "00".repeat(20001))).is_err());
}
#[test]
fn shared_phases_cannot_invent_uncarried_items() {
    let (o, p) = demo(Boss::Toa);
    let mut s = Setup::from_plan(&p, &o);
    s.inventory.clear();
    assert!(s.fragment().is_err());
}
#[test]
fn shared_capacity_and_slots_are_validated() {
    let (o, p) = demo(Boss::Barrows);
    let mut s = Setup::from_plan(&p, &o);
    s.equipment[0] = Some(9185);
    assert!(s.fragment().is_err());
    s = Setup::from_plan(&p, &o);
    s.inventory.resize(29, s.inventory[0].clone());
    assert!(s.fragment().is_err());
}
#[test]
fn custom_group_has_one_pool_for_all_styles() {
    let ids: Vec<_> = planner::monsters().iter().take(3).map(|m| m.id).collect();
    let o = Options {
        boss: Boss::Custom,
        access_override: Some(true),
        targets: ids
            .into_iter()
            .zip(["Melee", "Magic", "Ranged"])
            .map(|(id, style)| Target {
                id,
                style: style.into(),
            })
            .collect(),
        ..Default::default()
    };
    let p = planner::plan(&account::demo(), &o);
    assert_eq!(p.phases.len(), 3);
    assert!(p.exportable(), "{:?}", p.missing);
    let shared = Setup::from_plan(&p, &o);
    assert!(shared.fragment().is_ok());
    let mut casual = o.clone();
    casual.fewer_switches = true;
    assert!(planner::plan(&account::demo(), &casual).switches <= p.switches);
}
#[test]
fn custom_group_requires_targets_but_no_quest_checks() {
    let mut o = Options {
        boss: Boss::Custom,
        ..Default::default()
    };
    assert!(!planner::plan(&account::demo(), &o).exportable());
    o.targets.push(Target {
        id: planner::monsters()[0].id,
        style: "Ranged".into(),
    });
    assert!(planner::plan(&account::demo(), &o).exportable());
}
#[test]
fn manual_equipment_rebuilds_switch_pool() {
    let mut o = Options::default();
    o.gear_overrides.insert("Magic:1".into(), 2413);
    o.gear_overrides.insert("Ranged:1".into(), 10499);
    let p = planner::plan(&account::demo(), &o);
    let mut changed = o;
    changed.gear_overrides.insert("Ranged:1".into(), 0);
    let p2 = planner::plan(&account::demo(), &changed);
    assert_eq!(p2.phases[1].equipment[1], None);
    assert!(p2.switches < p.switches);
    assert!(p2.exportable());
}
#[test]
fn inventory_edits_cannot_duplicate_unowned_items() {
    let (o, p) = demo(Boss::Barrows);
    let mut changed = o;
    changed
        .inventory_overrides
        .insert(p.inventory.len(), 999999);
    assert!(!planner::plan(&account::demo(), &changed).exportable());
}
#[test]
fn replacing_required_switch_blocks_export() {
    let (mut o, p) = demo(Boss::Toa);
    let i = p.inventory.iter().position(|i| i.id == 11905).unwrap();
    o.inventory_overrides.insert(i, 385);
    let edited = planner::plan(&account::demo(), &o);
    assert!(!edited.exportable());
    assert!(edited.missing.iter().any(|m| m.contains("carried pool")));
}
#[test]
fn replacing_required_runes_blocks_export() {
    let (mut o, p) = demo(Boss::Barrows);
    let i = p
        .inventory
        .iter()
        .position(|i| [560, 565].contains(&i.id))
        .unwrap();
    o.inventory_overrides.insert(i, 385);
    assert!(!planner::plan(&account::demo(), &o).exportable());
}
#[test]
fn bank_tags_csv_matches_runelite_v1_header_and_unique_items() {
    let (o, p) = demo(Boss::Toa);
    let csv = export::bank_tags(&p, &o).unwrap();
    let fields = csv.split(',').collect::<Vec<_>>();
    assert_eq!(
        &fields[..3],
        &["banktags", "1", "Ironbound - Tombs of Amascut"]
    );
    let ids = fields[4..]
        .iter()
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), fields.len() - 4);
    for id in p
        .equipment
        .iter()
        .flatten()
        .copied()
        .chain(p.inventory.iter().map(|i| i.id))
    {
        assert!(ids.contains(&id));
    }
}
