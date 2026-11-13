//! Public synthetic fixtures. Loaded darts contribute strength, never worn ammo
//! accuracy; 100 shots is a preparation assumption, not a trip guarantee.
use ironbound::{
    account::{self, Member},
    combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    rules, share,
};
fn snapshot(stock: &[(i32, u32)]) -> account::Snapshot {
    let mut s = account::demo();
    s.members.truncate(1);
    s.members[0].bank = Some(
        stock
            .iter()
            .flat_map(|(id, q)| [i64::from(*id), i64::from(*q)])
            .collect(),
    );
    s.members[0].equipment = None;
    s.members[0].inventory = None;
    for skill in account::SKILLS {
        s.members[0].level_overrides.insert(skill.into(), 99);
    }
    s
}
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn settings(dart: i32) -> Settings {
    Settings {
        loaded_dart: Some(dart),
        ..Default::default()
    }
}
fn target() -> &'static planner::Monster {
    planner::monsters()
        .iter()
        .find(|m| m.name == "Giant Mole")
        .unwrap()
}
fn calc(m: &Member, s: &[Option<i32>], c: &Settings) -> combat::ResultStats {
    combat::calculate_configured(m, s, "Ranged", target(), "", 0, c).unwrap()
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: target().id,
            style: "Ranged".into(),
        }],
        food: 0,
        prayer_potions: 0,
        max_switches: Some(4),
        ..Default::default()
    }
}
#[test]
fn toxic_and_blazing_have_two_tick_rapid_and_three_tick_accurate() {
    let bank = snapshot(&[(12926, 1), (28688, 1), (11230, 100)]);
    for id in [12926, 28688] {
        let rapid = calc(bank.selected().unwrap(), &set(id), &settings(11230));
        assert_eq!(rapid.attack_ticks, 2);
        assert_eq!(rapid.attack_roll, 107 * (30 + 64));
        assert_eq!(rapid.max_hit, 20); // 20 weapon +35 Dragon dart strength.
        let mut c = settings(11230);
        c.ranged_stance = "Accurate".into();
        let accurate = calc(bank.selected().unwrap(), &set(id), &c);
        assert_eq!(accurate.attack_ticks, 3);
        assert_eq!(accurate.attack_roll, 110 * (30 + 64));
    }
}
#[test]
fn darts_add_strength_only_and_worn_arrows_are_not_fired() {
    let bank = snapshot(&[(12926, 1), (11230, 100), (811, 100)]);
    let m = bank.selected().unwrap();
    let dragon = calc(m, &set(12926), &settings(11230));
    let rune = calc(m, &set(12926), &settings(811));
    assert_eq!(dragon.attack_roll, rune.attack_roll);
    assert!(dragon.max_hit > rune.max_hit);
    let mut worn = set(12926);
    worn[13] = Some(892);
    let arrow = calc(m, &worn, &settings(11230));
    assert_eq!(arrow.attack_roll, dragon.attack_roll);
    assert_eq!(arrow.max_hit, dragon.max_hit);
}
#[test]
fn loaded_ammo_capabilities_reject_poison_and_respect_wooden_cap() {
    let bank = snapshot(&[
        (11230, 100),
        (810, 100),
        (811, 100),
        (11231, 100),
        (892, 100),
    ]);
    let m = bank.selected().unwrap();
    assert_eq!(
        rules::internal_ammo(m, 12926, &Settings::default()),
        Some(11230)
    );
    for weapon in [31575, 31579, 31583] {
        assert_eq!(
            rules::internal_ammo(m, weapon, &Settings::default()),
            Some(810)
        );
        assert_eq!(rules::internal_ammo(m, weapon, &settings(811)), None);
    }
    for bad in [11231, 892] {
        assert_eq!(rules::internal_ammo(m, 12926, &settings(bad)), None);
        assert!(
            combat::calculate_configured(m, &set(12926), "Ranged", target(), "", 0, &settings(bad))
                .is_err()
        );
    }
}
#[test]
fn automatic_loading_requires_one_hundred_and_base75_is_not_boostable() {
    let mut bank = snapshot(&[(12926, 1), (11230, 99), (811, 100)]);
    assert_eq!(
        rules::internal_ammo(bank.selected().unwrap(), 12926, &Settings::default()),
        Some(811)
    );
    bank.members[0].level_overrides.insert("Ranged".into(), 74);
    assert!(!planner::can_use(12926, bank.selected().unwrap()));
    let mut o = options();
    o.combat.boosts.ranged = "Ranging potion".into();
    // Arithmetic accepts hypothetical setups; the planner owns wear eligibility.
    assert_ne!(planner::plan(&bank, &o).equipment[3], Some(12926));
    bank.members[0].level_overrides.insert("Ranged".into(), 75);
    assert!(planner::can_use(12926, bank.selected().unwrap()));
    assert_eq!(planner::plan(&bank, &o).equipment[3], Some(12926));
}
#[test]
fn recommended_owned_blowpipe_keeps_loaded_darts_out_of_inventory_and_normalizes_sharing() {
    let bank = snapshot(&[(12926, 1), (11230, 100)]);
    let o = options();
    let p = planner::plan(&bank, &o);
    assert_eq!(p.equipment[3], Some(12926));
    assert_ne!(p.equipment[13], Some(11230));
    assert!(p.inventory.iter().all(|i| i.id != 11230));
    let loaded = p.loaded_ammo.as_ref().unwrap();
    assert_eq!((loaded.id, loaded.quantity), (11230, 100));
    assert!(p.inventory_slots() <= 28);
    assert!(
        p.phases
            .iter()
            .all(|phase| ironbound::optimization::changes(&p.equipment, &phase.equipment) <= 4)
    );
    let shared = share::Setup::from_plan(&p, &o);
    assert_eq!(shared.combat.loaded_dart, Some(11230));
    let roundtrip = share::Setup::parse(&shared.fragment().unwrap()).unwrap();
    assert_eq!(roundtrip.combat.loaded_dart, Some(11230));
    assert_eq!(roundtrip.plan().loaded_ammo.unwrap().id, 11230);
}
#[test]
fn absent_loaded_darts_need_explicit_confirmation_and_cannot_be_double_allocated() {
    let bank = snapshot(&[(12926, 1)]);
    let mut o = options();
    o.gear_overrides.insert("Ranged:3".into(), 12926);
    o.combat = settings(11230);
    assert!(
        planner::plan(&bank, &o)
            .missing
            .iter()
            .any(|m| m.contains("load"))
    );
    o.combat.loaded_ammo_confirmed = true;
    let confirmed = planner::plan(&bank, &o);
    assert!(confirmed.loaded_ammo.is_some());
    assert!(
        !confirmed
            .missing
            .iter()
            .any(|m| m.contains("needed to load"))
    );
    let bank = snapshot(&[(12926, 1), (11230, 100)]);
    o.combat.loaded_ammo_confirmed = false;
    o.inventory_overrides.insert(0, 11230);
    assert!(
        planner::plan(&bank, &o)
            .missing
            .iter()
            .any(|m| m.contains("in addition to carried darts"))
    );
}

#[test]
fn inventory_setup_export_keeps_loaded_darts_in_notes_and_bank_tags_only() {
    let bank = snapshot(&[(12926, 1), (11230, 100), (8007, 1)]);
    let mut o = options();
    o.combat.boosts.ranged = "Ranging potion".into();
    o.combat.boosts.defence = "Super defence".into();
    let p = planner::plan(&bank, &o);
    assert!(p.exportable(), "{:?}", p.missing);
    let text =
        ironbound::export::inventory_setup(&p, &o, "private account must not appear").unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let setup = &v["setup"];
    assert_eq!(setup["eq"].as_array().unwrap().len(), 14);
    assert_eq!(setup["inv"].as_array().unwrap().len(), 28);
    for field in ["eq", "inv"] {
        assert!(
            !setup[field]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["id"].as_i64() == Some(11230))
        );
    }
    assert_eq!(setup["eq"][3]["id"], 12926);
    assert!(setup["eq"][13].is_null());
    let notes = setup["notes"].as_str().unwrap();
    assert!(notes.contains(&planner::item_name(11230)));
    assert!(notes.contains("× 100 inside the blowpipe"));
    assert!(notes.contains("Ranging potion") && notes.contains("Super defence"));
    assert!(!text.contains("private account must not appear"));
    assert!(
        ironbound::export::bank_tags(&p, &o)
            .unwrap()
            .split(',')
            .any(|id| id == "11230")
    );
    assert!(
        !v["layout"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id.as_i64() == Some(11230))
    );
}
#[test]
fn shared_wooden_blowpipe_rejects_incompatible_loaded_dragon_darts() {
    let bank = snapshot(&[(31583, 1), (810, 100), (8007, 1)]);
    let o = options();
    let p = planner::plan(&bank, &o);
    assert_eq!(p.equipment[3], Some(31583));
    assert_eq!(p.loaded_ammo.as_ref().unwrap().id, 810);
    let mut shared = share::Setup::from_plan(&p, &o);
    assert!(shared.validate().is_ok());
    shared.combat.loaded_dart = Some(11230);
    assert!(shared.validate().is_err());
}
