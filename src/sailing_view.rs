//! Shipyard view: clickable OSRS parts and crew, with no dropdown controls.
use crate::{
    account::Member,
    bestiary, encounters,
    planner::{self, Target},
    sailing::{self, Boat, CREW, METALS, Settings, WOODS},
    tooltip::item_icon,
};
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn choice(key: &str, value: &str, label: &str, detail: &str, icon: i32, active: bool) -> String {
    format!(
        r#"<button type="button" class="ship-choice {}" data-action="sailing:{}:{}" aria-label="{}" aria-pressed="{}">{}<strong>{}</strong><small>{}</small></button>"#,
        if active { "selected" } else { "" },
        esc(key),
        esc(value),
        esc(&format!("Choose {label}")),
        active,
        item_icon(icon),
        esc(label),
        esc(detail)
    )
}
fn part(slot: &str, label: &str, value: &str, icon: i32, selected: &str) -> String {
    format!(
        r#"<button type="button" class="ship-part ship-part-{} {}" data-action="sailing-slot:{}" aria-label="Choose {}" aria-pressed="{}">{}<span>{}<strong>{}</strong></span></button>"#,
        esc(slot),
        if slot == selected { "selected" } else { "" },
        esc(slot),
        esc(label),
        slot == selected,
        item_icon(icon),
        esc(label),
        esc(value)
    )
}
fn face(person: usize) -> String {
    format!(
        r#"<span class="ship-face ship-face-{}" aria-hidden="true"></span>"#,
        person
    )
}
pub fn valid_slot(s: &str) -> bool {
    ["hull", "keel", "cargo", "cannon", "ammo", "player"].contains(&s)
        || s.split_once('-').is_some_and(|(field, n)| {
            ["person", "role"].contains(&field) && n.parse::<usize>().is_ok_and(|i| i < 5)
        })
}
fn picker(b: &Settings, slot: &str) -> String {
    let title = match slot {
        "hull" => "Hull",
        "keel" => "Keel",
        "cargo" => "Cargo hold",
        "cannon" => "Cannons",
        "ammo" => "Cannonballs",
        "player" => "Your job",
        _ if slot.starts_with("person-") => "Choose a crewmate",
        _ => "Crew job",
    };
    let mut h = format!(
        r#"<section class="ship-picker"><div class="ship-picker-heading"><h3>{title}</h3><span>Click to fit</span></div><div class="ship-choice-grid">"#
    );
    match slot {
        "hull" => {
            for (i, name) in WOODS.iter().enumerate() {
                let mut trial = b.clone();
                trial.hull = i;
                h.push_str(&choice(
                    "hull",
                    &i.to_string(),
                    name,
                    &format!("{} total HP", trial.hp()),
                    32118 + i as i32 * 3,
                    b.hull == i,
                ));
            }
        }
        "keel" if b.boat != Boat::Raft => {
            for (i, name) in METALS.iter().enumerate() {
                h.push_str(&choice(
                    "keel",
                    &i.to_string(),
                    name,
                    &format!("−{} damage / hit", sailing::FLAT_ARMOUR[i]),
                    32181 + i as i32 * 2 + if b.boat == Boat::Sloop { 1 } else { 0 },
                    b.keel == i,
                ));
            }
        }
        "keel" => h.push_str("<p>A raft has no keel.</p>"),
        "cargo" => {
            h.push_str(&choice(
                "cargo",
                "none",
                "No hold",
                "Empty facility",
                32213,
                b.cargo.is_none(),
            ));
            for (i, name) in [
                "Basic", "Oak", "Teak", "Mahogany", "Camphor", "Ironwood", "Rosewood",
            ]
            .iter()
            .enumerate()
            {
                let mut trial = b.clone();
                trial.cargo = Some(i);
                h.push_str(&choice(
                    "cargo",
                    &i.to_string(),
                    name,
                    &format!("{} cargo slots", trial.cargo_capacity()),
                    32213 + i as i32,
                    b.cargo == Some(i),
                ));
            }
        }
        "cannon" => {
            h.push_str(&choice(
                "cannon",
                "none",
                "No cannon",
                "Empty mount",
                32199,
                b.cannon.is_none(),
            ));
            for (i, name) in METALS.iter().enumerate() {
                h.push_str(&choice(
                    "cannon",
                    &i.to_string(),
                    name,
                    &format!("{} Sailing", sailing::CANNON_LEVELS[i]),
                    32199 + i as i32,
                    b.cannon == Some(i),
                ));
            }
        }
        "ammo" => {
            for (i, name) in sailing::BALL_NAMES.iter().enumerate() {
                h.push_str(&choice(
                    "ball",
                    &i.to_string(),
                    name,
                    "Cannonballs",
                    [31906, 31908, 2, 31910, 31912, 31914, 31916][i],
                    b.cannonball == i,
                ));
            }
        }
        "player" => {
            for (i, (name, detail, icon)) in [
                ("Weapon", "Ranged or magic", 9185),
                ("Cannon 1", "Fire it yourself", 32199),
                ("Helm", "Steer the boat", 32140),
            ]
            .iter()
            .enumerate()
            {
                h.push_str(&choice(
                    "player",
                    &i.to_string(),
                    name,
                    detail,
                    *icon,
                    b.player_role == i,
                ));
            }
        }
        _ => {
            if let Some((field, seat)) = slot
                .split_once('-')
                .and_then(|(f, n)| Some((f, n.parse::<usize>().ok()?)))
                .filter(|(_, n)| *n < 5)
            {
                let c = &b.crew[seat];
                if field == "person" {
                    h.push_str(&choice(
                        slot,
                        "none",
                        "Empty seat",
                        "Remove crewmate",
                        1169,
                        c.person.is_none(),
                    ));
                    for (i, p) in CREW.iter().enumerate() {
                        h.push_str(&format!(r#"<button type="button" class="ship-choice ship-crew-choice {}" data-action="sailing:{}:{}" aria-label="Choose {}" aria-pressed="{}" title="{} Sailing to recruit (boostable). {}">{}<strong>{}</strong><small>Privateering {}</small></button>"#,if c.person==Some(i){"selected"}else{""},esc(slot),i,esc(p.name),c.person==Some(i),p.level,esc(p.access),face(i),esc(p.name),p.privateering));
                    }
                } else {
                    for (i, role) in sailing::ROLES.iter().enumerate() {
                        h.push_str(&choice(
                            slot,
                            &i.to_string(),
                            role,
                            [
                                "Stand by",
                                "Steer the boat",
                                "Use the workbench",
                                "Port battery",
                                "Second mount",
                            ][i],
                            [32231, 32140, 32300, 32199, 32199][i],
                            c.role == i,
                        ));
                    }
                }
            }
        }
    }
    h.push_str("</div>");
    if slot == "cannon" {
        h.push_str("<div class=\"ship-mount-count\"><span>Mounts on this side</span>");
        for (i, label) in [(1, "One"), (2, "Two · sloop")] {
            h.push_str(&format!("<button type=\"button\" class=\"phase {}\" data-action=\"sailing:count:{i}\" aria-pressed=\"{}\">{label}</button>",if b.cannon_count==i{"active"}else{""},b.cannon_count==i));
        }
        h.push_str("</div>");
    }
    h.push_str("</section>");
    h
}
pub fn render(
    b: &Settings,
    member: Option<&Member>,
    targets: &[Target],
    slot: &str,
    readonly: bool,
) -> String {
    if !b.valid() {
        return "<p>Boat settings could not be read.</p>".into();
    }
    let slot = if valid_slot(slot) { slot } else { "hull" };
    let hull_icon = 32118 + b.hull as i32 * 3;
    let cannon_icon = 32199 + b.cannon.unwrap_or(0) as i32;
    let keel_icon = 32181 + b.keel as i32 * 2 + if b.boat == Boat::Sloop { 1 } else { 0 };
    let mut h = String::from(
        r#"<section class="shipyard"><div class="shipyard-title"><h2>Your boat</h2><p>Click a part to change it.</p></div><fieldset class="ship-controls""#,
    );
    if readonly {
        h.push_str(" disabled");
    }
    h.push_str("><legend class=\"sr-only\">Boat setup</legend><div class=\"ship-types\" aria-label=\"Boat type\">");
    for boat in Boat::ALL {
        let class = boat.name().to_lowercase();
        h.push_str(&format!(r#"<button type="button" class="ship-type {}" data-action="sailing:boat:{}" aria-label="Choose {}" aria-pressed="{}"><span class="ship-type-art ship-art-{}" aria-hidden="true"></span><strong>{}</strong><small>{} Sailing · {} crew</small></button>"#,if b.boat==boat{"selected"}else{""},boat.name(),boat.name(),b.boat==boat,class,boat.name(),boat.level(),boat.crew()));
    }
    h.push_str("</div><div class=\"shipyard-grid\"><div class=\"ship-overview\"><div class=\"ship-sea\"><span class=\"ship-sea-label\">Your ");
    h.push_str(&b.boat.name().to_lowercase());
    h.push_str("</span>");
    h.push_str(&format!(
        r#"<div class="ship-model ship-art-{}" role="img" aria-label="OSRS {}"></div>"#,
        b.boat.name().to_lowercase(),
        b.boat.name()
    ));
    h.push_str(&part("hull", "Hull", WOODS[b.hull], hull_icon, slot));
    if b.boat != Boat::Raft {
        h.push_str(&part("keel", "Keel", METALS[b.keel], keel_icon, slot));
    }
    h.push_str(&part(
        "cannon",
        "Cannons",
        b.cannon.map(|i| METALS[i]).unwrap_or("Not fitted"),
        cannon_icon,
        slot,
    ));
    h.push_str(&part(
        "cargo",
        "Cargo",
        b.cargo.map(|i| WOODS[i]).unwrap_or("Not fitted"),
        32213 + b.cargo.unwrap_or(0) as i32,
        slot,
    ));
    h.push_str(&part(
        "player",
        "You",
        sailing::PLAYER_ROLES[b.player_role],
        [9185, cannon_icon, 32140][b.player_role],
        slot,
    ));
    h.push_str("</div><div class=\"ship-stats\">");
    let crew_limit = member
        .and_then(|m| m.level("Sailing"))
        .map(|n| b.crew_limit(n).to_string())
        .unwrap_or_else(|| "?".into());
    for (icon, label, value) in [
        (32300, "Boat HP", b.hp().to_string()),
        (keel_icon, "Damage blocked", b.armour().to_string()),
        (32213, "Cargo slots", b.cargo_capacity().to_string()),
        (1169, "Crew slots", crew_limit),
    ] {
        h.push_str(&format!(
            "<div>{}<span>{label}<strong>{value}</strong></span></div>",
            item_icon(icon)
        ));
    }
    h.push_str("</div><div class=\"ship-facilities\" aria-label=\"Extra facilities\">");
    for (key, label, icon, on) in [
        ("attractor", "Attractor", 33142, b.attractor),
        ("perildance", "Perildance keg", 32231, b.perildance),
        ("workbench", "Repair bench", 33420, b.repair_workbench),
    ] {
        h.push_str(&format!(r#"<button type="button" class="ship-facility {}" data-action="sailing:{key}:{}" aria-pressed="{on}" aria-label="Toggle {label}">{}<span>{label}</span><b>{}</b></button>"#,if on{"selected"}else{""},!on,item_icon(icon),if on{"✓"}else{"+"}));
    }
    h.push_str("</div></div><div class=\"ship-inspector\">");
    h.push_str(&picker(b, slot));
    h.push_str(&format!(r#"<button type="button" class="ship-ammo" data-action="sailing-slot:ammo" aria-label="Choose cannonballs">{}<span>Cannonballs<strong>{}</strong></span><span>Change ›</span></button>"#,item_icon([31906,31908,2,31910,31912,31914,31916][b.cannonball]),sailing::BALL_NAMES[b.cannonball]));
    for t in targets.iter().filter(|t| sailing::is_sea(t.id)) {
        if let Some(m) = planner::monster(t.id) {
            let max = sailing::reduced_max(b, m).unwrap_or(0);
            let portrait = encounters::catalog()
                .iter()
                .find(|e| e.targets.len() == 1 && e.targets[0].id == m.id)
                .map(bestiary::portrait)
                .unwrap_or_default();
            h.push_str(&format!(r#"<article class="ship-enemy">{}<div><h3>{}</h3><span>{} HP</span><p>Max hit <s>{}</s> <strong class="{}">{max}</strong><small>with your keel</small></p></div></article>"#,portrait,esc(&m.name),m.skills.get("hp").unwrap_or(&0),esc(&m.max_hit),if max==0{"ship-safe"}else{"ship-damage"}));
        }
    }
    h.push_str("</div></div><section class=\"ship-crew\"><div class=\"ship-section-heading\"><h3>Crew</h3><span>One crewmate, one job.</span></div><div class=\"ship-crew-seats\">");
    let seats = b.boat.crew().max(
        b.crew
            .iter()
            .rposition(|c| c.person.is_some())
            .map(|n| n + 1)
            .unwrap_or(0),
    );
    for (i, c) in b.crew.iter().enumerate().take(seats) {
        let name = c.person.map(|p| CREW[p].name).unwrap_or("Empty seat");
        let portrait = c.person.map(face).unwrap_or_else(|| item_icon(1169));
        h.push_str(&format!(r#"<div class="ship-seat"><button type="button" class="ship-seat-person {}" data-action="sailing-slot:person-{i}" aria-label="Choose crew seat {}">{}<span>{}<small>{}</small></span></button><button type="button" class="ship-seat-job" data-action="sailing-slot:role-{i}" aria-label="Choose job for seat {}">{} <span>›</span></button></div>"#,if slot==format!("person-{i}"){"selected"}else{""},i+1,portrait,esc(name),c.person.map(|p|format!("Privateering {}",CREW[p].privateering)).unwrap_or_else(||"Choose crew".into()),i+1,sailing::ROLES[c.role]));
    }
    if seats == 0 {
        h.push_str("<p>A raft has no crew seats.</p>");
    }
    h.push_str("</div></section><section class=\"ship-trip\"><div class=\"ship-section-heading\"><h3>Trip supplies</h3><span>Cargo · separate from your inventory</span></div><div class=\"ship-trip-inputs\">");
    for (key, label, icon, value, min, max) in [
        (
            "balls",
            "Cannonballs",
            [31906, 31908, 2, 31910, 31912, 31914, 31916][b.cannonball],
            b.cannonballs,
            0,
            1_000_000,
        ),
        ("kits", "Repair kits", 32300, b.repair_kits, 0, 10_000),
        (
            "reserved",
            "Cargo slots used",
            32213,
            b.cargo_reserved,
            0,
            240,
        ),
        ("minutes", "Minutes firing", 32199, b.minutes, 1, 120),
        ("uptime", "Firing uptime %", 32160, b.uptime, 0, 100),
    ] {
        h.push_str(&format!(r#"<label>{}<span>{label}<input id="sailing-{key}" type="number" min="{min}" max="{max}" value="{value}"></span></label>"#,item_icon(icon)));
    }
    h.push_str(&format!(r#"<label>{}<span>Boosted Sailing<input id="sailing-visible" type="number" min="1" max="125" value="{}" placeholder="Base level"></span></label>"#,item_icon(32399),b.visible_sailing.map(|n|n.to_string()).unwrap_or_default()));
    h.push_str("</div></section></fieldset>");
    if readonly {
        h.push_str("<p class=\"small-note\">Shared boat assumptions. Rebuild using your items to edit.</p>");
    }
    if let Some(member) = member.filter(|_| !readonly) {
        let conflicts = b.conflicts();
        let checks = b.checks(member);
        if !checks.is_empty() {
            h.push_str(
                "<details class=\"ship-checks\" open><summary>Check your setup</summary><ul>",
            );
            for note in checks {
                h.push_str(&format!("<li>{}</li>", esc(&note)));
            }
            h.push_str("</ul></details>");
        }
        h.push_str("<div class=\"ship-results\">");
        for (i, c) in b
            .crew
            .iter()
            .enumerate()
            .filter(|(_, c)| c.person.is_some() && c.role >= 3)
        {
            match b.crew_max(member,i){Ok(max)=>h.push_str(&format!("<div class=\"ship-gunner-result\">{}<span>{}<strong>{max} raw max hit</strong></span></div>",face(c.person.unwrap()),esc(CREW[c.person.unwrap()].name))),Err(reason)=>h.push_str(&format!("<p class=\"small-note\">Crew max unavailable: {}</p>",esc(&reason)))}
        }
        if conflicts.is_empty() && b.ammo_compatible() {
            let (shots, consumed) = b.ammo_budget();
            h.push_str(&format!("<dl class=\"ship-budget\"><div><dt>Shots planned</dt><dd>{shots:.0}</dd></div><div><dt>Expected balls used</dt><dd>{consumed:.0}</dd></div><div><dt>Cargo slots free</dt><dd>{}</dd></div></dl>",b.cargo_capacity().saturating_sub(b.cargo_reserved)));
        }
        h.push_str("</div>");
    }
    h.push_str("<p class=\"ship-footnote\">Parts, crew and cargo are manual selections. Compare your weapon in Combat.</p><details class=\"ship-coverage\"><summary>Calculation details & sources</summary><p>Crew max hits are before target armour. Cannons fire every 7 ticks; the firing budget uses your chosen uptime. The attractor recovers 60% on average. Allow spare ammunition. Crew must have cannonballs in cargo.</p><p>Sea creatures hit your boat. Your armour and protection prayers do not protect it. Hull HP and keel damage reduction are supported; cannon accuracy, player cannon damage, combined DPS, repair consumption, movement and trip survival are unverified. Granite, chainshot and incendiary cannonballs are not modelled.</p><p>Installed parts and crew recruitment need your confirmation. Cargo counts do not allocate bank stock or change RuneLite exports.</p><a href=\"https://oldschool.runescape.wiki/w/Boat_combat\" target=\"_blank\" rel=\"noreferrer\">Boat combat ↗</a> · <a href=\"https://oldschool.runescape.wiki/w/Crew_Management\" target=\"_blank\" rel=\"noreferrer\">Crew ↗</a></details></section>");
    h
}
