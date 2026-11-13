use crate::planner::{self, Options, Plan};
use serde_json::{Value, json};

fn encounter_name(options: &Options) -> &str {
    crate::encounters::catalog()
        .iter()
        .find(|entry| entry.matches(&options.targets))
        .filter(|_| options.boss == crate::planner::Boss::Custom)
        .map(|entry| entry.name.as_str())
        .unwrap_or_else(|| options.boss.name())
}

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
        .chain(plan.loaded_ammo.iter().map(|ammo| ammo.id))
        .collect();
    let mut fields = vec![
        "banktags".into(),
        "1".into(),
        format!("Ironbound - {}", encounter_name(options)),
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
            id.map(|id| item(id, planner::equipped_quantity(slot, id, plan.ammo_shots)))
                .unwrap_or(Value::Null)
        })
        .collect();
    let mut layout: Vec<i32> = plan.equipment.iter().flatten().copied().collect();
    layout.extend(plan.inventory.iter().filter(|p| p.id > 0).map(|p| p.id));
    let mut notes = format!(
        "Ironbound / {}. {}. {}. Room plan: {}. Mark of Darkness: {}. Ranged stance: {}. Kandarin bolt bonus: {}. Slayer task targets: {}. Verify spellbook, charges and supplies. No automatic game actions.",
        encounter_name(options),
        plan.label,
        plan.spell,
        plan.phases
            .iter()
            .map(|p| format!("{}: {}", p.name, p.style))
            .collect::<Vec<_>>()
            .join("; "),
        if options.combat.mark_darkness {
            "active"
        } else {
            "inactive"
        },
        options.combat.ranged_stance,
        if options.combat.kandarin_diary {
            "active"
        } else {
            "inactive"
        },
        if options.combat.slayer_targets.is_empty() {
            "None".into()
        } else {
            options
                .combat
                .slayer_targets
                .iter()
                .map(|id| {
                    crate::planner::monster(*id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| id.to_string())
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    notes.push_str(&format!(
        " Ranged ammunition type: {}.",
        options.combat.ranged_type.label()
    ));
    if options.exclude_void {
        notes.push_str(" Void armour excluded by preference.");
    }
    if let Some(ammo) = &plan.loaded_ammo {
        notes.push_str(&format!(" Loaded weapon: {} × {} inside the blowpipe; verify contents and charges. Not worn ammo or inventory.",crate::planner::item_name(ammo.id),ammo.quantity));
    }
    for (id, hp) in &options.combat.target_hp {
        notes.push_str(&format!(
            " Target HP assumption: {} = {hp}.",
            crate::planner::monster(*id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| id.to_string())
        ));
    }
    if std::iter::once(&plan.equipment)
        .chain(plan.phases.iter().map(|p| &p.equipment))
        .any(|set| {
            ["dharok", "guthan", "verac", "torag", "karil", "ahrim"]
                .iter()
                .any(|effect| crate::rules::set_effect(set, effect))
        })
    {
        notes.push_str(&format!(" Dharok's current Hitpoints assumption: {}. Guthan's healing potential excludes overheal and is not credited as damage reduction; recoil and NPC stat-drain effects are not included.", options.combat.current_hitpoints.map(|hp|hp.to_string()).unwrap_or_else(||"full health".into())));
    }
    notes.push_str(&format!(" Spell budget: {} casts total across selected attack spells; rune stacks cover the worst-case mix. Ammunition budget: {} projectiles per ranged weapon. These are packing counts, not a duration or kill-count prediction; recovered projectiles and remaining charges need manual verification. {} User confirmations stay private and must be repeated by the recipient.", options.combat.spell_casts, plan.ammo_shots, crate::coverage::export_note(options)));
    let equipment_notes: std::collections::BTreeSet<_> = std::iter::once(&plan.equipment)
        .chain(plan.phases.iter().map(|phase| &phase.equipment))
        .flat_map(|set| crate::equipment_effects::notes(set, &options.combat))
        .collect();
    for note in equipment_notes {
        notes.push(' ');
        notes.push_str(&note);
    }
    let buffs = [
        &options.combat.boosts.melee,
        &options.combat.boosts.ranged,
        &options.combat.boosts.magic,
        &options.combat.boosts.defence,
        &options.combat.boosts.encounter,
    ]
    .into_iter()
    .filter(|id| id.as_str() != "None")
    .cloned()
    .collect::<Vec<_>>();
    if !buffs.is_empty() {
        notes.push_str(&format!(
            " Active peak boosts: {}. Potion timing/decay excluded.",
            buffs.join(", ")
        ));
    }
    for (id, r) in &options.combat.reductions {
        if !r.is_empty() {
            notes.push_str(&format!(
                " Successful reductions against {}: {}.",
                crate::planner::monster(*id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| id.to_string()),
                serde_json::to_string(r).unwrap_or_default()
            ));
        }
    }
    serde_json::to_string(&json!({"setup":{"name":format!("Ironbound - {}",encounter_name(options)),"inv":inv,"eq":eq,"afi":{},"notes":notes,"hc":"#FFFF0000","hd":true,"fb":true,"uh":false,"sb":if options.boss==crate::planner::Boss::Barrows{0}else{4},"iId":plan.equipment[3]},"layout":layout})).map_err(|e|e.to_string())
}
