//! Independent goldens from the pinned calculator stages in SOURCES.md.
use ironbound::{
    account::{self, Member},
    attack_budget, combat, coverage,
    defence::{self, Settings},
    encounters, huey,
    modifiers::{self, Reductions},
    planner::{self, Boss, Options, Target},
    royal_titans::{self, Strategy},
};
fn member() -> Member {
    let mut m = Member::default();
    for skill in account::SKILLS {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}
fn gear(weapon: i32, ammo: Option<i32>) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(weapon);
    s[13] = ammo;
    s
}
fn calc(id: i32, set: &[Option<i32>], style: &str, spell: &str) -> combat::ResultStats {
    combat::calculate(&member(), set, style, planner::monster(id).unwrap(), spell).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn mean(r: &combat::ResultStats) -> f64 {
    r.hit_distribution
        .iter()
        .map(|(d, p)| f64::from(*d) * p)
        .sum()
}
#[test]
fn tail_crush_misses_deal_one_without_becoming_accurate() {
    let r = calc(14014, &gear(1434, None), "Crush", "");
    assert_eq!((r.attack_roll, r.defence_roll, r.max_hit), (13640, 8576, 9));
    let p = 1.0 - 8578.0 / 27282.0;
    close(r.accuracy, p);
    // Raw 0..20 rolls: accurate 0 becomes 1; 9..20 all become 9.
    close(r.dps, ((1.0 - p) + p * 145.0 / 21.0) / 2.4);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 1));
    let mut options = Options::default();
    options.combat.target_hp.insert(14014, 1);
    let b = attack_budget::for_window(
        planner::monster(14014).unwrap(),
        &r,
        &options,
        &gear(1434, None),
    )
    .unwrap();
    close(b.attacks, 1.0);
    close(b.seconds, 2.4);
}
#[test]
fn tail_requires_strict_crush_priority_and_a_crush_attack() {
    // Dharok's axe has 103 slash and only 95 crush, so choosing Crush isn't enough.
    let r = calc(14014, &gear(4718, None), "Crush", "");
    assert_eq!(r.max_hit, 4);
    assert!(r.hit_distribution.iter().any(|(d, p)| *d == 0 && *p > 0.0));
    let mut bonuses = combat::Bonuses::default();
    for (kind, n) in [("crush", 60), ("stab", 60), ("slash", 0)] {
        bonuses.offensive.insert(kind.into(), n);
    }
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "crush", &bonuses, "");
    assert_eq!(transform.outcomes(20, 1.0), vec![(4, 1.0)]);
    assert_eq!(transform.outcomes(0, 1.0), vec![(0, 1.0)]);
    bonuses.offensive.insert("stab".into(), 59);
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "crush", &bonuses, "");
    assert_eq!(transform.outcomes(0, 1.0), vec![(1, 1.0)]);
    let transform = ironbound::monster_rules::transform_for(14014, 1434, "stab", &bonuses, "");
    assert_eq!(transform.outcomes(20, 1.0), vec![(4, 1.0)]);
}
#[test]
fn earth_tail_hits_have_nine_cap_but_keep_misses_at_zero() {
    let s = gear(1381, None);
    let earth = calc(14014, &s, "Magic", "Earth Bolt");
    assert_eq!(
        (earth.attack_roll, earth.defence_roll, earth.max_hit),
        (12787, 15576, 9)
    );
    let p = 12787.0 / 31154.0;
    close(earth.accuracy, p);
    close(earth.dps, p * 136.0 / 20.0 / 3.0);
    let fire = calc(14014, &s, "Magic", "Fire Bolt");
    assert_eq!(fire.max_hit, 4);
    close(fire.dps, 7992.0 / 31154.0 * 43.0 / 13.0 / 3.0);
    let pillar = calc(100161, &s, "Magic", "Earth Bolt");
    assert_eq!(pillar.max_hit, 11);
    close(pillar.dps, p * 165.0 / 20.0 / 3.0);
    assert!(
        pillar
            .hit_distribution
            .iter()
            .any(|(d, p)| *d == 0 && *p > 0.0)
    );
}
#[test]
fn independent_hammers_keep_both_miss_minima_and_pillar_rounding() {
    let s = gear(4747, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (16390, 8576, 18)
    );
    let p = 1.0 - 8578.0 / 32782.0;
    // The 23 raw maximum splits into independent 0..11 and 0..12 rolls.
    close(mean(&r), 2.0 * (1.0 - p) + p * (64.0 / 12.0 + 73.0 / 13.0));
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 2));
    let pillar = calc(100161, &s, "Crush", "");
    assert_eq!(pillar.max_hit, 22);
    close(
        mean(&pillar),
        2.0 * (1.0 - p) + p * (77.0 / 12.0 + 88.0 / 13.0),
    );
}
#[test]
fn chained_macuahuitl_miss_minimum_does_not_create_a_second_hit() {
    let s = gear(28997, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (20350, 8576, 18)
    );
    let p = 1.0 - 8578.0 / 40702.0;
    let accurate = 73.0 / 13.0;
    close(
        mean(&r),
        (1.0 - p) + p * accurate + p * ((1.0 - p) + p * accurate),
    );
    close(
        r.hit_distribution
            .iter()
            .filter(|(d, _)| *d == 1)
            .map(|(_, p)| p)
            .sum(),
        1.0 - p,
    );
}
#[test]
fn scythe_caps_each_splat_and_does_not_borrow_crush_priority() {
    let s = gear(22325, None);
    let r = calc(14014, &s, "Crush", "");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (10058, 8576, 12)
    );
    let p = 1.0 - 8578.0 / 20118.0;
    close(mean(&r), p * (91.0 / 25.0 + 43.0 / 13.0 + 19.0 / 7.0));
    let pillar = calc(100161, &s, "Crush", "");
    assert_eq!(pillar.max_hit, 15);
    close(mean(&pillar), p * (112.0 / 25.0 + 52.0 / 13.0 + 22.0 / 7.0));
}
#[test]
fn pillar_applies_after_tail_caps_and_after_each_ruby_proc() {
    let s = gear(9185, Some(9242));
    let r = calc(14014, &s, "Ranged", "");
    assert_eq!(r.max_hit, 4);
    assert_eq!(r.hp_dependent.unwrap().outcomes(300), vec![(4, 0.06)]);
    let p = calc(100161, &s, "Ranged", "");
    assert_eq!(p.max_hit, 5);
    let ruby = p.hp_dependent.unwrap();
    assert_eq!(ruby.outcomes(300), vec![(5, 0.06)]);
    assert_eq!(ruby.outcomes(10), vec![(2, 0.06)]);
    let head = calc(100160, &s, "Ranged", "");
    assert_eq!(head.hp_dependent.unwrap().outcomes(2500), vec![(130, 0.06)]);
    let body = calc(14017, &s, "Ranged", "");
    assert_eq!(body.hp_dependent.unwrap().outcomes(250), vec![(50, 0.06)]);
}
#[test]
fn protected_head_broken_tail_and_unverified_incoming_stay_separate() {
    let shield = calc(14011, &gear(9185, Some(9242)), "Ranged", "");
    assert_eq!((shield.dps, shield.max_hit), (0.0, 0));
    assert!(
        encounters::limitation(planner::monster(14015).unwrap())
            .unwrap()
            .contains("broken tail")
    );
    for id in [
        14009, 14014, 14017, 14011, 14015, 100160, 100161, 12596, 14147, 100162, 100163, 14150,
        14151,
    ] {
        assert!(
            defence::estimate(
                &member(),
                &gear(1434, None),
                planner::monster(id).unwrap(),
                "Magic",
                &Settings::default()
            )
            .is_err(),
            "{id}"
        );
    }
    for id in [14009, 100160, 14014, 100161, 14017] {
        let reduced = modifiers::reduce(
            planner::monster(id).unwrap(),
            &Reductions {
                bgs: 999,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], 120);
    }
}
#[test]
fn withdrawn_titans_multiply_ranged_attack_rolls_not_damage() {
    let s = gear(9185, Some(9144));
    for (close_id, far_id) in [(12596, 100162), (14147, 100163)] {
        let ordinary = calc(close_id, &s, "Ranged", "");
        let far = calc(far_id, &s, "Ranged", "");
        assert_eq!(
            (
                ordinary.attack_roll,
                far.attack_roll,
                far.defence_roll,
                far.max_hit
            ),
            (16478, 98868, 83276, 30)
        );
        close(ordinary.accuracy, 16478.0 / 166554.0);
        close(far.accuracy, 1.0 - 83278.0 / 197738.0);
        close(far.dps, far.accuracy * 466.0 / 31.0 / 3.0);
        assert_eq!(ordinary.max_hit, far.max_hit);
        assert!(
            combat::calculate(
                &member(),
                &gear(1434, None),
                "Crush",
                planner::monster(far_id).unwrap(),
                ""
            )
            .is_err()
        );
        assert_eq!(
            calc(far_id, &gear(1381, None), "Magic", "Fire Bolt").attack_roll,
            calc(close_id, &gear(1381, None), "Magic", "Fire Bolt").attack_roll
        );
    }
}
#[test]
fn titan_elemental_magic_accuracy_uses_bonus_not_levels_or_weakness() {
    let s = gear(1381, None);
    for id in [14150, 14151] {
        let r = calc(id, &s, "Magic", "Fire Bolt");
        close(r.accuracy, 0.4);
        let mut low = member();
        low.level_overrides.insert("Magic".into(), 13);
        close(
            combat::calculate(
                &low,
                &s,
                "Magic",
                planner::monster(id).unwrap(),
                "Fire Strike",
            )
            .unwrap()
            .accuracy,
            0.4,
        );
        let mut negative = s.clone();
        negative[4] = Some(10551);
        close(calc(id, &negative, "Magic", "Fire Bolt").accuracy, 0.3);
        let mut high = s.clone();
        for (slot, item) in [(1, 2412), (2, 12002), (4, 4091), (7, 4093), (10, 4097)] {
            high[slot] = Some(item);
        }
        assert_eq!(combat::bonuses(&high).offensive["magic"], 70);
        close(calc(id, &high, "Magic", "Fire Bolt").accuracy, 1.0);
        let mut reroll = s.clone();
        reroll[9] = Some(31106);
        close(calc(id, &reroll, "Magic", "Fire Bolt").accuracy, 0.6);
    }
    let fire = calc(14150, &s, "Magic", "Water Bolt");
    let ice = calc(14151, &s, "Magic", "Fire Bolt");
    assert_eq!((fire.max_hit, ice.max_hit), (24, 24));
    close(fire.dps, 0.4 * 24.0 / 3.0);
    for result in [fire, ice] {
        assert!(
            result
                .hit_distribution
                .iter()
                .all(|(damage, _)| [0, 24].contains(damage))
        );
        close(
            result
                .hit_distribution
                .iter()
                .filter(|(damage, _)| *damage == 0)
                .map(|(_, p)| p)
                .sum(),
            0.6,
        );
    }
}

#[test]
fn titan_waves_one_shot_only_a_matching_elemental_when_they_land() {
    for (id, matching, other) in [
        (14150, "Water Wave", "Fire Wave"),
        (14151, "Fire Wave", "Water Wave"),
    ] {
        let s = gear(1381, None);
        let correct = calc(id, &s, "Magic", matching);
        assert_eq!(correct.max_hit, 40);
        close(correct.accuracy, 0.4);
        close(attack_budget::for_hp(&correct, 40, 0).unwrap().attacks, 2.5);
        let wrong = calc(id, &s, "Magic", other);
        close(wrong.accuracy, 0.4);
        assert!(
            wrong
                .hit_distribution
                .iter()
                .any(|(d, p)| *d > 0 && *d < wrong.max_hit && *p > 0.0)
        );
        let powered = calc(id, &gear(11907, None), "Magic", "");
        assert!(
            powered
                .hit_distribution
                .iter()
                .any(|(d, p)| *d > 0 && *d < powered.max_hit && *p > 0.0)
        );
        let twinflame = calc(id, &gear(30634, None), "Magic", matching);
        // Twinflame adds 10% of the base 20, then echoes 40% of the resulting 42.
        assert_eq!(twinflame.max_hit, 58);
        assert!(
            twinflame
                .hit_distribution
                .iter()
                .all(|(d, _)| [0, 58].contains(d))
        );
    }
}

fn titans_trip() -> (account::Snapshot, Options) {
    let mut snapshot = account::demo();
    snapshot.members[0].bank = Some(vec![
        1434, 1, 9185, 1, 9142, 200, 1381, 1, 11907, 1, 565, 20, 554, 100, 555, 100, 8007, 5,
    ]);
    let options = Options {
        boss: Boss::Custom,
        targets: encounters::catalog()
            .iter()
            .find(|entry| entry.name == royal_titans::NAME)
            .unwrap()
            .targets
            .clone(),
        combat: Settings {
            spell_casts: 10,
            ..Default::default()
        },
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    (snapshot, options)
}

#[test]
fn titans_plan_both_elements_and_require_each_for_export() {
    let (mut snapshot, mut options) = titans_trip();
    for recommended in [true, false] {
        options.recommended = Some(recommended);
        let plan = planner::plan(&snapshot, &options);
        assert!(plan.exportable(), "{:?}", plan.missing);
        assert_eq!(plan.spell, "Fire Wave / Water Wave");
        assert!(
            plan.phases
                .iter()
                .filter(|phase| phase.style == "Magic")
                .all(|phase| phase.equipment[3] == Some(1381))
        );
        for (id, quantity) in [(554, 70), (555, 70), (565, 10)] {
            assert!(
                plan.inventory
                    .iter()
                    .any(|p| p.id == id && p.quantity == quantity)
            );
        }
    }

    snapshot.members[0].bank.as_mut().unwrap()[15] = 0;
    let plan = planner::plan(&snapshot, &options);
    assert!(!plan.exportable());
    assert_eq!(plan.spell, "Fire Wave");
    assert!(
        plan.missing
            .iter()
            .any(|message| message.contains("Water spell"))
    );
    assert!(
        !plan
            .missing
            .iter()
            .any(|message| message.contains("Fire spell"))
    );

    options.pin_gear("Magic:3".into(), 11907, true);
    let plan = planner::plan(&snapshot, &options);
    assert!(!plan.exportable());
    assert!(
        plan.missing
            .iter()
            .any(|message| message.contains("Water spell"))
    );
    assert!(
        plan.missing
            .iter()
            .any(|message| message.contains("Fire spell"))
    );
}

#[test]
fn titans_duo_roles_pack_only_assigned_spells_and_survive_sharing() {
    let (snapshot, mut options) = titans_trip();
    for (strategy, spell, required_runes) in [
        (Strategy::SoloBoth, "Fire Wave / Water Wave", vec![554, 555]),
        (Strategy::DuoBoth, "Fire Wave / Water Wave", vec![554, 555]),
        (Strategy::DuoFire, "Fire Wave", vec![554]),
        (Strategy::DuoWater, "Water Wave", vec![555]),
        (Strategy::DuoPartner, "", vec![]),
        (Strategy::SoloSkip, "", vec![]),
        (Strategy::DuoSkip, "", vec![]),
    ] {
        strategy.apply(&mut options);
        let plan = planner::plan(&snapshot, &options);
        assert!(plan.exportable(), "{strategy:?} {:?}", plan.missing);
        assert_eq!(plan.spell, spell);
        let runes = plan
            .inventory
            .iter()
            .filter(|p| [554, 555].contains(&p.id))
            .map(|p| p.id)
            .collect::<Vec<_>>();
        assert_eq!(runes, required_runes);
        assert_eq!(plan.phases.len(), 4 + required_runes.len());
        let setup = ironbound::share::Setup::from_plan(&plan, &options);
        let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
            .unwrap()
            .options();
        assert_eq!(restored.royal_titans, strategy);
        assert_eq!(restored.targets, options.targets);
        assert!(
            ironbound::export::bank_tags(&plan, &options)
                .unwrap()
                .contains(royal_titans::NAME)
        );
    }
}

#[test]
fn titans_twinflame_provides_both_elements_but_a_pinned_spell_covers_only_its_target() {
    let (mut snapshot, mut options) = titans_trip();
    let bank = snapshot.members[0].bank.as_mut().unwrap();
    bank[6] = 30634;
    bank[13] = 0;
    bank[15] = 0;
    bank.extend([556, 100]);
    let plan = planner::plan(&snapshot, &options);
    assert_eq!(plan.spell, "Fire Wave / Water Wave");
    assert!(plan.exportable(), "{:?}", plan.missing);
    assert!(!plan.inventory.iter().any(|p| [554, 555].contains(&p.id)));

    options.combat.spell_selection = ironbound::rules::SpellSelection::Spell("Water Wave".into());
    let plan = planner::plan(&snapshot, &options);
    assert!(!plan.exportable());
    assert_eq!(plan.spell, "Water Wave");
    assert!(
        plan.missing
            .iter()
            .any(|message| message.contains("Fire spell") && message.contains("Auto"))
    );
}
#[test]
fn elemental_void_accuracy_applies_once_and_requires_the_complete_set() {
    for (body, legs) in [(8839, 8840), (13072, 13073)] {
        let mut s = gear(1381, None);
        for (slot, item) in [(0, 11663), (4, body), (7, legs), (9, 8842)] {
            s[slot] = Some(item);
        }
        close(calc(14150, &s, "Magic", "Water Bolt").accuracy, 0.58);
        s[9] = None;
        close(calc(14150, &s, "Magic", "Water Bolt").accuracy, 0.4);
    }
    // A powered Shadow's intrinsic x3 equipment bonus enters the same formula.
    let s = gear(27275, None);
    close(calc(14151, &s, "Magic", "").accuracy, 1.0);
}
#[test]
fn new_windows_preserve_shared_pools_bounds_and_unsupported_burn_gates() {
    let entries = encounters::catalog();
    let titans = entries
        .iter()
        .find(|e| e.name == "The Royal Titans")
        .unwrap();
    assert_eq!(titans.targets.len(), 6);
    let labels: std::collections::BTreeSet<_> = titans
        .targets
        .iter()
        .map(|t| encounters::window(t.id).unwrap().label.as_str())
        .collect();
    assert_eq!(
        labels.len(),
        6,
        "Each Titan preview identifies its actual target"
    );
    let o = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 12596,
                style: "Ranged".into(),
            },
            Target {
                id: 100162,
                style: "Ranged".into(),
            },
        ],
        ..Default::default()
    };
    assert!(coverage::windows(&o).iter().all(|w| w.supported));
    assert_eq!(
        encounters::window(12596).unwrap().progress_group,
        encounters::window(100162).unwrap().progress_group
    );
    assert_eq!(
        combat::target_hp_bounds(planner::monster(100161).unwrap(), 0).unwrap(),
        (1, 300)
    );
    let mut shared_options = Options {
        boss: Boss::Custom,
        targets: vec![
            Target {
                id: 100161,
                style: "Crush".into(),
            },
            Target {
                id: 100162,
                style: "Ranged".into(),
            },
            Target {
                id: 14150,
                style: "Magic".into(),
            },
        ],
        ..Default::default()
    };
    shared_options.combat.target_hp.insert(100161, 150);
    let snapshot = account::demo();
    let plan = planner::plan(&snapshot, &shared_options);
    let setup = ironbound::share::Setup::from_plan(&plan, &shared_options);
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
        .unwrap()
        .options();
    assert_eq!(restored.targets, shared_options.targets);
    assert_eq!(restored.combat.target_hp[&100161], 150);
    let exported = coverage::export_note(&restored);
    assert!(
        exported.contains("pillars lit") && exported.contains("150 HP"),
        "{exported}"
    );
    let mut s = gear(29000, Some(28991));
    for (slot, item) in [(0, 29055), (4, 29049), (7, 29052)] {
        s[slot] = Some(item);
    }
    for id in [14014, 100161, 100160] {
        assert!(
            combat::calculate(&member(), &s, "Ranged", planner::monster(id).unwrap(), "")
                .unwrap_err()
                .contains("Eclipse burn")
        );
    }
}

fn huey_trip() -> (account::Snapshot, Options) {
    let mut snapshot = account::demo();
    snapshot.members[0].level_overrides = member().level_overrides;
    snapshot.members[0].bank = Some(vec![22978, 1, 29889, 1, 8007, 2]);
    let options = Options {
        boss: Boss::Custom,
        targets: encounters::catalog()
            .iter()
            .find(|entry| entry.name == huey::NAME)
            .unwrap()
            .targets
            .clone(),
        food: 0,
        prayer_potions: 0,
        ..Default::default()
    };
    (snapshot, options)
}

#[test]
fn huey_team_sizes_gate_pillars_and_loot_without_scaling_hp() {
    let (_, mut options) = huey_trip();
    let sizes = [
        (1, None),
        (2, None),
        (3, Some(1)),
        (5, Some(1)),
        (6, Some(2)),
        (8, Some(2)),
        (9, Some(3)),
        (11, Some(3)),
        (12, Some(4)),
        (14, Some(4)),
        (15, Some(5)),
        (17, Some(5)),
        (18, Some(6)),
        (20, Some(6)),
    ];
    for (players, per_pillar) in sizes {
        let mut settings = huey::Settings::default().with_players(players);
        assert_eq!(settings.per_pillar(), per_pillar);
        assert_eq!(settings.loot_rule().contains("95%"), players >= 6);
        settings.pillars_lit = per_pillar.is_some();
        assert!(settings.valid());
        settings.apply(&mut options);
        assert_eq!(
            options.targets.iter().map(|t| t.id).collect::<Vec<_>>(),
            if players < 3 {
                vec![huey::BODY, huey::HEAD, huey::TAIL]
            } else {
                vec![huey::BODY, huey::LIT_HEAD, huey::LIT_TAIL]
            }
        );
        let hp: f64 = options
            .targets
            .iter()
            .map(|t| {
                encounters::progress_hp(t.id, &t.style).unwrap_or_else(|| {
                    f64::from(combat::target_hitpoints(planner::monster(t.id).unwrap(), 0).unwrap())
                })
            })
            .sum();
        close(hp, 4050.0);
    }
    assert!(!huey::Settings::default().with_players(21).valid());
    assert!(
        !huey::Settings {
            players: 2,
            pillars_lit: true,
            ..Default::default()
        }
        .valid()
    );
    assert!(
        !huey::Settings {
            players: 3,
            pillars_lit: true,
            ..Default::default()
        }
        .with_players(6)
        .pillars_lit
    );
}

#[test]
fn huey_tail_uses_its_own_packed_set_and_respects_switch_limit() {
    let (snapshot, mut options) = huey_trip();
    for lit in [false, true] {
        huey::Settings {
            players: 3,
            pillars_lit: lit,
            ..Default::default()
        }
        .apply(&mut options);
        options.max_switches = Some(1);
        let plan = planner::plan(&snapshot, &options);
        assert!(plan.exportable(), "{:?}", plan.missing);
        assert_eq!(plan.equipment[3], Some(22978));
        assert_eq!(plan.phases[2].equipment[3], Some(29889));
        assert_eq!(plan.optimization.as_ref().unwrap().max_changes, 1);
        assert_eq!(plan.optimization.as_ref().unwrap().gear_slots, 1);
        assert!(
            plan.inventory
                .iter()
                .any(|item| item.id == 29889 && item.purpose == "Tail switch")
        );
        assert_eq!(
            plan.preview_inventory(Some(2))
                .iter()
                .filter(|item| item.id == 22978)
                .count(),
            1
        );
        options.max_switches = Some(0);
        let capped = planner::plan(&snapshot, &options);
        assert!(capped.exportable(), "{:?}", capped.missing);
        assert_eq!(capped.equipment, capped.phases[2].equipment);

        let comparison = ironbound::tradeoffs::compare(&snapshot, &options, 0, 1);
        let overview = ironbound::encounter_summary::compare(&comparison);
        assert_eq!(
            comparison.right.windows.last().unwrap().equipment[3],
            Some(29889)
        );
        assert!(overview.complete(), "{:?}", overview.omissions);
        close(
            overview.workloads.iter().map(|window| window.hp).sum(),
            4050.0,
        );
    }
}

#[test]
fn huey_prayer_role_is_summit_only_and_shares_with_lit_targets() {
    let (snapshot, mut options) = huey_trip();
    let settings = huey::Settings {
        players: 3,
        role: huey::Role::Melee,
        pillars_lit: true,
    };
    settings.apply(&mut options);
    let plan = planner::plan(&snapshot, &options);
    let setup = ironbound::share::Setup::from_plan(&plan, &options);
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
        .unwrap()
        .options();
    assert_eq!(restored.huey, settings);
    assert_eq!(restored.targets, options.targets);
    let body =
        ironbound::tradeoffs::measure(&snapshot, &restored, &plan.equipment, "Crush", huey::BODY);
    let head = ironbound::tradeoffs::measure(
        &snapshot,
        &restored,
        &plan.equipment,
        "Crush",
        huey::LIT_HEAD,
    );
    close(body.prayer_per_minute, 0.0);
    close(head.prayer_per_minute, 20.0);
    let segment = combat::calculate(
        &member(),
        &plan.equipment,
        "Crush",
        planner::monster(huey::BODY).unwrap(),
        "",
    )
    .unwrap();
    let budget = attack_budget::for_window(
        planner::monster(huey::BODY).unwrap(),
        &segment,
        &restored,
        &plan.equipment,
    )
    .unwrap();
    assert_eq!(budget.entities, 5);
    close(
        budget.attacks,
        5.0 * attack_budget::for_hp(&segment, 250, 0).unwrap().attacks,
    );
}

#[test]
fn blood_moon_cooldown_uses_accuracy_even_when_tail_misses_deal_damage() {
    let mut s = gear(28997, None);
    for (slot, item) in [(0, 29073), (4, 29067), (7, 29070)] {
        s[slot] = Some(item);
    }
    let r = calc(14014, &s, "Crush", "");
    let p = r.accuracy;
    assert!(p > 0.0 && p < 1.0);
    assert!(r.hit_distribution.iter().all(|(d, _)| *d >= 1));
    close(
        r.expected_attack_ticks,
        4.0 - 0.33 * p - 0.33 * 0.67 * p * p,
    );
    close(r.dps, mean(&r) / (0.6 * r.expected_attack_ticks));
}
#[test]
fn blue_moon_additions_keep_their_own_accuracy_and_npc_transforms() {
    let mut s = gear(28988, None);
    for (slot, item) in [(0, 29019), (4, 29013), (7, 29016)] {
        s[slot] = Some(item);
    }
    for id in [14014, 100161, 14150] {
        let target = planner::monster(id).unwrap();
        let mut settings = Settings {
            blue_moon_melee: "Stab".into(),
            ..Default::default()
        };
        let melee =
            combat::calculate_configured(&member(), &s, "Stab", target, "", 0, &settings).unwrap();
        let on = combat::calculate_configured(
            &member(),
            &s,
            "Magic",
            target,
            "Undead Grasp",
            0,
            &settings,
        )
        .unwrap();
        settings.blue_moon_melee = "Off".into();
        let off = combat::calculate_configured(
            &member(),
            &s,
            "Magic",
            target,
            "Undead Grasp",
            0,
            &settings,
        )
        .unwrap();
        close(on.dps, off.dps + off.accuracy * 0.5 * melee.dps);
        assert_eq!(on.max_hit, off.max_hit + melee.max_hit);
    }
}
