//! Raid room packing routes. These describe target selection, not combat mechanics.
use crate::planner::Target;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
fn source(id:i32)->i32 {crate::encounters::source_npc_id(id)}
fn scale(id:i32)->Option<&'static crate::raid_scaling::Scale> {crate::encounters::window(id).and_then(|w|w.raid_scale.as_ref())}

#[derive(Clone, Debug, Deserialize)]
pub struct Raid {
    pub id: String,
    pub name: String,
    pub rooms: Vec<Room>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub required: bool,
    pub choice_group: Option<String>,
    pub default: bool,
    pub targets: Vec<Target>,
}

impl Room {
    /// Styles remain editable; selecting a room includes every recorded NPC phase.
    pub fn selected(&self, targets: &[Target]) -> bool {
        self.targets
            .iter()
            .all(|phase| targets.iter().any(|t| source(t.id) == phase.id))
    }
}

impl Raid {
    pub fn default_targets(&self) -> Vec<Target> {
        self.rooms
            .iter()
            .filter(|r| r.required || r.default)
            .flat_map(|r| r.targets.clone())
            .collect()
    }

    fn accepts(&self, targets: &[Target]) -> bool {
        if targets.iter().any(|target| !target.valid()) {
            return false;
        }
        if targets.first().is_some_and(|first| targets.iter().any(|t|scale(t.id)!=scale(first.id))) {return false;}
        let ids: BTreeSet<_> = targets.iter().map(|t| source(t.id)).collect();
        if ids.len() != targets.len() {
            return false;
        }
        let mut covered = BTreeSet::new();
        let mut choices: BTreeMap<&str, usize> = BTreeMap::new();
        for room in &self.rooms {
            let count = room.targets.iter().filter(|t| ids.contains(&t.id)).count();
            if (count != 0 && count != room.targets.len()) || (room.required && count == 0) {
                return false;
            }
            if let Some(group) = room.choice_group.as_deref() {
                *choices.entry(group).or_default() += usize::from(count != 0);
            }
            if count != 0 {
                covered.extend(room.targets.iter().map(|t| t.id));
            }
        }
        covered == ids && choices.values().all(|count| *count == 1)
    }

    /// Toggle an optional room, or replace one fork choice with its sibling.
    /// Unchanged rooms retain their edited attack styles and complete phase groups.
    pub fn toggle(&self, targets: &[Target], room_id: &str) -> Result<Vec<Target>, String> {
        let room = self
            .rooms
            .iter()
            .find(|r| r.id == room_id)
            .ok_or_else(|| "Unknown raid room.".to_owned())?;
        if !self.accepts(targets) {
            return Err("The selected targets do not form a complete route for this raid.".into());
        }
        if room.required {
            return Err("This room is required for the raid.".into());
        }
        let was_selected = room.selected(targets);
        let mut next = Vec::new();
        for candidate in &self.rooms {
            let selected = if candidate.id == room.id {
                room.choice_group.is_some() || !was_selected
            } else if room.choice_group.is_some() && candidate.choice_group == room.choice_group {
                false
            } else {
                candidate.selected(targets)
            };
            if selected {
                for phase in &candidate.targets {
                    next.push(
                        targets
                            .iter()
                            .find(|t| source(t.id) == phase.id)
                            .cloned()
                            .or_else(||{
                                let context=targets.first().and_then(|t|scale(t.id));
                                crate::encounters::mechanics().windows.iter().find(|w|w.npc_id==phase.id && w.raid_scale.as_ref()==context && (context.is_some() || w.id==phase.id)).map(|w|Target{id:w.id,style:phase.style.clone()})
                            })
                            .ok_or("This raid room has no reviewed target at the selected party scale.")?,
                    );
                }
            }
        }
        Ok(next)
    }
}

pub fn raids() -> &'static [Raid] {
    static DATA: std::sync::OnceLock<Vec<Raid>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/raid-routes.json")).expect("raid room routes")
    })
}

pub fn raid(id: &str) -> Option<&'static Raid> {
    raids().iter().find(|raid| raid.id == id)
}

pub fn for_targets(targets: &[Target]) -> Option<&'static Raid> {
    raids().iter().find(|raid| raid.accepts(targets))
}
