//! Public synthetic fixtures; no imported bank or identity data.
use ironbound::{
    account, alternatives,
    planner::{self, Boss, Options, Packed, Phase, Plan, Target},
};
use std::collections::BTreeSet;
fn id(name: &str) -> i32 {
    planner::equipment()
        .iter()
        .filter(|g| g.name == name && !matches!(g.version.as_str(), "Broken" | "0" | "Inactive"))
        .map(|g| g.id)
        .min()
        .unwrap_or_else(|| panic!("Missingpublicitem{name}"))
}
fn bank(stock: &[(i32, u32)]) -> account::Snapshot {
    let mut s = account::demo();
    s.members[0].bank = Some(
        stock
            .iter()
            .flat_map(|(id, q)| [i64::from(*id), i64::from(*q)])
            .collect(),
    );
    s.members[0].inventory = None;
    s.members[0].equipment = None;
    s.members[0].shared_bank = None;
    s.members[0].rune_pouch = None;
    s.members[0].potion_storage = None;
    s.members[0].quests = None;
    for skill in account::SKILLS {
        s.members[0].level_overrides.insert(skill.into(), 99);
    }
    s
}
fn options(style: &str, target: i32) -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: target,
            style: style.into(),
        }],
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    }
}
fn plan(o: &Options, set: Vec<Option<i32>>) -> Plan {
    Plan {
        ammo_shots: o.combat.ammo_shots,
        loaded_ammo: None,
        optimization: None,
        equipment: set.clone(),
        inventory: vec![],
        phases: vec![Phase {
            name: o.targets[0].name(),
            style: o.targets[0].style.clone(),
            equipment: set,
            note: String::new(),
        }],
        checks: vec![],
        warnings: vec![],
        missing: vec![],
        switches: 0,
        supported: 0,
        owned: 0,
        label: "Synthetic owned plan".into(),
        spell: String::new(),
    }
}
fn melee() -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(4151);
    s
}
#[test]
fn owned_usable_excluded_and_level_gates_apply_without_quest_checks() {
    let rune = id("Rune full helm");
    let neit = 10828;
    let high = 24271;
    let mut s = bank(&[(4151, 1), (rune, 1), (neit, 1), (high, 1), (3755, 1)]);
    s.members[0].level_overrides.insert("Defence".into(), 60);
    s.members[0]
        .quest_overrides
        .insert("The Fremennik Isles".into(), 0);
    let mut o = options("Slash", 1673);
    o.excluded.insert(3755);
    let mut set = melee();
    set[0] = Some(rune);
    let p = plan(&o, set);
    let rows = alternatives::equipment(&s, &o, &p, None, 0, 8);
    assert!(rows.iter().any(|r| r.id == neit));
    assert!(
        rows.iter()
            .all(|r| r.id != rune && r.id != high && r.id != 3755)
    );
    assert!(rows.iter().all(|r| s.available(false).contains_key(&r.id)));
}
#[test]
fn ranged_ammunition_requires_compatible_owned_hundred_stack() {
    let s = bank(&[(9185, 1), (877, 100), (9242, 99), (9243, 100), (892, 1000)]);
    let o = options("Ranged", 1672);
    let mut set = vec![None; 14];
    set[3] = Some(9185);
    set[13] = Some(877);
    let p = plan(&o, set);
    let rows = alternatives::equipment(&s, &o, &p, None, 13, 8);
    assert!(rows.iter().any(|r| r.id == 9243));
    assert!(
        rows.iter()
            .all(|r| r.id != 9242 && r.id != 892 && r.id != 877)
    );
}
#[test]
fn two_handed_weapon_accounts_for_shield_ammo_and_pairwise_cap() {
    let s = bank(&[
        (9185, 1),
        (12788, 1),
        (1540, 1),
        (9144, 100),
        (892, 100),
        (4151, 1),
    ]);
    let mut o = options("Ranged", 1672);
    o.max_switches = Some(2);
    o.switch_slots = 10;
    let mut set = vec![None; 14];
    set[3] = Some(9185);
    set[5] = Some(1540);
    set[13] = Some(9144);
    let mut p = plan(&o, set);
    let mut slash = melee();
    slash[5] = Some(1540);
    p.phases.push(Phase {
        name: planner::monster(1673).unwrap().name.clone(),
        style: "Slash".into(),
        equipment: slash,
        note: String::new(),
    });
    o.targets.push(Target {
        id: 1673,
        style: "Slash".into(),
    });
    p.inventory = vec![Packed {
        id: 4151,
        quantity: 1,
        purpose: "Melee switch".into(),
    }];
    let rows = alternatives::equipment(&s, &o, &p, None, 3, 4);
    let bow = rows
        .iter()
        .find(|r| r.id == 12788)
        .expect("owned compatible bow what-if");
    assert!(!bow.fits_budget);
    assert!(bow.max_changes >= 3);
    let note = bow.note.as_deref().unwrap();
    assert!(note.contains("shield removed"));
    assert!(note.contains("ammunition"));
}
#[test]
fn style_replacement_updates_start_and_every_matching_phase() {
    let s = bank(&[(4151, 1), (1127, 1), (10551, 1)]);
    let mut o = options("Slash", 1673);
    o.max_switches = Some(0);
    o.switch_slots = 0;
    let mut set = melee();
    set[4] = Some(1127);
    let mut p = plan(&o, set.clone());
    p.phases.push(p.phases[0].clone());
    let rows = alternatives::equipment(&s, &o, &p, Some(0), 4, 2);
    let torso = rows.iter().find(|r| r.id == 10551).unwrap();
    assert_eq!(torso.max_changes, 0);
    assert!(
        torso.fits_budget,
        "Update the full style, not only one phase: {:?}",
        torso.note
    );
}
#[test]
fn replacing_gear_cannot_silently_drop_supplies_to_claim_bag_fit() {
    let rune = id("Rune full helm");
    let s = bank(&[
        (4151, 1),
        (9185, 1),
        (9144, 100),
        (rune, 1),
        (10828, 1),
        (385, 26),
    ]);
    let mut o = options("Slash", 1673);
    o.max_switches = Some(4);
    o.switch_slots = 10;
    o.targets.push(Target {
        id: 1672,
        style: "Ranged".into(),
    });
    let mut set = melee();
    set[0] = Some(rune);
    let mut p = plan(&o, set.clone());
    let mut ranged = set;
    ranged[3] = Some(9185);
    ranged[13] = Some(9144);
    p.phases.push(Phase {
        name: planner::monster(1672).unwrap().name.clone(),
        style: "Ranged".into(),
        equipment: ranged,
        note: String::new(),
    });
    p.inventory = vec![
        Packed {
            id: 9185,
            quantity: 1,
            purpose: "Ranged switch".into(),
        },
        Packed {
            id: 9144,
            quantity: 100,
            purpose: "Ammo".into(),
        },
    ];
    p.inventory.extend((0..26).map(|_| Packed {
        id: 385,
        quantity: 1,
        purpose: "Food".into(),
    }));
    let rows = alternatives::equipment(&s, &o, &p, None, 0, 4);
    let neit = rows.iter().find(|r| r.id == 10828).unwrap();
    assert!(!neit.fits_budget);
    assert!(neit.note.as_deref().unwrap().contains("29/28"));
}
#[test]
fn unsupported_target_never_becomes_zero_dps_delta() {
    let rune = id("Rune full helm");
    let s = bank(&[(4151, 1), (rune, 1), (10828, 1)]);
    let o = options("Slash", 11750);
    let mut set = melee();
    set[0] = Some(rune);
    let p = plan(&o, set);
    let rows = alternatives::equipment(&s, &o, &p, None, 0, 2);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.dps_delta.is_none()));
    assert!(
        rows.iter()
            .all(|r| r.note.as_deref().unwrap().contains("DPS delta unavailable"))
    );
}
#[test]
fn top_choices_are_unique_and_include_distinct_damage_prayer_tradeoffs() {
    let iron = id("Iron platebody");
    let prayer = id("Proselyte hauberk");
    let tank = id("Torag's platebody");
    let s = bank(&[(4151, 1), (iron, 1), (10551, 1), (prayer, 1), (tank, 1)]);
    let mut o = options("Slash", 1673);
    o.combat.offensive = "Piety".into();
    o.gear_overrides.insert("Slash:4".into(), iron);
    let mut set = melee();
    set[4] = Some(iron);
    let p = plan(&o, set);
    let rows = alternatives::equipment(&s, &o, &p, None, 4, 2);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.iter().map(|r| r.id).collect::<BTreeSet<_>>().len(), 2);
    assert!(
        rows.iter()
            .any(|r| r.id == 10551 && r.dps_delta.is_some_and(|d| d > 0.0))
    );
    assert!(rows.iter().any(|r| r.id == prayer && r.prayer_delta < 0.0));
}
#[test]
fn packed_supplies_spares_and_unverified_shared_setups_do_not_invent_context() {
    let s = bank(&[(4151, 1), (10551, 1), (1127, 1), (385, 10), (5698, 1)]);
    let o = options("Slash", 1673);
    let mut set = melee();
    set[4] = Some(1127);
    let mut p = plan(&o, set);
    assert!(
        alternatives::packed(
            &s,
            &o,
            &p,
            &Packed {
                id: 385,
                quantity: 1,
                purpose: "Food".into()
            },
            2
        )
        .is_empty()
    );
    assert!(
        alternatives::packed(
            &s,
            &o,
            &p,
            &Packed {
                id: 5698,
                quantity: 1,
                purpose: "Spare special".into()
            },
            2
        )
        .is_empty()
    );
    p.label = "Shared setup · unverified".into();
    assert!(alternatives::equipment(&s, &o, &p, None, 4, 2).is_empty());
}

#[test]
fn loaded_dart_settings_require_owned_stock_or_explicit_existing_contents() {
    let mut s = bank(&[(9185, 1), (877, 100), (12926, 1)]);
    let mut o = options("Ranged", 1672);
    o.combat.loaded_dart = Some(11230);
    let mut set = vec![None; 14];
    set[3] = Some(9185);
    set[13] = Some(877);
    let p = plan(&o, set);
    assert!(
        !alternatives::equipment(&s, &o, &p, None, 3, 4)
            .iter()
            .any(|r| r.id == 12926)
    );
    o.combat.loaded_ammo_confirmed = true;
    let rows = alternatives::equipment(&s, &o, &p, None, 3, 4);
    assert!(
        rows.iter().any(|r| r.id == 12926 && r.fits_budget),
        "Explicit configured contents are allowed without inventing bank darts"
    );
    o.combat.loaded_ammo_confirmed = false;
    s.members[0].bank.as_mut().unwrap().extend([11230, 100]);
    let mut packed = p.clone();
    packed.inventory.push(Packed {
        id: 11230,
        quantity: 100,
        purpose: "Spare darts".into(),
    });
    let rows = alternatives::equipment(&s, &o, &packed, None, 3, 4);
    let blowpipe = rows.iter().find(|r| r.id == 12926).unwrap();
    assert!(!blowpipe.fits_budget);
    assert!(blowpipe.note.as_deref().unwrap().contains("Dart stock"));
}
#[test]
fn shared_stock_is_opt_in_and_never_bypasses_exclusions() {
    let rune = id("Rune full helm");
    let mut s = bank(&[(4151, 1), (rune, 1)]);
    s.members[0].shared_bank = Some(vec![10828, 1]);
    let mut o = options("Slash", 1673);
    let mut set = melee();
    set[0] = Some(rune);
    let p = plan(&o, set);
    assert!(
        !alternatives::equipment(&s, &o, &p, None, 0, 2)
            .iter()
            .any(|r| r.id == 10828)
    );
    o.shared = true;
    assert!(
        alternatives::equipment(&s, &o, &p, None, 0, 2)
            .iter()
            .any(|r| r.id == 10828)
    );
    o.excluded.insert(10828);
    assert!(
        !alternatives::equipment(&s, &o, &p, None, 0, 2)
            .iter()
            .any(|r| r.id == 10828)
    );
}
#[test]
fn new_spell_rune_templates_are_a_rebuild_not_a_claimed_bag_fit() {
    let s = bank(&[(11907, 1), (29594, 1), (554, 3000), (566, 1000), (564, 10)]);
    let mut o = options("Magic", 100014);
    o.combat.mark_darkness = true;
    let mut set = vec![None; 14];
    set[3] = Some(11907);
    let p = plan(&o, set);
    let rows = alternatives::equipment(&s, &o, &p, None, 3, 4);
    let purging = rows.iter().find(|r| r.id == 29594).unwrap();
    assert!(purging.dps_delta.is_some_and(|d| d > 0.0));
    assert!(!purging.fits_budget);
    assert!(
        purging
            .note
            .as_deref()
            .unwrap()
            .contains("Spell supplies change")
    );
}

#[test]
fn strongest_current_weapon_still_offers_a_weaker_owned_backup() {
    let s = bank(&[(4151, 1), (4587, 1)]);
    let o = options("Slash", 1673);
    let p = plan(&o, melee());
    let rows = alternatives::equipment(&s, &o, &p, None, 3, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 4587);
    assert!(rows[0].dps_delta.unwrap() < 0.0);
    assert!(rows[0].fits_budget);
}

#[test]
fn unknown_target_weapon_backups_preserve_unknown_damage() {
    let s = bank(&[(4151, 1), (4587, 1)]);
    let o = options("Slash", 11750);
    let p = plan(&o, melee());
    let rows = alternatives::equipment(&s, &o, &p, None, 3, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 4587);
    assert!(rows[0].dps_delta.is_none());
    assert!(
        rows[0]
            .note
            .as_deref()
            .unwrap()
            .contains("DPS delta unavailable")
    );
}

#[test]
fn unknown_equipment_stats_do_not_manufacture_prayer_savings() {
    let s = bank(&[(4151, 1), (9_999_999, 1), (10828, 1)]);
    let o = options("Slash", 1673);
    let mut set = melee();
    set[0] = Some(9_999_999);
    let p = plan(&o, set);
    assert!(alternatives::equipment(&s, &o, &p, None, 0, 2).is_empty());
}

#[test]
fn carried_alternative_keeps_its_own_phase_style_for_direct_equipping() {
    let s = bank(&[(4151, 1), (11905, 1), (1127, 1), (4091, 1), (10462, 1)]);
    let mut o = options("Slash", 1673);
    o.targets.push(Target {
        id: 1672,
        style: "Magic".into(),
    });
    let mut p = plan(&o, melee());
    let mut magic = vec![None; 14];
    magic[3] = Some(11905);
    magic[4] = Some(4091);
    p.phases.push(Phase {
        name: o.targets[1].name(),
        style: "Magic".into(),
        equipment: magic,
        note: String::new(),
    });
    let rows = alternatives::packed(
        &s,
        &o,
        &p,
        &Packed {
            id: 4091,
            quantity: 1,
            purpose: "Magic switch".into(),
        },
        2,
    );
    assert!(!rows.is_empty());
    for row in &rows {
        assert_eq!(row.style, "Magic");
        assert_eq!(row.slot, 4);
    }
    let html = ironbound::tooltip::render("Magic switch", Some(4091), &rows, None);
    assert!(html.contains("data-action=\"pick-gear:Magic:4:"));
    assert!(!html.contains("pick-gear:Slash:"));
}
