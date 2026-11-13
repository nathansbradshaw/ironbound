//! Compare independently recorded first fights with numerical scenarios.
//! Missing observations stay missing. Training evidence never validates itself.
use crate::simulation::{self, Encounter, Model, Usage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 4_000_000;
pub const LIMITATION: &str = "Recorded evidence is user annotated, not independently authenticated. Agreement is a diagnostic, not gameplay certification. Defaults remain illustrative; no settings are changed automatically.";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    pub version: u32,
    pub model_revision: u32,
    /// The exact numerical setup exported before observing the fight.
    pub inputs: Model,
    pub observations: Vec<Observation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Training,
    Holdout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Completed,
    Died,
    Teleported,
    Timeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Video,
    ClientLog,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub kind: EvidenceKind,
    /// Public URL or local recording/log path; never fetched by this module.
    pub reference: String,
    pub recorded_on: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub uncertainty_ticks: u32,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub split: Split,
    pub evidence: Evidence,
    /// Stats, boosts, prayers, worn/carried sets, room-entry HP/Prayer, and
    /// solo/normal rules including raid level and Warden route were checked.
    pub setup_verified: bool,
    /// Specials, thralls, venom, teammate damage, invocation differences, etc.
    pub exclusions: Vec<String>,
    pub outcome: Outcome,
    pub ticks: u32,
    /// Only quantities actually observed. No implicit zeros.
    pub metrics: BTreeMap<String, f64>,
    /// Directly measured controls, not reverse-fitted from kill duration.
    pub timings: BTreeMap<String, Vec<u32>>,
    pub exposed_ticks: Option<u32>,
    /// Excludes ruby self-cost, glyphs, flares, poison and failed mechanics.
    pub residual_damage: Option<f64>,
    /// Ready attack slots while a target is reachable/vulnerable, excluding
    /// eating/casts/travel. Not every elapsed tick and not hits landed.
    pub eligible_opportunities: Option<u32>,
    pub used_opportunities: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub observation: String,
    pub split: Split,
    pub metric: String,
    pub observed: f64,
    pub predicted_p10: f64,
    pub predicted_median: f64,
    pub predicted_p90: f64,
    /// Predicted median minus observation. Positive means overprediction.
    pub error: f64,
    pub inside_predictive_interval: bool,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub split: Split,
    pub metric: String,
    pub samples: usize,
    pub mean_error: f64,
    pub mean_absolute_error: f64,
    pub interval_coverage: f64,
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    pub setting: String,
    pub current: u32,
    pub proposed: f64,
    pub allowed_min: u32,
    pub allowed_max: u32,
    pub within_bounds: bool,
    pub training_runs: usize,
    pub measurements: usize,
    pub observed_min: f64,
    pub observed_max: f64,
}

#[derive(Debug, Serialize)]
pub struct Completion {
    pub split: Split,
    pub attempts: usize,
    pub completed: usize,
    pub deaths: usize,
    pub teleports: usize,
    pub timeouts: usize,
    pub observed_rate: Option<f64>,
    pub wilson_95: Option<(f64, f64)>,
    pub predicted_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u32,
    pub encounter: Encounter,
    pub status: String,
    pub accepted: usize,
    pub skipped: BTreeMap<String, String>,
    pub comparisons: Vec<Comparison>,
    pub summaries: Vec<Summary>,
    pub proposals: Vec<Proposal>,
    pub completion: Vec<Completion>,
    pub notes: Vec<String>,
}

pub fn worksheet(model: &Model) -> Dataset {
    Dataset {
        version: 1,
        model_revision: simulation::MODEL_REVISION,
        inputs: model.clone(),
        observations: vec![],
    }
}

pub fn parse(raw: &str) -> Result<Dataset, String> {
    if raw.len() > MAX_BYTES {
        return Err("Gameplay worksheet exceeds 4 MB.".into());
    }
    serde_json::from_str(raw).map_err(|e| format!("Invalid gameplay worksheet: {e}"))
}

/// Require the exact setup, including numerical distributions, supplies and
/// execution assumptions. Notes are descriptive, not part of the experiment.
pub fn same_inputs(a: &Model, b: &Model) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.notes.clear();
    b.notes.clear();
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

pub fn capture_notes(encounter: Encounter) -> &'static [&'static str] {
    match encounter {
        Encounter::Vorkath => &[
            "Count six normal attacks between alternating acid and freeze specials. Mark special start/end ticks separately from Crumble Undead's cast.",
            "Record acid attack opportunities, utility runes, ruby self-damage and prayer reactivation. Projectiles fired and ammunition lost are different quantities.",
            "Keep protection, boosts and ammunition choice fixed. A mid-fight ruby-to-diamond swap, special attack, venom or passive healing needs a separate model.",
        ],
        Encounter::Yama => &[
            "Use normal solo Yama. Mark both Judge trips, Magic-first alternation, individual flare spawns/deadlines and the final phase separately.",
            "Record glyph visits, final melee-range autos, Mark recasts and flare explosions. Judge self-damage from lured fireballs and perfect-dodge haste are excluded effects.",
            "Attribute residual incoming damage separately from ruby cost, glyphs, flares, poison and failed mechanics. Otherwise leave residual damage unknown.",
        ],
        Encounter::Wardens => &[
            "Start the record at Wardens room entry. Confirm solo raid level 0–150, the P2 Warden and every invocation; earlier rooms are excluded.",
            "Mark obelisk, shield windows, every core exposure with starting body HP, skull sets and the one enrage transition.",
            "Separate skull damage, failed hazards, self-damage and residual incoming damage. Specials, thralls, ambrosia and preceding-room supply use need separate support.",
        ],
    }
}

fn evidence_key(reference: &str) -> String {
    let reference = reference.trim().split('#').next().unwrap_or_default();
    // Timestamp/share aliases must not duplicate the same YouTube recording.
    if let Some(rest) = reference.strip_prefix("https://youtu.be/") {
        return format!("youtube:{}", rest.split('?').next().unwrap_or_default());
    }
    if reference.starts_with("https://www.youtube.com/watch?")
        || reference.starts_with("https://youtube.com/watch?")
    {
        if let Some(id) = reference
            .split('?')
            .nth(1)
            .and_then(|q| q.split('&').find_map(|v| v.strip_prefix("v=")))
        {
            return format!("youtube:{id}");
        }
    }
    reference.into()
}

fn phases(e: Encounter) -> &'static [&'static str] {
    match e {
        Encounter::Vorkath => &["Normal attacks", "Acid", "Freeze / spawn"],
        Encounter::Yama => &[
            "Yama P1/P2",
            "Yama final",
            "Yama transition",
            "Judge travel",
            "Judge alternation",
            "Return from Judge",
        ],
        Encounter::Wardens => &[
            "Obelisk",
            "P1 to P2",
            "P2 shield",
            "Core ejection",
            "Core exposure",
            "Core return",
            "P2 to P3",
            "P3",
            "Skulls",
            "Enrage transition",
            "Enrage",
        ],
    }
}

pub fn metric_label(key: &str) -> String {
    if let Some(phase) = key.strip_prefix("phase:") {
        return format!("{phase} (ticks)");
    }
    for (prefix, label) in [("resource:", "consumed"), ("charge:", "charges")] {
        if let Some(id) = key.strip_prefix(prefix).and_then(|v| v.parse::<i32>().ok()) {
            return format!("{} {label}", crate::planner::item_name(id));
        }
    }
    match key {
        "ticks" => "Duration (ticks)",
        "attacks" => "Attacks",
        "casts" => "Rune casts",
        "shots" => "Projectiles fired",
        "food_bites" => "Food bites",
        "prayer_points" => "Prayer points drained",
        "prayer_doses" => "Prayer doses",
        "damage_taken" => "Damage taken (HP)",
        "ruby_damage" => "Ruby self-damage (HP)",
        "scales" => "Blowpipe scales",
        "crumble" => "Crumble Undead casts",
        "mark" => "Mark of Darkness casts",
        "acid" => "Acid specials",
        "freeze" => "Freeze specials",
        "judges" => "Judges",
        "flares" => "Flares spawned",
        "exploded" => "Flares exploded",
        "core_downs" => "Core exposures",
        "skulls" => "Skull sets",
        "enrage" => "Enrage transitions",
        "healing" => "Boss HP healed",
        _ => key,
    }
    .into()
}

fn timing_keys(e: Encounter) -> &'static [&'static str] {
    match e {
        Encounter::Vorkath => &["acid_ticks", "freeze_ticks"],
        Encounter::Yama => &[
            "judge_travel",
            "flare_interval",
            "flare_charge",
            "flare_regen",
        ],
        Encounter::Wardens => &["prayer_rotation", "skull_ticks"],
    }
}

fn metric(u: &Usage, key: &str, encounter: Encounter) -> Option<f64> {
    Some(match key {
        "ticks" => u.ticks as f64,
        "attacks" => u.attacks as f64,
        "casts" => u.casts as f64,
        "shots" => u.shots as f64,
        "food_bites" => u.food_bites as f64,
        "prayer_points" => u.prayer_points,
        "prayer_doses" => u.prayer_doses as f64,
        "damage_taken" => u.damage_taken,
        "ruby_damage" => u.ruby_damage,
        "scales" => u.scales as f64,
        "crumble" => u.crumble as f64,
        "mark" => u.mark as f64,
        "acid" => u.acid as f64,
        "freeze" => u.freeze as f64,
        "judges" => u.judges as f64,
        "flares" => u.flares as f64,
        "exploded" => u.exploded as f64,
        "core_downs" => u.core_downs as f64,
        "skulls" => u.skulls as f64,
        "enrage" => u.enrage as f64,
        "healing" => u.healing as f64,
        _ => {
            if let Some(phase) = key
                .strip_prefix("phase:")
                .filter(|p| phases(encounter).contains(p))
            {
                *u.phase_ticks.get(phase).unwrap_or(&0) as f64
            } else if let Some(id) = key
                .strip_prefix("resource:")
                .and_then(|id| id.parse::<i32>().ok())
                .filter(|id| *id > 0)
            {
                *u.resources.get(&id).unwrap_or(&0) as f64
            } else if let Some(id) = key
                .strip_prefix("charge:")
                .and_then(|id| id.parse::<i32>().ok())
                .filter(|id| *id > 0)
            {
                *u.charges.get(&id).unwrap_or(&0) as f64
            } else {
                return None;
            }
        }
    })
}

fn valid_observation(o: &Observation, e: Encounter) -> bool {
    let evidence = &o.evidence;
    let pair = |a: Option<u32>, b: Option<u32>| match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a > 0 && a <= 100_000 && b <= a,
        _ => false,
    };
    !o.id.trim().is_empty()
        && o.id.len() <= 100
        && !evidence.reference.trim().is_empty()
        && evidence.reference.len() <= 2000
        && evidence.recorded_on.len() == 10
        && evidence.notes.len() <= 4000
        && evidence.start_seconds.is_finite()
        && evidence.start_seconds >= 0.0
        && evidence.end_seconds.is_finite()
        && evidence.end_seconds > evidence.start_seconds
        && evidence.end_seconds <= 100_000_000.0
        && evidence.uncertainty_ticks <= 100
        && (1..=6000).contains(&o.ticks)
        && o.metrics.len() <= 256
        && o.metrics.iter().all(|(k, v)| {
            k != "ticks"
                && v.is_finite()
                && (0.0..=10_000_000.0).contains(v)
                && metric(&Usage::default(), k, e).is_some()
        })
        && o.exclusions.len() <= 32
        && o.exclusions.iter().all(|s| s.len() <= 500)
        && o.timings.len() <= 8
        && o.timings.iter().all(|(k, v)| {
            timing_keys(e).contains(&k.as_str())
                && !v.is_empty()
                && v.len() <= 128
                && v.iter().all(|n| *n <= 1000)
        })
        && pair(o.eligible_opportunities, o.used_opportunities)
        && match (o.exposed_ticks, o.residual_damage) {
            (None, None) => true,
            (Some(t), Some(d)) => {
                t > 0 && t <= o.ticks && d.is_finite() && (0.0..=100_000.0).contains(&d)
            }
            _ => false,
        }
}

fn quantile(values: &mut [f64], p: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() - 1) as f64 * p).ceil() as usize]
}

pub fn compare(data: &Dataset) -> Result<Report, String> {
    if data.version != 1
        || data.model_revision != simulation::MODEL_REVISION
        || data.observations.len() > 256
    {
        return Err(
            "Unsupported worksheet/model revision or more than 256 recorded attempts.".into(),
        );
    }
    // Calibration compares first fights; trip continuations cannot add evidence.
    let mut model = data.inputs.clone();
    model.settings.trip_limit = 1;
    let prediction = simulation::simulate(&model)?;
    let encounter = data.inputs.encounter;
    let mut ids = BTreeSet::new();
    let mut ranges: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    let mut accepted = vec![];
    let mut skipped = BTreeMap::new();
    for o in &data.observations {
        if !valid_observation(o, encounter) || !ids.insert(&o.id) {
            return Err(format!("Invalid or duplicate recorded attempt: {}", o.id));
        }
        // A duplicate or overlapping clip must never count as new evidence,
        // especially if it was copied into the holdout split.
        let previous = ranges
            .entry(evidence_key(&o.evidence.reference))
            .or_default();
        if previous
            .iter()
            .any(|(s, e)| o.evidence.start_seconds < *e && o.evidence.end_seconds > *s)
        {
            return Err(format!("Overlapping evidence spans: {}", o.id));
        }
        previous.push((o.evidence.start_seconds, o.evidence.end_seconds));
        if !o.setup_verified || !o.exclusions.is_empty() {
            skipped.insert(
                o.id.clone(),
                if !o.setup_verified {
                    "Setup/rules not verified"
                } else {
                    "Unsupported effects or mismatched rules"
                }
                .into(),
            );
        } else {
            accepted.push(o);
        }
    }
    let mut comparisons = vec![];
    for o in &accepted {
        // Death/escape durations are right-censored, never mixed into
        // completed-only time/consumption distributions.
        if o.outcome != Outcome::Completed {
            continue;
        }
        for (key, observed) in std::iter::once(("ticks", o.ticks as f64))
            .chain(o.metrics.iter().map(|(k, v)| (k.as_str(), *v)))
        {
            let mut samples: Vec<_> = prediction
                .runs
                .iter()
                .filter(|r| r.completed)
                .filter_map(|r| metric(&r.usage, key, encounter))
                .collect();
            if samples.is_empty() {
                continue;
            }
            let low = quantile(&mut samples, 0.1);
            let middle = quantile(&mut samples, 0.5);
            let high = quantile(&mut samples, 0.9);
            let uncertainty = if key == "ticks" || key.starts_with("phase:") {
                o.evidence.uncertainty_ticks as f64
            } else {
                0.0
            };
            comparisons.push(Comparison {
                observation: o.id.clone(),
                split: o.split,
                metric: key.into(),
                observed,
                predicted_p10: low,
                predicted_median: middle,
                predicted_p90: high,
                error: middle - observed,
                inside_predictive_interval: observed + uncertainty >= low
                    && observed - uncertainty <= high,
            });
        }
    }
    let mut summaries = vec![];
    let mut completion = vec![];
    for split in [Split::Training, Split::Holdout] {
        let keys: BTreeSet<_> = comparisons
            .iter()
            .filter(|r| r.split == split)
            .map(|r| r.metric.as_str())
            .collect();
        for key in keys {
            let rows: Vec<_> = comparisons
                .iter()
                .filter(|r| r.split == split && r.metric == key)
                .collect();
            summaries.push(Summary {
                split,
                metric: key.into(),
                samples: rows.len(),
                mean_error: rows.iter().map(|r| r.error).sum::<f64>() / rows.len() as f64,
                mean_absolute_error: rows.iter().map(|r| r.error.abs()).sum::<f64>()
                    / rows.len() as f64,
                interval_coverage: rows.iter().filter(|r| r.inside_predictive_interval).count()
                    as f64
                    / rows.len() as f64,
            });
        }
        let rows: Vec<_> = accepted.iter().filter(|o| o.split == split).collect();
        let n = rows.len();
        let completed = rows
            .iter()
            .filter(|o| o.outcome == Outcome::Completed)
            .count();
        let rate = completed as f64 / n.max(1) as f64;
        let wilson = if n == 0 {
            None
        } else {
            let z = 1.959963984540054;
            let denominator = 1.0 + z * z / n as f64;
            let centre = (rate + z * z / (2.0 * n as f64)) / denominator;
            let width = z
                * (rate * (1.0 - rate) / n as f64 + z * z / (4.0 * (n * n) as f64)).sqrt()
                / denominator;
            Some(((centre - width).max(0.0), (centre + width).min(1.0)))
        };
        completion.push(Completion {
            split,
            attempts: n,
            completed,
            deaths: rows.iter().filter(|o| o.outcome == Outcome::Died).count(),
            teleports: rows
                .iter()
                .filter(|o| o.outcome == Outcome::Teleported)
                .count(),
            timeouts: rows
                .iter()
                .filter(|o| o.outcome == Outcome::Timeout)
                .count(),
            observed_rate: (n > 0).then_some(rate),
            wilson_95: wilson,
            predicted_rate: prediction.completed() as f64 / prediction.runs.len() as f64,
        });
    }
    let training: Vec<_> = accepted
        .iter()
        .filter(|o| o.split == Split::Training)
        .collect();
    let mut proposals = vec![];
    for (key, _, current, min, max) in data.inputs.settings.fields() {
        if !timing_keys(encounter).contains(&key) && key != "incoming" && key != "uptime" {
            continue;
        }
        let mut values = vec![];
        let mut runs = 0;
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        for o in &training {
            let measured = if key == "incoming" {
                if let (Some(t), Some(d)) = (o.exposed_ticks, o.residual_damage) {
                    numerator += d * 100.0;
                    denominator += t as f64;
                    vec![d * 100.0 / t as f64]
                } else {
                    vec![]
                }
            } else if key == "uptime" {
                if let (Some(n), Some(used)) = (o.eligible_opportunities, o.used_opportunities) {
                    numerator += used as f64 * 100.0;
                    denominator += n as f64;
                    vec![used as f64 * 100.0 / n as f64]
                } else {
                    vec![]
                }
            } else {
                o.timings
                    .get(key)
                    .map(|v| v.iter().map(|n| *n as f64).collect())
                    .unwrap_or_default()
            };
            runs += usize::from(!measured.is_empty());
            values.extend(measured);
        }
        if values.is_empty() {
            continue;
        }
        values.sort_by(f64::total_cmp);
        let low = values[0];
        let high = values[values.len() - 1];
        let proposed = if denominator > 0.0 {
            numerator / denominator
        } else {
            quantile(&mut values, 0.5)
        };
        proposals.push(Proposal {
            setting: key.into(),
            current,
            proposed,
            allowed_min: min,
            allowed_max: max,
            within_bounds: proposed >= min as f64 && proposed <= max as f64,
            training_runs: runs,
            measurements: values.len(),
            observed_min: low,
            observed_max: high,
        });
    }
    let holdouts = accepted
        .iter()
        .filter(|o| o.split == Split::Holdout)
        .count();
    Ok(Report {version:1,encounter,status:if accepted.is_empty(){"Awaiting matched gameplay recordings"}else if holdouts==0{"Training evidence only; independent holdout required"}else{"Recorded comparison available; gameplay acceptance still requires review"}.into(),accepted:accepted.len(),skipped,comparisons,summaries,proposals,completion,notes:vec![LIMITATION.into(),"First-fight comparisons only. Failed/escaped attempts remain in completion counts; their censored times and consumption do not enter completed-only percentiles. Unknown quantities are omitted, not zero.".into(),"Timing uncertainty widens only timing comparisons. Predictive 10th–90th percentile coverage is descriptive, not a confidence interval. Small samples, clips from the same session, edited videos and selected successful kills can bias results.".into(),"Proposals use directly measured training observations only. Applying a proposal requires a new worksheet and new prospective holdout attempts; existing holdouts cannot validate settings selected after seeing their results.".into(),"No global defaults or account data are changed. A matching JSON setup is an annotator's assertion, not proof that the recording used it.".into()]})
}
