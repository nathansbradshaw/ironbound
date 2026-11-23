//! Short, account-private boat setup wizard. All choices use OSRS icon boxes.
use crate::{
    equipment_view::escape as esc,
    sailing::{self, Boat, Settings},
    sailing_owned::Draft,
    tooltip::item_icon,
};
fn phase(action: &str, label: &str) -> String {
    format!(
        "<button type=\"button\" class=\"phase\" data-action=\"{}\">{}</button>",
        esc(action),
        esc(label)
    )
}
fn fitting(_b: &Settings, slot: &str, label: &str, value: &str, id: i32, selected: &str) -> String {
    format!(
        r#"<button class="ship-part ship-part-{slot} {}" data-action="boat-slot:{slot}" aria-label="Choose {label}" aria-pressed="{}">{}<span>{label}<strong>{}</strong></span></button>"#,
        if selected == slot { "selected" } else { "" },
        selected == slot,
        item_icon(id),
        esc(value)
    )
}
pub fn wizard(
    d: &Draft,
    snapshot: &crate::account::Snapshot,
    options: &crate::planner::Options,
) -> String {
    let stock = crate::sailing_materials::stock(snapshot, options);
    let b = &d.settings;
    let mut h = format!(
        r#"<section class="panel boat-wizard" aria-label="Boat setup wizard"><div class="panel-heading"><h2>{}</h2><button class="text-button" data-action="boat-cancel">Cancel</button></div><ol class="boat-steps" aria-label="Setup steps">"#,
        if d.planning {
            "Plan upgrades"
        } else if d.index.is_some() {
            "Edit boat"
        } else {
            "Add a boat"
        }
    );
    if d.remove_pending {
        h.push_str(&format!("</ol><p>Remove {} from your fleet?</p><div class=\"boat-wizard-actions\">{}{}</div></section>",esc(&d.name),phase("boat-remove-cancel","Keep boat"),phase("boat-remove","Remove boat")));
        return h;
    }
    for (n, label) in ["Boat", "Fittings", "Crew"].iter().enumerate() {
        h.push_str(&format!(
            "<li {}>{}</li>",
            if n == d.step {
                "aria-current=\"step\""
            } else {
                ""
            },
            label
        ));
    }
    h.push_str("</ol><div class=\"boat-wizard-body\">");
    if !d.error.is_empty() {
        h.push_str(&format!(
            "<p class=\"boat-conflicts\" role=\"alert\">{}</p>",
            esc(&d.error)
        ));
    }
    match d.step {
        0 => {
            h.push_str(&format!(r#"<label class="boat-name">Boat name<input id="boat-name" maxlength="40" value="{}" placeholder="e.g. Kraken hunter"></label><div class="ship-types" aria-label="Boat type">"#,esc(&d.name)));
            for boat in Boat::ALL {
                h.push_str(&format!(r#"<button class="ship-type {}" data-action="boat-set:boat:{}" aria-pressed="{}"><span class="ship-type-art ship-art-{}" aria-hidden="true"></span><strong>{}</strong><small>{} Sailing · {} crew</small></button>"#,if b.boat==boat{"selected"}else{""},boat.name(),b.boat==boat,boat.name().to_lowercase(),boat.name(),boat.level(),boat.crew()));
            }
            h.push_str("</div><p class=\"small-note\">Save the boat you own. Edit it here after upgrades.</p>");
            if d.index.is_some() {
                h.push_str(&phase("boat-remove-review", "Remove boat"));
            }
        }
        1 => {
            let slot = if ["hull", "keel", "cargo", "cannon", "cannon2"].contains(&d.slot.as_str())
            {
                &d.slot
            } else {
                "hull"
            };
            h.push_str("<div class=\"boat-fitting-layout\"><div><div class=\"ship-sea\">");
            h.push_str(&format!(
                "<div class=\"ship-model ship-art-{}\" role=\"img\" aria-label=\"{}\"></div>",
                b.boat.name().to_lowercase(),
                b.boat.name()
            ));
            h.push_str(&fitting(
                b,
                "hull",
                "Hull",
                sailing::WOODS[b.hull],
                32118 + b.hull as i32 * 3,
                slot,
            ));
            if b.boat != Boat::Raft {
                h.push_str(&fitting(
                    b,
                    "keel",
                    "Keel",
                    sailing::METALS[b.keel],
                    32181 + b.keel as i32 * 2,
                    slot,
                ));
            }
            h.push_str(&fitting(
                b,
                "cargo",
                "Cargo",
                b.cargo.map(|i| sailing::WOODS[i]).unwrap_or("None"),
                32213 + b.cargo.unwrap_or(0) as i32,
                slot,
            ));
            h.push_str(&fitting(
                b,
                "cannon",
                "Cannon 1",
                b.cannon.map(|i| sailing::METALS[i]).unwrap_or("None"),
                32199 + b.cannon.unwrap_or(0) as i32,
                slot,
            ));
            if b.boat == Boat::Sloop && b.cannon.is_some() && b.cannon_count == 2 {
                h.push_str(&fitting(
                    b,
                    "cannon2",
                    "Cannon 2",
                    sailing::METALS[b.second_cannon.unwrap_or(b.cannon.unwrap())],
                    32199 + b.second_cannon.unwrap_or(b.cannon.unwrap()) as i32,
                    slot,
                ));
            }
            h.push_str("</div><dl class=\"ship-budget\">");
            for (label, value) in [
                ("Hull HP", b.hp()),
                ("Damage blocked", b.armour()),
                ("Cargo slots", b.cargo_capacity()),
            ] {
                h.push_str(&format!("<div><dt>{label}</dt><dd>{value}</dd></div>"));
            }
            h.push_str("</dl><div class=\"ship-facilities\" aria-label=\"Fitted facilities\">");
            for (key, label, id, on) in [
                ("attractor", "Attractor", 33142, b.attractor),
                ("perildance", "Perildance keg", 32231, b.perildance),
                ("workbench", "Repair bench", 33420, b.repair_workbench),
            ] {
                h.push_str(&format!(r#"<button class="ship-facility {}" data-action="boat-set:{key}:{}" aria-pressed="{on}">{}<span>{label}</span><b>{}</b></button>"#,if on{"selected"}else{""},!on,item_icon(id),if on{"✓"}else{"+"}));
            }
            h.push_str("</div></div>");
            h.push_str(
                &sailing::Settings::valid(b)
                    .then(|| {
                        crate::sailing_view::picker_with_stock(b, slot, Some(&stock))
                            .replace("data-action=\"sailing:", "data-action=\"boat-set:")
                    })
                    .unwrap_or_default(),
            );
            h.push_str("</div>");
        }
        _ => {
            h.push_str("<div class=\"boat-crew-layout\"><div><h3>Your job</h3><div class=\"ship-choice-grid\">");
            for (n, label) in sailing::PLAYER_ROLES.iter().enumerate() {
                h.push_str(&format!(r#"<button class="ship-choice {}" data-action="boat-set:player:{n}" aria-pressed="{}">{}<strong>{label}</strong></button>"#,if b.player_role==n{"selected"}else{""},b.player_role==n,item_icon([9185,32199,32140][n])));
            }
            h.push_str("</div><h3>Crew</h3><div class=\"ship-crew-seats\">");
            let seats = b.boat.crew().max(
                b.crew
                    .iter()
                    .rposition(|c| c.person.is_some())
                    .map(|n| n + 1)
                    .unwrap_or(0),
            );
            for (i, c) in b.crew.iter().enumerate().take(seats) {
                let name = c
                    .person
                    .map(|p| sailing::CREW[p].name)
                    .unwrap_or("Empty seat");
                let icon = c
                    .person
                    .map(|p| {
                        format!(
                            "<span class=\"ship-face ship-face-{p}\" aria-hidden=\"true\"></span>"
                        )
                    })
                    .unwrap_or_else(|| item_icon(1169));
                h.push_str(&format!(r#"<div class="ship-seat"><button class="ship-seat-person" data-action="boat-slot:person-{i}" aria-label="Choose crew seat {}">{icon}<span>{}</span></button><button class="ship-seat-job" data-action="boat-slot:role-{i}" aria-label="Choose job for seat {}">{}</button></div>"#,i+1,esc(name),i+1,sailing::ROLES[c.role]));
            }
            if seats == 0 {
                h.push_str("<p>A raft has no crew.</p>");
            }
            h.push_str("</div></div>");
            let slot = if d.slot.starts_with("person-") || d.slot.starts_with("role-") {
                d.slot.as_str()
            } else {
                "person-0"
            };
            if seats > 0 {
                h.push_str(
                    &crate::sailing_view::picker_with_stock(b, slot, Some(&stock))
                        .replace("data-action=\"sailing:", "data-action=\"boat-set:"),
                );
            }
            h.push_str("</div>");
            let conflicts = b.conflicts();
            if !conflicts.is_empty() {
                h.push_str("<ul class=\"boat-conflicts\">");
                for note in conflicts {
                    h.push_str(&format!("<li>{}</li>", esc(&note)));
                }
                h.push_str("</ul>");
            }
        }
    }
    if d.step == 1 {
        h.push_str(&crate::sailing_materials::wizard_cost(d, snapshot, options));
    }
    h.push_str("</div><div class=\"boat-wizard-actions\">");
    if d.step > 0 {
        h.push_str(&phase("boat-back", "Back"));
    }
    h.push_str(&phase(
        if d.step < 2 { "boat-next" } else { "boat-save" },
        if d.step < 2 {
            "Next"
        } else if d.planning {
            "Save upgrade plan"
        } else {
            "Save fitted boat"
        },
    ));
    if d.step == 2 && d.planning {
        h.push_str(&phase("boat-fit-plan", "Mark upgrades fitted"));
    }
    h.push_str("</div></section>");
    h
}
