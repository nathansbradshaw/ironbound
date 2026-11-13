//! Independent arithmetic from pinned Wiki scaling, NPC and hit-transform rules.
use ironbound::{
    account::{self, Member},
    bestiary::{self, Readiness, RequirementStatus},
    combat, encounters,
    monster_rules::Transform,
    planner::{self, Options},
    raid_scaling::{self, Scale},
};
fn member() -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m
}
fn set(w: i32, a: Option<i32>) -> Vec<Option<i32>> {
    let mut s = vec![None; 14];
    s[3] = Some(w);
    s[13] = a;
    s
}
fn calc(id: i32, w: i32, a: Option<i32>, style: &str) -> combat::ResultStats {
    combat::calculate(
        &member(),
        &set(w, a),
        style,
        planner::monster(id).unwrap(),
        "",
    )
    .unwrap()
}
fn mean(r: &combat::ResultStats) -> f64 {
    r.hit_distribution
        .iter()
        .map(|(d, p)| f64::from(*d) * p)
        .sum()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn scale(kind: &str, party: u32, cm: bool) -> Scale {
    serde_json::from_value(serde_json::json!({"kind":kind,"party_size":party,"challenge":cm}))
        .unwrap()
}
fn window(source: i32, party: u32, cm: bool) -> i32 {
    encounters::mechanics()
        .windows
        .iter()
        .find(|w| {
            w.npc_id == source
                && w.raid_scale
                    .as_ref()
                    .is_some_and(|s| s.party_size == party && s.challenge == cm)
        })
        .unwrap()
        .id
}
#[test]
fn chambers_scaling_has_flooring_cm_exceptions_and_no_double_scaling() {
    let solo = planner::monster(7540).unwrap();
    assert_eq!(
        (solo.skills["hp"], solo.skills["atk"], solo.skills["def"]),
        (300, 390, 205)
    );
    // Three players: offensive floor390*109/100, defence floor205*102/100.
    let trio = planner::monster(window(7540, 3, false)).unwrap();
    assert_eq!(
        (trio.skills["hp"], trio.skills["atk"], trio.skills["def"]),
        (600, 425, 209)
    );
    let cm = planner::monster(window(7540, 3, true)).unwrap();
    assert_eq!(
        (cm.skills["hp"], cm.skills["atk"], cm.skills["def"]),
        (900, 637, 250)
    );
    let four = raid_scaling::apply(solo, 7540, &scale("cox", 4, true)).unwrap();
    assert_eq!(four.skills["def"], 284); // 211+floor211*35/100.
    let crystal = planner::monster(window(7568, 3, true)).unwrap();
    assert_eq!((crystal.skills["hp"], crystal.skills["def"]), (240, 102));
    assert_eq!(solo.skills["ranged"], 1);
    assert_eq!(cm.skills["ranged"], 1);
}
#[test]
fn olm_hp_and_mage_hand_are_scaled_once_from_original_stats() {
    for (party, cm, hand, head, magic) in [
        (1, false, 600, 800, 87),
        (3, false, 1200, 1600, 89),
        (1, true, 600, 800, 131),
        (3, true, 1200, 1600, 133),
    ] {
        let m = planner::monster(window(7550, party, cm)).unwrap();
        assert_eq!((m.skills["hp"], m.skills["magic"]), (hand, magic));
        assert_eq!(
            planner::monster(window(7551, party, cm)).unwrap().skills["hp"],
            head
        );
    }
    let base = planner::monster(7552).unwrap();
    let p8 = raid_scaling::apply(base, 7552, &scale("cox", 8, false)).unwrap();
    assert_eq!(p8.skills["hp"], 1800);
    let p100 = raid_scaling::apply(base, 7552, &scale("cox", 100, false)).unwrap();
    assert_eq!(p100.skills["hp"], 10200);
}
#[test]
fn low_party_skills_singles_and_guardian_hp_follow_their_own_rules() {
    let mut s = scale("cox", 1, false);
    s.highest_combat = 3;
    s.highest_hp = 1;
    s.average_mining = 1;
    let low = raid_scaling::apply(planner::monster(7540).unwrap(), 7540, &s).unwrap();
    assert_eq!(
        (low.skills["hp"], low.skills["atk"], low.skills["def"]),
        (142, 216, 113)
    );
    let g = raid_scaling::apply(planner::monster(7569).unwrap(), 7569, &s).unwrap();
    assert_eq!(g.skills["hp"], 72);
    let mut scav = planner::monster(7569).unwrap().clone();
    scav.skills.insert("hp".into(), 100);
    scav.skills.insert("atk".into(), 100);
    scav.skills.insert("str".into(), 100);
    s.challenge = true;
    let solo = raid_scaling::apply(&scav, 7548, &s).unwrap();
    assert_eq!((solo.skills["hp"], solo.skills["atk"]), (71, 82));
    assert!(raid_scaling::apply(&scav, 7548, &scale("cox", 101, false)).is_err());
}
#[test]
fn guardians_require_pickaxes_and_player_mining() {
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(7569).unwrap(),
            ""
        )
        .unwrap_err()
        .contains("pickaxe")
    );
    let mut m = member();
    m.level_overrides.remove("Mining");
    assert!(
        combat::calculate(
            &m,
            &set(1275, None),
            "Crush",
            planner::monster(7569).unwrap(),
            ""
        )
        .unwrap_err()
        .contains("Mining")
    );
    let r = calc(7569, 1275, None, "Crush");
    // Rune pickaxe: naked max16, Mining99 plus tier41 => floor(roll*190/150).
    assert_eq!(r.max_hit, 20);
    let total = (0..=16).map(|d| d.max(1) * 190 / 150).sum::<i32>();
    close(mean(&r), r.accuracy * f64::from(total) / 17.0);
}
#[test]
fn chambers_off_style_reductions_apply_to_each_splat() {
    close(
        Transform::DivideMinimum { divisor: 5 }.outcomes(1, 1.0)[0].0 as f64,
        1.0,
    );
    assert_eq!(calc(7540, 11905, None, "Magic").max_hit, 5);
    assert_eq!(calc(7568, 11905, None, "Magic").max_hit, 9);
    assert_eq!(calc(7552, 11905, None, "Magic").max_hit, 9);
    assert_eq!(calc(7550, 9185, Some(9144), "Ranged").max_hit, 10);
    assert_eq!(calc(7551, 9185, Some(9144), "Ranged").max_hit, 30);
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(7533).unwrap(),
            ""
        )
        .is_err()
    );
    assert!(
        combat::calculate(
            &member(),
            &set(9185, Some(9144)),
            "Ranged",
            planner::monster(7540).unwrap(),
            ""
        )
        .is_err()
    );
}
#[test]
fn theatre_party_hp_and_nylocas_colours_are_explicit() {
    assert_eq!(planner::monster(8360).unwrap().skills["hp"], 2625);
    assert_eq!(planner::monster(203100).unwrap().skills["hp"], 3500);
    assert_eq!(planner::monster(10813).unwrap().skills["hp"], 1800);
    assert_eq!(planner::monster(10772).unwrap().skills["hp"], 4500);
    let mut raw = planner::monster(8360).unwrap().clone();
    raw.skills.insert("hp".into(), 3500);
    for party in 1..=3 {
        assert_eq!(
            raid_scaling::apply(&raw, 8360, &scale("tob", party, false))
                .unwrap()
                .skills["hp"],
            2625
        );
    }
    for (p, hp) in [(1, 875), (2, 1662), (3, 2362), (4, 2975), (5, 3500)] {
        assert_eq!(
            raid_scaling::apply(&raw, 8360, &scale("tob_entry", p, false))
                .unwrap()
                .skills["hp"],
            hp
        );
    }
    assert_eq!(calc(203000, 9185, Some(9144), "Ranged").max_hit, 30);
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(203000).unwrap(),
            ""
        )
        .is_err()
    );
    assert_eq!(encounters::source_npc_id(203001), 8354);
    assert_eq!(planner::monster(8354).unwrap().skills["hp"], 1875);
}
#[test]
fn verzik_random_limiter_and_loaned_dawnbringer_have_distinct_distributions() {
    let r = calc(8369, 4151, None, "Slash");
    assert_eq!(r.max_hit, 10);
    // Raw whip max24: E[min(max(1,roll),uniform0..10)].
    let sum = (0..=24)
        .flat_map(|d| (0..=10).map(move |cap| d.max(1).min(cap)))
        .sum::<i32>();
    close(mean(&r), r.accuracy * f64::from(sum) / (25.0 * 11.0));
    let normal = calc(203200, 4151, None, "Magic");
    assert_eq!((normal.max_hit, normal.attack_ticks), (15, 5));
    close(normal.accuracy, 1.0);
    close(mean(&normal), 121.0 / 16.0);
    let special = calc(203201, 4151, None, "Magic");
    assert_eq!(special.max_hit, 150);
    close(special.accuracy, 1.0);
    close(mean(&special), 112.5);
    assert!(
        special
            .hit_distribution
            .iter()
            .all(|(d, _)| (75..=150).contains(d))
    );
    let mut boosted = set(4151, None);
    boosted[2] = Some(12002);
    let result = combat::calculate(
        &member(),
        &boosted,
        "Magic",
        planner::monster(203201).unwrap(),
        "Fire Surge",
    )
    .unwrap();
    close(mean(&result), 112.5);
    assert!(encounters::window(8369).unwrap().readiness_exempt);
}
#[test]
fn nex_mage_negative_armour_changes_accurate_hits_and_bolt_procs_only() {
    let r = calc(11283, 9185, Some(9144), "Ranged");
    assert_eq!(r.max_hit, 32);
    close(mean(&r), r.accuracy * (15.0 + 1.0 / 31.0 + 2.0));
    close(
        r.hit_distribution
            .iter()
            .filter(|(d, _)| *d == 0)
            .map(|(_, p)| p)
            .sum(),
        1.0 - r.accuracy,
    );
    let ruby = calc(11283, 9185, Some(9242), "Ranged");
    assert_eq!(ruby.max_hit, 102);
    assert_eq!(ruby.hp_dependent.unwrap().outcomes(1), vec![(2, 0.06)]);
    let diamond = calc(11283, 9185, Some(9243), "Ranged");
    assert_eq!(diamond.max_hit, 34); // raw max28 *1.15=>32, then+2.
    for ammo in [9236, 9238, 9244, 9245] {
        let r = calc(11283, 9185, Some(ammo), "Ranged");
        close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
        assert!(r.hit_distribution.iter().all(|(d, _)| *d == 0 || *d >= 2));
    }
    let protected = calc(200010, 9185, Some(9242), "Ranged");
    assert_eq!(protected.max_hit, 0);
    close(mean(&protected), 0.0);
}
#[test]
fn nex_phase_hp_curses_and_healing_do_not_claim_a_complete_fight() {
    for (id, hp) in [
        (200000, 3400),
        (200001, 2720),
        (200002, 2040),
        (200003, 1360),
        (200004, 1180),
    ] {
        let m = planner::monster(id).unwrap();
        assert_eq!(
            combat::current_target_hp(m, 0, &Default::default()).unwrap(),
            hp
        );
        assert_eq!(combat::target_hp_bounds(m, 0).unwrap(), (1, 3400));
    }
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(200005).unwrap(),
            ""
        )
        .is_err()
    );
    for id in [200006, 200007] {
        close(mean(&calc(id, 9185, Some(9242), "Ranged")), 0.0);
    }
    assert!(
        encounters::window(200006)
            .unwrap()
            .description
            .contains("heal")
    );
}
#[test]
fn archive_uses_recorded_defences_grounding_and_smorks_hp_armour_states() {
    let flying = planner::monster(16659).unwrap();
    let ground = planner::monster(205000).unwrap();
    assert_eq!(
        (flying.defensive["light"], ground.defensive["light"]),
        (55, 50)
    );
    assert_eq!(
        (flying.defensive["magic"], ground.defensive["magic"]),
        (130, 50)
    );
    assert!(ground.attributes.is_empty());
    assert!(combat::calculate(&member(), &set(4151, None), "Slash", flying, "").is_err());
    assert!(calc(205000, 4151, None, "Slash").dps > 0.0);
    for (id, a, res) in [
        (16640, 8, 0),
        (205010, 10, 2),
        (205011, 12, 4),
        (205012, 14, 6),
        (205013, 0, 0),
    ] {
        let m = planner::monster(id).unwrap();
        assert_eq!(
            (m.defensive["flat_armour"], m.defensive["magic_resistance"]),
            (a, res)
        );
        assert_eq!(
            combat::current_target_hp(m, 0, &Default::default()).unwrap(),
            500
        );
        assert_eq!(combat::target_hp_bounds(m, 0).unwrap(), (1, 30000));
    }
    assert_eq!(planner::monster(205013).unwrap().defensive["crush"], -90);
    let ruby = calc(16641, 9185, Some(9242), "Ranged");
    assert_eq!(ruby.hp_dependent.unwrap().outcomes(500), vec![(98, 0.06)]);
}
#[test]
fn ocellus_explosions_transform_accurate_splats_and_recompute_ruby_zeroes() {
    let normal = calc(205024, 9185, Some(9144), "Ranged");
    let fast = calc(205023, 9185, Some(9144), "Ranged");
    assert_eq!(normal.max_hit, 13);
    close(normal.expected_attack_ticks, 5.0);
    close(fast.expected_attack_ticks, 2.0);
    close(mean(&normal), mean(&fast));
    let expected = (0..=30)
        .map(|d| f64::from(d.max(1) * 2 / 5) + 0.5)
        .sum::<f64>()
        / 31.0;
    close(mean(&normal), normal.accuracy * expected);
    let ruby = calc(205023, 9185, Some(9242), "Ranged");
    let dep = ruby.hp_dependent.unwrap();
    assert_eq!(dep.outcomes(1), vec![(0, 0.03), (1, 0.03)]);
    assert_eq!(dep.outcomes(500), vec![(40, 0.03), (41, 0.03)]);
    close(
        Transform::OcellusPulse {
            ticks: 5,
            accurate_zero: false,
        }
        .outcomes(0, 1.0)[0]
            .1,
        1.0,
    );
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(205021).unwrap(),
            ""
        )
        .is_err()
    );
}
#[test]
fn expert_toa_has_fixed_300_hp_defence_and_retains_core_models() {
    let expert = encounters::groups()
        .iter()
        .find(|g| g.name == "Tombs of Amascut: Expert Mode")
        .unwrap();
    for t in &expert.targets {
        let w = encounters::window(t.id).unwrap();
        assert_eq!(w.raid_scale.as_ref().unwrap().invocation, 300);
        assert_eq!(w.raid_scale.as_ref().unwrap().path_level, 0);
        assert!(w.comparison_probe);
    }
    let akkha = expert
        .targets
        .iter()
        .find(|t| encounters::source_npc_id(t.id) == 11789 && t.style == "Magic")
        .unwrap();
    let r = calc(akkha.id, 11905, None, "Magic");
    assert_eq!(r.defence_roll, (100 + 9) * (10 + 64) * 550 / 250);
    assert_eq!(planner::monster(akkha.id).unwrap().skills["hp"], 880);
    assert!(
        combat::calculate(
            &member(),
            &set(4151, None),
            "Slash",
            planner::monster(akkha.id).unwrap(),
            ""
        )
        .is_err()
    );
    let core = expert
        .targets
        .iter()
        .find(|t| encounters::source_npc_id(t.id) == 11755)
        .unwrap();
    assert_eq!(planner::monster(core.id).unwrap().skills["hp"], 5850);
    assert_eq!(
        encounters::window(core.id).unwrap().damage_model,
        encounters::DamageModel::WardenCore
    );
    close(calc(core.id, 4151, None, "Melee").accuracy, 1.0);
}
#[test]
fn presets_preserve_room_scale_and_reject_mixed_party_context() {
    for name in [
        "Chambers of Xeric (3-player)",
        "Chambers of Xeric: Challenge Mode",
    ] {
        let entry = encounters::groups()
            .iter()
            .find(|g| g.name == name)
            .unwrap();
        let raid = ironbound::raids::for_targets(&entry.targets).unwrap();
        let without = raid.toggle(&entry.targets, "vanguards").unwrap();
        assert_eq!(raid.toggle(&without, "vanguards").unwrap(), entry.targets);
        let mut mix = entry.targets.clone();
        mix[0].id = 7540;
        assert!(ironbound::raids::for_targets(&mix).is_none());
    }
}
#[test]
fn raid_readiness_is_separate_from_access_and_missing_mining_is_explicit() {
    let mut bank = account::demo();
    for s in account::SKILLS {
        bank.members[0].level_overrides.insert(s.into(), 99);
    }
    let rows = bestiary::assess_catalog(&bank, &Options::default());
    for name in [
        "Nex",
        "Chambers of Xeric",
        "Chambers of Xeric (3-player)",
        "Chambers of Xeric: Challenge Mode",
        "Theatre of Blood",
        "Theatre of Blood (5-player)",
        "Theatre of Blood: Hard Mode",
        "The Fractured Archive",
        "Tombs of Amascut: Expert Mode",
    ] {
        let i = encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap();
        assert_ne!(
            rows[i].readiness,
            Readiness::Unknown,
            "{name}: {}",
            rows[i].reason
        );
        assert_eq!(rows[i].requirements.status, RequirementStatus::NoLevelGate);
        assert!(!rows[i].detail.is_empty());
    }
    bank.members[0].skills = None;
    bank.members[0].level_overrides.remove("Mining");
    let rows = bestiary::assess_catalog(&bank, &Options::default());
    let i = encounters::catalog()
        .iter()
        .position(|e| e.name == "Chambers of Xeric")
        .unwrap();
    assert_eq!(rows[i].readiness, Readiness::Unknown);
    assert!(rows[i].reason.contains("Mining"));
}
#[test]
fn phase_choices_keep_the_selected_scale_and_do_not_hide_nex_special_states() {
    let nex = encounters::groups()
        .iter()
        .find(|g| g.name == "Nex")
        .unwrap();
    let choices =
        encounters::comparison_targets(&nex.targets.iter().map(|t| t.id).collect::<Vec<_>>());
    for id in [11278, 200005, 200006, 200007, 200010] {
        assert!(choices.contains(&id));
    }
    let cox = encounters::groups()
        .iter()
        .find(|g| g.name == "Chambers of Xeric (3-player)")
        .unwrap();
    let choices =
        encounters::comparison_targets(&cox.targets.iter().map(|t| t.id).collect::<Vec<_>>());
    assert!(choices.contains(&window(7533, 3, false)));
    assert!(choices.contains(&window(7568, 3, false)));
    assert!(!choices.contains(&7533));
    assert!(choices.iter().all(|id| {
        encounters::window(*id)
            .unwrap()
            .raid_scale
            .as_ref()
            .is_some_and(|s| s.party_size == 3 && !s.challenge)
    }));
    let toa = encounters::comparison_targets(&[11789]);
    assert!(!toa.iter().any(|id| *id >= 206000));
}
#[test]
fn raid_probes_do_not_invent_incoming_or_whole_fight_workloads() {
    for id in [200000, 201005, 203000, 203201, 16640, 205023, 206000] {
        let t = planner::monster(id).unwrap();
        let options = Options::default();
        let kit = if id == 203201 {
            set(4151, None)
        } else {
            set(9185, Some(9144))
        };
        let style = if id == 203201 {
            "Magic"
        } else if id == 201005 || id == 206000 {
            "Melee"
        } else {
            "Ranged"
        };
        let kit = if style == "Melee" {
            set(4151, None)
        } else {
            kit
        };
        let result = combat::calculate(&member(), &kit, style, t, "");
        if let Ok(r) = result {
            assert!(
                ironbound::attack_budget::for_window(t, &r, &options, &kit)
                    .unwrap_err()
                    .contains("probe")
            );
        }
        assert!(
            ironbound::defence::estimate(&member(), &kit, t, "Magic", &Default::default()).is_err()
        );
    }
    let mut o = Options {
        boss: planner::Boss::Custom,
        targets: vec![planner::Target {
            id: 203201,
            style: "Magic".into(),
        }],
        ..Default::default()
    };
    o.combat.target_hp.insert(203201, 1000);
    let note = ironbound::coverage::export_note(&o);
    assert!(note.contains("1000") && note.contains("raid-loaned Dawnbringer"));
}
#[test]
fn ruby_and_multi_hit_weapons_keep_verzik_limiters_per_splat() {
    let r = calc(8369, 9185, Some(9242), "Ranged");
    assert_eq!(r.max_hit, 3);
    let ruby = r.hp_dependent.unwrap();
    assert_eq!(
        ruby.outcomes(1500),
        vec![(0, 0.015), (1, 0.015), (2, 0.015), (3, 0.015)]
    );
    let twin = calc(8369, 4747, None, "Crush");
    assert_eq!(twin.max_hit, 20);
    close(twin.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    let mage = calc(205012, 11905, None, "Magic");
    assert_eq!(mage.max_hit, 22); // trident28 minus six resistance.
    let physical = calc(16641, 9185, Some(9144), "Ranged");
    assert_eq!(physical.max_hit, 28);
    close(
        mean(&physical),
        physical.accuracy * (1..=28).sum::<i32>() as f64 / 31.0,
    );
}
#[test]
fn newer_special_rotation_models_do_not_implicitly_enable_unreviewed_raid_states(){
    for id in [11278,11283,200000,201005,203200,205023,206003] {
        let result=ironbound::special_attacks::evaluate(&member(),&set(34725,None),"Magic",planner::monster(id).unwrap(),0,&Default::default());
        assert!(result.unwrap_err().contains("not verified"),"{id}");
    }
}
