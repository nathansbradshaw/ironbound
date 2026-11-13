//! Independent golden values transcribed from the pinned Wiki test suite.
//! Keep expected values in fixtures, never regenerate them from this engine.
use crate::{account, combat, defence, planner};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub struct ReferenceSuite {
    pub revision: String,
    pub source: String,
    #[serde(default)]
    pub live_revision: Option<String>,
    pub cases: Vec<ReferenceCase>,
}
#[derive(Deserialize)]
pub struct ReferenceCase {
    pub name: String,
    pub level: u32,
    #[serde(default)]
    pub levels: BTreeMap<String, u32>,
    #[serde(default)]
    pub target: Option<i32>,
    pub equipment: BTreeMap<usize, i32>,
    pub style: String,
    #[serde(default)]
    pub ranged_stance: Option<String>,
    #[serde(default)]
    pub prayer: Option<String>,
    #[serde(default)]
    pub kandarin_diary: bool,
    #[serde(default)]
    pub slayer_task: bool,
    #[serde(default)]
    pub attack_roll: Option<i64>,
    #[serde(default)]
    pub defence_roll: Option<i64>,
    #[serde(default)]
    pub dps: Option<f64>,
    #[serde(default)]
    pub accuracy: Option<f64>,
    pub max_hit: i32,
}
impl ReferenceCase {
    pub fn check(&self) -> Result<combat::ResultStats, String> {
        let mut m = account::Member::default();
        for skill in account::SKILLS {
            m.level_overrides.insert(skill.into(), self.level);
        }
        m.level_overrides.extend(self.levels.clone());
        let mut set = vec![None; 14];
        for (slot, id) in &self.equipment {
            if *slot >= 14 {
                return Err(format!("{}: invalid equipment slot", self.name));
            }
            set[*slot] = Some(*id);
        }
        let mut settings = defence::Settings::default();
        settings.kandarin_diary = self.kandarin_diary;
        if self.slayer_task {
            settings.slayer_targets.insert(self.target.unwrap_or(1673));
        }
        if let Some(stance) = &self.ranged_stance {
            settings.ranged_stance = stance.clone();
        }
        if let Some(prayer) = &self.prayer {
            settings.offensive = prayer.clone();
        }
        let target =
            planner::monster(self.target.unwrap_or(1673)).ok_or("Missing reference NPC")?;
        let actual = combat::calculate_configured(&m, &set, &self.style, target, "", 0, &settings)?;
        if !actual.dps.is_finite()
            || !actual.accuracy.is_finite()
            || actual.max_hit != self.max_hit
            || self
                .attack_roll
                .is_some_and(|expected| actual.attack_roll != expected)
            || self
                .defence_roll
                .is_some_and(|expected| actual.defence_roll != expected)
            || self
                .dps
                .is_some_and(|expected| (actual.dps - expected).abs() > 0.000500000001)
            || self
                .accuracy
                .is_some_and(|expected| (actual.accuracy - expected).abs() > 0.000050000001)
        {
            return Err(format!(
                "{}: max {} expected {}; attack {} expected {:?}; defence {} expected {:?}; DPS {:.9} expected {:?}; accuracy {:.9} expected {:?}",
                self.name,
                actual.max_hit,
                self.max_hit,
                actual.attack_roll,
                self.attack_roll,
                actual.defence_roll,
                self.defence_roll,
                actual.dps,
                self.dps,
                actual.accuracy,
                self.accuracy
            ));
        }
        Ok(actual)
    }
}
