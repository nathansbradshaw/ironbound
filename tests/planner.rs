use ironbound::{
    account::{self, Member, SKILLS, Snapshot},
    export,
    planner::{self, Boss, Options},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn an_empty_checklist_does_not_claim_readiness() {
    let mut plan = planner::plan(&account::demo(), &Options::default());
    plan.checks.clear();
    plan.missing.clear();
    assert_eq!(plan.status(), "No encounter-specific checks");
    plan.missing.push("Required supply".into());
    assert_eq!(plan.status(), "Preparation needed");
}

#[test]
fn local_corrections_preserve_unknown_skills() {
    let mut m = Member {
        name: "Iron".into(),
        ..Default::default()
    };
    m.level_overrides.insert("Magic".into(), 75);
    assert_eq!(m.level("Magic"), Some(75));
    assert_eq!(m.level("Attack"), None);
    assert_eq!(m.total_level(), None);
}
#[test]
fn rune_axe_is_not_a_puzzle_pickaxe() {
    let mut s = account::demo();
    for pair in s.members[0].bank.as_mut().unwrap().chunks_exact_mut(2) {
        if pair[0] == 1275 {
            pair[0] = 1359;
        }
    }
    let p = planner::plan(
        &s,
        &Options {
            boss: Boss::Toa,
            ..Default::default()
        },
    );
    assert!(p.missing.iter().any(|m| m.contains("pickaxe")));
    assert!(!p.inventory.iter().any(|p| p.id == 1359));
}
#[test]
fn quest_status_does_not_filter_owned_equipment() {
    let mut s = account::demo();
    s.members[0].quests = None;
    assert!(planner::can_use(10828, &s.members[0]));
    s.members[0].level_overrides.insert("Defence".into(), 50);
    assert!(!planner::can_use(10828, &s.members[0]));
}
#[test]
fn unsupported_skill_corrections_rejected() {
    assert!(
        Snapshot::parse(
            r#"[{"name":"Iron","level_overrides":{"Magic":100}}]"#,
            "now".into()
        )
        .is_err()
    );
}

#[test]
fn xp_boundaries() {
    assert_eq!(account::xp_for_level(2), 83);
    assert_eq!(account::xp_for_level(99), 13_034_431);
    assert_eq!(account::level_for_xp(82), 1);
    assert_eq!(account::level_for_xp(83), 2);
    assert_eq!(account::level_for_xp(200_000_000), 99);
}
#[test]
fn snapshot_alphabetical_xp_order() {
    let mut skills = vec![0; 24];
    skills[13] = account::xp_for_level(75);
    skills[1] = account::xp_for_level(60);
    let s = Snapshot::parse(
        &json!([{"name":"Iron","skills":skills}]).to_string(),
        "now".into(),
    )
    .unwrap();
    assert_eq!(s.selected().unwrap().level("Magic"), Some(75));
    assert_eq!(s.selected().unwrap().level("Attack"), Some(60));
    assert_eq!(s.selected().unwrap().level("Prayer"), Some(1));
}
#[test]
fn missing_bank_stays_unknown() {
    let s = Snapshot::parse(r#"[{"name":"Iron","inventory":[1381,1]}]"#, "now".into()).unwrap();
    assert!(s.selected().unwrap().bank.is_none());
    assert!(!planner::plan(&s, &Options::default()).exportable());
}
#[test]
fn malformed_pairs_rejected() {
    for raw in [
        r#"[{"name":"Iron","bank":[1381]}]"#,
        r#"[{"name":"Iron","bank":[1381,-1]}]"#,
        r#"[{"name":"Iron","skills":[75]}]"#,
    ] {
        assert!(Snapshot::parse(raw, "now".into()).is_err());
    }
}
#[test]
fn duplicate_members_rejected() {
    assert!(Snapshot::parse(r#"[{"name":"Iron"},{"name":"iron"}]"#, "now".into()).is_err());
}
#[test]
fn unsupported_items_preserved() {
    let s = Snapshot::parse(
        r#"[{"name":"Iron","bank":[999999,7,1381,1]}]"#,
        "now".into(),
    )
    .unwrap();
    assert_eq!(s.available(false)[&999999], 7);
}
#[test]
fn shared_bank_is_counted_once() {
    let s=Snapshot::parse(r#"[{"name":"One","bank":[385,2],"shared_bank":[385,10]},{"name":"Two","bank":[385,100],"shared_bank":[385,10]}]"#,"now".into()).unwrap();
    assert_eq!(s.available(false)[&385], 2);
    assert_eq!(s.available(true)[&385], 12);
}
#[test]
fn dedicated_shared_member_takes_precedence() {
    let s=Snapshot::parse(r#"[{"name":"One","bank":[385,2],"shared_bank":[385,10]},{"name":"@SHARED","bank":[385,4]}]"#,"now".into()).unwrap();
    assert_eq!(s.available(true)[&385], 6);
}
#[test]
fn dummy_container_slots_are_not_owned() {
    let s = Snapshot::parse(
        r#"[{"name":"Iron","bank":[-1,0,1381,1,385,0]}]"#,
        "now".into(),
    )
    .unwrap();
    assert_eq!(s.available(false).len(), 1);
}
#[test]
fn demo_barrows_is_a_complete_starting_plan() {
    let s = account::demo();
    let p = planner::plan(&s, &Options::default());
    assert!(p.missing.is_empty(), "{:?}", p.missing);
    assert!(p.exportable(), "{:?}", p.checks);
    assert_eq!(p.spell, "Wind Wave");
    assert!(p.inventory.iter().any(|p| p.id == 952));
}
#[test]
fn imported_quest_map_handles_access() {
    let s = account::demo();
    let m = s.selected().unwrap();
    assert_eq!(m.quest("Priest in Peril"), Some(2));
    assert_eq!(m.quest("Beneath Cursed Sands"), Some(2));
    assert_eq!(m.quest("His Faithful Servants"), Some(2));
}
#[test]
fn quest_status_never_blocks_exports() {
    let mut s = account::demo();
    s.members[0].quests = None;
    let o = Options {
        access_override: Some(false),
        started_barrows: Some(false),
        ..Default::default()
    };
    let p = planner::plan(&s, &o);
    assert!(p.exportable());
    assert!(export::inventory_setup(&p, &o, "Iron").is_ok());
    assert!(
        p.checks
            .iter()
            .all(|c| !c.detail.to_lowercase().contains("quest"))
    );
}
#[test]
fn starter_guidelines_are_advisory_but_item_levels_still_apply() {
    let mut bank = account::demo();
    bank.members[0].level_overrides.insert("Prayer".into(), 1);
    let p = planner::plan(&bank, &Options::default());
    assert!(p.checks.iter().any(|c| c.state == Some(false)));
    assert!(p.exportable());
    bank.members[0].level_overrides.insert("Magic".into(), 1);
    assert!(!planner::plan(&bank, &Options::default()).exportable());
}
#[test]
fn exclusions_are_honoured() {
    let s = account::demo();
    let mut o = Options::default();
    o.excluded.insert(1381);
    let p = planner::plan(&s, &o);
    assert!(!p.equipment.contains(&Some(1381)));
    assert!(!p.inventory.iter().any(|p| p.id == 1381));
}
#[test]
fn underlevelled_weapons_are_not_selected() {
    let mut s = account::demo();
    s.members[0].skills.as_mut().unwrap()[SKILLS.iter().position(|s| *s == "Magic").unwrap()] =
        account::xp_for_level(50);
    let o = Options {
        boss: Boss::Toa,
        ..Default::default()
    };
    let p = planner::plan(&s, &o);
    assert!(
        p.phases
            .iter()
            .find(|p| p.name == "Akkha")
            .unwrap()
            .equipment[3]
            .is_none()
    );
    assert!(!p.exportable());
}
#[test]
fn insufficient_ammunition_blocks_export() {
    let mut s = account::demo();
    let bank = s.members[0].bank.as_mut().unwrap();
    for p in bank.chunks_exact_mut(2) {
        if [9142, 9242].contains(&p[0]) {
            p[1] = 1;
        }
    }
    let p = planner::plan(&s, &Options::default());
    assert!(p.missing.iter().any(|m| m.contains("100 compatible")));
    assert!(!p.exportable());
}
#[test]
fn excess_supplies_do_not_silently_truncate() {
    let s = account::demo();
    let o = Options {
        boss: Boss::Toa,
        food: 20,
        prayer_potions: 10,
        ..Default::default()
    };
    let p = planner::plan(&s, &o);
    assert!(p.inventory.len() > 28);
    assert!(!p.exportable());
}
#[test]
fn toa_carries_every_phase_weapon() {
    let s = account::demo();
    let o = Options {
        boss: Boss::Toa,
        ..Default::default()
    };
    let p = planner::plan(&s, &o);
    assert!(p.exportable(), "{:?}", p.missing);
    assert_eq!(p.phases.len(), 5);
    let carried: Vec<i32> = p
        .inventory
        .iter()
        .map(|i| i.id)
        .chain(p.equipment.iter().flatten().copied())
        .collect();
    for phase in &p.phases {
        for id in phase.equipment.iter().flatten() {
            assert!(
                carried.contains(id),
                "{} needs absent item {id}",
                phase.name
            );
        }
    }
}
#[test]
fn all_plans_respect_owned_quantities() {
    let s = account::demo();
    for boss in [Boss::Barrows, Boss::Toa] {
        for fewer in [false, true] {
            let o = Options {
                boss,
                fewer_switches: fewer,
                ..Default::default()
            };
            let p = planner::plan(&s, &o);
            let mut used = BTreeMap::new();
            for (slot, id) in p.equipment.iter().enumerate() {
                if let Some(id) = id {
                    *used.entry(*id).or_insert(0u32) += if slot == 13 { 100 } else { 1 };
                }
            }
            for packed in &p.inventory {
                *used.entry(packed.id).or_insert(0) += packed.quantity;
            }
            for (id, q) in used {
                assert!(
                    q <= s.available(false).get(&id).copied().unwrap_or(0),
                    "{id}: {q} exceeds ownership"
                );
            }
        }
    }
}
#[test]
fn fewer_switches_reduces_carried_gear() {
    let s = account::demo();
    let o = Options {
        boss: Boss::Toa,
        ..Default::default()
    };
    let a = planner::plan(&s, &o);
    let b = planner::plan(
        &s,
        &Options {
            fewer_switches: true,
            ..o
        },
    );
    assert!(b.switches < a.switches);
    assert!(b.exportable());
}
#[test]
fn export_uses_portable_wrapper_and_full_containers() {
    let s = account::demo();
    for boss in [Boss::Barrows, Boss::Toa] {
        let o = Options {
            boss,
            ..Default::default()
        };
        let p = planner::plan(&s, &o);
        let v: Value =
            serde_json::from_str(&export::inventory_setup(&p, &o, &s.selected).unwrap()).unwrap();
        assert_eq!(v["setup"]["inv"].as_array().unwrap().len(), 28);
        assert_eq!(v["setup"]["eq"].as_array().unwrap().len(), 14);
        assert!(v["layout"].is_array());
        assert!(v["setup"]["afi"].is_object());
        assert_eq!(v["setup"]["fb"], true);
        assert!(v["setup"]["name"].as_str().unwrap().len() <= 50);
    }
}
#[test]
fn next_goals_follow_skills_and_access() {
    let s = account::demo();
    let goals = planner::goals(&s, Boss::Barrows);
    assert!(
        goals
            .iter()
            .find(|g| g.id == "barrows-magic")
            .unwrap()
            .complete
    );
    assert!(
        !goals
            .iter()
            .find(|g| g.id == "barrows-hilt")
            .unwrap()
            .complete
    );
    assert!(
        goals
            .iter()
            .find(|g| g.id == "barrows-hilt")
            .unwrap()
            .next
            .contains("medium Combat Achievements")
    );
}
#[test]
fn credentials_never_enter_snapshot() {
    let s = Snapshot::parse(
        r#"[{"name":"Iron","bank":[],"token":"secret","Authorization":"secret"}]"#,
        "now".into(),
    )
    .unwrap();
    let serialized = serde_json::to_string(&s).unwrap();
    assert!(!serialized.contains("secret"));
}
#[test]
fn potion_storage_doses_are_not_treated_as_bottles() {
    let mut s = account::demo();
    s.members[0].potion_storage = Some(vec![2434, 100]);
    assert_eq!(s.available(false).get(&2434), Some(&12));
    assert!(
        planner::plan(&s, &Options::default())
            .warnings
            .iter()
            .any(|w| w.contains("doses"))
    );
}
#[test]
fn equipment_stays_in_its_correct_slot() {
    let s = account::demo();
    let keys = [
        "head", "cape", "neck", "weapon", "body", "shield", "", "legs", "", "hands", "feet", "",
        "ring", "ammo",
    ];
    for b in [Boss::Barrows, Boss::Toa] {
        let p = planner::plan(
            &s,
            &Options {
                boss: b,
                ..Default::default()
            },
        );
        for phase in p.phases {
            for (slot, id) in phase.equipment.iter().enumerate() {
                if let Some(id) = id {
                    assert_eq!(
                        planner::equipment()
                            .iter()
                            .find(|g| g.id == *id)
                            .unwrap()
                            .slot,
                        keys[slot]
                    );
                }
            }
        }
    }
}
#[test]
fn no_phantom_member_on_empty_snapshot() {
    let s = Snapshot {
        members: vec![Member::default()],
        selected: "Absent".into(),
        fetched_at: "now".into(),
        source: "test".into(),
    };
    assert!(!planner::plan(&s, &Options::default()).exportable());
}
