//! Finite HP, alternating styles and cooldown commitments are testable without
//! pretending to simulate platform movement, dodging or environmental damage.
use ironbound::{
    account, combat, encounters,
    optimization::StyleSet,
    planner::{self, Boss, Options},
    yama,
};
use std::collections::BTreeMap;
fn member() -> account::Member {
    let mut m = account::Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: encounters::groups()
            .iter()
            .find(|g| g.name == "Yama")
            .unwrap()
            .targets
            .clone(),
        ..Default::default()
    }
}
fn gear(weapon: i32) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s
}
#[test]
fn finite_judge_starts_magic_caps_finisher_and_commits_both_cooldowns() {
    let result = yama::simulate_judges(40, 4, 30, 5, 100, 2).unwrap();
    // Per Judge: Magic40 -> Melee30 -> Magic30(capped), 4+5+4 ticks.
    assert_eq!(result.magic_hits, 4.0);
    assert_eq!(result.melee_hits, 2.0);
    assert_eq!(result.ticks, 26.0);
    assert_eq!(result.seconds, 15.6);
    let one = yama::simulate_judges(400, 5, 30, 4, 400, 2).unwrap();
    assert_eq!(one.magic_hits, 2.0);
    assert_eq!(one.melee_hits, 0.0);
    assert_eq!(one.ticks, 10.0);
}
#[test]
fn proc_breakpoint_uses_distribution_not_a_free_ceiling_every_cast() {
    // Half the Magic hits kill immediately; otherwise one melee finishes.
    let result =
        yama::simulate_judge_distributions(&[(40, 0.5), (60, 0.5)], 4, &[(20, 1.0)], 5, 50, 2)
            .unwrap();
    assert_eq!(result.magic_hits, 2.0);
    assert_eq!(result.melee_hits, 1.0);
    assert_eq!(result.ticks, 13.0);
    assert!(result.ticks > yama::simulate_judges(60, 4, 20, 5, 50, 2).unwrap().ticks);
    assert!(yama::simulate_judges(0, 4, 20, 4, 400, 2).is_err());
}
#[test]
fn actual_judge_distributions_and_paired_proxy_replacement_are_consistent() {
    let m = member();
    let o = options();
    let owned = BTreeMap::from([(19675, 1), (11905, 1)]);
    let melee = gear(19675);
    let magic = gear(11905);
    let cycle = yama::judge_cycle(&m, &melee, &magic, &o, &owned).unwrap();
    assert!(cycle.seconds > 0.0);
    assert_eq!(cycle.intermissions, 2);
    assert_eq!(cycle.hp_per_judge, 400);
    let sets = vec![
        StyleSet {
            style: "Slash".into(),
            equipment: melee.clone(),
        },
        StyleSet {
            style: "Melee".into(),
            equipment: melee.clone(),
        },
        StyleSet {
            style: "Magic".into(),
            equipment: magic.clone(),
        },
    ];
    let hit_m =
        combat::calculate(&m, &melee, "Melee", planner::monster(100013).unwrap(), "").unwrap();
    let hit_g =
        combat::calculate(&m, &magic, "Magic", planner::monster(100012).unwrap(), "").unwrap();
    let proxy = 400.0 / hit_m.dps + 400.0 / hit_g.dps;
    let adjustment = yama::paired_adjustment(&m, &sets, &o, &owned).unwrap();
    assert!(
        (proxy + adjustment - cycle.seconds).abs() < 1e-8,
        "Count one paired Judge cycle, not both proxies plus that cycle"
    );
    let mut weighted = o.clone();
    weighted.phase_priorities.insert(100012, 2);
    weighted.phase_priorities.insert(100013, 0);
    let weighted_proxy = 800.0 / hit_g.dps;
    assert!(
        (weighted_proxy + yama::paired_adjustment(&m, &sets, &weighted, &owned).unwrap()
            - cycle.seconds)
            .abs()
            < 1e-8
    );
}
#[test]
fn flare_pair_uses_discrete_casts_and_real_source_rules() {
    let m = member();
    let s = gear(11905);
    let result = combat::calculate(&m, &s, "Magic", planner::monster(100014).unwrap(), "").unwrap();
    assert_eq!(encounters::window(100014).unwrap().npc_id, 14179);
    assert_eq!(encounters::demonbane_vulnerability(14179), 200);
    assert!(encounters::hit_rule(14179).unwrap().always_max);
    assert_eq!(result.accuracy, 1.0);
    let casts = (140 + result.max_hit - 1) / result.max_hit;
    assert_eq!(
        yama::window_attack_time(100014, &result),
        Some(f64::from(casts * result.attack_ticks * 2) * 0.6)
    );
    assert!(
        combat::calculate(
            &m,
            &gear(4151),
            "Melee",
            planner::monster(100014).unwrap(),
            ""
        )
        .is_err()
    );
}
