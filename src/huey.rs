use crate::{
    account::Member,
    attack_budget, combat, optimization,
    planner::{self, Boss, Options, Packed, Plan, Target},
    rules, set_search,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

pub const NAME: &str = "The Hueycoatl";
pub const BODY: i32 = 14017;
pub const HEAD: i32 = 14009;
pub const TAIL: i32 = 14014;
pub const LIT_HEAD: i32 = 100160;
pub const LIT_TAIL: i32 = 100161;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Unassigned,
    Magic,
    Melee,
    Ranged,
}

impl Role {
    pub const CHOICES: &[(Self, &str, &str)] = &[
        (Self::Unassigned, "none", "Unassigned"),
        (Self::Magic, "magic", "Blue · Protect from Magic"),
        (Self::Melee, "melee", "Red · Protect from Melee"),
        (Self::Ranged, "ranged", "Green · Protect from Missiles"),
    ];

    pub fn protection(self) -> Option<&'static str> {
        match self {
            Self::Magic => Some("Magic"),
            Self::Melee => Some("Melee"),
            Self::Ranged => Some("Ranged"),
            Self::Unassigned => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct Settings {
    pub players: u8,
    pub role: Role,
    pub pillars_lit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            players: 1,
            role: Role::Unassigned,
            pillars_lit: false,
        }
    }
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn valid(self) -> bool {
        (1..=20).contains(&self.players)
            && (self.players >= 3 || (self.role == Role::Unassigned && !self.pillars_lit))
    }

    pub fn with_players(self, players: u8) -> Self {
        if players < 3 {
            Self {
                players,
                ..Self::default()
            }
        } else {
            Self {
                players,
                pillars_lit: false,
                ..self
            }
        }
    }

    pub fn per_pillar(self) -> Option<u8> {
        (self.players >= 3).then_some(self.players / 3)
    }

    pub fn loot_rule(self) -> &'static str {
        if self.players < 6 {
            "For loot, deal at least 25 damage to the body and 75 combined to the head and tail."
        } else {
            "For loot, deal 25 body damage or 75 total damage. Get 25 body damage to avoid the 95% reward reduction."
        }
    }

    pub fn apply(self, options: &mut Options) {
        options.huey = self;
        for target in &mut options.targets {
            target.id = match target.id {
                HEAD | LIT_HEAD if self.pillars_lit => LIT_HEAD,
                TAIL | LIT_TAIL if self.pillars_lit => LIT_TAIL,
                HEAD | LIT_HEAD => HEAD,
                TAIL | LIT_TAIL => TAIL,
                other => other,
            };
        }
    }
}

pub fn matches_targets(targets: &[Target]) -> bool {
    matches!(targets, [body, head, tail] if body.id == BODY
        && matches!(head.id, HEAD | LIT_HEAD) && matches!(tail.id, TAIL | LIT_TAIL)
        && targets.iter().all(Target::valid))
}

pub fn active(options: &Options) -> bool {
    options.boss == Boss::Custom && matches_targets(&options.targets)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GearRole {
    BodyAndHead,
    Tail,
}

impl GearRole {
    pub(crate) fn for_target(id: i32) -> Option<Self> {
        match id {
            BODY | HEAD | LIT_HEAD => Some(Self::BodyAndHead),
            TAIL | LIT_TAIL => Some(Self::Tail),
            _ => None,
        }
    }
}

pub fn is_tail(id: i32) -> bool {
    GearRole::for_target(id) == Some(GearRole::Tail)
}

/// The reviewed phase gear search uses Crush for both roles.
pub(crate) fn split_gear_supported(options: &Options) -> bool {
    active(options)
        && crate::recommendation::enabled(options)
        && options.targets.iter().all(|target| target.style == "Crush")
}

fn summit_protection(options: &Options, target: i32) -> Option<&'static str> {
    (active(options) && matches!(target, HEAD | LIT_HEAD | TAIL | LIT_TAIL))
        .then(|| options.huey.role.protection())
        .flatten()
}

pub(crate) fn combat_settings(options: &Options, target: i32) -> Cow<'_, crate::defence::Settings> {
    if let Some(protection) = summit_protection(options, target) {
        let mut settings = options.combat.clone();
        settings.protection = protection.into();
        Cow::Owned(settings)
    } else {
        Cow::Borrowed(&options.combat)
    }
}

/// Compare complete plans using identical personal workloads and pillar state.
pub(crate) fn attack_seconds(options: &Options, member: &Member, plan: &Plan) -> Option<f64> {
    if options.targets.len() != plan.phases.len() {
        return None;
    }
    options
        .targets
        .iter()
        .zip(&plan.phases)
        .try_fold(0.0, |total, (target, phase)| {
            let monster = planner::monster(target.id)?;
            let settings = combat_settings(options, target.id);
            let attack = combat::calculate_configured(
                member,
                &phase.equipment,
                &phase.style,
                monster,
                "",
                options.raid_level,
                &settings,
            )
            .ok()?;
            let time = if matches!(target.id, HEAD | LIT_HEAD) {
                f64::from(combat::target_hitpoints(monster, options.raid_level)?) / attack.dps
            } else {
                attack_budget::for_window(monster, &attack, options, &phase.equipment)
                    .ok()?
                    .seconds
            };
            let priority = options
                .phase_priorities
                .get(&target.id)
                .copied()
                .unwrap_or(1)
                .min(10);
            time.is_finite()
                .then_some(total + time * f64::from(priority))
        })
}

/// Compare a weapon swap and any complete owned Blood Moon set against the real
/// tail distribution, then pack a faster set if it fits the existing trip.
pub(crate) fn pack_tail_switch(
    options: &Options,
    member: &Member,
    owned: &BTreeMap<i32, u32>,
    plan: &mut Plan,
    essentials: &mut BTreeMap<i32, u32>,
    seed: Option<&[Option<i32>]>,
) {
    if !crate::recommendation::enabled(options) || options.max_switches == Some(0) {
        return;
    }
    let Some(index) = options
        .targets
        .iter()
        .position(|t| is_tail(t.id) && t.style == "Crush")
    else {
        return;
    };
    let target = planner::monster(options.targets[index].id).expect("reviewed tail window");
    let Some(phase) = plan.phases.get(index) else {
        return;
    };
    let base = &phase.equipment;
    let seconds = |set: &[Option<i32>]| -> Option<f64> {
        let result =
            combat::calculate_configured(member, set, "Crush", target, "", 0, &options.combat)
                .ok()?;
        attack_budget::for_window(target, &result, options, set)
            .ok()
            .map(|budget| budget.seconds)
    };
    let baseline = seconds(base).unwrap_or(f64::INFINITY);
    let mut trials: Vec<_> = set_search::candidates(base, "Crush", member, owned, options)
        .into_iter()
        .map(|seed| seed.equipment)
        .filter(|set| rules::set_effect(set, "blood_moon"))
        .collect();
    trials.extend(seed.map(<[Option<i32>]>::to_vec));
    if !options.gear_overrides.contains_key("Crush:3") {
        for (&id, &quantity) in owned {
            if quantity == 0
                || !planner::can_use(id, member)
                || rules::weapon_for_style(id, "Crush").is_none()
            {
                continue;
            }
            let mut set = base.clone();
            set[3] = Some(id);
            if planner::gear(id).is_some_and(|gear| gear.two_handed) {
                set[5] = None;
            }
            trials.push(set);
        }
    }

    let mut best = None;
    for set in trials {
        if options.fewer_switches
            && [0, 1, 2, 9, 10, 12]
                .into_iter()
                .any(|slot| set[slot] != plan.equipment[slot])
        {
            continue;
        }
        if set.iter().enumerate().any(|(slot, id)| {
            options
                .gear_overrides
                .get(&format!("Crush:{slot}"))
                .is_some_and(|pin| (*pin != 0).then_some(*pin) != *id)
        }) || options.max_switches.is_some_and(|limit| {
            std::iter::once(&plan.equipment)
                .chain(plan.phases.iter().map(|p| &p.equipment))
                .any(|other| optimization::changes(other, &set) > limit)
        }) {
            continue;
        }
        let Some(time) = seconds(&set).filter(|time| *time + 1e-9 < baseline) else {
            continue;
        };
        let required: BTreeSet<_> = set
            .iter()
            .zip(&plan.equipment)
            .filter_map(|(id, start)| id.filter(|_| id != start))
            .collect();
        let extra: Vec<_> = required
            .iter()
            .copied()
            .filter(|&id| !plan.inventory.iter().any(|item| item.id == id))
            .collect();
        if options.max_switches.is_some() && plan.switches + extra.len() > options.switch_slots {
            continue;
        }
        let key = (
            time,
            extra.len(),
            optimization::changes(&plan.equipment, &set),
        );
        if best
            .as_ref()
            .is_some_and(|(previous, _, _)| key >= *previous)
        {
            continue;
        }
        let mut trial = plan.clone();
        trial.phases[index].equipment = set;
        trial.switches += extra.len();
        trial.inventory.extend(extra.into_iter().map(|id| Packed {
            id,
            quantity: 1,
            purpose: "Tail switch".into(),
        }));
        if trial.inventory.len() <= 28
            && (0..trial.phases.len()).all(|i| trial.preview_inventory(Some(i)).len() <= 28)
        {
            best = Some((key, trial, required));
        }
    }
    if let Some((_, trial, required)) = best {
        *plan = trial;
        for id in required {
            essentials.entry(id).or_insert(1);
        }
        if let Some(report) = &mut plan.optimization {
            let tail = &plan.phases[index];
            report.max_changes = report.max_changes.max(
                std::iter::once(&plan.equipment)
                    .chain(plan.phases.iter().map(|phase| &phase.equipment))
                    .map(|equipment| optimization::changes(equipment, &tail.equipment))
                    .max()
                    .unwrap_or(0),
            );
            report.gear_slots = plan.switches;
            let settings = combat_settings(options, target.id);
            if let Some(row) = report
                .comparisons
                .iter_mut()
                .find(|row| row.name == tail.name && row.style == tail.style)
            {
                row.after = combat::calculate_configured(
                    member,
                    &tail.equipment,
                    "Crush",
                    target,
                    "",
                    options.raid_level,
                    &settings,
                )
                .ok()
                .map(|result| result.dps);
                row.prayer_after = crate::prayer::drain_per_minute(
                    crate::prayer::bonus(&tail.equipment),
                    &settings,
                );
            }
            report
                .note
                .push_str(" A dedicated tail set was compared after the shared gear search.");
        }
    }
}
