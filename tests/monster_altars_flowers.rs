//! Independent arithmetic and scope checks for Wiki Skotizo/Hespori mechanics.
use ironbound::{
    account::{self, Member},
    attack_budget, combat,
    defence::Settings,
    encounters,
    monster_rules::{self, Transform},
    planner::{self, Options},
};

fn member(level: u32) -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), level);
    }
    m
}
fn gear(weapon: i32, ammo: Option<i32>) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s[13] = ammo;
    s
}
fn calc(id: i32, weapon: i32, ammo: Option<i32>, style: &str, spell: &str) -> combat::ResultStats {
    combat::calculate(
        &member(99),
        &gear(weapon, ammo),
        style,
        planner::monster(id).unwrap(),
        spell,
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
const STATES: [i32; 5] = [7286, 100110, 100111, 100112, 100113];

#[test]
fn altar_percentages_round_each_damage_outcome_and_keep_misses_zero() {
    for (light, percentages) in [(true, [100, 85, 70, 55, 40]), (false, [100, 75, 50, 25, 0])] {
        for (altars, percent) in percentages.into_iter().enumerate() {
            for damage in 0..=200 {
                let transform = Transform::Skotizo {
                    altars: altars as u8,
                    light,
                };
                assert_eq!(
                    transform.outcomes(damage, 0.25),
                    vec![(damage * percent / 100, 0.25)]
                );
            }
        }
    }
}

#[test]
fn skotizo_runite_attacks_use_independent_roll_and_damage_arithmetic() {
    // Rapid RCB: attack107*154=16478. Defence209*194=40546. Max30.
    let accuracy = 16478.0 / 81094.0;
    for (id, percent) in STATES.into_iter().zip([100, 75, 50, 25, 0]) {
        let r = calc(id, 9185, Some(9144), "Ranged", "");
        assert_eq!((r.attack_roll, r.defence_roll), (16478, 40546));
        assert_eq!(r.max_hit, 30 * percent / 100);
        close(r.accuracy, accuracy);
        let sum: i32 = (0..=30).map(|d| (d.max(1) * percent) / 100).sum();
        close(r.dps, accuracy * f64::from(sum) / 31.0 / 3.0);
    }
}

#[test]
fn altar_reductions_follow_ruby_and_diamond_and_survive_hp_changes() {
    let r = calc(100110, 9185, Some(9242), "Ranged", "");
    assert_eq!(r.hit_distribution.last(), Some(&(67, 0.06))); // floor(450*20%)*75%.
    let proc = r.hp_dependent.unwrap();
    assert_eq!(proc.outcomes(101), vec![(15, 0.06)]);
    assert_eq!(proc.outcomes(24), vec![(3, 0.06)]);
    let hit = 16478.0 / 81094.0;
    let ordinary_sum: i32 = (0..=28).map(|d| d.max(1) * 75 / 100).sum();
    close(
        r.dps,
        (0.94 * hit * f64::from(ordinary_sum) / 29.0 + 0.06 * 67.0) / 3.0,
    );
    let diamond = calc(100110, 9185, Some(9243), "Ranged", "");
    let proc_sum: i32 = (0..=32).map(|d| d.max(1) * 75 / 100).sum();
    assert_eq!(diamond.max_hit, 24);
    close(
        diamond.dps,
        (0.9 * hit * f64::from(ordinary_sum) / 29.0 + 0.1 * f64::from(proc_sum) / 33.0) / 3.0,
    );
}

#[test]
fn skotizo_reduces_scythe_splats_before_they_are_added() {
    let r = calc(100110, 22325, None, "Slash", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20790, 30096, 28)
    );
    // Separate23/11/5 maxima become17/8/3, not floor(39*75%)=29.
    // Conditional means198/24 +45/12 +9/6 =13.5.
    close(r.dps, 20790.0 / 60194.0 * 13.5 / 3.0);
}

#[test]
fn only_arclight_and_emberlight_keep_damage_at_four_altars() {
    for weapon in [19675, 29589] {
        let full = calc(7286, weapon, None, "Slash", "");
        let four = calc(100113, weapon, None, "Slash", "");
        assert_eq!(four.max_hit, full.max_hit * 40 / 100);
        assert!(four.dps > 0.0);
        assert_eq!(
            monster_rules::transform_for(100113, weapon, "slash", &Default::default(), ""),
            Transform::Skotizo {
                altars: 4,
                light: true
            }
        );
    }
    for weapon in [2402, 6746, 29591, 29594, 4151] {
        assert_eq!(
            monster_rules::transform_for(100113, weapon, "slash", &Default::default(), ""),
            Transform::Skotizo {
                altars: 4,
                light: false
            }
        );
    }
    let ruby = calc(100113, 9185, Some(9242), "Ranged", "");
    assert_eq!((ruby.max_hit, ruby.dps), (0, 0.0));
    assert_eq!(ruby.hp_dependent.unwrap().outcomes(450), vec![(0, 0.06)]);
    assert_eq!(calc(100113, 9185, Some(9243), "Ranged", "").dps, 0.0);
}

#[test]
fn active_altars_only_have_the_two_named_weapon_finishers() {
    for weapon in [19675, 29589] {
        let r = calc(7288, weapon, None, "Slash", "");
        assert_eq!(r.hit_distribution, vec![(100, 1.0)]);
        assert_eq!((r.max_hit, r.accuracy), (100, 1.0));
        assert!(r.hp_dependent.is_none());
    }
    for weapon in [2402, 6746, 29591, 29594] {
        assert_eq!(monster_rules::instant_defeat(7288, weapon), None);
    }
    let ordinary = calc(7288, 4151, None, "Slash", "");
    assert!(ordinary.max_hit < 100 && ordinary.accuracy < 1.0);
}

#[test]
fn farming_guild_flowers_die_to_weak_attacks_without_borrowing_ordinary_max_hits() {
    let target = planner::monster(8584).unwrap();
    for (weapon, style, spell) in [(4587, "Slash", ""), (1381, "Magic", "Wind Strike")] {
        let s = gear(weapon, None);
        let mut weak = member(1);
        weak.level_overrides.insert("Attack".into(), 60);
        let r = combat::calculate(&weak, &s, style, target, spell).unwrap();
        assert_eq!(r.hit_distribution, vec![(10, 1.0)]);
        assert_eq!((r.max_hit, r.accuracy), (10, 1.0));
        let budget = attack_budget::for_window(target, &r, &Options::default(), &s).unwrap();
        assert_eq!((budget.attacks, budget.entities), (12.0, 12));
        assert_eq!((budget.start_hp, budget.stop_hp), (10, 0));
        close(budget.seconds, 12.0 * f64::from(r.attack_ticks) * 0.6);
    }
    assert_eq!(encounters::progress_hp(8584, "Ranged"), Some(120.0));
    let ruby = calc(8584, 9185, Some(9242), "Ranged", "");
    assert_eq!(ruby.hit_distribution, vec![(10, 1.0)]);
    assert!(ruby.hp_dependent.is_none());
}

#[test]
fn living_flowers_block_direct_melee_magic_and_bolt_damage_to_the_body() {
    for (weapon, ammo, style, spell) in [
        (4151, None, "Slash", ""),
        (22325, None, "Slash", ""),
        (9185, Some(9242), "Ranged", ""),
        (9185, Some(9243), "Ranged", ""),
        (1381, None, "Magic", "Fire Bolt"),
    ] {
        let r = calc(100120, weapon, ammo, style, spell);
        assert_eq!((r.max_hit, r.dps), (0, 0.0));
        assert!(r.hit_distribution.iter().all(|(d, _)| *d == 0));
        close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
        if let Some(proc) = r.hp_dependent {
            assert_eq!(proc.outcomes(100), vec![(0, 0.06)]);
        }
    }
    let fire = calc(8583, 1381, None, "Magic", "Fire Bolt");
    assert_eq!(fire.max_hit, 24);
    assert_eq!(fire.elemental.unwrap().weakness_percent, 100);
}

#[test]
fn quest_flowers_and_full_fight_timing_do_not_inherit_farming_guarantees() {
    assert_eq!(monster_rules::instant_defeat(11193, 1205), None);
    for id in [11192, 11193] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
    }
    let r = calc(8583, 4151, None, "Slash", "");
    assert!(
        attack_budget::for_window(
            planner::monster(8583).unwrap(),
            &r,
            &Options::default(),
            &gear(4151, None)
        )
        .is_err()
    );
    let mut settings = Settings::default();
    settings.blue_moon_melee = "Stab".into();
    // Incoming attacks are a separate unfinished layer, even in the vulnerable state.
    assert!(
        ironbound::defence::estimate(
            &member(99),
            &gear(4151, None),
            planner::monster(8583).unwrap(),
            "Magic",
            &settings
        )
        .is_err()
    );
}

#[test]
fn related_npcs_are_comparison_choices_without_growing_default_boss_hp() {
    for (name, body, related) in [("Skotizo", 7286, 7288), ("Hespori", 8583, 8584)] {
        let entry = encounters::catalog()
            .iter()
            .find(|e| e.name == name)
            .unwrap();
        assert_eq!(entry.targets.len(), 1);
        assert_eq!(entry.targets[0].id, body);
        assert_eq!(
            encounters::window(related).unwrap().comparison_sources,
            vec![body]
        );
    }
    for (id, altars) in STATES.into_iter().zip(0..=4) {
        let w = encounters::window(id).unwrap();
        assert_eq!(w.active_altars, Some(altars));
        assert_eq!(w.progress_group.as_deref(), Some("skotizo_altars"));
        assert_eq!(encounters::source_npc_id(id), 7286);
        assert_eq!(planner::monster(id).unwrap().skills["hp"], 450);
    }
}

#[test]
fn flower_protection_and_instant_death_do_not_leave_sustained_burn_damage() {
    let mut s = gear(29000, Some(28991));
    for (slot, id) in [(0, 29010), (4, 29004), (7, 29007)] {
        s[slot] = Some(id);
    }
    for (id, damage) in [(8584, 10), (100120, 0)] {
        let r = combat::calculate(&member(99), &s, "Ranged", planner::monster(id).unwrap(), "")
            .unwrap();
        assert_eq!(r.max_hit, damage);
        assert_eq!(r.burn_dps, 0.0);
    }
    let mut blood = gear(28997, None);
    for (slot, id) in [(0, 29028), (4, 29022), (7, 29025)] {
        blood[slot] = Some(id);
    }
    assert!(
        combat::calculate(
            &member(99),
            &blood,
            "Crush",
            planner::monster(8584).unwrap(),
            ""
        )
        .unwrap_err()
        .contains("first hitsplat")
    );
}
