use ironbound::{
    account::{self, Member},
    burn::{self, BurnAttack, BurnHit, Burns, Phase, PulseRule},
    combat,
    defence::Settings,
    monster_rules::Transform,
    planner::{self, Options},
};
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn attack(damage: i32, chance: f64) -> BurnAttack {
    BurnAttack {
        hits: vec![
            BurnHit {
                damage,
                probability: 1.0 - chance,
                applies_burn: false,
            },
            BurnHit {
                damage,
                probability: chance,
                applies_burn: true,
            },
        ],
        pulse: PulseRule {
            transform: Transform::Identity,
            divisor: 1,
        },
    }
}
fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn eclipse() -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    for (slot, id) in [(0, 29010), (4, 29004), (7, 29007), (3, 29000), (13, 28991)] {
        s[slot] = Some(id);
    }
    s
}
#[test]
fn pulses_have_buildup_ten_ticks_of_damage_and_a_shared_clock() {
    let mut b = Burns::default();
    assert_eq!(b.tick(), 0);
    assert!(b.apply());
    for _ in 0..3 {
        assert_eq!(b.tick(), 0);
    }
    assert!(b.apply()); // New stack at tick three keeps the original tick-four pulse.
    assert_eq!(b.tick(), 2);
    let damage = (0..36).map(|_| b.tick()).sum::<u32>();
    assert_eq!(damage, 18);
    assert_eq!(b.stacks(), 0);
    for _ in 0..50 {
        assert_eq!(b.tick(), 0);
    }
    assert!(b.apply());
    for _ in 0..3 {
        assert_eq!(b.tick(), 0);
    }
    assert_eq!(b.tick(), 1);
}
#[test]
fn full_stacks_discard_procs_without_refreshing_old_stacks() {
    let mut b = Burns::default();
    for _ in 0..5 {
        assert!(b.apply());
    }
    let mut damage = (0..20).map(|_| b.tick()).sum::<u32>();
    assert_eq!(b.stacks(), 5);
    assert!(!b.apply());
    damage += (0..20).map(|_| b.tick()).sum::<u32>();
    assert_eq!(damage, 50);
    assert_eq!(b.stacks(), 0);
    assert_eq!(b.tick(), 0);
}
#[test]
fn npc_reductions_apply_to_the_combined_ranged_burn_pulse() {
    assert_eq!(
        PulseRule {
            transform: Transform::Corporeal,
            divisor: 1
        }
        .damage(5),
        2
    );
    assert_eq!(
        PulseRule {
            transform: Transform::Kraken,
            divisor: 1
        }
        .damage(5),
        1
    );
    assert_eq!(
        PulseRule {
            transform: Transform::Identity,
            divisor: 2
        }
        .damage(5),
        2
    );
    let mut profile = attack(1, 1.0);
    profile.pulse.transform = Transform::Skotizo {
        altars: 0,
        light: false,
    };
    close(burn::estimate(&profile, 3, 4, 0).unwrap().attacks, 2.0);
    profile.pulse.transform = Transform::Skotizo {
        altars: 4,
        light: false,
    };
    close(burn::estimate(&profile, 3, 4, 0).unwrap().attacks, 4.0);
}
#[test]
fn burn_kills_stop_shots_and_cap_damage_at_actual_remaining_hp() {
    let b = burn::estimate(&attack(1, 1.0), 3, 4, 0).unwrap();
    close(b.attacks, 2.0);
    close(b.cooldown_ticks, 6.0);
    close(b.death_ticks, 4.0);
    close(b.burn_damage, 2.0);
    let direct = burn::estimate(&attack(1, 0.0), 3, 4, 0).unwrap();
    close(direct.attacks, 4.0);
    close(direct.death_ticks, 9.0);
    let one = burn::estimate(&attack(100, 1.0), 3, 1, 0).unwrap();
    close(one.attacks, 1.0);
    close(one.death_ticks, 0.0);
    close(one.burn_damage, 0.0);
    let zero = burn::estimate(&attack(0, 1.0), 3, 2, 0).unwrap();
    close(zero.attacks, 2.0);
    close(zero.death_ticks, 4.0);
    close(zero.burn_damage, 2.0);
    close(
        burn::estimate(&attack(0, 1.0), 3, 2, 1)
            .unwrap()
            .burn_damage,
        1.0,
    );
    let same_tick = burn::estimate(&attack(0, 1.0), 4, 1, 0).unwrap();
    close(same_tick.attacks, 1.0);
    close(same_tick.death_ticks, 4.0);
    close(same_tick.cooldown_ticks, 4.0);
}
#[test]
fn phase_reset_clears_stacks_but_preserves_the_weapon_cooldown() {
    let run = |clear, downtime| {
        burn::estimate_phases(
            &attack(1, 1.0),
            3,
            8,
            &[
                Phase {
                    stop_hp: 6,
                    downtime,
                    clear_burns: clear,
                },
                Phase {
                    stop_hp: 0,
                    downtime: 0,
                    clear_burns: true,
                },
            ],
            16,
            1,
        )
        .unwrap()
    };
    let carry = run(false, 0);
    close(carry.attacks, 3.0);
    close(carry.death_ticks, 8.0);
    let reset = run(true, 0);
    close(reset.attacks, 5.0);
    close(reset.death_ticks, 14.0);
    close(reset.cooldown_ticks, 15.0);
    let expired = run(false, 40);
    close(expired.attacks, 5.0);
    close(expired.death_ticks, 51.0);
    close(expired.death_ticks, run(true, 40).death_ticks);
}
#[test]
fn exact_small_tree_matches_seeded_estimate_and_reported_sampling_error() {
    // HP4, direct1, cooldown3, p=.2: N=2 with p², N=3 with
    // 2p(1-p), N=4 with (1-p)². Death ticks are 4, 6/7, 9.
    let sample = burn::estimate_phases(
        &attack(1, 0.2),
        3,
        4,
        &[Phase {
            stop_hp: 0,
            downtime: 0,
            clear_burns: true,
        }],
        16384,
        97,
    )
    .unwrap();
    assert!((sample.attacks - 3.6).abs() < 4.0 * sample.attacks_error);
    assert!((sample.death_ticks - 8.0).abs() < 4.0 * sample.death_error);
    assert!((sample.burn_damage - 0.4).abs() < 0.03);
    let again = burn::estimate_phases(
        &attack(1, 0.2),
        3,
        4,
        &[Phase {
            stop_hp: 0,
            downtime: 0,
            clear_burns: true,
        }],
        16384,
        97,
    )
    .unwrap();
    assert_eq!(sample.attacks.to_bits(), again.attacks.to_bits());
}
#[test]
fn misses_cannot_apply_burn_while_accurate_zeroes_still_can() {
    let s = eclipse();
    for id in [26, 319, 100020] {
        let r =
            combat::calculate(&member(), &s, "Ranged", planner::monster(id).unwrap(), "").unwrap();
        let b = r.burn.as_ref().unwrap();
        assert!(b.matches(&r.hit_distribution));
        close(
            b.hits
                .iter()
                .filter(|h| h.applies_burn)
                .map(|h| h.probability)
                .sum(),
            r.accuracy * 0.2,
        );
        if id == 26 {
            assert!(
                b.hits
                    .iter()
                    .filter(|h| h.applies_burn)
                    .all(|h| h.damage > 0)
            );
        } else {
            assert!(
                b.hits
                    .iter()
                    .any(|h| h.applies_burn && h.damage == 0 && h.probability > 0.0)
            );
        }
        assert_eq!(b.pulse.divisor, if id == 100020 { 2 } else { 1 });
    }
    let immune = combat::calculate(
        &member(),
        &s,
        "Ranged",
        planner::monster(14779).unwrap(),
        "",
    )
    .unwrap();
    assert!(immune.burn.is_none());
}
#[test]
fn finite_hp_budget_starts_empty_respects_current_hp_and_never_credits_tail_damage() {
    let s = eclipse();
    let target = planner::monster(26).unwrap();
    let r = combat::calculate(&member(), &s, "Ranged", target, "").unwrap();
    let mut o = Options::default();
    o.combat.target_hp.insert(26, 1);
    let b = ironbound::attack_budget::for_window(target, &r, &o, &s).unwrap();
    let estimate = b.burn.unwrap();
    close(estimate.burn_damage, 0.0);
    assert!((b.attacks - 1.0 / r.accuracy).abs() < 0.02);
    o.combat.target_hp.insert(26, 22);
    let full = ironbound::attack_budget::for_window(target, &r, &o, &s).unwrap();
    assert!(full.attacks > b.attacks);
    assert!(full.burn.as_ref().unwrap().burn_damage > 0.0);
    assert!(
        full.seconds > 22.0 / r.dps,
        "stationary DPS cannot shortcut startup and overkill"
    );
    let default: Settings = serde_json::from_str("{}").unwrap();
    assert!(default.valid());
}
#[test]
fn malformed_or_nonprogressing_models_are_rejected_without_completed_only_bias() {
    let mut b = attack(1, 0.2);
    b.hits[0].probability = f64::NAN;
    assert!(burn::estimate(&b, 3, 20, 0).is_err());
    assert!(
        burn::estimate_phases(
            &attack(0, 0.0),
            3,
            1,
            &[Phase {
                stop_hp: 0,
                downtime: 0,
                clear_burns: true
            }],
            1,
            1
        )
        .is_err()
    );
    assert!(burn::estimate(&attack(1, 1.0), 0, 20, 0).is_err());
    assert!(burn::estimate(&attack(1, 1.0), 3, 20, 20).is_err());
}
