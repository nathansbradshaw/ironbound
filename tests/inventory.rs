use ironbound::{
    account, export,
    inventory::{Category, category},
    planner::{self, Options},
    share,
};

#[test]
fn item_groups_distinguish_consumables_equipment_and_unknowns() {
    for (id, name, expected) in [
        (385, "Shark", Category::Food),
        (391, "Manta ray", Category::Food),
        (2434, "Prayer potion(4)", Category::Potions),
        (6685, "Saradomin brew(4)", Category::Potions),
        (2442, "Super defence(4)", Category::Potions),
        (2440, "Super strength(4)", Category::Potions),
        (999997, "Supercompost", Category::Other),
        (4587, "Dragon scimitar", Category::Equipment),
        (560, "Death rune", Category::Runes),
        (8007, "Teleport to house", Category::Teleports),
        (1275, "Rune pickaxe", Category::Tools),
        (999999, "Raw shark", Category::Other),
        (999998, "Mystery item", Category::Other),
    ] {
        assert_eq!(category(id, name), expected, "{name}");
    }
}

#[test]
fn chosen_inventory_positions_survive_export_and_sharing() {
    let snapshot = account::demo();
    let mut options = Options::default();
    let base = planner::plan(&snapshot, &options);
    assert!(base.inventory.len() < 27);
    options.inventory_overrides.insert(27, 385);
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(plan.inventory.len(), 28);
    assert_eq!(plan.inventory[27].id, 385);
    assert_eq!(plan.inventory[26].quantity, 0);
    let json: serde_json::Value =
        serde_json::from_str(&export::inventory_setup(&plan, &options, "private").unwrap())
            .unwrap();
    assert!(json["setup"]["inv"][26].is_null());
    assert_eq!(json["setup"]["inv"][27]["id"], 385);
    let fragment = share::Setup::from_plan(&plan, &options).fragment().unwrap();
    let restored = share::Setup::parse(&fragment).unwrap().plan();
    assert_eq!(restored.inventory[26].quantity, 0);
    assert_eq!(restored.inventory[27].id, 385);
    assert!(
        !export::bank_tags(&plan, &options)
            .unwrap()
            .split(',')
            .skip(4)
            .any(|id| id == "0")
    );
}

#[test]
fn removing_a_slot_does_not_shift_required_items() {
    let snapshot = account::demo();
    let mut options = Options::default();
    let original = planner::plan(&snapshot, &options);
    let item = original.inventory[1].id;
    options.inventory_overrides.insert(0, 0);
    let plan = planner::plan(&snapshot, &options);
    assert_eq!(plan.inventory[0].quantity, 0);
    assert_eq!(plan.inventory[1].id, item);
    assert!(
        !plan.exportable(),
        "Removing required switch gear must block export"
    );
}
