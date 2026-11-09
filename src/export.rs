use crate::planner::{Options, Plan};
use serde_json::{Value, json};

/// RuneLite Bank Tags v1 CSV. Filtering only; quantities belong in Inventory Setups.
pub fn bank_tags(plan: &Plan, options: &Options) -> Result<String, String> {
    if !plan.exportable() {
        return Err("Resolve setup blockers before exporting.".into());
    }
    let ids: std::collections::BTreeSet<_> = plan
        .equipment
        .iter()
        .flatten()
        .copied()
        .chain(plan.inventory.iter().filter(|p| p.id > 0).map(|p| p.id))
        .collect();
    let mut fields = vec![
        "banktags".into(),
        "1".into(),
        format!("Ironbound - {}", options.boss.name()),
        plan.equipment[3].unwrap_or(952).to_string(),
    ];
    fields.extend(ids.into_iter().map(|id| id.to_string()));
    Ok(fields.join(","))
}

/// InventorySetupPortable -> InventorySetupSerializable, as used by the plugin's Import setup button.
pub fn inventory_setup(plan: &Plan, options: &Options, _account: &str) -> Result<String, String> {
    if !plan.exportable() {
        return Err(
            "Resolve missing equipment, skill checks and inventory capacity before exporting."
                .into(),
        );
    }
    let item = |id: i32, q: u32| json!({"id":id,"q":q});
    let mut inv: Vec<Value> = plan
        .inventory
        .iter()
        .map(|p| {
            if p.id > 0 && p.quantity > 0 {
                item(p.id, p.quantity)
            } else {
                Value::Null
            }
        })
        .collect();
    inv.resize(28, Value::Null);
    let eq: Vec<Value> = plan
        .equipment
        .iter()
        .enumerate()
        .map(|(slot, id)| {
            id.map(|id| item(id, if slot == 13 { 100 } else { 1 }))
                .unwrap_or(Value::Null)
        })
        .collect();
    let mut layout: Vec<i32> = plan.equipment.iter().flatten().copied().collect();
    layout.extend(plan.inventory.iter().filter(|p| p.id > 0).map(|p| p.id));
    let notes = format!(
        "Ironbound / {}. {}. {}. Room plan: {}. Starter catalog; verify charges and supplies. No automatic game actions.",
        options.boss.name(),
        plan.label,
        plan.spell,
        plan.phases
            .iter()
            .map(|p| format!("{}: {}", p.name, p.style))
            .collect::<Vec<_>>()
            .join("; ")
    );
    serde_json::to_string(&json!({"setup":{"name":format!("Ironbound - {}",options.boss.name()),"inv":inv,"eq":eq,"afi":{},"notes":notes,"hc":"#FFFF0000","hd":true,"fb":true,"uh":false,"sb":if options.boss==crate::planner::Boss::Barrows{0}else{4},"iId":plan.equipment[3]},"layout":layout})).map_err(|e|e.to_string())
}
