use ironbound::{
    account::{self, Member},
    defence::{self, Settings},
    modifiers::Reductions,
    planner,
};
fn member() -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m
}
fn calc(id: i32, set: &[Option<i32>], s: &Settings) -> defence::Estimate {
    defence::estimate(
        &member(),
        set,
        planner::monster(id).unwrap(),
        if id == 1673 { "Slash" } else { "Stab" },
        s,
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[test]
fn dharok_maximum_tracks_npc_hp_but_accuracy_and_interval_do_not() {
    let gear = vec![None; 14];
    let p = 1.0 - 6850.0 / 13954.0;
    for (hp, max) in [(100, 29), (99, 29), (96, 30), (50, 43), (1, 57)] {
        let mut s = Settings::default();
        s.target_hp.insert(1673, hp);
        let r = calc(1673, &gear, &s);
        close(r.hit_chance, p);
        close(r.damage_per_second, p * f64::from(max) / 2.0 / 4.2);
        assert!(r.note.contains(&format!("0–{max}")));
        assert!(r.note.contains(&format!("{hp}/100 HP")));
    }
}
#[test]
fn dharok_matching_prayer_blocks_every_health_state() {
    for hp in [100, 50, 1] {
        let mut s = Settings {
            protection: "Melee".into(),
            ..Default::default()
        };
        s.target_hp.insert(1673, hp);
        close(calc(1673, &vec![None; 14], &s).damage_per_second, 0.0);
        s.protection = "Magic".into();
        assert!(calc(1673, &vec![None; 14], &s).damage_per_second > 0.0);
    }
}
#[test]
fn verac_has_an_accuracy_bypass_mixture_with_zero_damage_rolls() {
    let s = Settings::default();
    let p = 1.0 - 6850.0 / 13954.0;
    let r = calc(1677, &vec![None; 14], &s);
    close(r.hit_chance, 0.75 * p + 0.25);
    close(r.damage_per_second, (0.75 * p + 0.25) * 11.5 / 3.0);
    assert!(r.note.contains("0–23"));
}
#[test]
fn verac_protected_bypass_keeps_its_own_fifteen_maximum() {
    let mut s = Settings {
        protection: "Melee".into(),
        ..Default::default()
    };
    let empty = vec![None; 14];
    close(calc(1677, &empty, &s).damage_per_second, 0.625);
    let mut armour = empty.clone();
    for (slot, id) in [(0, 1163), (4, 1127), (7, 1079), (5, 1201)] {
        armour[slot] = Some(id);
    }
    close(calc(1677, &armour, &s).damage_per_second, 0.625);
    s.protection = "Magic".into();
    assert!(calc(1677, &empty, &s).damage_per_second > 0.625);
}
#[test]
fn verac_defensive_passives_and_damage_stat_drains_are_not_silently_guessed() {
    for id in [12817, 21015] {
        let mut gear = vec![None; 14];
        gear[if id == 12817 { 5 } else { 3 }] = Some(id);
        assert!(
            defence::estimate(
                &member(),
                &gear,
                planner::monster(1677).unwrap(),
                "Stab",
                &Settings::default()
            )
            .is_err()
        );
    }
    let mut s = Settings::default();
    s.reductions.insert(
        1673,
        Reductions {
            bgs: 400,
            ..Default::default()
        },
    );
    assert!(
        defence::estimate(
            &member(),
            &vec![None; 14],
            planner::monster(1673).unwrap(),
            "Slash",
            &s
        )
        .is_err()
    );
}
#[test]
fn npc_hp_does_not_borrow_the_players_dharok_or_torag_health_setting() {
    let gear = vec![None; 14];
    let full = calc(1673, &gear, &Settings::default());
    let s = Settings {
        current_hitpoints: Some(1),
        ..Default::default()
    };
    close(
        calc(1673, &gear, &s).damage_per_second,
        full.damage_per_second,
    );
    assert!(defence::profile(12317).is_none());
    assert!(defence::profile(12323).is_none());
}
