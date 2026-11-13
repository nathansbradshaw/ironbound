//! Public synthetic fixtures only; explanation text never needs a private bank.
use ironbound::{
    account, explanations,
    planner::{self, Boss, Options, Packed, Phase, Plan},
};
fn fixture() -> (account::Snapshot, Options, Plan) {
    let mut bank = account::demo();
    bank.members.truncate(1);
    bank.members[0].inventory = None;
    bank.members[0].equipment = None;
    bank.members[0].bank = Some(vec![4151, 1, 11905, 1, 10551, 1, 10462, 1]);
    for skill in account::SKILLS {
        bank.members[0].level_overrides.insert(skill.into(), 99);
    }
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![
            planner::Target {
                id: 100013,
                style: "Melee".into(),
            },
            planner::Target {
                id: 100012,
                style: "Magic".into(),
            },
        ],
        ..Default::default()
    };
    o.combat.protection = "Melee".into();
    let mut melee = vec![None; 14];
    melee[3] = Some(4151);
    melee[4] = Some(10551);
    let mut magic = vec![None; 14];
    magic[3] = Some(11905);
    magic[4] = Some(10462);
    let p = Plan {
        ammo_shots: 100,
        loaded_ammo: None,
        optimization: None,
        equipment: melee.clone(),
        inventory: vec![
            Packed {
                id: 11905,
                quantity: 1,
                purpose: "Magic switch".into(),
            },
            Packed {
                id: 10462,
                quantity: 1,
                purpose: "Magic switch".into(),
            },
        ],
        phases: vec![
            Phase {
                name: o.targets[0].name(),
                style: "Melee".into(),
                equipment: melee,
                note: String::new(),
            },
            Phase {
                name: o.targets[1].name(),
                style: "Magic".into(),
                equipment: magic,
                note: String::new(),
            },
        ],
        checks: vec![],
        warnings: vec![],
        missing: vec![],
        switches: 2,
        supported: 2,
        owned: 4,
        label: "Owned test setup".into(),
        spell: "Built-in spell".into(),
    };
    (bank, o, p)
}
#[test]
fn prayer_switch_explains_same_damage_and_verified_drain_saving() {
    let (bank, o, p) = fixture();
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 4);
    assert!(reason.contains("Same modeled DPS"), "{reason}");
    assert!(reason.contains("saves 3.3 prayer/min"), "{reason}");
    assert!(reason.contains(&planner::item_name(10551)), "{reason}");
    assert_eq!(explanations::packed(&bank, &o, &p, &p.inventory[1]), reason);
    assert!(reason.chars().count() <= 260);
}
#[test]
fn weapons_explain_the_actual_attack_window_and_calculated_damage() {
    let (bank, o, p) = fixture();
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 3);
    assert!(reason.contains("Provides Magic attacks"), "{reason}");
    assert!(reason.contains("Judge"), "{reason}");
    assert!(reason.contains("modeled DPS"), "{reason}");
    assert!(!reason.contains("best") && !reason.contains("BIS"));
}
#[test]
fn manual_choices_take_precedence_over_automatic_recommendation_claims() {
    let (bank, mut o, p) = fixture();
    o.gear_overrides.insert("Magic:4".into(), 10462);
    assert_eq!(
        explanations::equipment(&bank, &o, &p, Some(1), 4),
        "You chose this item for Magic."
    );
    o.inventory_overrides.insert(0, 10462);
    assert_eq!(
        explanations::packed(&bank, &o, &p, &p.inventory[1]),
        "You selected this inventory item."
    );
}
#[test]
fn supplies_explain_their_packing_purpose_and_bound_untrusted_text() {
    let (bank, o, p) = fixture();
    let supply = Packed {
        id: 385,
        quantity: 1,
        purpose: "Emergency healing".into(),
    };
    assert_eq!(
        explanations::packed(&bank, &o, &p, &supply),
        "Emergency healing"
    );
    let long = Packed {
        purpose: "a long supply explanation ".repeat(100),
        ..supply
    };
    assert!(explanations::packed(&bank, &o, &p, &long).chars().count() <= 180);
}
#[test]
fn unsupported_windows_and_shared_imports_do_not_invent_damage_benefits() {
    let (bank, mut o, mut p) = fixture();
    o.targets[1].id = 100001;
    p.phases[1].name = o.targets[1].name();
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 3);
    assert!(
        reason.contains("damage benefits are unverified"),
        "{reason}"
    );
    assert!(!reason.contains("modeled DPS"));
    p.label = "Shared setup; account requirements unverified".into();
    let shared = explanations::equipment(&bank, &o, &p, Some(1), 4);
    assert!(shared.contains("ownership, levels and benefits are unverified"));
    assert!(!shared.contains("saves") && !shared.contains("DPS"));
    assert!(explanations::packed(&bank, &o, &p, &p.inventory[1]).contains("unverified"));
}
#[test]
fn shared_equipment_describes_observed_switch_savings() {
    let (bank, o, mut p) = fixture();
    p.phases[1].equipment[4] = Some(10551);
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 4);
    assert!(reason.contains("Worn across 2 styles"), "{reason}");
    assert!(reason.contains("body switch"));
}
#[test]
fn intentional_empty_slots_are_explained_from_weapon_capabilities() {
    let (bank, o, mut p) = fixture();
    p.equipment[3] = Some(20997);
    p.equipment[5] = None;
    assert!(explanations::equipment(&bank, &o, &p, None, 5).contains("two-handed"));
    p.equipment[3] = Some(12926);
    p.equipment[13] = None;
    assert!(explanations::equipment(&bank, &o, &p, None, 13).contains("Darts are stored inside"));
    p.equipment[3] = Some(25865);
    assert!(
        explanations::equipment(&bank, &o, &p, None, 13).contains("supplies its own ammunition")
    );
}
#[test]
fn an_unlisted_item_does_not_inherit_the_entire_slot_guide_rationale() {
    let (bank, o, p) = fixture();
    // Guthix prayer robes are not in the reviewed Yama magic-armour progression.
    let mut o = o;
    o.combat.protection = "None".into();
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 4);
    assert!(!reason.contains("reviewed"), "{reason}");
    assert!(reason.contains("prayer bonus"), "{reason}");
}

#[test]
fn melee_substyles_are_not_counted_as_different_combat_styles() {
    let (bank, mut o, mut p) = fixture();
    p.phases[1].equipment = p.equipment.clone();
    p.phases[1].style = "Slash".into();
    o.targets[1].style = "Slash".into();
    let reason = explanations::equipment(&bank, &o, &p, Some(1), 4);
    assert!(!reason.contains("across 2 styles"), "{reason}");
    assert!(reason.contains("+4 melee strength"), "{reason}");
}
#[test]
fn loaded_weapon_explanations_use_filtered_and_shared_stock() {
    let (mut bank, mut o, mut p) = fixture();
    let mole = planner::monsters()
        .iter()
        .find(|m| m.name == "Giant Mole")
        .unwrap();
    o.targets = vec![planner::Target {
        id: mole.id,
        style: "Ranged".into(),
    }];
    let mut set = vec![None; 14];
    set[3] = Some(12926);
    p.equipment = set.clone();
    p.phases = vec![Phase {
        name: o.targets[0].name(),
        style: "Ranged".into(),
        equipment: set.clone(),
        note: String::new(),
    }];
    bank.members[0].bank = Some(vec![12926, 1, 811, 100, 11230, 100]);
    o.excluded.insert(11230);
    let mut expected_member = bank.members[0].clone();
    expected_member.bank = Some(vec![12926, 1, 811, 100]);
    let expected = ironbound::combat::calculate_configured(
        &expected_member,
        &set,
        "Ranged",
        mole,
        "",
        0,
        &o.combat,
    )
    .unwrap();
    let filtered = explanations::equipment(&bank, &o, &p, Some(0), 3);
    assert!(
        filtered.contains(&format!("{:.2} modeled DPS", expected.dps)),
        "{filtered}"
    );
    o.excluded.clear();
    o.shared = true;
    bank.members[0].bank = Some(vec![12926, 1, 811, 100]);
    let mut group = bank.members[0].clone();
    group.name = "@SHARED".into();
    group.bank = Some(vec![11230, 100]);
    bank.members.push(group);
    expected_member.bank = Some(vec![12926, 1, 811, 100, 11230, 100]);
    let expected = ironbound::combat::calculate_configured(
        &expected_member,
        &set,
        "Ranged",
        mole,
        "",
        0,
        &o.combat,
    )
    .unwrap();
    let shared_stock = explanations::equipment(&bank, &o, &p, Some(0), 3);
    assert!(
        shared_stock.contains(&format!("{:.2} modeled DPS", expected.dps)),
        "{shared_stock}"
    );
    assert_ne!(filtered, shared_stock);
}
