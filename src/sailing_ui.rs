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
fn own(key: &str, n: usize, label: &str, icon: String, active: bool, detail: &str) -> String {
    format!(
        r#"<button class="ship-choice {}" data-action="sailing-own:{key}:{n}" aria-pressed="{active}">{icon}<strong>{}</strong><small>{}</small></button>"#,
        if active { "selected" } else { "" },
        esc(label),
        esc(detail)
    )
}
pub fn controls(snapshot: &Snapshot, o: &Options, open: bool) -> String {
    let Some(m) = snapshot.selected() else {
        return "<p>Select your account to confirm your fleet.</p>".into();
    };
    let b = &o.sailing;
    let f = &m.sailing_owned;
    let i = b.boat.index();
    let mut h = format!(
        r#"<section class="ship-fleet"><h3>Your fleet</h3><p>Bank items supply your gear, cannonballs and kits. Confirm boats, fitted parts and recruited crew here.</p><div class="actions"><button data-action="sailing-confirm">Confirm this boat &amp; crew</button><button data-action="sailing-owned-open" aria-expanded="{open}">Available parts &amp; crew</button><button data-action="sailing-calculate">Calculate current trip</button><button class="primary" data-action="sailing-recommend">Recommend my boat</button></div><p class="small-note">{} · Your confirmations stay private on this account.</p>"#,
        if f.permits(b) {
            "Current parts confirmed"
        } else {
            "Current parts not yet confirmed"
        }
    );
    if open {
        h.push_str(
            "<div class=\"ship-picker\"><h4>Boats you own</h4><div class=\"ship-choice-grid\">",
        );
        for boat in Boat::ALL {
            let n = boat.index();
            h.push_str(&own("boat",n,boat.name(),format!(r#"<img src="assets/sailing/{}.png" alt="" style="width:64px;height:48px;object-fit:contain">"#,boat.name().to_lowercase()),f.boats[n],"Available"));
        }
        h.push_str("</div>");
        for (field, title) in [
            ("hull", "Hulls"),
            ("keel", "Keels"),
            ("cargo", "Cargo holds"),
        ] {
            if field == "keel" && i == 0 {
                continue;
            }
            h.push_str(&format!(
                "<h4>{} — {title}</h4><div class=\"ship-choice-grid\">",
                b.boat.name()
            ));
            for n in 0..7 {
                let (label, id, yes) = match field {
                    "hull" => (sailing::WOODS[n], 32118 + n as i32 * 3, f.hulls[i][n]),
                    "keel" => (
                        sailing::METALS[n],
                        32181 + n as i32 * 2 + if i == 2 { 1 } else { 0 },
                        f.keels[i][n],
                    ),
                    _ => (sailing::WOODS[n], 32213 + n as i32, f.cargo[i][n]),
                };
                h.push_str(&own(field, n, label, item_icon(id), yes, "Available"));
            }
            h.push_str("</div>");
        }
        h.push_str(
            "<h4>Cannons owned — click to cycle 0 / 1 / 2</h4><div class=\"ship-choice-grid\">",
        );
        for n in 0..7 {
            h.push_str(&own(
                "cannon",
                n,
                sailing::METALS[n],
                item_icon(32199 + n as i32),
                f.cannons[n] > 0,
                &format!("{} owned", f.cannons[n]),
            ));
        }
        h.push_str("</div><h4>Recruited crew</h4><div class=\"ship-choice-grid\">");
        for (n, p) in sailing::CREW.iter().enumerate() {
            h.push_str(&own(
                "crew",
                n,
                p.name,
                format!("<span class=\"ship-face ship-face-{n}\" aria-hidden=\"true\"></span>"),
                f.crew[n],
                &format!("Privateering {}", p.privateering),
            ));
        }
        h.push_str("</div><h4>Available facilities</h4><div class=\"ship-choice-grid\">");
        for (n, label, id, yes) in [
            (0, "Ballistic attractor", 33142, f.attractor),
            (1, "Perildance keg", 32231, f.perildance),
            (2, "Bosun's workbench", 33420, f.workbench),
        ] {
            h.push_str(&own(
                "facility",
                n,
                label,
                item_icon(id),
                yes,
                "Installed / access confirmed",
            ));
        }
        h.push_str("</div><h4>Already aboard (not in import)</h4><p class=\"small-note\">Add only supplies missing from your imported containers. Counts are added to imported stock; avoid counting them twice.</p><div class=\"ship-trip-inputs\">");
        for (id, label) in BALL_IDS
            .into_iter()
            .map(|id| (id, planner::item_name(id)))
            .chain(KIT_IDS.into_iter().map(|id| (id, planner::item_name(id))))
        {
            h.push_str(&format!(r#"<label>{}<span>{}<input id="sailing-loaded-{id}" type="number" min="0" max="1000000" value="{}"></span></label>"#,item_icon(id),esc(&label),f.loaded.get(&id).copied().unwrap_or(0)));
        }
        h.push_str("</div></div>");
    }
    h.push_str("</section>");
    h
}
pub fn scenario(b: &Settings) -> String {
    let mut h="<details class=\"ship-checks\"><summary>Trip goal &amp; calculation assumptions</summary><div class=\"ship-trip-inputs\">".to_string();
    for (key, label, value, min, max) in [
        ("kills", "Target kills", b.trip_kills, 1, 100),
        (
            "repair-ticks",
            "Ticks between repairs",
            b.repair_ticks,
            1,
            20,
        ),
        (
            "aggressors",
            "Creatures attacking at once",
            b.aggressors,
            1,
            10,
        ),
        (
            "charges",
            "Loaded weapon charges / scales",
            b.powered_charges,
            0,
            1_000_000,
        ),
    ] {
        h.push_str(&format!(r#"<label>{}<span>{label}<input id="sailing-{key}" type="number" min="{min}" max="{max}" value="{value}"></span></label>"#,item_icon(32300)));
    }
    h.push_str(&format!(r#"</div><button data-action="sailing:manual-repairs:{}" aria-pressed="{}">Manual repairs: {}</button><p class="small-note">Stationary scenario. Hull accuracy uses standard NPC defence rolls; repair cadence is your assumption. Crew repairs when a kit's healing is missing; manual repairs wait until half hull HP. No movement, respawn/loot delays, boost decay or gameplay calibration. Crew repairs credit base kit healing only. Personal ammo/charges are conservatively consumed per attack.</p></details>"#,!b.manual_repairs,b.manual_repairs,if b.manual_repairs{"On"}else{"Off"}));
    h.push_str(
        "<section class=\"ship-picker\"><h3>Repair kits</h3><div class=\"ship-choice-grid\">",
    );
    for n in 0..7 {
        h.push_str(&format!(r#"<button class="ship-choice {}" data-action="sailing:kit:{n}" aria-pressed="{}">{}<strong>{}</strong><small>Heals {} HP · owned stock only</small></button>"#,if b.repair_tier==n{"selected"}else{""},b.repair_tier==n,item_icon(KIT_IDS[n]),sailing::WOODS[n],crate::sailing_combat::KIT_HEAL[n]));
    }
    h.push_str("</div></section>");
    if b.cannon_count == 2 && b.boat == Boat::Sloop {
        h.push_str(
            "<section class=\"ship-picker\"><h3>Second cannon</h3><div class=\"ship-choice-grid\">",
        );
        for n in 0..7 {
            h.push_str(&format!(r#"<button class="ship-choice {}" data-action="sailing:cannon2:{n}" aria-pressed="{}">{}<strong>{}</strong></button>"#,if b.second_cannon.unwrap_or(b.cannon.unwrap_or(0))==n{"selected"}else{""},b.second_cannon.unwrap_or(b.cannon.unwrap_or(0))==n,item_icon(32199+n as i32),sailing::METALS[n]));
        }
        h.push_str("</div><p class=\"small-note\">Both modelled mounts use the selected cannonball type.</p></section>");
    }
    h
}
pub fn report(r: &Report, title: &str) -> String {
    let mut h = format!(
        r#"<section class="ship-forecast"><h3>{}</h3><p class="small-note">Modelled scenario · {} seeded trials</p><dl class="ship-budget"><div><dt>Combined DPS</dt><dd>{:.2}</dd></div><div><dt>Player / crew DPS</dt><dd>{:.2} / {:.2}</dd></div><div><dt>Hull damage / second</dt><dd>{:.2}</dd></div><div><dt>Trip completed</dt><dd>{:.1}%</dd></div><div><dt>Average kills</dt><dd>{:.1}</dd></div><div><dt>Hull HP / max taken</dt><dd>{} / {}</dd></div><div><dt>Enemy hit chance</dt><dd>{:.1}%</dd></div><div><dt>Average trip</dt><dd>{:.1} min</dd></div><div><dt>Kits used · mean / p95</dt><dd>{:.1} / {}</dd></div><div><dt>Cannon shots fired · mean</dt><dd>{:.1}</dd></div></dl><p class="small-note">Outcomes: {} completed · {} sunk · {} out of supplies · {} timed out. DPS is sustained attack potential before prayer depletion and repair delays; trip outcomes include those effects.</p>"#,
        esc(title),
        r.trials,
        r.outgoing_dps,
        r.player_dps,
        r.crew_dps,
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
    h.push_str("</ul></details></section>");
    h
}
pub fn recommendation(r: &Recommendation) -> String {
    let mut h = format!(
        "<section class=\"ship-recommendation\"><h3>Recommended from your items</h3><p>{} · {} hull · {} keel · {}</p><p class=\"small-note\">Compared {} finalists after screening {} combinations. Bounded search: confirmed boats/parts/crew, regular cannonballs, one repair-kit tier and the planner's Ranged/Magic outfits plus cannon armour. Ranks completion, kills, then DPS.</p><div class=\"ship-choice-grid\">",
        r.boat.boat.name(),
        sailing::WOODS[r.boat.hull],
        sailing::METALS[r.boat.keel],
        esc(&r.style),
        r.compared,
        r.screened
    );
    if let Some(tier) = r.boat.cannon {
        h.push_str(&format!(
            "<p>Cannon 1: {} · {} balls. {} cannon(s) firing on the target side.</p>",
            sailing::METALS[tier],
            sailing::METALS[r.boat.cannonball],
            r.boat.cannon_count
        ));
        if r.boat.cannon_count == 2 {
            h.push_str(&format!(
                "<p>Cannon 2: {}</p>",
                sailing::METALS[r.boat.second_cannon.unwrap_or(tier)]
            ));
        }
    }
    h.push_str(&format!(
        "<p>Your job: {} · {} {} repair kits · {} cargo slots reserved.</p>",
        sailing::PLAYER_ROLES[r.boat.player_role],
        r.boat.repair_kits,
        sailing::WOODS[r.boat.repair_tier],
        r.boat.cargo_reserved
    ));
    for c in r.boat.crew.iter().filter(|c| c.person.is_some()) {
        let p = c.person.unwrap();
        h.push_str(&format!("<span class=\"ship-choice\"><span class=\"ship-face ship-face-{p}\" aria-hidden=\"true\"></span><strong>{}</strong><small>{}</small></span>",esc(sailing::CREW[p].name),sailing::ROLES[c.role]));
    }
    for id in r.equipment.iter().flatten() {
        h.push_str(&format!(
            "<span class=\"ship-choice\">{}<small>{}</small></span>",
            item_icon(*id),
            esc(&planner::item_name(*id))
        ));
    }
    h.push_str("</div><button class=\"primary\" data-action=\"sailing-use\">Use this setup</button></section>");
    h.push_str(&report(&r.report, "Recommended trip"));
    h
}
