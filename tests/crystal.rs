use ironbound::{account::Member, combat, planner, rules};

fn member() -> Member {
    let mut member = Member::default();
    for skill in ["Ranged", "Magic", "Defence", "Prayer", "Agility"] {
        member.level_overrides.insert(skill.into(), 99);
    }
    member
}
fn equipment(weapon: i32, pieces: &[usize]) -> Vec<Option<i32>> {
    let mut set = vec![None; 14];
    set[3] = Some(weapon);
    for slot in pieces {
        set[*slot] = Some(match slot {
            0 => 23971,
            4 => 23975,
            7 => 23979,
            _ => unreachable!(),
        });
    }
    set
}
fn calculate(set: &[Option<i32>]) -> combat::ResultStats {
    combat::calculate(
        &member(),
        set,
        "Ranged",
        planner::monster(11730).unwrap(),
        "",
    )
    .unwrap()
}

// OSRS Wiki PlayerVsNPCCalc: add piece weights, floor once after base rolls.
// Helm: 5% accuracy/2.5% damage; body:15%/7.5%; legs:10%/5%.
#[test]
fn crystal_bows_apply_each_partial_set_additively() {
    for (weapon, base_max) in [(23983, 24), (25865, 28), (25867, 28)] {
        for (slots, weight) in [
            (vec![], 0),
            (vec![0], 1),
            (vec![4], 3),
            (vec![7], 2),
            (vec![0, 4], 4),
            (vec![4, 7], 5),
            (vec![0, 4, 7], 6),
        ] {
            let set = equipment(weapon, &slots);
            let result = calculate(&set);
            let base_roll = 107 * (i64::from(combat::bonuses(&set).offensive["ranged"]) + 64);
            assert_eq!(result.attack_roll, base_roll * (20 + weight) / 20);
            assert_eq!(result.max_hit, base_max * (40 + weight as i32) / 40);
            assert_eq!(result.attack_ticks, 4);
            assert!(result.dps > 0.0);
        }
    }
}

#[test]
fn armour_does_not_grant_passive_to_other_bows_or_shadow() {
    let mut set = equipment(861, &[0, 4, 7]);
    set[13] = Some(892);
    let result = calculate(&set);
    assert_eq!(result.attack_roll, 107 * (69 + 9 + 31 + 18 + 64));
    let base = calculate(&{
        let mut s = equipment(861, &[]);
        s[13] = Some(892);
        s
    });
    assert_eq!(result.max_hit, base.max_hit);
    let shadow = rules::weapon(27275).unwrap();
    assert_eq!(
        shadow.scale_equipment(&equipment(27275, &[0, 4, 7]), 1000, 40),
        (1000, 40)
    );
}

#[test]
fn self_ammunition_bows_ignore_arrows_and_enchanted_bolt_procs() {
    let set = equipment(25865, &[0, 4, 7]);
    let base = calculate(&set);
    for ammunition in [892, 9242, 9243] {
        let mut with_ammo = set.clone();
        with_ammo[13] = Some(ammunition);
        let result = calculate(&with_ammo);
        assert_eq!(result.attack_roll, base.attack_roll);
        assert_eq!(result.max_hit, base.max_hit);
        assert_eq!(result.dps, base.dps);
    }
}

#[test]
fn only_real_active_variants_have_the_passive() {
    for id in [25862, 4214, 23901, 33021] {
        assert!(rules::weapon(id).is_none());
    }
    for id in [
        25865, 25867, 25896, 25890, 25892, 25886, 25884, 25894, 25888,
    ] {
        let set = equipment(id, &[0, 4, 7]);
        assert_eq!(calculate(&set).max_hit, 32);
        let rule = rules::weapon(id).unwrap();
        assert_eq!(rule.weapon.requirements["Ranged"], 80);
        assert_eq!(rule.weapon.requirements["Agility"], 70);
    }
    let mut inactive = equipment(23983, &[]);
    inactive[0] = Some(23973);
    inactive[4] = Some(23977);
    inactive[7] = Some(23981);
    assert_eq!(
        rules::weapon(23983)
            .unwrap()
            .scale_equipment(&inactive, 1000, 40),
        (1000, 40)
    );
    let mut coloured = equipment(23983, &[]);
    coloured[0] = Some(27777);
    coloured[4] = Some(27769);
    coloured[7] = Some(27773);
    assert_eq!(calculate(&coloured).max_hit, 27);
}

#[test]
fn crystal_armour_only_counts_in_its_equipment_slot() {
    let mut misplaced = equipment(23983, &[]);
    misplaced[13] = Some(23975);
    assert_eq!(
        rules::weapon(23983)
            .unwrap()
            .scale_equipment(&misplaced, 1000, 40),
        (1000, 40)
    );
}
