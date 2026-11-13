//! Task settings and explanations share the calculator's eligibility/effect rules.
use crate::{defence::Settings, effects, encounters, planner::Monster, rules};

pub fn set_task(settings: &mut Settings, target: i32, on_task: bool) {
    if on_task && encounters::slayer_eligible(target) {
        settings.slayer_targets.insert(target);
    } else {
        settings.slayer_targets.remove(&target);
    }
}

pub fn bonus_note(
    settings: &Settings,
    target: &Monster,
    set: &[Option<i32>],
    style: &str,
) -> &'static str {
    if !encounters::slayer_eligible(target.id) {
        return "This target cannot receive a Slayer task bonus.";
    }
    if !settings.slayer_targets.contains(&target.id) {
        return "Off task · no Slayer bonus. On task reranks owned gear with eligible helmet and mask bonuses.";
    }
    let style = rules::base_style(style);
    if let Some(bonus) = effects::target_bonus(set, style, target, settings) {
        if bonus.slayer_task {
            return if style == "Melee" {
                "Slayer bonus active · +16.67% melee accuracy and damage."
            } else {
                "Slayer bonus active · +15% accuracy and damage from your imbued helmet or mask."
            };
        }
        return "On task · Salve takes priority for this style; Slayer and Salve bonuses do not stack.";
    }
    if style != "Melee" && rules::equipped_effect(set, "slayer_melee") {
        "On task · Ranged and Magic need an imbued Slayer helmet or black mask for the bonus."
    } else {
        "On task · equip an eligible Slayer helmet or black mask to receive the bonus."
    }
}
