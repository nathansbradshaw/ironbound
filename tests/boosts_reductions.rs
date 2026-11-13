//! Synthetic numerical cases cross-checked against Wiki PotionMap and
//! DefenceReduction.ts at74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f.
use ironbound::{
    account::Member,
    modifiers::{self, Boosts, Reductions},
    planner,
};
fn member(level: u32) -> Member {
    let mut m = Member::default();
    for s in [
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
        "Magic",
        "Hitpoints",
        "Prayer",
    ] {
        m.level_overrides.insert(s.into(), level);
    }
    m
}
fn target() -> planner::Monster {
    let mut m = planner::monsters()
        .iter()
        .find(|m| m.name.starts_with("Vorkath"))
        .unwrap()
        .clone();
    m.skills = [
        ("def", 100),
        ("str", 70),
        ("atk", 90),
        ("magic", 80),
        ("ranged", 60),
        ("hp", 1000),
    ]
    .into_iter()
    .map(|(s, l)| (s.into(), l))
    .collect();
    m.attributes = vec!["demon".into()];
    m
}
#[test]
fn boost_defaults_and_old_backups_preserve_base_levels() {
    let b: Boosts = serde_json::from_str("{}").unwrap();
    assert_eq!(b, Boosts::default());
    assert!(b.valid());
    assert_eq!(modifiers::level(&member(89), "Attack", &b), Some(89));
    assert_eq!(modifiers::level(&Member::default(), "Magic", &b), None);
    assert_eq!(modifiers::level(&member(118), "Attack", &b), Some(118));
    assert_eq!(modifiers::level(&member(u32::MAX), "Magic", &b), None);
}
#[test]
fn standard_boosts_use_base_level_floors_and_never_stack() {
    let mut b = Boosts {
        melee: "Super combat".into(),
        ranged: "Ranging potion".into(),
        magic: "Magic potion".into(),
        ..Default::default()
    };
    let m = member(99);
    for skill in ["Attack", "Strength", "Defence"] {
        assert_eq!(modifiers::level(&m, skill, &b), Some(118));
    }
    assert_eq!(modifiers::level(&m, "Ranged", &b), Some(112));
    assert_eq!(modifiers::level(&m, "Magic", &b), Some(103));
    b.encounter = "Smelling salts".into();
    for skill in ["Attack", "Strength", "Defence", "Ranged", "Magic"] {
        assert_eq!(modifiers::level(&m, skill, &b), Some(125));
    }
    assert_eq!(modifiers::level(&m, "Hitpoints", &b), Some(99));
}
#[test]
fn hearts_brews_and_cox_tiers_are_distinct() {
    for (id, expected) in [
        ("Imbued heart", 109),
        ("Saturated heart", 112),
        ("Forgotten brew", 109),
    ] {
        let b = Boosts {
            magic: id.into(),
            ..Default::default()
        };
        assert_eq!(modifiers::level(&member(99), "Magic", &b), Some(expected));
    }
    for (id, expected) in [
        ("Overload(-)", 112),
        ("Overload", 116),
        ("Overload(+)", 120),
    ] {
        let b = Boosts {
            encounter: id.into(),
            ..Default::default()
        };
        assert_eq!(modifiers::level(&member(99), "Magic", &b), Some(expected));
    }
}
#[test]
fn forgotten_brew_keeps_its_melee_drain_and_rejects_ambiguous_drinking_order() {
    let mut b = Boosts {
        magic: "Forgotten brew".into(),
        ..Default::default()
    };
    assert_eq!(modifiers::level(&member(89), "Attack", &b), Some(78));
    assert_eq!(modifiers::level(&member(89), "Ranged", &b), Some(89));
    b.melee = "Super combat".into();
    assert!(!b.valid());
    assert_eq!(modifiers::level(&member(89), "Magic", &b), None);
}
#[test]
fn boosted_damage_level_cannot_bypass_base_weapon_eligibility() {
    let m = member(69);
    let b = Boosts {
        magic: "Saturated heart".into(),
        ..Default::default()
    };
    assert_eq!(modifiers::level(&m, "Magic", &b), Some(79));
    assert!(!planner::can_use(29594, &m));
    let b = Boosts {
        magic: "made up".into(),
        ..Default::default()
    };
    assert!(!b.valid());
}
#[test]
fn percentage_hits_are_ordered_and_repeated_from_current_defence() {
    let m = target();
    let r = Reductions {
        elder_maul: 1,
        dwh: 1,
        ..Default::default()
    };
    assert_eq!(modifiers::reduce(&m, &r).unwrap().skills["def"], 46);
    let r = Reductions {
        dwh: 2,
        ..Default::default()
    };
    assert_eq!(modifiers::reduce(&m, &r).unwrap().skills["def"], 49);
    let r = Reductions {
        elder_maul: 2,
        ..Default::default()
    };
    assert_eq!(modifiers::reduce(&m, &r).unwrap().skills["def"], 43);
}
#[test]
fn full_reference_order_uses_original_arclight_stats_and_accursed_priority() {
    let m = target();
    let r = Reductions {
        accursed: true,
        vulnerability: true,
        elder_maul: 1,
        dwh: 1,
        arclight: 1,
        emberlight: 1,
        bgs: 5,
        ..Default::default()
    };
    let result = modifiers::reduce(&m, &r).unwrap();
    assert_eq!(result.skills["def"], 8);
    assert_eq!(result.skills["magic"], 68);
    assert_eq!(result.skills["atk"], 66);
    assert_eq!(result.skills["str"], 51);
    assert_eq!(m.skills["def"], 100);
    let r = Reductions {
        arclight: 2,
        ..Default::default()
    };
    assert_eq!(modifiers::reduce(&m, &r).unwrap().skills["def"], 78);
}
#[test]
fn bgs_spills_in_defence_strength_attack_magic_ranged_order() {
    let m = target();
    let r = Reductions {
        bgs: 125,
        ..Default::default()
    };
    let x = modifiers::reduce(&m, &r).unwrap();
    assert_eq!(x.skills["def"], 0);
    assert_eq!(x.skills["str"], 45);
    assert_eq!(x.skills["atk"], 90);
    let all = modifiers::reduce(
        &m,
        &Reductions {
            bgs: 400,
            ..Default::default()
        },
    )
    .unwrap();
    for s in ["def", "str", "atk", "magic", "ranged"] {
        assert_eq!(all.skills[s], 0);
    }
    assert_eq!(all.skills["hp"], 1000);
}
#[test]
fn yama_and_internal_phase_windows_keep_145_floor_and_bgs_does_not_spill() {
    for id in [14176, 100010, 100011] {
        let m = planner::monster(id).unwrap();
        let r = Reductions {
            bgs: 9999,
            dwh: 20,
            ..Default::default()
        };
        let x = modifiers::reduce(m, &r).unwrap();
        assert_eq!(x.skills["def"], 145);
        assert_eq!(x.skills["str"], m.skills["str"]);
        assert_eq!(x.skills["magic"], m.skills["magic"]);
    }
}
#[test]
fn toa_base_defence_floors_are_not_scaled_as_invocation_rolls() {
    for (id, floor) in [(11789, 70), (11778, 60), (11719, 60), (11730, 50)] {
        let m = planner::monster(id).unwrap();
        let x = modifiers::reduce(
            m,
            &Reductions {
                bgs: 9999,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(x.skills["def"], floor);
    }
}
#[test]
fn unknown_or_contract_windows_never_borrow_normal_reduction_rules() {
    let contract = planner::monster(100001).unwrap();
    assert!(
        modifiers::reduce(
            contract,
            &Reductions {
                dwh: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut unknown = target();
    unknown.id = 999999;
    assert!(
        modifiers::reduce(
            &unknown,
            &Reductions {
                dwh: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        modifiers::reduce(&unknown, &Reductions::default())
            .unwrap()
            .skills,
        unknown.skills
    );
}
#[test]
fn invalid_counts_and_unknown_payload_fields_fail_closed() {
    assert!(
        !Reductions {
            dwh: 21,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Reductions {
            bgs: 10001,
            ..Default::default()
        }
        .valid()
    );
    assert!(serde_json::from_str::<Reductions>(r#"{"bogus":1}"#).is_err());
    let r: Reductions = serde_json::from_str("{}").unwrap();
    assert!(r.is_empty());
    assert!(r.valid());
}
#[test]
fn tonalztic_uses_accursed_current_magic_before_seercull_and_bgs() {
    let mut m = target();
    m.defensive.insert("magic".into(), 100);
    let x = modifiers::reduce(
        &m,
        &Reductions {
            accursed: true,
            tonalztic: 2,
            seercull: 20,
            bgs: 5,
            ayak: 25,
            ..Default::default()
        },
    )
    .unwrap();
    // Defence85 - 2*floor(68/8) - 5; Magic68 - 20.
    assert_eq!(x.skills["def"], 64);
    assert_eq!(x.skills["magic"], 48);
    assert_eq!(x.defensive["magic"], 75);
    assert_eq!(m.defensive["magic"], 100);
}
#[test]
fn ayak_changes_bonus_only_and_all_extra_reductions_saturate() {
    let mut m = target();
    for bonus in [-10, 0, 12] {
        m.defensive.insert("magic".into(), bonus);
        let x = modifiers::reduce(
            &m,
            &Reductions {
                tonalztic: 20,
                seercull: 10000,
                ayak: 10000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(x.skills["def"], 0);
        assert_eq!(x.skills["magic"], 0);
        assert_eq!(x.defensive["magic"], bonus.min(0));
    }
    assert!(
        !Reductions {
            tonalztic: 21,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Reductions {
            seercull: 10001,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Reductions {
            ayak: 10001,
            ..Default::default()
        }
        .valid()
    );
}

#[test]
fn independent_defence_channel_changes_only_defence_and_never_stacks() {
    let m = member(99);
    let mut b = Boosts {
        defence: "Defence potion".into(),
        ranged: "Ranging potion".into(),
        ..Default::default()
    };
    assert!(b.valid());
    assert_eq!(modifiers::level(&m, "Defence", &b), Some(111));
    assert_eq!(modifiers::level(&m, "Attack", &b), Some(99));
    assert_eq!(modifiers::level(&m, "Strength", &b), Some(99));
    assert_eq!(modifiers::level(&m, "Ranged", &b), Some(112));
    b.defence = "Super defence".into();
    assert_eq!(modifiers::level(&m, "Defence", &b), Some(118));
    b.melee = "Super combat".into();
    assert_eq!(modifiers::level(&m, "Defence", &b), Some(118));
    b.encounter = "Smelling salts".into();
    assert_eq!(modifiers::level(&m, "Defence", &b), Some(125));
    b.magic = "Forgotten brew".into();
    assert!(!b.valid());
    let bad = Boosts {
        melee: "Super defence".into(),
        ..Default::default()
    };
    assert!(!bad.valid());
}
fn spell_bank(weapon: i32, magic: u32, runes: &[(i32, u32)]) -> ironbound::account::Snapshot {
    let mut s = ironbound::account::demo();
    s.members.truncate(1);
    s.members[0] = member(99);
    s.members[0].name = "Public spell fixture".into();
    s.selected = s.members[0].name.clone();
    s.members[0].level_overrides.insert("Magic".into(), magic);
    s.members[0].bank = Some(
        std::iter::once((weapon, 1))
            .chain(std::iter::once((8007, 1)))
            .chain(runes.iter().copied())
            .flat_map(|(id, q)| [i64::from(id), i64::from(q)])
            .collect(),
    );
    s
}
fn spell_options(weapon: i32) -> planner::Options {
    let mole = planner::monsters()
        .iter()
        .find(|m| m.name == "Giant Mole")
        .unwrap();
    let mut o = planner::Options {
        boss: planner::Boss::Custom,
        targets: vec![planner::Target {
            id: mole.id,
            style: "Magic".into(),
        }],
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    o.gear_overrides.insert("Magic:3".into(), weapon);
    o
}
#[test]
fn boosted_spell_choice_packs_wave_runes_instead_of_blast_runes() {
    use ironbound::{combat, rules};
    let bank = spell_bank(1381, 61, &[(565, 150), (560, 150)]);
    let mut o = spell_options(1381);
    let base = planner::plan(&bank, &o);
    assert_eq!(base.spell, "Wind Blast");
    assert!(
        base.inventory
            .iter()
            .any(|i| i.id == 560 && i.quantity == 150)
    );
    o.combat.boosts.magic = "Magic potion".into();
    let boosted = planner::plan(&bank, &o);
    assert_eq!(boosted.spell, "Wind Wave");
    assert!(
        boosted
            .inventory
            .iter()
            .any(|i| i.id == 565 && i.quantity == 150)
    );
    assert!(!boosted.inventory.iter().any(|i| i.id == 560));
    let m = bank.selected().unwrap();
    assert!(rules::casting_supported_configured(
        m,
        &boosted.equipment,
        &bank.available(false),
        150,
        &o.combat
    ));
    let target = planner::monster(o.targets[0].id).unwrap();
    let damage = combat::calculate_configured(
        m,
        &boosted.equipment,
        "Magic",
        target,
        &boosted.spell,
        0,
        &o.combat,
    )
    .unwrap();
    assert_eq!(damage.max_hit, 18); // Magic61+4 reaches the65 Wave tier.
    let mut short = bank.clone();
    short.members[0].bank = Some(vec![1381, 1, 8007, 1, 565, 149, 560, 150]);
    let fallback = planner::plan(&short, &o);
    assert_eq!(fallback.spell, "Wind Blast");
    assert!(
        fallback
            .inventory
            .iter()
            .any(|i| i.id == 560 && i.quantity == 150)
    );
}
#[test]
fn boosted_purging_spell_reserves_dark_runes_and_separate_mark_precast() {
    let bank = spell_bank(29594, 77, &[(554, 1050), (566, 301), (564, 1)]);
    let mut o = spell_options(29594);
    o.combat.mark_darkness = true;
    o.combat.boosts.magic = "Magic potion".into();
    let superior = planner::plan(&bank, &o);
    assert_eq!(superior.spell, "Superior Demonbane");
    assert!(
        superior
            .inventory
            .iter()
            .any(|i| i.id == 554 && i.quantity == 750)
    );
    assert!(
        superior
            .inventory
            .iter()
            .any(|i| i.id == 566 && i.quantity == 151)
    );
    o.combat.boosts.magic = "Saturated heart".into();
    let dark = planner::plan(&bank, &o);
    assert_eq!(dark.spell, "Dark Demonbane");
    assert!(
        dark.inventory
            .iter()
            .any(|i| i.id == 554 && i.quantity == 1050)
    );
    assert!(
        dark.inventory
            .iter()
            .any(|i| i.id == 566 && i.quantity == 301)
    );
    assert!(
        dark.inventory
            .iter()
            .any(|i| i.id == 564 && i.quantity == 1)
    );
    assert!(dark.missing.is_empty(), "{:?}", dark.missing);
    // The heart may unlock a spell, but not the staff's base77 equip gate.
    let mut below = bank.clone();
    below.members[0].level_overrides.insert("Magic".into(), 76);
    assert!(!ironbound::rules::casting_supported_configured(
        below.selected().unwrap(),
        &dark.equipment,
        &below.available(false),
        150,
        &o.combat
    ));
}

#[test]
fn independent_super_defence_improves_incoming_accuracy_without_changing_outgoing_dps() {
    use ironbound::{combat, defence};
    let m = member(99);
    let mole = planner::monsters()
        .iter()
        .find(|m| m.name == "Giant Mole")
        .unwrap();
    let mut set = vec![None; 14];
    set[3] = Some(4151);
    let base = defence::Settings::default();
    let boosted = defence::Settings {
        boosts: Boosts {
            defence: "Super defence".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let attack = defence::attacks(mole).into_iter().next().unwrap();
    let incoming = defence::estimate(&m, &set, mole, &attack, &base).unwrap();
    let safer = defence::estimate(&m, &set, mole, &attack, &boosted).unwrap();
    assert!(safer.hit_chance < incoming.hit_chance);
    assert!(safer.damage_per_second < incoming.damage_per_second);
    let outgoing = combat::calculate_configured(&m, &set, "Slash", mole, "", 0, &base).unwrap();
    let unchanged = combat::calculate_configured(&m, &set, "Slash", mole, "", 0, &boosted).unwrap();
    assert_eq!(outgoing.attack_roll, unchanged.attack_roll);
    assert_eq!(outgoing.max_hit, unchanged.max_hit);
    assert_eq!(outgoing.dps, unchanged.dps);
}
