//! Independent arithmetic and bounded-state checks; see docs/zorya-kisten.md.
use ironbound::{
    account::{self, Member, Snapshot},
    combat,
    defence::Settings,
    planner::{self, Boss, Options, Target},
    special_attacks::{self, Report},
};
use std::collections::BTreeMap;
fn member() -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
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
fn target() -> planner::Monster {
    let mut t = planner::monster(26).unwrap().clone();
    t.attributes.clear();
    t.weakness = None;
    t.defensive.clear();
    for k in [
        "stab", "slash", "crush", "magic", "heavy", "standard", "light",
    ] {
        t.defensive.insert(k.into(), 0);
    }
    t.skills.insert("def".into(), 99);
    t.skills.insert("magic".into(), 99);
    t.skills.insert("hp".into(), 1000);
    t
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(min: i32, max: i32, transform: impl Fn(i32) -> i32) -> f64 {
    (min..=max).map(|d| transform(d) as f64).sum::<f64>() / (max - min + 1) as f64
}
fn report(s: &[Option<i32>], style: &str, t: &planner::Monster, settings: &Settings) -> Report {
    special_attacks::evaluate(&member(), s, style, t, 0, settings).unwrap()
}
fn valid(r: &Report) {
    close(r.outcomes.iter().map(|o| o.probability).sum(), 1.0);
    assert!(
        r.outcomes
            .iter()
            .all(|o| o.remaining_hp <= r.start_hp && o.empowered_hits <= 3 && o.probability >= 0.0)
    );
    assert!(r.effective_damage <= r.start_hp as f64 + 1e-9);
    assert!((0.0..=1.0 + 1e-9).contains(&r.kill_chance));
    for attack in [r.opener.as_ref(), r.empowered.as_ref()]
        .into_iter()
        .flatten()
    {
        close(attack.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
        close(
            attack.dps,
            attack
                .hit_distribution
                .iter()
                .map(|(d, p)| *d as f64 * p)
                .sum::<f64>()
                / (attack.attack_ticks as f64 * 0.6),
        );
    }
}

#[test]
fn zorya_opener_and_empowered_casts_apply_final_floors_and_actual_intervals() {
    let s = set(&[(3, 34725)]);
    let r = report(&s, "Magic", &target(), &Settings::default());
    let opening = r.opener.as_ref().unwrap();
    let empowered = r.empowered.as_ref().unwrap();
    assert_eq!(
        (opening.attack_roll, opening.max_hit, opening.attack_ticks),
        (29975, 41, 3)
    );
    assert_eq!(
        (
            empowered.attack_roll,
            empowered.max_hit,
            empowered.attack_ticks
        ),
        (29975, 41, 2)
    );
    let p = combat::accuracy(29975, 108 * 64);
    close(opening.accuracy, p);
    close(empowered.accuracy, p);
    let damage = p * mean(0, 41, |d| d.max(1));
    close(r.damage, damage * (1.0 + 3.0 * p));
    close(r.effective_damage, r.damage);
    close(r.expected_attacks, 1.0 + 3.0 * p);
    close(r.cooldown_ticks, 3.0 + 6.0 * p);
    close(r.kill_chance, 0.0);
    close(r.remaining_empowered_hits, 0.0);
    assert_eq!(r.energy_spent, 50);
    valid(&r);
    let failure = r
        .outcomes
        .iter()
        .find(|o| o.remaining_hp == 1000 && o.empowered_hits == 0)
        .unwrap();
    close(failure.probability, 1.0 - p);
}

#[test]
fn zorya_accurate_zero_grants_followups_but_failed_accuracy_does_not() {
    let mut t = target();
    t.defensive.insert("magic_resistance".into(), 100);
    let r = report(&set(&[(3, 34725)]), "Magic", &t, &Settings::default());
    let p = combat::accuracy(29975, 108 * 64);
    close(r.buff_chance.unwrap(), p);
    close(r.expected_attacks, 1.0 + 3.0 * p);
    close(r.damage, 0.0);
    close(r.effective_damage, 0.0);
    close(r.kill_chance, 0.0);
    valid(&r);
}

#[test]
fn zorya_target_death_stops_attacks_and_retains_unused_empowerment() {
    let mut settings = Settings::default();
    settings.target_hp.insert(26, 1);
    let r = report(&set(&[(3, 34725)]), "Magic", &target(), &settings);
    let p = combat::accuracy(29975, 108 * 64);
    close(r.expected_attacks, 1.0);
    close(r.cooldown_ticks, 3.0);
    close(r.kill_chance, p);
    close(r.effective_damage, p);
    close(r.remaining_empowered_hits, 3.0 * p);
    valid(&r);
}

#[test]
fn held_zorya_attacks_consume_buffs_on_misses_without_spending_special_energy() {
    let mut settings = Settings {
        special_energy: 0,
        zorya_empowered_hits: 3,
        ..Default::default()
    };
    settings.target_hp.insert(26, 1);
    let r = report(&set(&[(3, 34725)]), "Magic", &target(), &settings);
    let p = combat::accuracy(29975, 108 * 64);
    let q = 1.0 - p;
    assert!(r.opener.is_none());
    assert_eq!(r.energy_spent, 0);
    assert!(r.buff_chance.is_none());
    close(r.expected_attacks, 1.0 + q + q * q);
    close(r.cooldown_ticks, 2.0 * (1.0 + q + q * q));
    close(r.kill_chance, 1.0 - q.powi(3));
    close(r.remaining_empowered_hits, 2.0 * p + q * p);
    valid(&r);
    settings.target_hp.clear();
    settings.zorya_empowered_hits = 1;
    let redirected = report(&set(&[(3, 34725)]), "Magic", &target(), &settings);
    close(redirected.expected_attacks, 1.0);
    close(redirected.cooldown_ticks, 2.0);
    close(redirected.remaining_empowered_hits, 0.0);
}

#[test]
fn zorya_overkill_and_remaining_buffs_match_an_independent_recursive_oracle() {
    // Direct recursive expectations over raw rolls; the implementation instead
    // advances and merges explicit HP/buff states.
    fn pending(hp: u32, left: u32, p: f64, memo: &mut BTreeMap<(u32, u32), [f64; 4]>) -> [f64; 4] {
        if hp == 0 {
            return [0.0, 0.0, 1.0, left as f64];
        }
        if left == 0 {
            return [0.0; 4];
        }
        if let Some(value) = memo.get(&(hp, left)) {
            return *value;
        }
        let mut value = [0.0, 1.0, 0.0, 0.0];
        let miss = pending(hp, left - 1, p, memo);
        for i in 0..4 {
            value[i] += (1.0 - p) * miss[i];
        }
        for raw in 0..=41 {
            let damage = raw.max(1).min(hp);
            let after = pending(hp - damage, left - 1, p, memo);
            value[0] += p / 42.0 * (damage as f64 + after[0]);
            for i in 1..4 {
                value[i] += p / 42.0 * after[i];
            }
        }
        memo.insert((hp, left), value);
        value
    }
    let mut settings = Settings::default();
    settings.target_hp.insert(26, 30);
    let r = report(&set(&[(3, 34725)]), "Magic", &target(), &settings);
    let p = combat::accuracy(29975, 108 * 64);
    let mut expected = [0.0, 1.0, 0.0, 0.0];
    let mut memo = BTreeMap::new();
    for raw in 0..=41 {
        let damage = raw.max(1).min(30);
        let after = pending(30 - damage, 3, p, &mut memo);
        expected[0] += p / 42.0 * (damage as f64 + after[0]);
        for i in 1..4 {
            expected[i] += p / 42.0 * after[i];
        }
    }
    close(r.effective_damage, expected[0]);
    close(r.expected_attacks, expected[1]);
    close(r.kill_chance, expected[2]);
    close(r.remaining_empowered_hits, expected[3]);
    close(r.cooldown_ticks, 3.0 + 2.0 * (expected[1] - 1.0));
    valid(&r);
}

#[test]
fn zorya_boosts_slayer_salve_and_two_handed_restrictions_keep_their_order() {
    let mut settings = Settings::default();
    settings.slayer_targets.insert(26);
    let r = report(
        &set(&[(3, 34725), (0, 11865)]),
        "Magic",
        &target(),
        &settings,
    );
    assert_eq!(r.opener.as_ref().unwrap().max_hit, 46); // floor(floor(26*1.15)*1.60)
    assert_eq!(r.opener.as_ref().unwrap().attack_roll, 35420); // helmet adds +3: floor(floor(110*112*1.15)*2.50)
    settings = Settings::default();
    settings.boosts.magic = "Saturated heart".into();
    let boosted = report(&set(&[(3, 34725)]), "Magic", &target(), &settings);
    assert_eq!(boosted.opener.as_ref().unwrap().max_hit, 49); // Magic112: floor(27*1.15)=31, then floor(*1.6)
    let mut undead = target();
    undead.attributes.push("undead".into());
    let salve = report(
        &set(&[(3, 34725), (2, 12018)]),
        "Magic",
        &undead,
        &Settings::default(),
    );
    assert_eq!(salve.opener.as_ref().unwrap().max_hit, 49); // floor(23*1.35)=31, then floor(*1.6)
    assert!(
        special_attacks::evaluate(
            &member(),
            &set(&[(3, 34725), (5, 12825)]),
            "Magic",
            &target(),
            0,
            &Settings::default()
        )
        .unwrap_err()
        .contains("two-handed")
    );
}

#[test]
fn kisten_four_accuracy_results_match_all_sixteen_independent_roll_patterns() {
    let r = report(
        &set(&[(3, 33631)]),
        "Crush",
        &target(),
        &Settings::default(),
    );
    let a = r.opener.as_ref().unwrap();
    assert_eq!((a.attack_roll, a.max_hit, a.attack_ticks), (15840, 34, 4));
    let p = combat::accuracy(110 * 144, 108 * 64);
    let q = 1.0 - p;
    let mut expected = BTreeMap::new();
    for mask in 0_u32..16 {
        let n = mask.count_ones();
        let weight = p.powi(n as i32) * q.powi(4 - n as i32);
        if n == 0 {
            *expected.entry(0).or_insert(0.0) += weight;
            continue;
        }
        let min = 20 * (50 + n as i32 * 20) / 100;
        let max = 20 * (90 + n as i32 * 20) / 100;
        for d in min..=max {
            *expected.entry(d.max(1)).or_insert(0.0) += weight / (max - min + 1) as f64;
        }
    }
    let mut actual = BTreeMap::new();
    for (d, p) in &a.hit_distribution {
        *actual.entry(*d).or_insert(0.0) += p;
    }
    for (d, p) in expected {
        close(actual.get(&d).copied().unwrap_or(0.0), p);
    }
    close(a.accuracy, 1.0 - q.powi(4));
    close(r.expected_attacks, 1.0);
    close(r.cooldown_ticks, 4.0);
    assert_eq!(r.energy_spent, 50);
    assert!(r.empowered.is_none());
    valid(&r);
}

#[test]
fn kisten_uses_selected_offence_and_stance_but_always_crush_defence() {
    let mut t = target();
    t.defensive.insert("stab".into(), 1000);
    let stab = report(&set(&[(3, 33631)]), "Stab", &t, &Settings::default());
    let a = stab.opener.as_ref().unwrap();
    assert_eq!(a.attack_roll, 108 * 64);
    assert_eq!(a.defence_roll, 108 * 64);
    assert_eq!(a.max_hit, 34);
    close(
        a.accuracy,
        1.0 - (1.0 - combat::accuracy(108 * 64, 108 * 64)).powi(4),
    );
    valid(&stab);
}

#[test]
fn kisten_flat_armour_is_applied_once_to_its_single_hit_not_to_four_splats() {
    let mut t = target();
    t.defensive.insert("flat_armour".into(), 4);
    let r = report(&set(&[(3, 33631)]), "Crush", &t, &Settings::default());
    assert_eq!(r.opener.as_ref().unwrap().max_hit, 30);
    let p = combat::accuracy(15840, 108 * 64);
    let weights = [4.0, 6.0, 4.0, 1.0];
    let expected = (1..=4)
        .map(|n| {
            weights[(n - 1) as usize]
                * p.powi(n)
                * (1.0 - p).powi(4 - n)
                * mean(20 * (50 + 20 * n) / 100, 20 * (90 + 20 * n) / 100, |d| {
                    (d.max(1) - 4).max(0)
                })
        })
        .sum::<f64>();
    close(r.damage, expected);
    valid(&r);
}

#[test]
fn special_npc_caps_halving_and_guaranteed_maxima_follow_the_special_distribution() {
    for (weapon, style, raw_max, corp_max) in [(34725, "Magic", 41, 41), (33631, "Crush", 34, 17)] {
        let mut t = target();
        t.id = 319;
        let r = report(&set(&[(3, weapon)]), style, &t, &Settings::default());
        assert_eq!(r.opener.as_ref().unwrap().max_hit, corp_max);
        valid(&r);
        t.id = 14014;
        let tail = report(&set(&[(3, weapon)]), style, &t, &Settings::default());
        assert_eq!(
            tail.opener.as_ref().unwrap().max_hit,
            if style == "Crush" { 9 } else { 4 }
        );
        valid(&tail);
        assert!(raw_max >= corp_max); // Corp takes full Magic damage.
    }
    let t = planner::monster(9466).unwrap();
    let r = report(&set(&[(3, 33631)]), "Crush", t, &Settings::default());
    close(r.opener.as_ref().unwrap().accuracy, 1.0);
    assert_eq!(r.opener.as_ref().unwrap().hit_distribution, vec![(34, 1.0)]);
    valid(&r);
    for (weapon, style, max) in [(34725, "Magic", 20), (33631, "Crush", 17)] {
        let acid = report(
            &set(&[(3, weapon)]),
            style,
            planner::monster(100020).unwrap(),
            &Settings::default(),
        );
        assert_eq!(acid.opener.as_ref().unwrap().max_hit, max);
        valid(&acid);
    }
}

#[test]
fn burst_energy_bounds_requirements_and_unreviewed_windows_are_explicit() {
    for (weapon, style) in [(34725, "Magic"), (33631, "Crush")] {
        let s = set(&[(3, weapon)]);
        for id in [100180, 100181, 100185] {
            assert!(
                special_attacks::evaluate(
                    &member(),
                    &s,
                    style,
                    planner::monster(id).unwrap(),
                    0,
                    &Settings::default()
                )
                .unwrap_err()
                .contains("not verified")
            );
        }
        assert!(
            special_attacks::evaluate(
                &member(),
                &s,
                style,
                &target(),
                0,
                &Settings {
                    special_energy: 49,
                    ..Default::default()
                }
            )
            .unwrap_err()
            .contains("50%")
        );
        let rich = report(
            &s,
            style,
            &target(),
            &Settings {
                special_energy: 100,
                ..Default::default()
            },
        );
        assert_eq!(rich.energy_spent, 50);
        let mut m = member();
        m.level_overrides.insert(
            if weapon == 34725 { "Magic" } else { "Strength" }.into(),
            84,
        );
        assert!(
            special_attacks::evaluate(&m, &s, style, &target(), 0, &Settings::default())
                .unwrap_err()
                .contains("requirements")
        );
        assert!(
            special_attacks::evaluate(
                &member(),
                &s,
                style,
                planner::monster(11753).unwrap(),
                0,
                &Settings::default()
            )
            .is_err()
        );
    }
    assert!(!special_attacks::supported(34727));
    assert!(!special_attacks::supported(4151));
    assert!(
        !Settings {
            special_energy: 101,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !Settings {
            zorya_empowered_hits: 4,
            ..Default::default()
        }
        .valid()
    );
}

#[test]
fn special_settings_do_not_change_normal_damage_ranking_or_packing() {
    for (weapon, style) in [(34725, "Magic"), (33631, "Crush")] {
        let s = set(&[(3, weapon)]);
        let ordinary = combat::calculate_configured(
            &member(),
            &s,
            style,
            &target(),
            "",
            0,
            &Settings::default(),
        )
        .unwrap();
        let setting = Settings {
            special_energy: 0,
            zorya_empowered_hits: 3,
            ..Default::default()
        };
        let alternate =
            combat::calculate_configured(&member(), &s, style, &target(), "", 0, &setting).unwrap();
        assert_eq!(
            serde_json::to_value(&ordinary).unwrap(),
            serde_json::to_value(&alternate).unwrap()
        );
    }
    let snapshot = Snapshot {
        members: vec![Member {
            name: "Special QA".into(),
            bank: Some(vec![34725, 1, 8007, 1]),
            ..member()
        }],
        selected: "Special QA".into(),
        source: "Synthetic".into(),
        fetched_at: String::new(),
    };
    let mut options = Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 26,
            style: "Magic".into(),
        }],
        recommended: Some(false),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    options.combat.zorya_empowered_hits = 2;
    options.combat.special_energy = 0;
    let plan = planner::plan(&snapshot, &options);
    assert!(plan.exportable(), "{:?}", plan.missing);
    let setup = ironbound::share::Setup::from_plan(&plan, &options);
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap()).unwrap();
    assert_eq!(restored.options().combat.zorya_empowered_hits, 2);
    assert_eq!(restored.options().combat.special_energy, 0);
    let export = ironbound::export::inventory_setup(&plan, &options, "Special QA").unwrap();
    assert!(export.contains("2 empowered attacks held"));
    assert!(export.contains("0% energy available"));
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!((old.special_energy, old.zorya_empowered_hits), (50, 0));
}
