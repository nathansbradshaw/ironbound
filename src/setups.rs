//! Private, device-local configurations. Public sharing uses `share::Setup` instead.
use crate::{account::Snapshot, planner::Options};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn default_budget_caps() -> [usize; 2] {
    [4, 8]
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub snapshot: Snapshot,
    pub options: Options,
    pub pins: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub last_setup_fragment: Option<String>,
    #[serde(default)]
    pub setups: Library,
    #[serde(default = "default_budget_caps")]
    pub budget_caps: [usize; 2],
}
impl Saved {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() > 4_000_000 {
            return Err("Account backup exceeds 4 MB.".into());
        }
        let saved: Self = serde_json::from_str(raw).map_err(|_| "Could not read saved account.")?;
        if saved.version != 1
            || saved.snapshot.selected().is_none()
            || saved.budget_caps.iter().any(|cap| *cap > 11)
        {
            return Err("This backup version or account is not supported.".into());
        }
        Snapshot::parse(
            &serde_json::to_string(&saved.snapshot).map_err(|e| e.to_string())?,
            saved.snapshot.fetched_at.clone(),
        )?;
        validate_options(&saved.options)?;
        saved.setups.validate()?;
        Ok(saved)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedSetup {
    pub name: String,
    pub options: Options,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Library(pub Vec<NamedSetup>);
impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() > 64 {
            return Err("Keep at most 64 named setups.".into());
        }
        for (i, entry) in self.0.iter().enumerate() {
            self.check_name(&entry.name, Some(i))?;
            validate_options(&entry.options)?;
        }
        Ok(())
    }
    fn check_name(&self, name: &str, except: Option<usize>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err("Use a setup name of 1–64 characters.".into());
        }
        if self
            .0
            .iter()
            .enumerate()
            .any(|(i, entry)| Some(i) != except && entry.name.to_lowercase() == name.to_lowercase())
        {
            return Err(
                "That name is already used. Choose another name or replace that setup.".into(),
            );
        }
        Ok(name.into())
    }
    pub fn save(&mut self, name: &str, options: &Options) -> Result<(), String> {
        if self.0.len() >= 64 {
            return Err("Keep at most 64 named setups.".into());
        }
        let name = self.check_name(name, None)?;
        validate_options(options)?;
        self.0.push(NamedSetup {
            name,
            options: options.clone(),
        });
        Ok(())
    }
    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), String> {
        let name = self.check_name(name, Some(index))?;
        self.0.get_mut(index).ok_or("Setup no longer exists.")?.name = name;
        Ok(())
    }
    pub fn replace(&mut self, index: usize, options: &Options) -> Result<(), String> {
        validate_options(options)?;
        self.0
            .get_mut(index)
            .ok_or("Setup no longer exists.")?
            .options = options.clone();
        Ok(())
    }
}

pub fn validate_options(o: &Options) -> Result<(), String> {
    if !o.combat.valid()
        || !o.ranking.weights.valid()
        || o.food > 20
        || o.prayer_potions > 12
        || o.raid_level > 150
        || o.max_switches.is_some_and(|n| n > 11)
        || o.switch_tolerance > crate::efficient::MAX_TOLERANCE_PERCENT
        || o.damage_tolerance > crate::efficient::MAX_TOLERANCE_PERCENT
        || o.prayer_tolerance > crate::efficient::MAX_TOLERANCE_PERCENT
        || o.switch_slots > 20
        || o.targets.len() > crate::rules::MAX_TARGETS
        || o.targets.iter().any(|t| !t.valid())
        || !(o.focus_style == "Balanced" || crate::rules::STYLES.contains(&o.focus_style.as_str()))
        || o.inventory_overrides
            .iter()
            .any(|(i, id)| *i >= 28 || *id < 0 || *id > 1_000_000)
        || o.inventory_quantities
            .iter()
            .any(|(i, q)| *i >= 28 || *q == 0 || *q > i32::MAX as u32)
        || o.phase_priorities.len() > crate::rules::MAX_TARGETS
        || o.phase_priorities
            .iter()
            .any(|(id, w)| *w > 10 || crate::planner::monster(*id).is_none())
        || o.confirmations.len() > 4
        || o.confirmations.iter().any(|(id, value)| {
            !crate::preparation::CONFIRMATIONS
                .iter()
                .any(|(key, _)| *key == id)
                || value.len() != 16
                || !value.bytes().all(|b| b.is_ascii_hexdigit())
        })
        || o.gear_overrides.iter().any(|(key, id)| {
            let Some((style, slot)) = key.rsplit_once(':') else {
                return true;
            };
            let Ok(slot) = slot.parse::<usize>() else {
                return true;
            };
            !crate::rules::STYLES.contains(&style)
                || slot >= 14
                || (*id != 0
                    && crate::planner::gear(*id)
                        .is_none_or(|g| g.slot != crate::planner::SLOT_KEYS[slot]))
        })
    {
        return Err("Setup contains invalid settings. Your current account is unchanged.".into());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    Pending,
    Saved,
    Unavailable,
    Unreadable,
}
impl SaveStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Not saved yet",
            Self::Saved => "Saved on this device",
            Self::Unavailable => "Changes are not saved",
            Self::Unreadable => "Saved account could not be read",
        }
    }
}
