//! Private fleet controls and modelled trip results, using the shipyard's icon boxes.
use crate::{
    account::Snapshot,
    planner::{self, Options},
    sailing::{self, Boat, Settings},
    sailing_combat::Report,
    sailing_owned::{BALL_IDS, KIT_IDS},
    sailing_recommendation::Recommendation,
    tooltip::item_icon,
};
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn key(snapshot: &Snapshot, o: &Options) -> String {
    serde_json::to_string(&(snapshot, o)).unwrap_or_default()
}
pub fn controls(snapshot: &Snapshot, o: &Options, _open: bool) -> String {
    let Some(m) = snapshot.selected() else {
        return "<p>Select your account first.</p>".into();
    };
    let f = &m.sailing_owned;
    let mut h = format!(
        r#"<section class="panel ship-fleet"><div class="panel-heading"><h2>Your boats</h2><span>{}/5</span></div><div class="fleet-list">"#,
        f.profiles.len()
    );
    let active = f.active.as_deref().or_else(|| {
        f.profiles
            .iter()
            .find(|p| p.matches(&o.sailing))
            .map(|p| p.name.as_str())
    });
    for (i, p) in f.profiles.iter().enumerate() {
        let b = &p.settings;
        h.push_str(&format!(r#"<div class="fleet-row"><button class="fleet-boat {}" data-action="boat-select:{i}" aria-pressed="{}"><span class="ship-type-art ship-art-{}" aria-hidden="true"></span><span><strong>{}</strong><small>{} · {} hull · {}</small></span></button><button class="phase" data-action="boat-edit:{i}" aria-label="Edit {}">Edit</button><button class="phase boat-plan" data-action="boat-plan:{i}" aria-label="Plan upgrades for {}">{}</button></div>"#,if active == Some(p.name.as_str()) && p.matches(&o.sailing){"selected"}else{""},active == Some(p.name.as_str()) && p.matches(&o.sailing),b.boat.name().to_lowercase(),esc(&p.name),b.boat.name(),sailing::WOODS[b.hull],if b.boat==Boat::Raft{"No keel".into()}else{format!("{} keel",sailing::METALS[b.keel])},esc(&p.name),esc(&p.name),if p.upgrade.is_some(){"Review plan"}else{"Plan upgrades"}));
    }
    h.push_str("</div><div class=\"fleet-actions\">");
    if f.profiles.len() < 5 {
        h.push_str("<button class=\"phase\" data-action=\"boat-add\">Add boat</button>");
    }
    if f.profiles.is_empty() {
        h.push_str("<span>Add your first boat to compare it.</span>");
    } else {
        if !f.profiles.iter().any(|p| p.matches(&o.sailing)) {
            h.push_str("<span>Unsaved boat selected</span>");
        }
        h.push_str("<button class=\"phase\" data-action=\"sailing-calculate\">Calculate trip</button><button class=\"phase\" data-action=\"sailing-recommend\">Find best setup</button>");
    }
    h.push_str("</div></section>");
    h.push_str(&crate::sailing_materials::fleet_cost(snapshot, o));
    h.push_str("<details class=\"panel boat-loaded\"><summary>Supplies already aboard</summary><p>Add only quantities missing from your bank import.</p><div class=\"ship-trip-inputs\">");
    for id in BALL_IDS.into_iter().chain(KIT_IDS) {
        h.push_str(&format!(r#"<label>{}<span>{}<input id="sailing-loaded-{id}" type="number" min="0" max="1000000" value="{}"></span></label>"#,item_icon(id),esc(&planner::item_name(id)),f.loaded_stock().get(&id).copied().unwrap_or(0)));
    }
    h.push_str("</div></details>");
    h
}
pub fn scenario(b: &Settings) -> String {
    let mut h = format!(
        r#"<details class="panel sailing-trip"><summary>Trip supplies · {} kills · {} min</summary><div class="ship-trip-inputs">"#,
        b.trip_kills, b.minutes
    );
    for (key, label, value, min, max) in [
        ("kills", "Target kills", b.trip_kills, 1, 100),
        ("minutes", "Time limit (minutes)", b.minutes, 1, 120),
        (
            "balls",
            "Cannonballs (0 = owned stock)",
            b.cannonballs,
            0,
            1_000_000,
        ),
        ("kits", "Repair kits", b.repair_kits, 0, 10_000),
        ("reserved", "Other cargo slots", b.cargo_reserved, 0, 240),
        (
            "charges",
            "Loaded weapon charges / scales",
            b.powered_charges,
            0,
            1_000_000,
        ),
    ] {
        h.push_str(&format!(r#"<label><span>{label}<input id="sailing-{key}" type="number" min="{min}" max="{max}" value="{value}"></span></label>"#));
    }
    h.push_str("</div><details><summary>Cannonballs</summary><div class=\"ship-choice-grid\">");
    for n in 0..7 {
        h.push_str(&format!(r#"<button class="ship-choice {}" data-action="sailing:ball:{n}" aria-pressed="{}">{}<strong>{}</strong></button>"#,if b.cannonball==n{"selected"}else{""},b.cannonball==n,item_icon(BALL_IDS[n]),sailing::BALL_NAMES[n]));
    }
    h.push_str("</div></details><details><summary>Repair kit type</summary><div class=\"ship-choice-grid\">");
    for n in 0..7 {
        h.push_str(&format!(r#"<button class="ship-choice {}" data-action="sailing:kit:{n}" aria-pressed="{}">{}<strong>{}</strong><small>{} HP</small></button>"#,if b.repair_tier==n{"selected"}else{""},b.repair_tier==n,item_icon(KIT_IDS[n]),sailing::WOODS[n],crate::sailing_combat::KIT_HEAL[n]));
    }
    h.push_str("</div></details><details><summary>Calculation assumptions</summary><div class=\"ship-trip-inputs\">");
    for (key, label, value, min, max) in [
        (
            "repair-ticks",
            "Ticks between repairs",
            b.repair_ticks,
            1,
            20,
        ),
        ("aggressors", "Creatures attacking", b.aggressors, 1, 10),
        ("uptime", "Firing uptime %", b.uptime, 0, 100),
    ] {
        h.push_str(&format!(r#"<label><span>{label}<input id="sailing-{key}" type="number" min="{min}" max="{max}" value="{value}"></span></label>"#));
    }
    h.push_str(&format!(r#"<label><span>Boosted Sailing<input id="sailing-visible" type="number" min="1" max="125" value="{}" placeholder="Base level"></span></label></div><button class="phase" data-action="sailing:manual-repairs:{}" aria-pressed="{}">Manual repairs: {}</button><p>Stationary estimate. No movement, respawn delays or boost decay. Repair cadence is an assumption; crew healing uses base kit values. Personal ammo and charges are consumed per attack.</p></details></details>"#,b.visible_sailing.map(|n|n.to_string()).unwrap_or_default(),!b.manual_repairs,b.manual_repairs,if b.manual_repairs{"On"}else{"Off"}));
    h
}
pub fn report(r: &Report, title: &str) -> String {
    let mut h = format!(
        r#"<section class="panel ship-forecast"><div class="panel-heading"><h2>{}</h2></div><div class="boat-report-body"><p class="small-note">Modelled scenario · {} seeded trials</p><dl class="ship-budget"><div><dt>Combined DPS</dt><dd>{:.2}</dd></div><div><dt>Player / crew DPS</dt><dd>{:.2} / {:.2}</dd></div><div><dt>Hull damage / second</dt><dd>{:.2}</dd></div><div><dt>Trip completed</dt><dd>{:.1}%</dd></div><div><dt>Average kills</dt><dd>{:.1}</dd></div><div><dt>Hull HP / max taken</dt><dd>{} / {}</dd></div><div><dt>Enemy hit chance</dt><dd>{:.1}%</dd></div><div><dt>Average trip</dt><dd>{:.1} min</dd></div><div><dt>Kits used · mean / p95</dt><dd>{:.1} / {}</dd></div><div><dt>Cannon shots fired · mean</dt><dd>{:.1}</dd></div></dl><p class="small-note">Outcomes: {} completed · {} sunk · {} out of supplies · {} timed out. DPS is sustained attack potential before prayer depletion and repair delays; trip outcomes include those effects.</p>"#,
        esc(title),
        r.trials,
        r.outgoing_dps,
        r.player_dps,
        if r.crew_dps == 0.0 { 0.0 } else { r.crew_dps },
        r.incoming_dps,
        100.0 * f64::from(r.completed) / f64::from(r.trials),
        r.mean_kills,
        r.boat_hp,
        r.max_taken,
        r.hit_chance * 100.0,
        r.mean_seconds / 60.0,
        r.mean_kits,
        r.p95_kits,
        r.mean_cannonballs,
        r.completed,
        r.sunk,
        r.depleted,
        r.timed_out
    );
    h.push_str("<details><summary>Model details</summary><ul>");
    for n in &r.notes {
        h.push_str(&format!("<li>{}</li>", esc(n)));
    }
    h.push_str("</ul></details></div></section>");
    h
}
pub fn recommendation(r: &Recommendation) -> String {
    let name = r.saved_boat.as_deref().unwrap_or(r.boat.boat.name());
    let mut h = format!(
        r#"<section class="panel ship-recommendation"><div class="panel-heading"><h2>Recommended setup</h2><span>{}</span></div><p class="recommendation-boat">{} · {} · Your job: {}</p><div class="loadout-layout sailing-loadout"><section class="panel loadout-panel"><div class="panel-heading"><h2>{} Equipment</h2></div><div class="loadout-content">{}</div></section><section class="panel recommendation-crew"><div class="panel-heading"><h2>Crew</h2></div>"#,
        esc(&r.style),
        esc(name),
        r.boat.boat.name(),
        sailing::PLAYER_ROLES[r.boat.player_role],
        item_icon(12954),
        crate::equipment_view::preview(&r.equipment, &r.style)
    );
    for c in r.boat.crew.iter().filter(|c| c.person.is_some()) {
        let p = c.person.unwrap();
        h.push_str(&format!(r#"<div class="recommended-crewmate"><span class="ship-face ship-face-{p}" aria-hidden="true"></span><span>{}<small>{}</small></span></div>"#,esc(sailing::CREW[p].name),sailing::ROLES[c.role]));
    }
    if r.boat.crew.iter().all(|c| c.person.is_none()) {
        h.push_str("<p>No crew assigned</p>");
    }
    h.push_str(&format!(r#"</section></div><div class="fleet-actions"><button class="phase" data-action="sailing-use">Use this setup</button></div><details class="recommendation-details"><summary>Comparison details</summary><p>Compared {} finalists from {} combinations. Ranks trip completion, kills, then DPS. Saved boats keep their fitted parts and crew assignments. Gear, regular cannonballs and one repair-kit tier are compared from your stock.</p><p>{} {} kits · {} cargo slots reserved.</p></details></section>"#,r.compared,r.screened,r.boat.repair_kits,sailing::WOODS[r.boat.repair_tier],r.boat.cargo_reserved));
    h.push_str(&report(&r.report, "Recommended trip"));
    h
}
