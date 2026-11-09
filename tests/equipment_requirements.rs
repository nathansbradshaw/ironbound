use ironbound::{account::Member, planner, rules};

fn member() -> Member {
    let mut m = Member::default();
    for skill in ["Defence", "Strength", "Magic", "Ranged", "Agility"] {
        m.level_overrides.insert(skill.into(), 99);
    }
    m
}

#[test]
fn reviewed_family_requirements_cover_real_public_item_ids() {
    for group in &rules::registry().equipment_requirements {
        for id in &group.ids {
            assert!(planner::gear(*id).is_some(), "Unknown requirement ID {id}");
            assert!(planner::can_use(*id, &member()));
            for (skill, minimum) in &group.requirements {
                let mut low = member();
                low.level_overrides.insert(skill.clone(), minimum - 1);
                assert!(!planner::can_use(*id, &low), "{id} ignored {skill}");
                low.level_overrides.insert(skill.clone(), *minimum);
                assert!(planner::can_use(*id, &low), "{id} boundary {skill}");
                low.level_overrides.remove(skill);
                assert!(
                    !planner::can_use(*id, &low),
                    "{id} accepted unknown {skill}"
                );
            }
        }
    }
}

#[test]
fn moon_combat_requirements_are_independent_of_attack_style() {
    for (id, skill) in [(29022, "Strength"), (29013, "Magic"), (29004, "Ranged")] {
        let mut m = member();
        m.level_overrides.insert("Defence".into(), 50);
        m.level_overrides.insert(skill.into(), 75);
        assert!(planner::can_use(id, &m));
        m.level_overrides.insert(skill.into(), 74);
        assert!(!planner::can_use(id, &m));
    }
}

#[test]
fn crystal_base_and_recolour_share_real_wear_levels_without_quest_checks() {
    let mut m = Member::default();
    m.level_overrides.insert("Defence".into(), 70);
    m.level_overrides.insert("Agility".into(), 50);
    for id in [23971, 23975, 23979, 27753, 27745, 27749] {
        assert!(planner::can_use(id, &m));
        let mut low = m.clone();
        low.level_overrides.insert("Agility".into(), 49);
        assert!(!planner::can_use(id, &low));
    }
}

#[test]
fn avernic_requires_all_four_stats_even_with_null_catalog_metadata() {
    let mut m = member();
    for skill in ["Defence", "Strength", "Magic", "Ranged"] {
        m.level_overrides.insert(skill.into(), 80);
    }
    for id in [31088, 31093, 31097] {
        assert!(planner::can_use(id, &m));
        let mut low = m.clone();
        low.level_overrides.insert("Magic".into(), 79);
        assert!(!planner::can_use(id, &low));
    }
}

#[test]
fn bowfa_central_gate_uses_current_eighty_ranged_requirement() {
    let mut m = member();
    m.level_overrides.insert("Agility".into(), 70);
    m.level_overrides.insert("Ranged".into(), 79);
    for id in [25865, 25867, 25896] {
        assert!(!planner::can_use(id, &m));
    }
    m.level_overrides.insert("Ranged".into(), 80);
    for id in [25865, 25867, 25896] {
        assert!(planner::can_use(id, &m));
    }
}
