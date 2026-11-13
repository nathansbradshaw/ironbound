//! Public synthetic banks only. Packing is not a claim that mechanics execute themselves.
use ironbound::{
    account, combat, encounters, export,
    planner::{self, Boss, Options, Packed},
    rules, share,
};
use std::collections::BTreeMap;
fn bank(purging: bool, poison: bool) -> account::Snapshot {
    let mut s = account::demo();
    let mut stock = s.available(false);
    for id in [
        4151, 28810, 10828, 6585, 6570, 7462, 11840, 12954, 11773, 4712, 4714, 21795, 12002, 19544,
        11907,
    ] {
        stock.insert(id, 1);
    }
    if purging {
        stock.insert(29594, 1);
    }
    if poison {
        stock.insert(10925, 4);
    }
    for (id, q) in [
        (566, 1000),
        (554, 3000),
        (564, 10),
        (391, 12),
        (12695, 3),
        (3040, 3),
        (8007, 4),
        (12625, 2),
    ] {
        stock.insert(id, q);
    }
    s.members[0].bank = Some(
        stock
            .into_iter()
            .flat_map(|(id, q)| [i64::from(id), i64::from(q)])
            .collect(),
    );
    s.members[0].inventory = None;
    s.members[0].equipment = None;
    s.members[0].rune_pouch = None;
    s.members[0].quests = None;
    for skill in account::SKILLS {
        s.members[0].level_overrides.insert(skill.into(), 99);
    }
    s
}
fn options() -> Options {
    let mut o = Options {
        boss: Boss::Custom,
        targets: encounters::groups()
            .iter()
            .find(|g| g.name == "Yama")
            .unwrap()
            .targets
            .clone(),
        recommended: Some(true),
        max_switches: Some(4),
        switch_slots: 10,
        food: 8,
        prayer_potions: 3,
        ..Default::default()
    };
    o.combat.mark_darkness = true;
    o
}
fn all_owned(s: &account::Snapshot, p: &planner::Plan) {
    let owned = s.available(false);
    let mut pool = BTreeMap::<i32, u32>::new();
    for (slot, id) in p.equipment.iter().enumerate() {
        if let Some(id) = id {
            *pool.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
        }
    }
    for item in &p.inventory {
        *pool.entry(item.id).or_default() += item.quantity;
    }
    for (id, q) in pool {
        assert!(
            q <= owned.get(&id).copied().unwrap_or(0),
            "Borrowed{}",
            planner::item_name(id)
        );
    }
    let sets: Vec<_> = std::iter::once(&p.equipment)
        .chain(p.phases.iter().map(|p| &p.equipment))
        .collect();
    for a in &sets {
        for b in &sets {
            assert!(ironbound::optimization::changes(a, b) <= 4);
        }
    }
    assert!(p.inventory_slots() <= 28);
}
#[test]
fn owned_yama_kit_packs_demon_runes_mark_sanfew_and_useful_food() {
    let s = bank(true, true);
    let o = options();
    let p = planner::plan(&s, &o);
    assert!(p.exportable(), "{:?}", p.missing);
    all_owned(&s, &p);
    let magic = p.phases.iter().find(|p| p.style == "Magic").unwrap();
    assert_eq!(magic.equipment[3], Some(29594));
    assert!(p.inventory.iter().any(|p| p.id == 566 && p.quantity >= 301));
    assert!(p.inventory.iter().any(|p| p.id == 564 && p.quantity >= 1));
    assert!(planner::is_stackable(566) && planner::is_stackable(564));
    assert!(!p.inventory.iter().any(|p| p.id == 12625));
    assert!(
        !p.inventory.iter().any(|p| p.purpose == "Magic boost"),
        "Fixed Demonbane casts on guaranteed-hit Judge/flares gain no damage from a Magic potion"
    );
    assert_eq!(
        p.inventory.iter().filter(|p| p.id == 10925).count(),
        3,
        "Sanfew counts as poison AND prayer"
    );
    assert!(
        p.inventory
            .iter()
            .filter(|p| p.purpose == "Food")
            .all(|p| p.id == 391)
    );
    assert!(p.inventory.iter().any(|p| p.id == 12695));
    assert!(
        p.checks
            .iter()
            .any(|c| c.name == "Yama poison protection" && c.state == Some(true))
    );
}
#[test]
fn no_purging_uses_real_magic_fallback_and_missing_poison_is_not_ready() {
    let s = bank(false, false);
    let o = options();
    let p = planner::plan(&s, &o);
    all_owned(&s, &p);
    let m = s.selected().unwrap();
    let stock = s.available(false);
    for phase in p.phases.iter().filter(|p| p.style == "Magic") {
        assert!(phase.equipment[3].is_some());
        assert!(rules::casting_supported(m, &phase.equipment, &stock, 150));
    }
    assert!(
        p.checks
            .iter()
            .any(|c| c.name == "Yama poison protection" && c.state == Some(false))
    );
    assert!(p.warnings.iter().any(|w| w.contains("poison protection")));
    assert_eq!(p.status(), "Preparation needed");
    assert!(
        !p.inventory.iter().any(|p| p.id == 564),
        "Mark does not buff a powered Trident"
    );
    assert!(
        p.inventory.iter().any(|p| p.purpose == "Magic boost"),
        "A powered Trident benefits from a visible Magic boost"
    );
}
#[test]
fn mark_is_optional_and_one_precast_does_not_steal_attack_spell_runes() {
    let s = bank(true, true);
    let mut o = options();
    o.combat.mark_darkness = false;
    let p = planner::plan(&s, &o);
    assert!(!p.inventory.iter().any(|p| p.id == 564));
    assert!(p.inventory.iter().any(|p| p.id == 566 && p.quantity >= 300));
}
#[test]
fn sanfew_restoration_matches_super_restore_and_edited_poison_is_rechecked() {
    assert_eq!(
        ironbound::prayer::restore_points(
            99,
            &[Packed {
                id: 10925,
                quantity: 1,
                purpose: String::new()
            }]
        ),
        (99 / 4 + 8) * 4
    );
    let s = bank(true, true);
    let mut o = options();
    let p = planner::plan(&s, &o);
    for (i, item) in p.inventory.iter().enumerate() {
        if item.id == 10925 {
            o.inventory_overrides.insert(i, 0);
        }
    }
    let edited = planner::plan(&s, &o);
    assert!(
        edited
            .checks
            .iter()
            .any(|c| c.name == "Yama poison protection" && c.state == Some(false))
    );
}
#[test]
fn full_yama_kit_export_has_real_counts_identity_and_mark_assumption() {
    let s = bank(true, true);
    let o = options();
    let p = planner::plan(&s, &o);
    let json: serde_json::Value =
        serde_json::from_str(&export::inventory_setup(&p, &o, "").unwrap()).unwrap();
    assert_eq!(json["setup"]["name"], "Ironbound - Yama");
    assert_eq!(json["setup"]["eq"].as_array().unwrap().len(), 14);
    assert_eq!(json["setup"]["inv"].as_array().unwrap().len(), 28);
    assert!(json["setup"]["notes"].as_str().unwrap().contains("Mark"));
    let shared = share::Setup::from_plan(&p, &o);
    shared.validate().unwrap();
    assert!(shared.options().combat.mark_darkness);
    all_owned(&s, &p);
}
#[test]
fn judge_uses_a_fast_owned_melee_swap_and_optimizer_runs_without_manual_cap() {
    let s = bank(true, true);
    let mut o = options();
    o.max_switches = None;
    let p = planner::plan(&s, &o);
    assert!(p.optimization.is_some());
    let judge = p
        .phases
        .iter()
        .find(|p| p.name.contains("Judge") && rules::base_style(&p.style) == "Melee")
        .unwrap();
    let weapon = judge.equipment[3].unwrap();
    assert_eq!(
        combat::stats(weapon).unwrap().speed,
        4,
        "Judge needs an owned fast commitment rather than the slow axe"
    );
}
