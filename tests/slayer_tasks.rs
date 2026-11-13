use ironbound::{account, combat, defence::Settings, planner, slayer};

#[test]
fn task_controls_keep_targets_independent_and_reject_ineligible_targets() {
    let mut settings = Settings::default();
    slayer::set_task(&mut settings, 415, true);
    slayer::set_task(&mut settings, 8059, true);
    slayer::set_task(&mut settings, 100014, true);
    assert_eq!(settings.slayer_targets, [415, 8059].into());
    slayer::set_task(&mut settings, 415, false);
    assert_eq!(settings.slayer_targets, [8059].into());
    assert!(settings.valid());
}

#[test]
fn bonus_explanation_matches_actual_task_effects_for_each_style_and_imbue() {
    let target = planner::monster(415).unwrap();
    let mut member = account::Member::default();
    for skill in account::SKILLS {
        member.level_overrides.insert(skill.into(), 99);
    }
    for (style, weapon, head, active) in [
        ("Slash", 4151, 8921, true),
        ("Ranged", 9185, 8921, false),
        ("Ranged", 9185, 11865, true),
        ("Magic", 11905, 8921, false),
        ("Magic", 11905, 11865, true),
    ] {
        let mut set = vec![None; 14];
        set[0] = Some(head);
        set[3] = Some(weapon);
        if style == "Ranged" {
            set[13] = Some(9144);
        }
        let mut settings = Settings::default();
        let before =
            combat::calculate_configured(&member, &set, style, target, "", 0, &settings).unwrap();
        assert!(slayer::bonus_note(&settings, target, &set, style).starts_with("Off task"));
        slayer::set_task(&mut settings, target.id, true);
        let after =
            combat::calculate_configured(&member, &set, style, target, "", 0, &settings).unwrap();
        assert_eq!(after.dps > before.dps, active, "{style}/{head}");
        assert_eq!(
            slayer::bonus_note(&settings, target, &set, style).starts_with("Slayer bonus active"),
            active
        );
        if !active {
            assert!(slayer::bonus_note(&settings, target, &set, style).contains("need an imbued"));
        }
        slayer::set_task(&mut settings, target.id, false);
        assert_eq!(
            combat::calculate_configured(&member, &set, style, target, "", 0, &settings)
                .unwrap()
                .dps,
            before.dps
        );
    }
}

#[test]
fn bonus_note_explains_salve_priority_and_missing_gear() {
    let target = planner::monster(8059).unwrap();
    let mut settings = Settings::default();
    slayer::set_task(&mut settings, target.id, true);
    let mut set = vec![None; 14];
    assert!(slayer::bonus_note(&settings, target, &set, "Ranged").contains("equip an eligible"));
    set[0] = Some(11865);
    set[2] = Some(12018);
    assert!(slayer::bonus_note(&settings, target, &set, "Ranged").contains("Salve takes priority"));
    assert!(
        slayer::bonus_note(&settings, planner::monster(100014).unwrap(), &set, "Magic")
            .contains("cannot receive")
    );
}
