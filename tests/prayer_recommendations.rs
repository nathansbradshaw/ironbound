use ironbound::{
    account::{self, SKILLS, Snapshot},
    defence::Settings,
    planner::{self, Boss, Options, Target},
};

fn bank(ids: &[i32]) -> Snapshot {
    let mut bank = account::demo();
    bank.members[0].bank = Some(ids.iter().flat_map(|id| [i64::from(*id), 1]).collect());
    for skill in SKILLS {
        bank.members[0].level_overrides.insert(skill.into(), 99);
    }
    bank
}
fn options() -> Options {
    Options {
        boss: Boss::Custom,
        targets: vec![Target {
            id: 1673,
            style: "Slash".into(),
        }],
        combat: Settings {
            safety: 0,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn damage_ties_keep_defence_instead_of_unrequested_prayer_gear() {
    let bank = bank(&[4151, 4087, 9676]);
    let o = options();
    assert_eq!(
        planner::equipment_for_style(&bank, &o, "Slash")[7],
        Some(4087)
    );
    assert_eq!(planner::plan(&bank, &o).equipment[7], Some(4087));
}

#[test]
fn prayer_weight_does_not_choose_a_strictly_worse_armour_tie() {
    let bank = bank(&[4151, 10551, 29562, 9674]);
    let mut o = options();
    o.combat.protection = "Melee".into();
    o.combat.prayer_weight = 3;
    o.ranking.weights = ironbound::ranking::Weights {
        dps: 0.0,
        defence: 0.0,
        prayer: 100.0,
        weight: 0.0,
    };
    assert_eq!(
        planner::equipment_for_style(&bank, &o, "Slash")[4],
        Some(9674)
    );
    assert_eq!(planner::plan(&bank, &o).equipment[4], Some(9674));
}

#[test]
fn unavailable_damage_fallback_does_not_break_armour_ties_with_prayer() {
    let bank = bank(&[4151, 4087, 9676]);
    let mut o = options();
    // A prayer whose requirements are not met makes the combat estimate unavailable.
    let mut low_prayer = bank.clone();
    low_prayer.members[0]
        .level_overrides
        .insert("Prayer".into(), 60);
    o.combat.offensive = "Piety".into();
    assert_eq!(
        planner::equipment_for_style(&low_prayer, &o, "Slash")[7],
        Some(4087)
    );
}

#[test]
fn an_explicit_gear_override_survives_automatic_tie_breaking() {
    let bank = bank(&[4151, 4087, 9676]);
    let mut o = options();
    o.gear_overrides.insert("Slash:7".into(), 9676);
    assert_eq!(
        planner::equipment_for_style(&bank, &o, "Slash")[7],
        Some(9676)
    );
}

#[test]
fn unsupported_encounters_still_use_defensive_armour_for_offensive_ties() {
    let bank = bank(&[4151, 4087, 9676]);
    let mut o = options();
    o.targets[0].id = 16659; // Archive Guardian: damage mechanics are not modelled.
    assert_eq!(
        planner::equipment_for_style(&bank, &o, "Slash")[7],
        Some(4087)
    );
}

#[test]
fn prayer_weight_without_active_prayers_does_not_reduce_defence() {
    let bank = bank(&[4151, 4087, 9676]);
    let mut o = options();
    o.combat.prayer_weight = 3;
    assert_eq!(
        planner::equipment_for_style(&bank, &o, "Slash")[7],
        Some(4087)
    );
}
