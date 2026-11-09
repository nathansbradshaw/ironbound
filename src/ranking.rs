//! Relative weighted ranking for alternatives in one equipment slot.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Weights {
    pub dps: f64,
    pub defence: f64,
    pub prayer: f64,
    pub weight: f64,
}
impl Default for Weights {
    fn default() -> Self {
        Self {
            dps: 99.0,
            defence: 1.0,
            prayer: 0.01,
            weight: 0.0,
        }
    }
}
impl Weights {
    pub fn values(&self) -> [f64; 4] {
        [self.dps, self.defence, self.prayer, self.weight].map(|v| {
            if v.is_finite() {
                v.clamp(0.0, 100.0)
            } else {
                0.0
            }
        })
    }
    pub fn valid(&self) -> bool {
        [self.dps, self.defence, self.prayer, self.weight]
            .into_iter()
            .all(|v| v.is_finite() && (0.0..=100.0).contains(&v))
    }
    pub fn set(&mut self, key: &str, value: f64) {
        let value = if value.is_finite() {
            value.clamp(0.0, 100.0)
        } else {
            0.0
        };
        match key {
            "dps" => self.dps = value,
            "defence" => self.defence = value,
            "prayer" => self.prayer = value,
            "weight" => self.weight = value,
            _ => {}
        }
    }
    pub fn preset(name: &str) -> Option<Self> {
        Some(match name {
            "damage" => Self::default(),
            "prayer" => Self {
                dps: 50.0,
                defence: 0.0,
                prayer: 50.0,
                weight: 0.0,
            },
            "defence" => Self {
                dps: 50.0,
                defence: 50.0,
                prayer: 0.0,
                weight: 0.0,
            },
            "equal" => Self {
                dps: 1.0,
                defence: 1.0,
                prayer: 1.0,
                weight: 0.0,
            },
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Defence {
    #[default]
    Total,
    Stab,
    Slash,
    Crush,
    Ranged,
    Magic,
}
impl Defence {
    pub const ALL: [Self; 6] = [
        Self::Total,
        Self::Stab,
        Self::Slash,
        Self::Crush,
        Self::Ranged,
        Self::Magic,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Total => "total",
            Self::Stab => "stab",
            Self::Slash => "slash",
            Self::Crush => "crush",
            Self::Ranged => "ranged",
            Self::Magic => "magic",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Total => "All defence bonuses",
            Self::Stab => "Stab defence",
            Self::Slash => "Slash defence",
            Self::Crush => "Crush defence",
            Self::Ranged => "Ranged defence",
            Self::Magic => "Magic defence",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub weights: Weights,
    pub defence: Defence,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Metrics {
    pub dps: Option<f64>,
    pub defence: Option<f64>,
    pub prayer: Option<f64>,
    pub weight: Option<f64>,
}
impl Metrics {
    fn values(self) -> [Option<f64>; 4] {
        [self.dps, self.defence, self.prayer, self.weight].map(|v| v.filter(|n| n.is_finite()))
    }
}

pub struct Ranking {
    pub scores: Vec<Option<f64>>,
    /// Enabled metrics unavailable for every candidate are omitted, not treated as zero.
    pub unavailable: Vec<&'static str>,
}
/// Normalize across the full usable candidate pool before filtering search results.
/// Each enabled metric contributes its relative importance, independent of its units.
pub fn rank(metrics: &[Metrics], weights: &Weights) -> Ranking {
    let importance = weights.values();
    let mut bounds = [None; 4];
    let mut unavailable = vec![];
    for column in 0..4 {
        if importance[column] == 0.0 {
            continue;
        }
        for row in metrics {
            if let Some(value) = row.values()[column] {
                let (min, max) = bounds[column].unwrap_or((value, value));
                bounds[column] = Some((value.min(min), value.max(max)));
            }
        }
        if bounds[column].is_none() {
            unavailable.push(["DPS", "Defence", "Prayer", "Weight"][column]);
        }
    }
    let denominator: f64 = importance
        .iter()
        .zip(bounds)
        .filter_map(|(w, b)| b.map(|_| *w))
        .sum();
    let scores = metrics
        .iter()
        .map(|row| {
            if denominator == 0.0 {
                return None;
            }
            let mut total = 0.0;
            for column in 0..4 {
                let Some((min, max)) = bounds[column] else {
                    continue;
                };
                let value = row.values()[column]?;
                let normalized = if max == min {
                    1.0
                } else if column == 3 {
                    (max - value) / (max - min)
                } else {
                    (value - min) / (max - min)
                };
                total += normalized * importance[column];
            }
            Some(100.0 * total / denominator)
        })
        .collect();
    Ranking {
        scores,
        unavailable,
    }
}

pub fn item_weight(id: i32) -> Option<f64> {
    static WEIGHTS: OnceLock<BTreeMap<i32, f64>> = OnceLock::new();
    WEIGHTS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../assets/item-weights.json")).expect("item weights")
        })
        .get(&id)
        .copied()
}
pub fn equipment_weight(set: &[Option<i32>]) -> Option<f64> {
    set.iter()
        .flatten()
        .try_fold(0.0, |sum, id| Some(sum + item_weight(*id)?))
}

pub fn equipment_metrics(set: &[Option<i32>], dps: Option<f64>, kind: Defence) -> Metrics {
    let known = set
        .iter()
        .flatten()
        .all(|id| crate::combat::stats(*id).is_some());
    let bonuses = crate::combat::bonuses(set);
    Metrics {
        dps,
        defence: known.then(|| {
            if kind == Defence::Total {
                bonuses.defensive.values().sum::<i32>() as f64
            } else {
                bonuses.defensive.get(kind.key()).copied().unwrap_or(0) as f64
            }
        }),
        prayer: known.then(|| crate::prayer::bonus(set) as f64),
        weight: equipment_weight(set),
    }
}

/// Stable relative utility for comparing complete constrained loadouts. The ideal
/// style set is the fixed reference across every anchor/switch candidate.
/// Offsets keep zero/negative bonuses finite; they are scaling constants, not
/// predictions of damage prevented or run energy.
pub fn relative_utility(metrics: Metrics, reference: Metrics, weights: &Weights) -> Option<f64> {
    let current = metrics.values();
    let baseline = reference.values();
    let mut utility = 0.0;
    let mut total = 0.0;
    for (column, importance) in weights.values().into_iter().enumerate() {
        if importance == 0.0 {
            continue;
        }
        let Some(base) = baseline[column] else {
            continue;
        };
        let value = current[column]?;
        let relative = match column {
            0 => {
                if base > 0.0 {
                    value / base
                } else {
                    0.0
                }
            }
            1 => (100.0 + value).max(1.0) / (100.0 + base).max(1.0),
            2 => (30.0 + value).max(1.0) / (30.0 + base).max(1.0),
            _ => (30.0 + base).max(1.0) / (30.0 + value).max(1.0),
        };
        utility += importance * relative;
        total += importance;
    }
    (total > 0.0).then(|| (utility / total).max(0.000001))
}
