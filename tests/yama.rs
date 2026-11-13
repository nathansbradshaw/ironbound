//! Reviewed normal-fight windows, not a claim of a full Yama encounter simulation.
//! Sources: https://oldschool.runescape.wiki/w/Yama
//! https://oldschool.runescape.wiki/w/Yama/Strategies
//! https://oldschool.runescape.wiki/w/Judge_of_Yama
use ironbound::{
    account, combat, encounters,
    planner::{self, Boss, Options},
    rules,
};
fn member() -> account::Member {
    let mut member = account::Member::default();
    for skill in account::SKILLS {
        member.level_overrides.insert(skill.into(), 99);
    }
    member
}
fn dps(target: i32, style: &str, weapon: i32) -> f64 {
    let mut set = vec![None; 14];
    set[3] = Some(weapon);
    combat::calculate(
        &member(),
        &set,
        style,
        planner::monster(target).unwrap(),
        "",
    )
    .unwrap()
    .dps
}
#[test]
fn normal_yama_has_explicit_slash_and_required_judge_magic_windows() {
    let entry = encounters::catalog()
        .iter()
        .find(|entry| entry.name == "Yama")
        .unwrap();
    assert_eq!(
        entry
            .targets
            .iter()
            .map(|t| (t.id, t.style.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (14176, "Slash"),
            (100012, "Magic"),
            (100013, "Melee"),
            (100011, "Slash"),
            (100014, "Magic")
        ]
    );
    assert!(entry.coverage().contains("partial"));
    assert!(entry.note.contains("Normal Yama"));
    assert!(
        encounters::browser_catalog()
            .popular
            .contains(&"Yama".into())
    );
}
#[test]
fn phase_states_preserve_real_npc_identity_and_verified_defences() {
    assert_eq!(encounters::window(14176).unwrap().label, "Phases 1–2");
    assert_eq!(encounters::window(100012).unwrap().label, "Judge · Magic");
    assert_eq!(encounters::window(100013).unwrap().label, "Judge · Melee");
    assert_eq!(encounters::window(100011).unwrap().label, "Final phase");
    assert!(
        !planner::monster(14176)
            .unwrap()
            .name
            .contains("tank using Magic")
    );
    for (id, magic, slash, speed) in [
        (14176, 60, 80, 8),
        (100010, -30, 80, 8),
        (100011, 81, 108, 7),
    ] {
        let target = planner::monster(id).unwrap();
        assert_eq!(encounters::window(id).unwrap().npc_id, 14176);
        assert_eq!(target.defensive["magic"], magic);
        assert_eq!(target.defensive["slash"], slash);
        assert_eq!(target.speed, speed);
        assert_eq!(
            target.skills["hp"], 2500,
            "Keep actual NPC max HP separate from progress budget"
        );
        assert!(target.attributes.contains(&"demon".into()));
    }
    assert_eq!(encounters::progress_hp(14176, "Slash"), Some(1667.0));
    assert_eq!(encounters::progress_hp(100011, "Slash"), Some(833.0));
    assert_eq!(encounters::demonbane_vulnerability(100011), 120);
    assert_eq!(encounters::demonbane_vulnerability(14180), 100);
}
#[test]
fn state_changes_affect_actual_supported_damage() {
    assert!(dps(100010, "Magic", 11905) > dps(14176, "Magic", 11905));
    assert!(dps(14176, "Magic", 11905) > dps(100011, "Magic", 11905));
    assert!(dps(14176, "Slash", 4151) > dps(100011, "Slash", 4151));
}
#[test]
fn reactive_judge_uses_guaranteed_windows_and_contract_stays_unsupported() {
    for (id, style, weapon) in [
        (14180, "Magic", 11905),
        (100012, "Magic", 11905),
        (100013, "Melee", 4151),
    ] {
        let target = planner::monster(id).unwrap();
        assert!(encounters::limitation(target).is_none());
        assert!(encounters::hit_rule(id).unwrap().always_max);
        let mut set = vec![None; 14];
        set[3] = Some(weapon);
        let hit = combat::calculate(&member(), &set, style, target, "").unwrap();
        assert_eq!(hit.accuracy, 1.0);
        assert_eq!(
            hit.guaranteed_hits.as_deref(),
            Some(&[(hit.max_hit, 1.0)][..])
        );
    }
    assert!(encounters::limitation(planner::monster(100001).unwrap()).is_some());
    assert_eq!(encounters::progress_hp(100012, "Magic"), Some(400.0));
    assert_eq!(encounters::progress_hp(100013, "Slash"), Some(400.0));
}
#[test]
fn normal_yama_plan_carries_an_actual_magic_switch_without_quest_checks() {
    let mut bank = account::demo();
    bank.members[0].quests = None;
    for skill in account::SKILLS {
        bank.members[0].level_overrides.insert(skill.into(), 99);
    }
    let group = encounters::groups()
        .iter()
        .find(|group| group.name == "Yama")
        .unwrap();
    let options = Options {
        boss: Boss::Custom,
        targets: group.targets.clone(),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    let plan = planner::plan(&bank, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert_eq!(plan.phases.len(), 5);
    let magic = plan
        .phases
        .iter()
        .find(|phase| phase.style == "Magic")
        .unwrap();
    assert_eq!(
        rules::weapon(magic.equipment[3].unwrap())
            .unwrap()
            .family
            .style,
        "Magic"
    );
    assert!(plan.inventory_slots() <= 28);
}

#[test]
fn synthetic_owned_yama_group_keeps_usable_judge_switch_and_hard_budgets() {
    use std::collections::BTreeMap;
    // Public synthetic stock only: no imported bank/account fixture.
    let mut stock = BTreeMap::<i32, u32>::from([
        (19675, 1), // Charged Arclight
        (4151, 1),  // Whip, a weaker demon alternative
        (11905, 1), // Charged Trident of the Seas
        (10828, 1),
        (6585, 1),
        (6570, 1),
        (21795, 1),
        (29022, 1),
        (2503, 1),
        (2497, 1),
        (4712, 1),
        (4714, 1),
        (7462, 1),
        (11840, 1),
        (11773, 1),
        (12954, 1),
        (3840, 1),
        (385, 16),
        (2434, 6),
        (8007, 1),
    ]);
    for id in [556, 554, 560, 562, 565] {
        stock.insert(id, 10_000);
    }
    let mut character = member();
    character.name = "Public Yama bank fixture".into();
    character.bank = Some(
        stock
            .iter()
            .flat_map(|(id, q)| [i64::from(*id), i64::from(*q)])
            .collect(),
    );
    character.quests = None;
    let bank = account::Snapshot {
        selected: character.name.clone(),
        members: vec![character],
        source: "Synthetic public regression".into(),
        fetched_at: "Local fixture".into(),
    };
    let group = encounters::groups()
        .iter()
        .find(|group| group.name == "Yama")
        .unwrap();
    let options = Options {
        boss: Boss::Custom,
        targets: group.targets.clone(),
        recommended: Some(true),
        max_switches: Some(4),
        switch_slots: 8,
        food: 4,
        prayer_potions: 2,
        ..Default::default()
    };
    let plan = planner::plan(&bank, &options);
    assert!(
        plan.exportable(),
        "missing {:?}; warnings {:?}",
        plan.missing,
        plan.warnings
    );
    assert!(plan.inventory_slots() <= 28);
    let report = plan
        .optimization
        .as_ref()
        .expect("Selected budgets must be checked");
    assert!(report.max_changes <= 4);
    assert!(report.gear_slots <= 8);
    let owned = bank.available(false);
    let selected = bank.selected().unwrap();
    let sets: Vec<_> = std::iter::once(&plan.equipment)
        .chain(plan.phases.iter().map(|phase| &phase.equipment))
        .collect();
    let mut packed = BTreeMap::<i32, u32>::new();
    for (slot, id) in plan.equipment.iter().enumerate() {
        if let Some(id) = id {
            *packed.entry(*id).or_default() += if slot == 13 { 100 } else { 1 };
        }
    }
    for item in &plan.inventory {
        if item.id > 0 {
            *packed.entry(item.id).or_default() += item.quantity;
        }
    }
    for (id, quantity) in &packed {
        assert!(
            *quantity <= owned.get(id).copied().unwrap_or(0),
            "Packed unowned stock {}",
            planner::item_name(*id)
        );
    }
    for set in &sets {
        for (slot, id) in set.iter().enumerate() {
            if let Some(id) = id {
                assert!(
                    planner::can_use(*id, selected),
                    "Unusable {}",
                    planner::item_name(*id)
                );
                assert_eq!(planner::gear(*id).unwrap().slot, planner::SLOT_KEYS[slot]);
                assert!(
                    packed.get(id).copied().unwrap_or(0) >= if slot == 13 { 100 } else { 1 },
                    "Phase item missing from physical pack: {}",
                    planner::item_name(*id)
                );
            }
        }
    }
    let magic = plan
        .phases
        .iter()
        .find(|phase| phase.name.contains("Judge") && phase.style == "Magic")
        .unwrap();
    assert_eq!(
        magic.equipment[3],
        Some(11905),
        "Unverified Judge DPS must preserve a viable owned powered staff seed"
    );
    assert!(rules::casting_supported(
        selected,
        &magic.equipment,
        &owned,
        rules::DEFAULT_CASTS
    ));
    assert!(magic.note.contains("guaranteed maximum"));
    assert!(
        combat::calculate_configured(
            selected,
            &magic.equipment,
            "Magic",
            planner::monster(100012).unwrap(),
            rules::spell_name(selected, &magic.equipment, &owned),
            0,
            &options.combat
        )
        .is_ok(),
        "Judge Magic window should expose its guaranteed-hit damage"
    );
    for phase in plan.phases.iter().filter(|phase| phase.style == "Slash") {
        assert_eq!(
            phase.equipment[3],
            Some(19675),
            "Owned charged Arclight should beat the ordinary whip at this demon boss"
        );
    }
    let share = ironbound::share::Setup::from_plan(&plan, &options);
    share
        .validate()
        .expect("The physical multi-phase setup must remain exportable/shareable");
}
