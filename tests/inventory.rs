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

#[test]
fn boss_previews_conserve_carried_items_and_leave_starting_exports_unchanged() {
    use std::collections::BTreeMap;
    fn carried(equipment: &[Option<i32>], inventory: &[planner::Packed]) -> BTreeMap<i32, u32> {
        let mut counts = BTreeMap::new();
        for (slot, id) in equipment.iter().enumerate() {
            if let Some(id) = id {
                *counts.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
            }
        }
        for item in inventory
            .iter()
            .filter(|item| item.id > 0 && item.quantity > 0)
        {
            *counts.entry(item.id).or_default() += item.quantity;
        }
        counts
    }
    let options = Options {
        boss: planner::Boss::Toa,
        ..Default::default()
    };
    let plan = planner::plan(&account::demo(), &options);
    let export_before = export::inventory_setup(&plan, &options, "test").unwrap();
    let expected = carried(&plan.equipment, &plan.inventory);
    let mut changed = false;
    for (index, phase) in plan.phases.iter().enumerate() {
        let preview = plan.preview_inventory(Some(index));
        assert_eq!(
            carried(&phase.equipment, &preview),
            expected,
            "{}",
            phase.name
        );
        changed |= preview != plan.inventory;
        for (slot, item) in plan.inventory.iter().enumerate() {
            if category(item.id, &planner::item_name(item.id)) == Category::Food {
                assert_eq!(preview[slot], *item, "Food must stay in place");
            }
        }
    }
    assert!(changed, "At least one boss should swap inventory gear");
    assert_eq!(plan.preview_inventory(None), plan.inventory);
    assert_eq!(plan.preview_inventory(Some(usize::MAX)), plan.inventory);
    assert_eq!(
        export::inventory_setup(&plan, &options, "test").unwrap(),
        export_before
    );
}

#[test]
fn two_handed_preview_returns_weapon_and_shield_without_shifting_supplies() {
    use planner::{Packed, Phase};
    let mut plan = planner::plan(&account::demo(), &Options::default());
    plan.equipment = vec![None; 14];
    plan.equipment[3] = Some(4587);
    plan.equipment[5] = Some(12954);
    plan.inventory = vec![
        Packed {
            id: 861,
            quantity: 1,
            purpose: "Switch".into(),
        },
        Packed {
            id: 385,
            quantity: 1,
            purpose: "Food".into(),
        },
        Packed {
            id: 0,
            quantity: 0,
            purpose: String::new(),
        },
        Packed {
            id: 560,
            quantity: 200,
            purpose: "Runes".into(),
        },
    ];
    let mut equipment = vec![None; 14];
    equipment[3] = Some(861);
    plan.phases = vec![Phase {
        name: "Ranged".into(),
        style: "Ranged".into(),
        equipment,
        note: String::new(),
    }];
    let inventory = plan.preview_inventory(Some(0));
    assert_eq!(inventory[0].id, 4587);
    assert_eq!(inventory[2].id, 12954);
    assert_eq!(inventory[1], plan.inventory[1]);
    assert_eq!(inventory[3], plan.inventory[3]);
    assert_eq!(plan.preview_inventory(None)[0].id, 861);
    // Ammunition uses the same 100-projectile allocation as planning/export.
    plan.equipment[13] = Some(892);
    plan.phases[0].equipment[13] = Some(9242);
    plan.inventory.push(Packed {
        id: 9242,
        quantity: 200,
        purpose: "Ammo".into(),
    });
    let inventory = plan.preview_inventory(Some(0));
    assert_eq!(
        inventory
            .iter()
            .find(|item| item.id == 9242)
            .unwrap()
            .quantity,
        100
    );
    assert_eq!(
        inventory
            .iter()
            .find(|item| item.id == 892)
            .unwrap()
            .quantity,
        100
    );
}
