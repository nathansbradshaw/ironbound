use ironbound::{
    account, optimization,
    planner::{self, Boss, Options},
};

#[test]
fn switch_preferences_roundtrip_without_private_account_data() {
    let bank = account::demo();
    let options = Options {
        boss: Boss::Toa,
        max_switches: Some(4),
        switch_slots: 8,
        focus_style: "Magic".into(),
        food: 4,
        ..Default::default()
    };
    let p = planner::plan(&bank, &options);
    let hash = ironbound::share::Setup::from_plan(&p, &options)
        .fragment()
        .unwrap();
    let decoded = ironbound::share::Setup::parse(&hash).unwrap().options();
    assert_eq!(decoded.max_switches, Some(4));
    assert_eq!(decoded.switch_slots, 8);
    assert_eq!(decoded.focus_style, "Magic");
}
#[test]
fn a_phase_priority_can_trade_damage_between_targets() {
    let bank = account::demo();
    let mut low = Options {
        max_switches: Some(2),
        switch_slots: 8,
        ..Default::default()
    };
    low.phase_priorities.insert(1672, 0);
    let mut high = low.clone();
    high.phase_priorities.insert(1672, 10);
    let low = planner::plan(&bank, &low).optimization.unwrap();
    let high = planner::plan(&bank, &high).optimization.unwrap();
    let damage = |r: &optimization::Report| {
        r.comparisons
            .iter()
            .find(|p| p.name.starts_with("Ahrim"))
            .unwrap()
            .after
            .unwrap()
    };
    assert!(damage(&high) >= damage(&low));
    assert!(low.max_changes <= 2 && high.max_changes <= 2);
}
#[test]
fn plain_partisan_does_not_grant_any_upgrade() {
    let bank = account::demo();
    let p = planner::plan(
        &bank,
        &Options {
            boss: Boss::Toa,
            max_switches: Some(4),
            switch_slots: 8,
            food: 4,
            ..Default::default()
        },
    );
    let equipped = p
        .equipment
        .iter()
        .flatten()
        .copied()
        .chain(p.inventory.iter().map(|i| i.id))
        .collect::<Vec<_>>();
    assert!(equipped.contains(&25979));
    for upgrade in [25981, 27291, 27287] {
        assert!(!equipped.contains(&upgrade));
        assert!(
            p.phases
                .iter()
                .all(|phase| !phase.equipment.contains(&Some(upgrade)))
        );
    }
    assert_eq!(
        p.phases
            .iter()
            .find(|p| p.name == "Kephri")
            .unwrap()
            .equipment[3],
        Some(25979)
    );
}
#[test]
fn four_way_limit_applies_between_every_pair_of_phases() {
    let p = planner::plan(
        &account::demo(),
        &Options {
            boss: Boss::Toa,
            max_switches: Some(4),
            switch_slots: 8,
            food: 6,
            ..Default::default()
        },
    );
    let report = p.optimization.as_ref().expect("feasible ToA budget");
    assert!(report.max_changes <= 4);
    assert!(report.gear_slots <= 8);
    assert!(p.exportable(), "{:?}", p.missing);
    let sets: Vec<_> = std::iter::once(&p.equipment)
        .chain(p.phases.iter().map(|p| &p.equipment))
        .collect();
    for a in &sets {
        for b in &sets {
            assert!(optimization::changes(a, b) <= 4);
        }
    }
}
#[test]
fn focused_magic_set_stays_intact() {
    let bank = account::demo();
    let o = Options {
        boss: Boss::Toa,
        max_switches: Some(4),
        switch_slots: 10,
        focus_style: "Magic".into(),
        food: 4,
        ..Default::default()
    };
    let expected = planner::equipment_for_style(&bank, &o, "Magic");
    let p = planner::plan(&bank, &o);
    assert!(p.optimization.is_some(), "{:?}", p.missing);
    assert_eq!(
        p.phases
            .iter()
            .find(|p| p.name == "Akkha")
            .unwrap()
            .equipment,
        expected
    );
}
#[test]
fn impossible_comfort_limits_never_silently_export() {
    let p = planner::plan(
        &account::demo(),
        &Options {
            boss: Boss::Toa,
            max_switches: Some(0),
            switch_slots: 0,
            ..Default::default()
        },
    );
    assert!(p.optimization.is_none());
    assert!(!p.exportable());
    assert!(p.missing.iter().any(|m| m.contains("No setup found")));
}
#[test]
fn constrained_sets_still_come_from_the_same_owned_pool() {
    let bank = account::demo();
    let p = planner::plan(
        &bank,
        &Options {
            max_switches: Some(4),
            switch_slots: 6,
            ..Default::default()
        },
    );
    assert!(p.optimization.is_some());
    let owned = bank.available(false);
    let mut carried = std::collections::BTreeMap::<i32, u32>::new();
    for (slot, id) in p.equipment.iter().enumerate() {
        if let Some(id) = id {
            *carried.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
        }
    }
    for i in &p.inventory {
        *carried.entry(i.id).or_default() += i.quantity;
    }
    for (id, q) in carried {
        assert!(q <= owned[&id]);
    }
}
#[test]
fn full_owned_catalog_fills_common_magic_alternatives() {
    let mut bank = account::demo();
    let m = &mut bank.members[0];
    m.bank
        .as_mut()
        .unwrap()
        .extend([3755, 1, 21795, 1, 4097, 1, 28327, 1]);
    m.level_overrides.insert("Magic".into(), 86);
    let p = planner::plan(&bank, &Options::default());
    assert_eq!(p.equipment[0], Some(3755));
    assert_eq!(p.equipment[1], Some(21795));
    assert_eq!(p.equipment[10], Some(4097));
    assert_eq!(p.equipment[12], Some(28327));
}

fn prayer_fixture() -> (
    account::Member,
    std::collections::BTreeMap<i32, u32>,
    Vec<optimization::StyleSet>,
    Options,
) {
    let mut m = account::Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    let owned = std::collections::BTreeMap::from([(4151, 1), (11905, 1), (10551, 1), (10462, 1)]);
    let mut melee = vec![None; 14];
    melee[3] = Some(4151);
    melee[4] = Some(10551);
    let mut magic = vec![None; 14];
    magic[3] = Some(11905);
    magic[4] = Some(10462);
    let ideals = vec![
        optimization::StyleSet {
            style: "Magic".into(),
            equipment: magic,
        },
        optimization::StyleSet {
            style: "Melee".into(),
            equipment: melee,
        },
    ];
    let mut o = Options {
        boss: Boss::Custom,
        targets: vec![
            planner::Target {
                id: 100012,
                style: "Magic".into(),
            },
            planner::Target {
                id: 100013,
                style: "Melee".into(),
            },
        ],
        max_switches: Some(2),
        switch_slots: 2,
        focus_style: "Melee".into(),
        recommended: Some(true),
        ..Default::default()
    };
    o.combat.protection = "Melee".into();
    (m, owned, ideals, o)
}
#[test]
fn equal_damage_uses_prayer_armour_when_the_extra_switch_fits() {
    let (m, owned, ideals, o) = prayer_fixture();
    let (sets, report) = optimization::solve(&m, &owned, ideals.clone(), &o).unwrap();
    let magic = &sets.iter().find(|s| s.style == "Magic").unwrap().equipment;
    let melee = &sets.iter().find(|s| s.style == "Melee").unwrap().equipment;
    assert_eq!(magic[4], Some(10462)); // Guthix robes add prayer without magic damage.
    assert_eq!(melee[4], Some(10551)); // Focused melee keeps its strength-bearing torso.
    assert_eq!(report.max_changes, 2);
    assert_eq!(report.gear_slots, 2);
    assert!(report.comparisons.iter().all(|c| c.before == c.after));
    let mut hybrid = magic.clone();
    hybrid[4] = Some(10551);
    assert!(
        ironbound::prayer::drain_per_minute(ironbound::prayer::bonus(magic), &o.combat)
            < ironbound::prayer::drain_per_minute(ironbound::prayer::bonus(&hybrid), &o.combat)
    );
    let full = ironbound::yama::judge_cycle(&m, melee, magic, &o, &owned).unwrap();
    let shared = ironbound::yama::judge_cycle(&m, melee, &hybrid, &o, &owned).unwrap();
    assert_eq!(full.seconds, shared.seconds);
    assert!(report.note.contains("supported attack windows"));
}
#[test]
fn prayer_tie_break_never_exceeds_the_switch_or_carried_item_budget() {
    let (m, owned, ideals, mut o) = prayer_fixture();
    o.max_switches = Some(1);
    let (sets, report) = optimization::solve(&m, &owned, ideals.clone(), &o).unwrap();
    assert_eq!(report.max_changes, 1);
    assert_eq!(
        sets.iter().find(|s| s.style == "Magic").unwrap().equipment[4],
        Some(10551)
    );
    o.max_switches = Some(2);
    o.switch_slots = 1;
    let (sets, report) = optimization::solve(&m, &owned, ideals, &o).unwrap();
    assert_eq!(report.gear_slots, 1);
    assert_eq!(
        sets.iter().find(|s| s.style == "Magic").unwrap().equipment[4],
        Some(10551)
    );
}
#[test]
fn inactive_prayers_keep_equal_damage_setups_to_fewer_switches() {
    let (m, owned, ideals, mut o) = prayer_fixture();
    o.combat.protection = "None".into();
    let (sets, report) = optimization::solve(&m, &owned, ideals, &o).unwrap();
    assert_eq!(report.gear_slots, 1);
    assert_eq!(report.max_changes, 1);
    assert_eq!(
        sets.iter().find(|s| s.style == "Magic").unwrap().equipment[4],
        Some(10551)
    );
}
#[test]
fn prayer_savings_never_replace_a_better_primary_damage_score() {
    let (m, mut owned, mut ideals, o) = prayer_fixture();
    // An occult adds damage; a prayer-bearing stole must not replace it.
    owned.insert(12002, 1);
    owned.insert(10472, 1);
    ideals[0].equipment[2] = Some(12002);
    ideals[1].equipment[2] = Some(10472);
    let mut o = o;
    o.max_switches = Some(3);
    o.switch_slots = 3;
    let (sets, _) = optimization::solve(&m, &owned, ideals, &o).unwrap();
    assert_eq!(
        sets.iter().find(|s| s.style == "Magic").unwrap().equipment[2],
        Some(12002)
    );
}
