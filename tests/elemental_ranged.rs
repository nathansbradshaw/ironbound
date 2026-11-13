//! Independent arithmetic from Jagex Project Rebalance and the pinned Wiki stages.
//! See docs/combat-validation.md; these are formula checks, not in-game observations.
use ironbound::{
    account::{self, Member},
    combat,
    defence::Settings,
    planner::{self, Boss, Options, Target, Weakness},
    rules,
};
use std::collections::BTreeMap;
fn member(level: u32) -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), level);
    }
    m
}
fn set(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
fn weak(element: &str, severity: i32) -> planner::Monster {
    let mut t = planner::monster(415).unwrap().clone();
    t.weakness = Some(Weakness {
        element: element.into(),
        severity,
    });
    t
}
#[test]
fn all_elements_gain_only_the_matching_weakness() {
    for (element, spell) in [
        ("air", "Wind Wave"),
        ("water", "Water Wave"),
        ("earth", "Earth Wave"),
        ("fire", "Fire Wave"),
    ] {
        let r =
            combat::calculate(&member(99), &set(1381), "Magic", &weak(element, 50), spell).unwrap();
        assert_eq!((r.max_hit, r.attack_roll), (30, 11988)); // 20+floor(20*.5);7992+floor(7992*.5).
        let other =
            combat::calculate(&member(99), &set(1381), "Magic", &weak("other", 50), spell).unwrap();
        assert_eq!((other.max_hit, other.attack_roll), (20, 7992));
    }
    for weapon in [11907, 12658, 29594] {
        let spell = match weapon {
            12658 => "Iban Blast",
            29594 => "Dark Demonbane",
            _ => "",
        };
        let a = combat::calculate(
            &member(99),
            &set(weapon),
            "Magic",
            &weak("fire", 100),
            spell,
        )
        .unwrap();
        let b =
            combat::calculate(&member(99), &set(weapon), "Magic", &weak("fire", 0), spell).unwrap();
        assert_eq!((a.max_hit, a.attack_roll), (b.max_hit, b.attack_roll));
        assert!(a.elemental.is_none());
    }
}
#[test]
fn every_elemental_tier_scales_at_each_unlock_and_caps() {
    for (class, levels, hits) in [
        ("Strike", [1, 5, 9, 13], [2, 4, 6, 8]),
        ("Bolt", [17, 23, 29, 35], [9, 10, 11, 12]),
        ("Blast", [41, 47, 53, 59], [13, 14, 15, 16]),
        ("Wave", [62, 65, 70, 75], [17, 18, 19, 20]),
        ("Surge", [81, 85, 90, 95], [21, 22, 23, 24]),
    ] {
        for (i, element) in ["Wind", "Water", "Earth", "Fire"].iter().enumerate() {
            let spell = format!("{element} {class}");
            assert_eq!(rules::spell_max_hit(&spell, levels[i] - 1), None);
            for (level, hit) in levels.iter().zip(hits).skip(i) {
                assert_eq!(rules::spell_max_hit(&spell, *level), Some(hit));
            }
            assert_eq!(rules::spell_max_hit(&spell, 126), Some(hits[3]));
        }
    }
    let mut settings = Settings::default();
    settings.boosts.magic = "Magic potion".into();
    let boosted = combat::calculate_configured(
        &member(61),
        &set(1381),
        "Magic",
        &weak("fire", 0),
        "Wind Wave",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!((boosted.max_hit, boosted.attack_roll), (17, 5476)); // Current65+9, base spell damage17.
    assert!(
        combat::calculate(
            &member(61),
            &set(1381),
            "Magic",
            &weak("fire", 0),
            "Wind Wave"
        )
        .is_err()
    );
}
#[test]
fn weakness_adds_after_gear_and_task_without_multiplying_them() {
    let mut gear = set(1381);
    gear[2] = Some(12002);
    gear[0] = Some(11865);
    let settings = Settings {
        slayer_targets: [415].into_iter().collect(),
        ..Default::default()
    };
    let r = combat::calculate_configured(
        &member(99),
        &gear,
        "Magic",
        &weak("air", 50),
        "Wind Wave",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!(r.max_hit, 34); // floor(floor(20*1.05)*1.15)+floor(20*.50)=24+10.
    let e = r.elemental.unwrap();
    assert_eq!(
        (e.base_max, e.damage_before_weakness, e.weakness_damage),
        (20, 24, 10)
    );
    // Staff+helmet+occult give +25: base108*89=9612;floor(*1.15)+floor(*.5)=15859.
    assert_eq!(r.attack_roll, 15859);
}
#[test]
fn charged_tomes_apply_after_weakness_and_empty_tomes_do_not() {
    for (element, spell, charged, empty, rune) in [
        ("fire", "Fire Wave", 20714, 20716, 554),
        ("water", "Water Wave", 25574, 25576, 555),
        ("earth", "Earth Wave", 30064, 30066, 557),
    ] {
        let mut gear = set(1381);
        gear[5] = Some(charged);
        let r = combat::calculate(&member(99), &gear, "Magic", &weak(element, 50), spell).unwrap();
        assert_eq!(r.max_hit, 33); // floor((20+10)*1.10).
        assert!(r.elemental.unwrap().tome);
        assert!(
            !rules::rune_supplies_for_set(spell, 150, &gear)
                .iter()
                .any(|(id, _)| *id == rune)
        );
        gear[5] = Some(empty);
        let e = combat::calculate(&member(99), &gear, "Magic", &weak(element, 50), spell).unwrap();
        assert_eq!(e.max_hit, 30);
        assert!(
            rules::rune_supplies_for_set(spell, 150, &gear)
                .iter()
                .any(|(id, _)| *id == rune)
        );
    }
}
#[test]
fn ammunition_selects_separate_defences_without_changing_max_hit() {
    let mut target = weak("air", 0);
    target.skills.insert("def".into(), 100);
    for (key, value) in [("light", -20), ("standard", 100), ("heavy", 200)] {
        target.defensive.insert(key.into(), value);
    }
    let mut m = member(99);
    m.bank = Some(vec![11230, 100]);
    let mut bow = set(861);
    bow[13] = Some(892);
    let mut crossbow = set(9185);
    crossbow[13] = Some(9144);
    for (gear, category, roll) in [
        (set(12926), "light", 4796),
        (bow, "standard", 17876),
        (crossbow, "heavy", 28776),
    ] {
        let r = combat::calculate(&m, &gear, "Ranged", &target, "").unwrap();
        assert_eq!(r.attack_type, category);
        assert_eq!(r.defence_roll, roll);
        let mut equal = target.clone();
        equal.defensive.insert(category.into(), 0);
        let other = combat::calculate(&m, &gear, "Ranged", &equal, "").unwrap();
        assert_eq!(
            (r.attack_roll, r.max_hit),
            (other.attack_roll, other.max_hit)
        );
        target.defensive.remove(category);
        assert!(
            combat::calculate(&m, &gear, "Ranged", &target, "")
                .unwrap_err()
                .contains("category")
        );
        target.defensive = weak("air", 0).defensive;
        for (key, value) in [("light", -20), ("standard", 100), ("heavy", 200)] {
            target.defensive.insert(key.into(), value);
        }
    }
}
#[test]
fn best_owned_spell_and_trip_runes_follow_each_targets_element() {
    let owned = BTreeMap::from([(565, 150), (555, 1050), (557, 1050), (554, 1050)]);
    let gear = set(1381);
    for (element, spell) in [
        ("air", "Wind Wave"),
        ("water", "Water Wave"),
        ("earth", "Earth Wave"),
        ("fire", "Fire Wave"),
    ] {
        assert_eq!(
            rules::spell_name_for_target(
                &member(99),
                &gear,
                &owned,
                &Settings::default(),
                &weak(element, 50),
                0
            ),
            spell
        );
    }
    let short = BTreeMap::from([(565, 150), (555, 1049)]);
    assert_eq!(
        rules::spell_name_for_target(
            &member(99),
            &gear,
            &short,
            &Settings::default(),
            &weak("water", 50),
            0
        ),
        "Wind Wave"
    );
    let mut bank = account::demo();
    bank.members[0] = member(99);
    bank.members[0].name = bank.selected.clone();
    bank.members[0].bank = Some(vec![1381, 1, 565, 150, 555, 1050, 557, 1050, 554, 1050]);
    let options = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 5779,
                style: "Magic".into(),
            },
            Target {
                id: 8059,
                style: "Magic".into(),
            },
        ],
        gear_overrides: BTreeMap::from([("Magic:3".into(), 1381)]),
        ..Default::default()
    };
    let plan = planner::plan(&bank, &options);
    assert!(
        plan.spell.contains("Earth Wave") && plan.spell.contains("Fire Wave"),
        "{}",
        plan.spell
    );
    for (id, q) in [(565, 150), (557, 1050), (554, 1050)] {
        assert!(
            plan.inventory.iter().any(|p| p.id == id && p.quantity == q),
            "missing {id}: {:?}",
            plan.inventory
        );
    }
    assert!(!plan.inventory.iter().any(|p| p.id == 556));
}
