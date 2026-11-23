use ironbound::{
    account, combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    rules,
};
use std::collections::BTreeMap;
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn member() -> account::Member {
    let mut m = account::Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn stock() -> BTreeMap<i32, u32> {
    BTreeMap::from([
        (554, 10000),
        (566, 10000),
        (558, 10000),
        (560, 10000),
        (564, 10000),
    ])
}
fn target() -> &'static planner::Monster {
    planner::monster(100190).unwrap()
}
#[test]
fn reviewed_arceuus_autocasters_accept_demonbane_without_purging_staff() {
    let spell = rules::spells()
        .iter()
        .find(|s| s.name == "Dark Demonbane")
        .unwrap();
    for id in [4170, 21255, 4710, 4862, 4863, 4864, 4865, 28988, 29594] {
        assert!(
            rules::spell_compatible(&rules::weapon(id).unwrap(), spell, &set(id)),
            "{}",
            planner::item_name(id)
        );
    }
    for id in [1381, 11907, 4675, 1409] {
        assert!(!rules::spell_compatible(
            &rules::weapon(id).unwrap(),
            spell,
            &set(id)
        ));
    }
}
#[test]
fn ordinary_staff_demonbane_uses_spell_bonuses_and_bypasses_the_td_shield() {
    let m = member();
    let s = set(4170);
    let settings = Settings::default();
    let ordinary =
        combat::calculate_configured(&m, &s, "Magic", target(), "Dark Demonbane", 0, &settings)
            .unwrap();
    // (99 + 9) * (12 Magic attack + 64), then the spell's 20% accuracy bonus.
    assert_eq!(
        (
            ordinary.attack_roll,
            ordinary.max_hit,
            ordinary.attack_ticks
        ),
        (9849, 30, 5)
    );
    let marked = Settings {
        mark_darkness: true,
        ..settings
    };
    let r = combat::calculate_configured(&m, &s, "Magic", target(), "Dark Demonbane", 0, &marked)
        .unwrap();
    assert_eq!((r.attack_roll, r.max_hit, r.attack_ticks), (11491, 37, 5));
    let mut ordinary_target = target().clone();
    ordinary_target.id = 1;
    ordinary_target.name = "Reference demon".into();

    let reference = combat::calculate_configured(
        &m,
        &s,
        "Magic",
        &ordinary_target,
        "Dark Demonbane",
        0,
        &marked,
    )
    .unwrap();
    assert_eq!(r.hit_distribution, reference.hit_distribution);
    ordinary_target.attributes.clear();
    assert!(
        combat::calculate_configured(
            &m,
            &s,
            "Magic",
            &ordinary_target,
            "Dark Demonbane",
            0,
            &marked
        )
        .is_err()
    );
}
#[test]
fn automatic_selection_compares_demonbane_on_slayer_and_ahrim_staves() {
    let m = member();
    let settings = Settings {
        mark_darkness: true,
        ..Default::default()
    };
    for id in [4170, 21255, 4710, 28988] {
        assert_eq!(
            rules::spell_name_for_target(&m, &set(id), &stock(), &settings, target(), 0),
            "Dark Demonbane"
        );
    }
    let mut none = stock();
    none.remove(&566);
    assert_ne!(
        rules::spell_name_for_target(&m, &set(4170), &none, &settings, target(), 0),
        "Dark Demonbane"
    );
}
fn snapshot() -> account::Snapshot {
    let mut s = account::demo();
    s.members[0].level_overrides = member().level_overrides;
    s.members[0].bank = Some(
        [
            vec![4710, 1, 11907, 1],
            stock()
                .into_iter()
                .flat_map(|(id, q)| [i64::from(id), i64::from(q)])
                .collect(),
        ]
        .concat(),
    );
    s.members[0].inventory = None;
    s.members[0].equipment = None;
    s.members[0].rune_pouch = None;
    s
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 100190,
            style: "Magic".into(),
        }],
        recommended: Some(false),
        combat: Settings {
            mark_darkness: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn funded_ahrim_demonbane_beats_shield_reduced_trident_and_packs_actual_spell_runes() {
    let s = snapshot();
    let o = options();
    let p = planner::plan(&s, &o);
    assert_eq!(p.equipment[3], Some(4710));
    assert!(p.spell.contains("Dark Demonbane"), "{}", p.spell);
    for (id, minimum) in [(554, 1050), (566, 301), (564, 1)] {
        assert!(
            p.inventory
                .iter()
                .filter(|i| i.id == id)
                .map(|i| i.quantity)
                .sum::<u32>()
                >= minimum,
            "missing {id}"
        );
    }
    assert!(p.checks.iter().any(|c| c.name == "Mark of Darkness"));
    let mut missing = s;
    let bank = missing.members[0].bank.as_mut().unwrap();
    for pair in bank.chunks_exact_mut(2) {
        if pair[0] == 566 {
            pair[1] = 0;
        }
    }
    let p = planner::plan(&missing, &o);
    assert_eq!(p.equipment[3], Some(11907));
}
#[test]
fn trident_does_not_receive_demonbane_or_mark_bonuses() {
    let m = member();
    let s = set(11907);
    let plain = Settings::default();
    let marked = Settings {
        mark_darkness: true,
        ..plain.clone()
    };
    let a = combat::calculate_configured(&m, &s, "Magic", target(), "", 0, &plain).unwrap();
    let b = combat::calculate_configured(&m, &s, "Magic", target(), "", 0, &marked).unwrap();
    assert_eq!(
        (a.attack_roll, a.max_hit, a.dps),
        (b.attack_roll, b.max_hit, b.dps)
    );
    assert_eq!(a.max_hit, 22);
}

#[test]
fn level_86_owned_enhanced_slayer_staff_beats_trident_with_funded_demonbane() {
    let mut s = snapshot();
    let m = &mut s.members[0];
    m.level_overrides.insert("Magic".into(), 86);
    let bank = m.bank.as_mut().unwrap();
    for pair in bank.chunks_exact_mut(2) {
        if pair[0] == 4710 {
            pair[0] = 21255;
        }
    }
    let p = planner::plan(&s, &options());
    assert_eq!(p.equipment[3], Some(21255));
    assert!(p.spell.contains("Dark Demonbane"), "{}", p.spell);
    let selection = rules::SpellSelection::Spell("Dark Demonbane".into());
    assert!(selection.permits_weapon(21255));
    assert!(!selection.permits_weapon(11907));
}
