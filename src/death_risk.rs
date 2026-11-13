//! Packed-item replacement risk for Wilderness PvP, separate from gear screening.
//! Unknown protection, loss behaviour and contents never silently become zero.
use crate::{
    inventory,
    planner::{self, Plan},
    rules, tooltip,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};
const MAX_GP: u64 = 1_000_000_000_000;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    #[default]
    Three,
    Four,
    None,
    One,
    HighRisk,
}
impl Scenario {
    pub fn slots(self) -> u32 {
        match self {
            Self::Three => 3,
            Self::Four => 4,
            Self::One => 1,
            _ => 0,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Three => "three",
            Self::Four => "four",
            Self::None => "none",
            Self::One => "one",
            Self::HighRisk => "high_risk",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Three => "Unskulled · 3 kept",
            Self::Four => "Unskulled + Protect Item · 4",
            Self::None => "Skulled · 0 kept",
            Self::One => "Skulled + Protect Item · 1",
            Self::HighRisk => "High-risk world · 0 kept",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "three" => Self::Three,
            "four" => Self::Four,
            "none" => Self::None,
            "one" => Self::One,
            "high_risk" => Self::HighRisk,
            _ => return None,
        })
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Input {
    pub protection: Option<u64>,
    pub loss: Option<u64>,
    pub charges: Option<u32>,
    pub loaded: Option<u32>,
    pub charge_loss: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub scenario: Scenario,
    pub manual_kept: bool,
    pub kept: BTreeMap<i32, u32>,
    pub items: BTreeMap<i32, Input>,
    pub other_gp: u64,
    pub for_plan: Option<String>,
}
impl Settings {
    pub fn valid(&self) -> bool {
        self.for_plan
            .as_ref()
            .is_none_or(|key| key.len() == 16 && key.bytes().all(|c| c.is_ascii_hexdigit()))
            && self.items.len() <= 64
            && self.kept.len() <= 64
            && self.other_gp <= MAX_GP
            && self
                .kept
                .iter()
                .all(|(id, q)| *id > 0 && *id <= 1_000_000 && *q <= 4)
            && self.items.iter().all(|(id, v)| {
                *id > 0
                    && *id <= 1_000_000
                    && [v.protection, v.loss, v.charge_loss]
                        .into_iter()
                        .flatten()
                        .all(|n| n <= MAX_GP)
                    && v.charges.is_none_or(|n| n <= 1_000_000)
                    && v.loaded.is_none_or(|n| n <= 1_000_000)
            })
    }
    pub fn set(&mut self, field: &str, id: i32, raw: &str) -> bool {
        let value = if raw.trim().is_empty() {
            None
        } else {
            let Ok(n) = raw.parse::<u64>() else {
                return false;
            };
            if n > MAX_GP {
                return false;
            }
            Some(n)
        };
        if field == "other" {
            self.other_gp = value.unwrap_or(0);
            return true;
        }
        if id <= 0 || id > 1_000_000 || !self.items.contains_key(&id) && self.items.len() >= 64 {
            return false;
        }
        if ["charges", "loaded"].contains(&field) && value.is_some_and(|n| n > 1_000_000) {
            return false;
        }
        let input = self.items.entry(id).or_default();
        match field {
            "protection" => input.protection = value,
            "loss" => input.loss = value,
            "charges" => input.charges = value.map(|n| n as u32),
            "loaded" => input.loaded = value.map(|n| n as u32),
            "charge_loss" => input.charge_loss = value,
            _ => return false,
        }
        true
    }
}
#[derive(Debug, Deserialize)]
pub struct ItemValue {
    pub market: Option<u64>,
    pub protect: Option<u64>,
}
#[derive(Deserialize)]
pub struct Catalog {
    pub captured_at: String,
    pub items: BTreeMap<i32, ItemValue>,
}
pub fn values() -> &'static Catalog {
    static DATA: OnceLock<Catalog> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/death-risk-values.json"))
            .expect("public risk values")
    })
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    WithItem,
    Always,
}
#[derive(Debug, Deserialize)]
pub struct ChargeRule {
    pub ids: Vec<i32>,
    pub base: i32,
    pub label: String,
    pub max: u32,
    pub policy: Policy,
    pub components: Vec<(i32, u64, u64)>,
    #[serde(default)]
    pub loaded_ammo: bool,
    #[serde(default)]
    pub full_ids: Vec<i32>,
}
#[derive(Deserialize)]
pub struct Rules {
    pub never_kept: Vec<i32>,
    pub always_kept: Vec<i32>,
    pub charges: Vec<ChargeRule>,
}
pub fn death_rules() -> &'static Rules {
    static DATA: OnceLock<Rules> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/death-risk-rules.json"))
            .expect("reviewed death rules")
    })
}
pub fn charge_rule(id: i32) -> Option<&'static ChargeRule> {
    death_rules().charges.iter().find(|r| r.ids.contains(&id))
}
fn price(id: i32) -> Option<u64> {
    values().items.get(&id)?.market
}
fn unreviewed_contents(id: i32) -> bool {
    if charge_rule(id).is_some() {
        return false;
    }
    let name = planner::item_name(id).to_lowercase();
    if name.contains("uncharged")
        || name.contains("empty")
        || name.contains("inactive")
        || name.contains("corrupted")
    {
        return false;
    }
    rules::weapon(id).is_some_and(|r| {
        r.family.formula == rules::Formula::Powered || !r.family.internal_ammo.is_empty()
    }) || [
        "tome of",
        "scythe of vitur",
        "toxic staff",
        "ash sanctifier",
        "blood fury",
        "sanguinesti",
        "serpentine helm",
        "charged",
        "crystal bow",
        "crystal shield",
        "crystal halberd",
        "bow of faerdhinen",
        "venator bow",
        "soul bearer",
        "ring of suffering",
    ]
    .iter()
    .any(|s| name.contains(s))
}
#[derive(Clone, Debug)]
pub struct Row {
    pub id: i32,
    pub quantity: u32,
    pub kept: u32,
    pub protection: Option<u64>,
    pub unit_loss: Option<u64>,
    pub gear_loss: Option<u64>,
    pub contents_loss: Option<u64>,
    pub category: &'static str,
    pub note: String,
    pub contents: bool,
}
#[derive(Debug)]
pub struct Report {
    pub rows: Vec<Row>,
    pub kept_complete: bool,
    pub complete: bool,
    pub known_gp: u64,
    pub subtotals: BTreeMap<&'static str, u64>,
    pub issues: Vec<String>,
    pub slots: u32,
}
fn category(id: i32) -> &'static str {
    match inventory::category(id, &planner::item_name(id)) {
        inventory::Category::Equipment => "Equipment",
        inventory::Category::Ammunition => "Ammunition",
        inventory::Category::Runes => "Runes",
        _ => "Supplies & other",
    }
}
/// Phases are alternative arrangements of one carried pool and are never added again.
pub fn report(plan: &Plan, declared: &Settings) -> Report {
    let expired = declared
        .for_plan
        .as_ref()
        .is_some_and(|key| *key != plan_key(plan));
    let fallback = Settings {
        scenario: declared.scenario,
        ..Default::default()
    };
    let settings = if expired { &fallback } else { declared };
    let mut carried = Vec::<(i32, u32)>::new();
    let mut add = |id: i32, q: u32| {
        if id <= 0 || q == 0 {
            return;
        }
        if let Some(row) = carried.iter_mut().find(|(item, _)| *item == id) {
            row.1 = row.1.saturating_add(q);
        } else {
            carried.push((id, q));
        }
    };
    for (slot, id) in plan.equipment.iter().enumerate() {
        if let Some(id) = id {
            add(*id, planner::equipped_quantity(slot, *id, plan.ammo_shots));
        }
    }
    for item in &plan.inventory {
        add(item.id, item.quantity);
    }
    let mut result = Report {
        rows: vec![],
        kept_complete: true,
        complete: true,
        known_gp: settings.other_gp,
        subtotals: BTreeMap::new(),
        issues: vec![],
        slots: settings.scenario.slots(),
    };
    if settings.other_gp > 0 {
        result
            .subtotals
            .insert("Other entered GP", settings.other_gp);
    }
    if expired {
        result.issues.push(
            "Packed items changed. Reconfirm kept items and remaining contents for this trip."
                .into(),
        );
        result.kept_complete = false;
    }
    if !settings.valid() {
        result.issues.push("Risk settings are invalid.".into());
        result.kept_complete = false;
    }
    for (id, q) in carried {
        let input = settings.items.get(&id).cloned().unwrap_or_default();
        let never = death_rules().never_kept.contains(&id);
        let always = death_rules().always_kept.contains(&id);
        let rule = charge_rule(id);
        // A full, GE-tradeable variant no longer describes the carried object
        // once its declared contents differ. Do not reuse its protection price.
        let changed_full_state = rule.is_some_and(|r| {
            r.full_ids.contains(&id) && input.charges.is_some_and(|n| n != r.max.saturating_mul(q))
        });
        let protection = input.protection.or_else(|| {
            (!changed_full_state)
                .then(|| values().items.get(&id).and_then(|v| v.protect))
                .flatten()
        });
        let raw = values().items.get(&id);
        let unit_loss = input.loss.or_else(|| {
            if let Some(rule) = rule {
                price(rule.base)
            } else if raw.and_then(|v| v.protect).is_some() {
                raw.and_then(|v| v.market)
            } else {
                None
            }
        });
        let mut note = String::new();
        if never {
            note.push_str("Always lost in this PvP model. ");
        }
        if always {
            note.push_str("Automatically kept, outside the protected slots. ");
        }
        if !always && !never && protection.is_none() && result.slots > 0 && !settings.manual_kept {
            result.kept_complete = false;
            result.issues.push(format!("{}: protection value needs confirmation, or select the kept items from the in-game screen.",planner::item_name(id)));
        }
        result.rows.push(Row {
            id,
            quantity: q,
            kept: if always { q } else { 0 },
            protection,
            unit_loss,
            gear_loss: None,
            contents_loss: None,
            category: category(id),
            note,
            contents: rule.is_some() || unreviewed_contents(id),
        });
    }
    if settings.manual_kept && result.slots > 0 {
        let mut selected = 0u32;
        let mut eligible = 0u32;
        for row in &mut result.rows {
            if death_rules().always_kept.contains(&row.id)
                || death_rules().never_kept.contains(&row.id)
            {
                continue;
            }
            eligible = eligible.saturating_add(row.quantity);
            let count = settings.kept.get(&row.id).copied().unwrap_or(0);
            if count > row.quantity {
                result.kept_complete = false;
            }
            row.kept = count.min(row.quantity);
            selected = selected.saturating_add(row.kept);
        }
        if selected != result.slots.min(eligible) {
            result.kept_complete = false;
            result.issues.push(format!(
                "Confirm {} kept individual items; currently {selected} selected.",
                result.slots.min(eligible)
            ));
        }
    } else if result.slots > 0 {
        let mut order: Vec<_> = result
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                !death_rules().never_kept.contains(&r.id)
                    && !death_rules().always_kept.contains(&r.id)
                    && r.protection.is_some()
            })
            .map(|(index, row)| (index, row.protection.unwrap()))
            .collect();
        order.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut remaining = result.slots;
        for (index, _) in &order {
            let row = &mut result.rows[*index];
            let count = row.quantity.min(remaining);
            row.kept = count;
            remaining -= count;
        }
        // Exact ties at the cutoff depend on the in-game layout; never promise a winner.
        if order.windows(2).any(|pair| {
            pair[0].1 == pair[1].1
                && result.rows[pair[0].0].kept > 0
                && result.rows[pair[1].0].kept == 0
        }) {
            result.kept_complete = false;
            result.issues.push("A protection-value tie crosses the kept-item cutoff. Confirm the selection in game.".into());
        }
    }
    for row in &mut result.rows {
        let lost = row.quantity.saturating_sub(row.kept);
        row.gear_loss = if lost == 0 {
            Some(0)
        } else {
            row.unit_loss.and_then(|v| v.checked_mul(u64::from(lost)))
        };
        if let Some(gp) = row.gear_loss {
            result.known_gp = result.known_gp.saturating_add(gp);
            let subtotal = result.subtotals.entry(row.category).or_default();
            *subtotal = subtotal.saturating_add(gp);
        } else {
            result.issues.push(format!(
                "{}: enter its GP loss if not kept (replacement or repair cost).",
                planner::item_name(row.id)
            ));
        }
        let input = settings.items.get(&row.id).cloned().unwrap_or_default();
        if let Some(gp) = input.charge_loss {
            row.contents_loss = Some(gp);
            row.note.push_str("Contents GP lost supplied by you. ");
        } else if let Some(rule) = charge_rule(row.id) {
            if rule.policy == Policy::WithItem && lost == 0 {
                row.contents_loss = Some(0);
                row.note.push_str("Contents stay with the kept item. ");
            } else if row.kept > 0 && lost > 0 {
                result.issues.push(format!("{}: duplicate items are partly kept; enter the total contents GP actually lost.",planner::item_name(row.id)));
            } else {
                let count = input.charges.or_else(|| {
                    rule.full_ids
                        .contains(&row.id)
                        .then_some(rule.max.saturating_mul(row.quantity))
                });
                let mut cost = Some(0u64);
                if let Some(count) = count.filter(|n| *n <= rule.max.saturating_mul(row.quantity)) {
                    for (id, n, d) in &rule.components {
                        cost = cost.zip(price(*id)).map(|(sum, p)| {
                            sum.saturating_add(
                                p.saturating_mul(u64::from(count))
                                    .saturating_mul(*n)
                                    .div_ceil(*d),
                            )
                        });
                    }
                } else {
                    cost = None;
                    result.issues.push(format!(
                        "{}: enter remaining {} (not simulated usage).",
                        planner::item_name(row.id),
                        rule.label
                    ));
                }
                if rule.loaded_ammo {
                    let dart = plan.loaded_ammo.as_ref().map(|ammo| ammo.id);
                    match (dart, input.loaded) {
                        (Some(id), Some(count)) if count <= 16383 => {
                            cost = cost.zip(price(id)).map(|(sum, p)| {
                                sum.saturating_add(p.saturating_mul(u64::from(count)))
                            })
                        }
                        _ => {
                            cost = None;
                            result.issues.push(format!("{}: choose the loaded dart type in Combat and enter the actual loaded dart count here.",planner::item_name(row.id)));
                        }
                    }
                }
                row.contents_loss = cost;
                if rule.policy == Policy::Always {
                    row.note
                        .push_str("Ether is lost even if this item is kept. ");
                }
            }
        } else if row.contents {
            result.issues.push(format!("{}: charge/death behaviour is not reviewed. Enter total contents GP lost, including 0 only if confirmed kept.",planner::item_name(row.id)));
        } else {
            row.contents_loss = Some(0);
        }
        if let Some(gp) = row.contents_loss {
            result.known_gp = result.known_gp.saturating_add(gp);
            let subtotal = result.subtotals.entry("Charges & loaded ammo").or_default();
            *subtotal = subtotal.saturating_add(gp);
        }
    }
    if result.rows.iter().any(|r| r.contents_loss.is_none()) {
        result.issues.push(
            "Some contents or component prices remain unknown; enter their total lost GP override."
                .into(),
        );
    }
    if result
        .rows
        .iter()
        .flat_map(|r| [r.gear_loss, r.contents_loss])
        .flatten()
        .try_fold(settings.other_gp, u64::checked_add)
        .is_none()
    {
        result
            .issues
            .push("GP values exceed the supported range.".into());
    }
    result.complete = result.kept_complete
        && result.issues.is_empty()
        && result
            .rows
            .iter()
            .all(|r| r.gear_loss.is_some() && r.contents_loss.is_some());
    result
}
pub fn gp(n: u64) -> String {
    let text = n.to_string();
    let mut out = String::new();
    for (index, c) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{out} GP")
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn number(field: &str, id: i32, label: &str, value: Option<u64>, max: u64) -> String {
    format!(
        "<label>{}<input type=\"number\" id=\"risk-{field}-{id}\" min=\"0\" max=\"{max}\" step=\"1\" value=\"{}\" placeholder=\"Unknown\"></label>",
        esc(label),
        value.map(|n| n.to_string()).unwrap_or_default()
    )
}
/// The report is private device-local state; shared gear never inherits your overrides.
pub fn render(plan: &Plan, declared: &Settings, open: bool, editable: bool) -> String {
    let report = report(plan, declared);
    let expired = declared
        .for_plan
        .as_ref()
        .is_some_and(|key| *key != plan_key(plan));
    let fallback = Settings {
        scenario: declared.scenario,
        ..Default::default()
    };
    let settings = if expired { &fallback } else { declared };
    let mut html = format!(
        "<section class=\"death-risk-panel panel\" aria-label=\"Wilderness death risk\"><div class=\"panel-heading\"><h2>Kept items & death risk</h2><span>{}</span></div><div class=\"death-risk-summary\"><div><span>{}</span><strong>{}</strong><small>{}</small></div><p>Wilderness PvP · standard account · {}. Protection values and replacement prices are separate.</p></div><div class=\"death-kept\"><h3>{}</h3>",
        esc(settings.scenario.label()),
        if report.complete {
            "Total GP at risk"
        } else {
            "Known subtotal · incomplete"
        },
        gp(report.known_gp),
        if report.complete {
            "Packed equipment, supplies, runes, ammo and declared contents included."
        } else {
            "Resolve the missing values below before treating this as a total."
        },
        esc(settings.scenario.label()),
        if report.kept_complete {
            "Items kept"
        } else {
            "Kept candidates · confirmation needed"
        }
    );
    let mut kept = 0;
    for row in report.rows.iter().filter(|r| r.kept > 0) {
        kept += row.kept;
        html.push_str(&format!(
            "<div class=\"death-kept-item\">{}<span>{} ×{}<small>{}</small></span></div>",
            tooltip::item_icon(row.id),
            esc(&planner::item_name(row.id)),
            row.kept,
            if settings.manual_kept {
                "Selected by you from the in-game screen"
            } else {
                "Estimated protection order"
            }
        ));
    }
    if kept == 0 {
        html.push_str("<p>No kept items selected.</p>");
    }
    html.push_str("</div><dl class=\"death-risk-breakdown\">");
    for (label, value) in &report.subtotals {
        if *value > 0 {
            html.push_str(&format!(
                "<div><dt>{label}</dt><dd>{}</dd></div>",
                gp(*value)
            ));
        }
    }
    html.push_str("</dl>");
    html.push_str(&format!("<details id=\"death-risk-details\" {}><summary>Complete risk details · {} unresolved checks</summary><p>Snapshot: {}. Counts refer to actual starting contents, not casts/shots in the simulator. Untradeables, special death rules and equal-value ties may require in-game confirmation. UIM, Deadman, Trouver protection and grave fees are outside this PvP estimate.</p>",if open{"open"}else{""},report.issues.len(),esc(&values().captured_at)));
    if editable {
        html.push_str("<div class=\"death-scenarios\" role=\"group\" aria-label=\"Death protection scenario\">");
        for scenario in [
            Scenario::Three,
            Scenario::Four,
            Scenario::None,
            Scenario::One,
            Scenario::HighRisk,
        ] {
            html.push_str(&format!("<button data-action=\"risk-scenario:{}\" aria-pressed=\"{}\" class=\"encounter-filter {}\">{}</button>",scenario.key(),scenario==settings.scenario,if scenario==settings.scenario{"active"}else{""},scenario.label()));
        }
        html.push_str("</div>");
        html.push_str(&format!("<button data-action=\"risk-kept-mode\" aria-pressed=\"{}\" class=\"encounter-filter {}\">{}</button>",settings.manual_kept,if settings.manual_kept{"active"}else{""},if settings.manual_kept{"Use automatic protection order"}else{"Select kept items from the in-game screen"}));
    }
    if !report.issues.is_empty() {
        html.push_str("<ul class=\"death-risk-issues\">");
        for issue in &report.issues {
            html.push_str(&format!("<li>{}</li>", esc(issue)));
        }
        html.push_str("</ul>");
    }
    html.push_str("<div class=\"death-risk-rows\">");
    for row in &report.rows {
        let input = settings.items.get(&row.id).cloned().unwrap_or_default();
        let lost = row.quantity.saturating_sub(row.kept);
        html.push_str(&format!("<article class=\"death-risk-row\"><div class=\"death-risk-row-heading\">{}<strong>{} ×{}</strong><span>Kept {} · lost {lost}</span></div><p>{} · base loss: {} · contents loss: {}</p><small>{}</small>",tooltip::item_icon(row.id),esc(&planner::item_name(row.id)),row.quantity,row.kept,row.category,row.gear_loss.map(gp).unwrap_or_else(||"Unknown".into()),row.contents_loss.map(gp).unwrap_or_else(||"Unknown".into()),esc(&row.note)));
        if editable {
            if settings.manual_kept
                && !death_rules().never_kept.contains(&row.id)
                && !death_rules().always_kept.contains(&row.id)
            {
                html.push_str(&format!("<button data-action=\"risk-keep:{}\" class=\"text-button\">Keep {} (click to change)</button>",row.id,settings.kept.get(&row.id).copied().unwrap_or(0)));
            }
            html.push_str("<div class=\"death-risk-inputs\">");
            if row.protection.is_none() || input.protection.is_some() {
                html.push_str(&number(
                    "protection",
                    row.id,
                    "Optional auto-order protection value / item (GP)",
                    input.protection,
                    MAX_GP,
                ));
            }
            if row.unit_loss.is_none() || input.loss.is_some() {
                html.push_str(&number(
                    "loss",
                    row.id,
                    "Base GP loss per unkept item (excluding contents)",
                    input.loss,
                    MAX_GP,
                ));
            }
            if let Some(rule) = charge_rule(row.id) {
                html.push_str(&number(
                    "charges",
                    row.id,
                    &format!(
                        "{} remaining (max {})",
                        rule.label,
                        rule.max.saturating_mul(row.quantity)
                    ),
                    input.charges.map(u64::from).or_else(|| {
                        rule.full_ids
                            .contains(&row.id)
                            .then_some(u64::from(rule.max.saturating_mul(row.quantity)))
                    }),
                    u64::from(rule.max.saturating_mul(row.quantity)),
                ));
                if rule.loaded_ammo {
                    html.push_str(&number(
                        "loaded",
                        row.id,
                        "Actual darts inside this blowpipe",
                        input.loaded.map(u64::from),
                        16383,
                    ));
                }
            }
            if row.contents {
                html.push_str(&number(
                    "charge_loss",
                    row.id,
                    "Override: total contents GP LOST on death",
                    input.charge_loss,
                    MAX_GP,
                ));
            }
            html.push_str("</div>");
        }
        html.push_str("</article>");
    }
    html.push_str("</div>");
    if editable {
        html.push_str(&number(
            "other",
            0,
            "Other GP at risk (entry fees, bag contents or new loot)",
            Some(settings.other_gp),
            MAX_GP,
        ));
        html.push_str("<button class=\"text-button\" data-action=\"risk-reset\">Clear risk confirmations & charge counts</button>");
    }
    html.push_str("<p>Confirm the kept selection in-game immediately before leaving. A declared zero is different from an unknown value. Stackable quantities are priced in full, but protection slots apply to individual items. Phases reuse one carried pool. New loot and bag/pouch contents must be packed or added as other GP; unentered contents are not inferred.</p></details></section>");
    html
}

pub fn plan_key(plan: &Plan) -> String {
    let raw = serde_json::to_vec(&(
        &plan.equipment,
        &plan.inventory,
        &plan.loaded_ammo,
        plan.ammo_shots,
    ))
    .unwrap_or_default();
    let mut hash = 0xcbf29ce484222325u64;
    for b in raw {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
impl Settings {
    pub fn bind(&mut self, plan: &Plan) {
        let key = plan_key(plan);
        if self.for_plan.as_ref() != Some(&key) {
            let scenario = self.scenario;
            *self = Self::default();
            self.scenario = scenario;
            self.for_plan = Some(key);
        }
    }
}
