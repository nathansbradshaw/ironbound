//! Independent roll enumeration for the Archive's final outgoing gaps.
use ironbound::{
    account::{self, Member},
    combat, coverage, encounters, modifiers,
    planner::{self, Boss, Options, Target},
    raids,
};
fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn set(items: &[(usize, i32)]) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for &(slot, id) in items {
        s[slot] = Some(id);
    }
    s
}
fn result(id: i32, s: &[Option<i32>], style: &str, spell: &str) -> combat::ResultStats {
    combat::calculate(&member(), s, style, planner::monster(id).unwrap(), spell).unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
fn mean(max: i32, armour: i32) -> f64 {
    (0..=max)
        .map(|d| (d.max(1) - armour).max(0) as f64)
        .sum::<f64>()
        / (max + 1) as f64
}
fn valid(r: &combat::ResultStats) {
    near(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    assert!(r.hit_distribution.iter().all(|(d, p)| *d >= 0 && *p >= 0.0));
}
#[test]
fn bork_four_armour_states_match_independent_melee_magic_and_ranged_rolls() {
    for (id, armour, resistance) in [
        (16640, 8, 0),
        (205010, 10, 2),
        (205011, 12, 4),
        (205012, 14, 6),
    ] {
        let melee = result(id, &set(&[(3, 1434)]), "Crush", "");
        assert_eq!(
            (melee.attack_roll, melee.defence_roll, melee.max_hit),
            (13640, 22866, 20 - armour)
        );
        near(
            melee.dps,
            13640.0 / (2.0 * 22867.0) * mean(20, armour) / 2.4,
        );
        let magic = result(id, &set(&[(3, 11905)]), "Magic", "");
        assert_eq!(
            (magic.attack_roll, magic.defence_roll, magic.max_hit),
            (8690, 36036, 28 - resistance)
        );
        near(
            magic.dps,
            8690.0 / (2.0 * 36037.0) * mean(28, resistance) / 2.4,
        );
        let ranged = result(id, &set(&[(3, 9185), (13, 9144)]), "Ranged", "");
        assert_eq!(
            (ranged.attack_roll, ranged.defence_roll, ranged.max_hit),
            (16478, 21321, 30 - armour)
        );
        near(
            ranged.dps,
            16478.0 / (2.0 * 21322.0) * mean(30, armour) / 3.0,
        );
        for r in [&melee, &magic, &ranged] {
            valid(r);
        }
    }
}
#[test]
fn stunned_bork_has_no_armour_and_guarantees_only_melee_hits() {
    let target = planner::monster(205013).unwrap();
    assert_eq!(
        (
            target.defensive["stab"],
            target.defensive["slash"],
            target.defensive["crush"]
        ),
        (40, 30, -90)
    );
    let mace = result(205013, &set(&[(3, 1434)]), "Crush", "");
    near(mace.accuracy, 1.0);
    for (weapon, style) in [(4151, "Slash"), (26219, "Stab")] {
        near(
            result(205013, &set(&[(3, weapon)]), style, "").accuracy,
            1.0,
        );
    }
    near(mace.dps, mean(20, 0) / 2.4);
    let ranged = result(205013, &set(&[(3, 9185), (13, 9144)]), "Ranged", "");
    let magic = result(205013, &set(&[(3, 11905)]), "Magic", "");
    assert!(ranged.accuracy < 1.0 && magic.accuracy < 1.0);
    assert_eq!((ranged.max_hit, magic.max_hit), (30, 28));
    assert_eq!(encounters::source_npc_id(205013), 16640);
}
#[test]
fn smorks_two_flat_armour_affects_melee_and_ranged_but_not_magic() {
    let r = result(16641, &set(&[(3, 9185), (13, 9144)]), "Ranged", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (16478, 32976, 28)
    );
    near(r.dps, 16478.0 / (2.0 * 32977.0) * mean(30, 2) / 3.0);
    let m = result(16641, &set(&[(3, 1434)]), "Crush", "");
    assert_eq!(m.max_hit, 18);
    near(m.dps, m.accuracy * mean(20, 2) / 2.4);
    let magic = result(16641, &set(&[(3, 11905)]), "Magic", "");
    assert_eq!((magic.defence_roll, magic.max_hit), (90016, 28));
    near(magic.dps, 8690.0 / (2.0 * 90017.0) * mean(28, 0) / 2.4);
    // Accurate hits can become zero after armour; they are distinct from misses.
    let zero = r
        .hit_distribution
        .iter()
        .filter(|(d, _)| *d == 0)
        .map(|(_, p)| p)
        .sum::<f64>();
    near(zero, 1.0 - r.accuracy + r.accuracy * 3.0 / 31.0);
}
#[test]
fn ruby_and_diamond_procs_are_reduced_once_after_their_own_damage_rules() {
    let r = result(16640, &set(&[(3, 9185), (13, 9242)]), "Ranged", "");
    assert_eq!(r.max_hit, 92);
    let proc = r.hp_dependent.unwrap();
    assert_eq!(proc.outcomes(1500)[0].0, 92);
    assert_eq!(proc.outcomes(100)[0].0, 12);
    assert_eq!(proc.outcomes(40)[0].0, 0);
    near(r.dps, (0.94 * r.accuracy * mean(28, 8) + 0.06 * 92.0) / 3.0);
    let d = result(16640, &set(&[(3, 9185), (13, 9243)]), "Ranged", "");
    assert_eq!(d.max_hit, 24);
    near(
        d.dps,
        (0.9 * d.accuracy * mean(28, 8) + 0.1 * mean(32, 8)) / 3.0,
    );
    let smork = result(16641, &set(&[(3, 9185), (13, 9242)]), "Ranged", "");
    assert_eq!(smork.hp_dependent.unwrap().outcomes(100)[0].0, 18);
    valid(&r);
    valid(&d);
    valid(&smork);
}
#[test]
fn armour_is_subtracted_from_each_scythe_and_karil_splat_before_summing() {
    // Aggressive crush scythe: effective Strength 110, +75 strength => raw max24.
    // Bork is size4: splat maxima24,12,6; after armour8:16,4,0, not34.
    let scythe = result(16640, &set(&[(3, 22325)]), "Crush", "");
    assert_eq!(
        (scythe.attack_roll, scythe.defence_roll, scythe.max_hit),
        (10058, 22866, 20)
    );
    let acc = 10058.0 / (2.0 * 22867.0);
    near(
        scythe.dps,
        acc * (mean(24, 8) + mean(12, 8) + mean(6, 8)) / 3.0,
    );
    let k = set(&[
        (0, 4732),
        (2, 12851),
        (3, 4734),
        (4, 4736),
        (7, 4738),
        (13, 4740),
    ]);
    let karil = result(16641, &k, "Ranged", "");
    assert_eq!(karil.max_hit, 26); // raw20 + secondary10, minus2 each.
    let expected = (0..=20)
        .map(|d: i32| {
            let first = (d.max(1) - 2).max(0) as f64;
            first + 0.25 * ((d / 2).max(1) - 2).max(0) as f64
        })
        .sum::<f64>()
        / 21.0;
    near(karil.dps, karil.accuracy * expected / 1.8);
    valid(&scythe);
    valid(&karil);
}
#[test]
fn flat_armour_does_not_reduce_eclipse_burn_pulses_or_lose_accurate_zero_procs() {
    let s = set(&[(0, 29010), (4, 29004), (7, 29007), (3, 29000), (13, 28991)]);
    for id in [16640, 16641, 16684] {
        let r = result(id, &s, "Ranged", "");
        let burn = r.burn.as_ref().unwrap();
        assert!(burn.matches(&r.hit_distribution));
        assert_eq!(burn.pulse.damage(3), 3);
        if id != 16684 {
            assert!(
                burn.hits
                    .iter()
                    .any(|h| h.damage == 0 && h.applies_burn && h.probability > 0.0)
            );
        }
        near(
            burn.hits
                .iter()
                .filter(|h| h.applies_burn)
                .map(|h| h.probability)
                .sum(),
            r.accuracy * 0.2,
        );
    }
}
#[test]
fn xarth_has_ordinary_body_damage_water_weakness_and_reduced_demonbane() {
    let trident = result(16684, &set(&[(3, 11905)]), "Magic", "");
    assert_eq!(
        (trident.attack_roll, trident.defence_roll, trident.max_hit),
        (8690, 96571, 28)
    );
    near(trident.dps, 8690.0 / (2.0 * 96572.0) * mean(28, 0) / 2.4);
    let water = result(16684, &set(&[(3, 1383)]), "Magic", "Water Surge");
    assert_eq!(water.max_hit, 44);
    assert_eq!(encounters::demonbane_vulnerability(16684), 80);
    // Pinned Emberlight: Slash Accurate, attack+70 and strength+13. 70%*80%=56%.
    let ember = result(16684, &set(&[(3, 29589)]), "Slash", "");
    assert_eq!(ember.attack_roll, 14740 + 14740 * 56 / 100);
    assert_eq!(ember.max_hit, 13 + 13 * 56 / 100);
    for id in [16640, 16684, 16641] {
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &modifiers::Reductions {
                bgs: 10_000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            reduced.skills["def"],
            if id == 16640 {
                180
            } else if id == 16684 {
                102
            } else {
                0
            }
        );
    }
}
#[test]
fn every_archive_route_has_outgoing_coverage_and_armour_variants_share_one_body() {
    let raid = raids::raid("tfa").unwrap();
    for first in ["ezekiel", "bork-smorks"] {
        for second in ["moia", "dreadnoth"] {
            for third in ["xarth", "dagonhai"] {
                let t = [first, second, third]
                    .iter()
                    .fold(raid.default_targets(), |t, id| raid.toggle(&t, id).unwrap());
                let o = Options {
                    boss: Boss::Custom,
                    targets: t,
                    ..Default::default()
                };
                assert!(
                    coverage::windows(&o).iter().all(|w| w.supported),
                    "{first} {second} {third}"
                );
            }
        }
    }
    for id in [16640, 205010, 205011, 205012, 205013] {
        let w = encounters::window(id).unwrap();
        assert_eq!(w.progress_group.as_deref(), Some("npc_16640"));
        assert!(w.comparison_probe);
        assert_eq!(w.comparison_alternative, id != 16640);
    }
    // The public route keeps one Bork target; comparison variants add no extra boss.
    let t = raid.toggle(&raid.default_targets(), "bork-smorks").unwrap();
    assert_eq!(
        t.iter()
            .filter(|Target { id, .. }| encounters::source_npc_id(*id) == 16640)
            .count(),
        1
    );
}

#[test]
fn ascension_bolts_apply_archive_armour_per_shot_after_enchantment_rolls() {
    for (target, armour) in [(16640, 8), (205012, 14), (205013, 0), (16641, 2)] {
        for stance in ["Rapid", "Accurate"] {
            for diary in [false, true] {
                for ammo in [34684, 34690, 34696, 34702, 34708] {
                    let gear = set(&[(3, 34681), (13, ammo)]);
                    let b = combat::bonuses(&gear);
                    let effective = if stance == "Rapid" { 107 } else { 110 };
                    let attack = i64::from(effective) * (i64::from(b.offensive["ranged"]) + 64);
                    let raw = (effective * (b.other["ranged_str"] + 64) + 320) / 640;
                    let m = planner::monster(target).unwrap();
                    let defence =
                        i64::from(m.skills["def"] + 9) * i64::from(m.defensive["heavy"] + 64);
                    let chance = attack as f64 / (2.0 * (defence + 1) as f64);
                    let settings = ironbound::defence::Settings {
                        ranged_stance: stance.into(),
                        kandarin_diary: diary,
                        ..Default::default()
                    };
                    let r = combat::calculate_configured(
                        &member(),
                        &gear,
                        "Ranged",
                        m,
                        "",
                        0,
                        &settings,
                    )
                    .unwrap();
                    near(r.accuracy, chance);
                    let bonus = if diary { 1.1 } else { 1.0 };
                    let expected = match ammo {
                        34696 => {
                            let p = 0.05 * bonus;
                            (1.0 - p) * chance * mean(raw, armour)
                                + p * mean(raw * 115 / 100, armour)
                        }
                        34708 => {
                            let p = 0.055 * bonus;
                            chance
                                * ((1.0 - p) * mean(raw, armour)
                                    + p * mean(raw * 120 / 100, armour))
                        }
                        _ => chance * mean(raw, armour),
                    };
                    near(
                        r.dps,
                        expected / (if stance == "Rapid" { 1.2 } else { 1.8 }),
                    );
                    assert_eq!(
                        r.max_hit,
                        (match ammo {
                            34696 => raw * 115 / 100,
                            34708 => raw * 120 / 100,
                            _ => raw,
                        } - armour)
                            .max(0)
                    );
                    valid(&r);
                }
            }
        }
    }
    let gear = set(&[(3, 34681), (13, 34684)]);
    let mut other = planner::monster(26).unwrap().clone();
    other.defensive.insert("flat_armour".into(), 2);
    assert!(
        combat::calculate(&member(), &gear, "Ranged", &other, "")
            .unwrap_err()
            .contains("for this encounter")
    );
}
