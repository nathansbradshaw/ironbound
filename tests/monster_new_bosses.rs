//! Independent arithmetic for pinned Wiki NPC, HitDist and player-attack rules.
use ironbound::{
    account::{self, Member},
    bestiary::{self, Readiness, RequirementStatus},
    combat, encounters,
    modifiers::{self, Reductions},
    monster_rules::{self, Transform},
    planner::{self, Options},
};
fn member() -> Member {
    let mut m = Member::default();
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m
}
fn gear(weapon: i32, ammo: Option<i32>) -> Vec<Option<i32>> {
    let mut set = vec![None; 14];
    set[3] = Some(weapon);
    set[13] = ammo;
    set
}
fn try_calc(
    id: i32,
    weapon: i32,
    ammo: Option<i32>,
    style: &str,
    spell: &str,
) -> Result<combat::ResultStats, String> {
    combat::calculate(
        &member(),
        &gear(weapon, ammo),
        style,
        planner::monster(id).unwrap(),
        spell,
    )
}
fn calc(id: i32, weapon: i32, ammo: Option<i32>, style: &str) -> combat::ResultStats {
    try_calc(id, weapon, ammo, style, "").unwrap()
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
fn accuracy(a: i64, d: i64) -> f64 {
    if a > d {
        1.0 - (d + 2) as f64 / (2 * a + 2) as f64
    } else {
        a as f64 / (2 * d + 2) as f64
    }
}
#[test]
fn doom_delves_keep_exact_hp_and_shared_source_defences() {
    for (i, hp) in [525, 550, 575, 600, 625, 650, 650, 675]
        .into_iter()
        .enumerate()
    {
        let m = planner::monster(100170 + i as i32).unwrap();
        assert_eq!(m.skills["hp"], hp);
        assert_eq!(m.skills["def"], 90);
        assert_eq!(encounters::source_npc_id(m.id), 14707);
        let r = calc(m.id, 9185, Some(9242), "Ranged");
        assert_eq!(r.max_hit, 100);
        let reduced = modifiers::reduce(
            m,
            &Reductions {
                bgs: 1000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reduced.skills["def"], 60);
    }
    assert_eq!(planner::monster(14707).unwrap().skills["hp"], 625);
}
#[test]
fn doom_shield_is_a_separate_guaranteed_demonbane_counter() {
    let normal = calc(14707, 19675, None, "Slash");
    let shield = calc(100178, 19675, None, "Slash");
    assert!(normal.accuracy < 1.0);
    close(shield.accuracy, 1.0);
    assert_eq!(normal.max_hit, shield.max_hit);
    assert_eq!(planner::monster(100178).unwrap().skills["hp"], 500);
    assert_eq!(
        encounters::window(100178).unwrap().resource,
        encounters::Resource::Shield
    );
    assert!(
        try_calc(100178, 4151, None, "Slash", "")
            .unwrap_err()
            .contains("demonbane")
    );
    assert!(try_calc(100178, 9185, Some(9242), "Ranged", "").is_err());
    assert!(try_calc(100178, 13913, None, "Magic", "").is_err()); // unknown/non-demonbane staff cannot bypass
    let spell = try_calc(100178, 29594, None, "Magic", "Dark Demonbane").unwrap();
    close(spell.accuracy, 1.0);
    let burrow = calc(100179, 9185, Some(9144), "Ranged");
    close(burrow.accuracy, 1.0);
}
#[test]
fn doom_punish_adds_visible_strength_bonus_per_splat() {
    // Naked dragon mace: +55 equipment Strength => +11, regardless of prayers.
    let r = calc(100180, 1434, None, "Crush");
    assert_eq!(r.max_hit, 31);
    close(r.accuracy, 1.0);
    // Accurate zero becomes 1 in ordinary attacks: (1+1+2+...+20)/21.
    close(mean(&r), 211.0 / 21.0 + 11.0);
    let twin = calc(100180, 4747, None, "Crush");
    // Torag's +72 Strength -> two +14 bonuses, after split 11/12 maxima.
    assert_eq!(twin.max_hit, 51);
    close(twin.accuracy, 1.0);
    assert!(twin.hit_distribution.iter().all(|(d, _)| *d >= 30));
    assert!(try_calc(100180, 9185, Some(9144), "Ranged", "").is_err());
}
#[test]
fn maggot_reach_and_static_defence_states_are_distinct() {
    assert!(
        try_calc(15742, 1434, None, "Crush", "")
            .unwrap_err()
            .contains("out of melee reach")
    );
    let near = calc(100183, 1434, None, "Crush");
    assert_eq!(near.defence_roll, 209 * 109);
    let roar = calc(100184, 9185, Some(9144), "Ranged");
    assert_eq!(roar.defence_roll, 109 * 107);
    let far = try_calc(15742, 1381, None, "Magic", "Wind Bolt").unwrap();
    let near = try_calc(100183, 1381, None, "Magic", "Wind Bolt").unwrap();
    assert_eq!(far.defence_roll, 209 * 214);
    assert_eq!(near.defence_roll, 209 * 464);
    assert!(near.accuracy < far.accuracy);
    assert!(
        try_calc(15742, 1387, None, "Magic", "Fire Bolt")
            .unwrap_err()
            .contains("stacking fire burn")
    );
}
#[test]
fn maggot_punish_scales_rolled_damage_and_first_accuracy_only() {
    let r = calc(100185, 1434, None, "Crush");
    assert_eq!(
        (r.attack_roll, r.defence_roll, r.max_hit),
        (13640, 22781, 30)
    );
    let p = accuracy(13640, 3417);
    close(r.accuracy, p);
    // +50% is applied after rolling, with a floor on each outcome.
    close(mean(&r), p * 311.0 / 21.0);
    let twin = calc(100185, 4747, None, "Crush");
    assert_eq!(twin.max_hit, 28);
    let first = accuracy(twin.attack_roll, 3417);
    let second = accuracy(twin.attack_roll, 22781);
    let first_mean = (1..=11).map(|d| (d * 3 / 2) as f64).sum::<f64>() / 12.0 + 1.0 / 12.0;
    close(mean(&twin), first * first_mean + second * 79.0 / 13.0);
    close(twin.accuracy, 1.0 - (1.0 - first) * (1.0 - second));
    assert!(try_calc(100185, 4151, None, "Slash", "").is_err());
}
#[test]
fn angel_cleave_clamps_low_rolls_and_lightning_maximises_only_first_splat() {
    let cleave = calc(100181, 1434, None, "Crush");
    let perfect = calc(100182, 1434, None, "Crush");
    assert_eq!(cleave.max_hit, 20);
    close(cleave.accuracy, 1.0);
    close(mean(&cleave), 265.0 / 21.0);
    assert!(perfect.hit_distribution.iter().all(|(d, _)| *d == 20));
    close(mean(&perfect), 20.0);
    let twin = calc(100182, 4747, None, "Crush");
    let p = accuracy(twin.attack_roll, 184 * 104);
    close(mean(&twin), 11.0 + p * 79.0 / 13.0);
    assert!(
        twin.hit_distribution
            .iter()
            .any(|(d, p)| *d == 11 && *p > 0.0)
    );
    let cleave_twin = calc(100181, 4747, None, "Crush");
    close(mean(&cleave_twin), mean(&twin));
}
#[test]
fn angel_scythe_followups_keep_normal_accuracy() {
    let perfect = calc(100182, 22325, None, "Slash");
    assert_eq!(perfect.max_hit, 39);
    let p = accuracy(perfect.attack_roll, 184 * 144);
    close(mean(&perfect), 23.0 + p * (67.0 / 12.0 + 16.0 / 6.0));
    let cleave = calc(100181, 22325, None, "Slash");
    close(mean(&cleave), 466.0 / 24.0 + p * (67.0 / 12.0 + 16.0 / 6.0));
}
#[test]
fn leafy_mobs_require_matching_weapon_spell_or_ammunition() {
    for id in [410, 411, 427, 428, 429, 430, 7405, 10397] {
        assert!(
            try_calc(id, 4151, None, "Slash", "")
                .unwrap_err()
                .contains("leafy monster")
        );
        assert!(try_calc(id, 9185, Some(9144), "Ranged", "").is_err());
        assert!(try_calc(id, 9185, Some(11875), "Ranged", "").is_ok());
        assert!(try_calc(id, 20727, None, "Slash", "").is_ok());
        assert!(try_calc(id, 1381, None, "Magic", "Wind Bolt").is_err());
    }
    assert!(monster_rules::leafy_allowed(
        410,
        4170,
        None,
        "Magic",
        "Magic Dart"
    ));
    assert!(!monster_rules::leafy_allowed(
        410,
        20727,
        None,
        "Magic",
        "Wind Bolt"
    ));
}
#[test]
fn magic_dart_has_real_runes_skill_gates_and_explicit_enchanted_task_bonus() {
    let normal = try_calc(410, 4170, None, "Magic", "Magic Dart").unwrap();
    assert_eq!((normal.attack_roll, normal.max_hit), (108 * 76, 19));
    let enchanted = try_calc(410, 21255, None, "Magic", "Magic Dart").unwrap();
    assert_eq!(enchanted.max_hit, 19);
    let mut settings = ironbound::defence::Settings::default();
    ironbound::slayer::set_task(&mut settings, 410, true);
    let task = combat::calculate_configured(
        &member(),
        &gear(21255, None),
        "Magic",
        planner::monster(410).unwrap(),
        "Magic Dart",
        0,
        &settings,
    )
    .unwrap();
    assert_eq!(task.max_hit, 29); // floor(99/6)+13, without a Slayer helmet.
    let mut low = member();
    low.level_overrides.insert("Slayer".into(), 54);
    assert!(
        combat::calculate(
            &low,
            &gear(4170, None),
            "Magic",
            planner::monster(410).unwrap(),
            "Magic Dart"
        )
        .is_err()
    );
    let recipe = ironbound::rules::spells()
        .iter()
        .find(|r| r.name == "Magic Dart")
        .unwrap();
    assert_eq!(recipe.runes[&560], 1);
    assert_eq!(recipe.runes[&558], 4);
    assert_eq!(ironbound::rules::spell_max_hit("Magic Dart", 110), Some(21));
}
#[test]
fn leafy_readiness_finds_owned_required_weapons_and_keeps_slayer_separate() {
    let mut a = account::demo();
    let m = &mut a.members[0];
    for s in account::SKILLS {
        m.level_overrides.insert(s.into(), 99);
    }
    m.level_overrides.insert("Slayer".into(), 60);
    m.bank = Some(vec![
        20727, 1, 4151, 1, 1127, 1, 1079, 1, 10828, 1, 4131, 1, 7462, 1,
    ]);
    m.inventory = None;
    m.equipment = None;
    m.shared_bank = None;
    let i = encounters::catalog()
        .iter()
        .position(|e| e.name == "Kurasks")
        .unwrap();
    let first = bestiary::assess_catalog(&a, &Options::default())[i].clone();
    assert_ne!(first.readiness, Readiness::Unknown);
    assert!(
        first.reason.contains("Leaf-bladed battleaxe"),
        "{}",
        first.reason
    );
    assert_eq!(first.requirements.status, RequirementStatus::Missing);
    a.members[0].level_overrides.insert("Slayer".into(), 80);
    let next = bestiary::assess_catalog(&a, &Options::default())[i].clone();
    assert_eq!(next.readiness, first.readiness);
    assert_eq!(next.reason, first.reason);
    assert_eq!(next.requirements.status, RequirementStatus::LevelMet);
}
#[test]
fn tormented_shield_reduction_exceptions_and_protection_are_explicit() {
    let r = calc(13599, 1434, None, "Crush");
    assert_eq!(r.max_hit, 16);
    let p = accuracy(13640, 159 * 132);
    let total: i32 = (0..=20).map(|d| (d.max(1) * 4 / 5).max(1)).sum();
    close(mean(&r), p * f64::from(total) / 21.0);
    assert!(try_calc(13599, 1381, None, "Magic", "Wind Bolt").is_err());
    let abyssal = calc(13599, 4151, None, "Slash");
    assert_eq!(abyssal.max_hit, 24); // 107*(82+64)+320, divided by 640.
    let other = calc(100190, 9185, Some(9242), "Ranged");
    assert_eq!(other.max_hit, 80);
    assert_eq!(
        other.hp_dependent.unwrap().transform,
        Transform::TormentedShield
    );
    close(calc(100192, 1434, None, "Crush").accuracy, 1.0);
    assert!(try_calc(100193, 1434, None, "Crush", "").is_err());
}
#[test]
fn seren_limiter_is_per_splat_and_reaches_ruby_hp_model() {
    assert_eq!(
        Transform::Seren.outcomes(23, 1.0),
        vec![(22, 1.0 / 3.0), (23, 1.0 / 3.0), (23, 1.0 / 3.0)]
    );
    assert_eq!(
        monster_rules::transform_splats(&[30, 30], 1.0, Transform::Seren)
            .iter()
            .map(|(d, p)| f64::from(*d) * p)
            .sum::<f64>(),
        46.0
    );
    let ruby = calc(8917, 9185, Some(9242), "Ranged");
    assert_eq!(ruby.max_hit, 24);
    assert_eq!(ruby.hp_dependent.unwrap().transform, Transform::Seren);
    let s = gear(1381, None);
    let mut target = planner::monster(8917).unwrap().clone();
    target.skills.insert("magic".into(), 999);
    let r = combat::calculate(&member(), &s, "Magic", &target, "Wind Bolt").unwrap();
    assert_eq!(r.defence_roll, 111 * 74); // Defence level, not the altered Magic level.
}
#[test]
fn slagilith_divides_non_pickaxe_splats_separately() {
    let ordinary = calc(1362, 4747, None, "Crush");
    assert_eq!(ordinary.max_hit, 7); // 11/3 + 12/3, not floor(23/3) by coincidence only.
    let axe = calc(1362, 1434, None, "Crush");
    assert_eq!(axe.max_hit, 6);
    let pick = calc(1362, 1275, None, "Crush");
    assert!(pick.max_hit > 6);
    assert_eq!(
        monster_rules::transform_splats(&[5, 5], 1.0, Transform::Divide { divisor: 3 }),
        vec![(2, 1.0)]
    );
}
#[test]
fn unreviewed_quest_variants_stay_gated_and_zuk_cannot_be_meleed() {
    for id in [15010, 16309, 6324, 6385] {
        assert!(encounters::limitation(planner::monster(id).unwrap()).is_some());
    }
    for id in [7706, 7708] {
        assert!(try_calc(id, 1434, None, "Crush", "").is_err());
        assert!(calc(id, 9185, Some(9144), "Ranged").dps > 0.0);
    }
}
#[test]
fn newly_reviewed_bosses_get_gear_bands_separate_from_access() {
    let mut a = account::demo();
    a.members[0].level_overrides.insert("Slayer".into(), 1);
    let results = bestiary::assess_catalog(&a, &Options::default());
    for name in [
        "Doom of Mokhaiotl",
        "Mad Angel",
        "Maggot King",
        "Shellbane Gryphon",
        "Sol Heredit",
        "TzKal-Zuk",
        "Inferno",
        "Fortis Colosseum",
        "Tormented Demons",
    ] {
        let i = encounters::catalog()
            .iter()
            .position(|e| e.name == name)
            .unwrap();
        assert_ne!(
            results[i].readiness,
            Readiness::Unknown,
            "{name}: {}",
            results[i].reason
        );
        if name == "Shellbane Gryphon" {
            assert_eq!(results[i].requirements.status, RequirementStatus::Missing);
        }
    }
}
#[test]
fn doom_shield_readiness_accepts_ranged_and_magic_routes_without_demonbane_melee() {
    let mut a = account::demo();
    for s in account::SKILLS {
        a.members[0].level_overrides.insert(s.into(), 99);
    }
    a.members[0]
        .bank
        .get_or_insert_with(Vec::new)
        .extend([29591, 1, 892, 1000, 29594, 1, 554, 1000, 566, 1000, 1434, 1]);
    let i = encounters::catalog()
        .iter()
        .position(|e| e.name == "Doom of Mokhaiotl")
        .unwrap();
    let mut base = Options::default();
    for id in a.available(false).keys() {
        if monster_rules::is_demonbane(*id, "slash", "") {
            base.excluded.insert(*id);
        }
    }
    let mut ranged = base.clone();
    for id in a.available(false).keys() {
        if ironbound::rules::weapon(*id).is_some_and(|w| w.family.style == "Magic") {
            ranged.excluded.insert(*id);
        }
    }
    let r = bestiary::assess_catalog(&a, &ranged)[i].clone();
    assert!(!r.reason.starts_with("No usable"), "{}", r.reason);
    assert_eq!(r.reason.split(';').count(), 3, "{}", r.reason);
    assert!(r.reason.contains("Scorching bow"));
    base.excluded.insert(29591);
    let magic = bestiary::assess_catalog(&a, &base)[i].clone();
    assert!(!magic.reason.starts_with("No usable"), "{}", magic.reason);
    assert!(magic.reason.contains("Purging staff"), "{}", magic.reason);
}
#[test]
fn counters_normalise_and_maggot_templates_do_not_duplicate_hp() {
    assert_eq!(
        planner::monster(100183).unwrap().name,
        "Maggot King · nearby"
    );
    assert_eq!(
        planner::monster(100190).unwrap().name,
        "Tormented Demon · shielded, protects Melee"
    );
    for (id, weapon, style) in [
        (100180, 4747, "Crush"),
        (100181, 28997, "Crush"),
        (100182, 22325, "Slash"),
        (100185, 4747, "Crush"),
        (13599, 4151, "Slash"),
    ] {
        let r = calc(id, weapon, None, style);
        close(r.hit_distribution.iter().map(|(_, p)| p).sum(), 1.0);
    }
    assert_eq!(
        encounters::window(15742).unwrap().progress_group,
        encounters::window(100183).unwrap().progress_group
    );
    assert_eq!(
        encounters::window(16305).unwrap().progress_group,
        encounters::window(100181).unwrap().progress_group
    );
}
#[test]
fn newly_supported_outgoing_does_not_claim_incoming_or_lose_shared_state() {
    for id in [
        14707, 100170, 100178, 100180, 16305, 100181, 15742, 100185, 14860, 12821, 7706, 13599,
        100192,
    ] {
        assert!(
            ironbound::defence::estimate(
                &member(),
                &gear(1434, None),
                planner::monster(id).unwrap(),
                "Magic",
                &Default::default()
            )
            .is_err(),
            "{id}"
        );
    }
    let mut o = Options {
        boss: planner::Boss::Custom,
        targets: vec![planner::Target {
            id: 100178,
            style: "Slash".into(),
        }],
        ..Default::default()
    };
    o.combat.target_hp.insert(100178, 200);
    let a = account::demo();
    let plan = planner::plan(&a, &o);
    let setup = ironbound::share::Setup::from_plan(&plan, &o);
    let restored = ironbound::share::Setup::parse(&setup.fragment().unwrap())
        .unwrap()
        .options();
    assert_eq!(restored.targets[0].id, 100178);
    assert_eq!(restored.combat.target_hp[&100178], 200);
    assert!(ironbound::coverage::export_note(&restored).contains("200 shield points"));
}
