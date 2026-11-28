use crate::{
    account::Member,
    defence::Settings,
    planner::{self, Boss, Check, Options, Target},
    rules,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NAME: &str = "The Royal Titans";
pub const FIRE_ELEMENTAL: i32 = 14150;
pub const ICE_ELEMENTAL: i32 = 14151;
const BODY_WINDOWS: [i32; 4] = [12596, 14147, 100162, 100163];
const BOTH_ADDS: &[i32] = &[FIRE_ELEMENTAL, ICE_ELEMENTAL];

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    #[default]
    SoloBoth,
    SoloSkip,
    DuoBoth,
    DuoFire,
    DuoWater,
    DuoPartner,
    DuoSkip,
}

impl Strategy {
    pub const CHOICES: &[(Self, &str, &str)] = &[
        (Self::SoloBoth, "solo-both", "Solo · Fire and Water"),
        (Self::SoloSkip, "solo-skip", "Solo · Skip add clearing"),
        (Self::DuoBoth, "duo-both", "Duo · I bring Fire and Water"),
        (Self::DuoFire, "duo-fire", "Duo · I bring Fire"),
        (Self::DuoWater, "duo-water", "Duo · I bring Water"),
        (Self::DuoPartner, "duo-partner", "Duo · Partner brings both"),
        (Self::DuoSkip, "duo-skip", "Duo · Skip add clearing"),
    ];

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn label(self) -> &'static str {
        Self::CHOICES
            .iter()
            .find(|choice| choice.0 == self)
            .unwrap()
            .2
    }

    pub fn advice(self) -> &'static str {
        match self {
            Self::SoloBoth => {
                "Cover both elements yourself. Stagger the titans' melee windows when you can and switch to ranged when both withdraw."
            }
            Self::SoloSkip => {
                "Focus on melee and ranged. Avoid barriers and lure adds into the opposite titan's attacks when possible. Bring food for explosions and plan a quick reset between kills. Thralls are optional."
            }
            Self::DuoBoth => {
                "Clear both sets of elementals and barriers while your partner focuses on the titans. Agree which titan to focus first."
            }
            Self::DuoFire => {
                "Use Fire on ice elementals and icicles. Confirm your partner brings Water for fire elementals and fire walls."
            }
            Self::DuoWater => {
                "Use Water on fire elementals and fire walls. Confirm your partner brings Fire for ice elementals and icicles."
            }
            Self::DuoPartner => {
                "Focus on melee and ranged while your partner clears both sets of elementals and barriers. Confirm they can cast Fire and Water."
            }
            Self::DuoSkip => {
                "Both players focus on boss damage with melee and ranged. Agree to skip clears, plan for add explosions and reset between kills if needed. Thralls are optional."
            }
        }
    }

    fn adds(self) -> &'static [i32] {
        match self {
            Self::SoloBoth | Self::DuoBoth => BOTH_ADDS,
            Self::DuoFire => &[ICE_ELEMENTAL],
            Self::DuoWater => &[FIRE_ELEMENTAL],
            Self::SoloSkip | Self::DuoPartner | Self::DuoSkip => &[],
        }
    }

    pub fn apply(self, options: &mut Options) {
        options.royal_titans = self;
        options
            .targets
            .retain(|target| element(target.id).is_none());
        options.targets.extend(self.adds().iter().map(|&id| Target {
            id,
            style: "Magic".into(),
        }));
    }
}

pub fn matches_targets(targets: &[Target]) -> bool {
    BODY_WINDOWS
        .iter()
        .all(|id| targets.iter().any(|t| t.id == *id))
        && targets.iter().all(|t| {
            BODY_WINDOWS.contains(&t.id) || (element(t.id).is_some() && t.style == "Magic")
        })
        && targets.len() <= BODY_WINDOWS.len() + BOTH_ADDS.len()
}

pub fn active(options: &Options) -> bool {
    options.boss == Boss::Custom && matches_targets(&options.targets)
}

pub fn element(id: i32) -> Option<&'static str> {
    match id {
        FIRE_ELEMENTAL => Some("water"),
        ICE_ELEMENTAL => Some("fire"),
        _ => None,
    }
}

pub fn spell_for_add(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &Settings,
    id: i32,
) -> Option<&'static str> {
    let element = element(id)?;
    if settings.spell_selection.is_auto() {
        return rules::elemental_candidates(member, set, owned, settings)
            .into_iter()
            .find(|recipe| recipe.element.as_deref() == Some(element))
            .map(|recipe| recipe.name.as_str());
    }
    let spell = rules::spell_name_configured(member, set, owned, settings);
    rules::spells()
        .iter()
        .any(|recipe| recipe.name == spell && recipe.element.as_deref() == Some(element))
        .then_some(spell)
        .filter(|spell| rules::spell_requirement(member, set, owned, settings, spell).is_none())
}

fn missing_spell(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    settings: &Settings,
    id: i32,
) -> String {
    let element = element(id).expect("A Royal Titans add is required");
    if settings.spell_selection.is_auto() {
        return rules::elemental_unavailability(member, set, settings, element);
    }
    let spell = rules::spell_name_configured(member, set, owned, settings);
    if rules::spells()
        .iter()
        .any(|recipe| recipe.name == spell && recipe.element.as_deref() == Some(element))
        && let Some(reason) = rules::spell_requirement(member, set, owned, settings, spell)
    {
        return reason;
    }
    "The pinned spell uses a different element. Choose Auto to let the planner select both elements as needed.".into()
}

fn clears_wall(spell: &str) -> bool {
    ["Blast", "Wave", "Surge"]
        .iter()
        .any(|tier| spell.ends_with(tier))
}

pub fn spell_checks(
    member: &Member,
    set: &[Option<i32>],
    owned: &BTreeMap<i32, u32>,
    options: &Options,
) -> Vec<(Check, Option<&'static str>)> {
    options
        .targets
        .iter()
        .filter_map(|target| {
            let name = match target.id {
                FIRE_ELEMENTAL => "Water spell for fire elementals",
                ICE_ELEMENTAL => "Fire spell for ice elementals",
                _ => return None,
            };
            let settings = &options.combat;
            let spell = spell_for_add(member, set, owned, settings, target.id);
            let detail = if let Some(spell) = spell {
                let damage = planner::monster(target.id)
                    .and_then(|target| {
                        crate::combat::calculate_configured(
                            member,
                            set,
                            "Magic",
                            target,
                            spell,
                            options.raid_level,
                            settings,
                        )
                        .ok()
                    })
                    .map_or_else(String::new, |result| {
                        format!(
                            " {:.0}% hit chance, up to {} damage per cast.",
                            result.accuracy * 100.0,
                            result.max_hit
                        )
                    });
                let wall = if clears_wall(spell) {
                    " Also clears matching barriers."
                } else {
                    " Barriers need Blast or stronger."
                };
                format!("{spell} · Standard spellbook.{damage}{wall}")
            } else {
                missing_spell(member, set, owned, settings, target.id)
            };
            Some((
                Check {
                    name: name.into(),
                    state: Some(spell.is_some()),
                    detail,
                },
                spell,
            ))
        })
        .collect()
}
