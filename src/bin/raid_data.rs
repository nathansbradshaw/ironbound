//! Refresh reviewed raid states from the pinned public NPC catalog, never an account.
//! Usage: cargo run --bin raid_data -- public-monsters.json
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};
fn rows(path: &str, key: Option<&str>) -> Vec<Value> {
    let root: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    key.map(|k| root[k].clone())
        .unwrap_or(root)
        .as_array()
        .unwrap()
        .clone()
}
// Preserve unrelated rows and field ordering in these one-object-per-line catalogs.
fn write_rows(path: &str, key: Option<&str>, keep: impl Fn(&Value) -> bool, add: &[Value]) {
    let text = fs::read_to_string(path).unwrap();
    let marker = key
        .map(|k| format!("  \"{k}\": [\n"))
        .unwrap_or("[\n".into());
    let start = text.find(&marker).unwrap() + marker.len();
    let end = if key.is_some() {
        text[start..].find("\n  ]").unwrap() + start
    } else {
        text.rfind(']').unwrap()
    };
    let mut output = Vec::new();
    for line in text[start..end].lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(line).expect("one JSON row per line");
        if keep(&row) {
            output.push(line.to_owned());
        }
    }
    output.extend(add.iter().map(Value::to_string));
    let block = output
        .into_iter()
        .map(|r| format!("    {r}"))
        .collect::<Vec<_>>()
        .join(",\n");
    let tail = if key.is_some() { &text[end..] } else { "\n]\n" };
    fs::write(path, format!("{}{block}{tail}", &text[..start])).unwrap();
}
struct Builder {
    npcs: Vec<Value>,
    windows: Vec<Value>,
    profiles: BTreeMap<i32, Value>,
    old: BTreeMap<i32, Value>,
}
impl Builder {
    fn npc(&self, id: i32, version: Option<&str>) -> Value {
        self.npcs
            .iter()
            .find(|r| r["id"] == id && version.is_none_or(|v| r["version"] == v))
            .unwrap_or_else(|| panic!("Missing public NPC {id} {version:?}"))
            .clone()
    }
    fn window(
        &self,
        id: i32,
        source: i32,
        label: &str,
        style: &str,
        version: Option<&str>,
    ) -> Value {
        let npc = self.npc(source, version);
        let mut skills = npc["skills"].as_object().unwrap().clone();
        skills.remove("hp");
        let mut w = json!({"id":id,"npc_id":source,"label":label,"display_name":label,"phase":label,
            "default_style":style,"damage_model":"verified_standard","comparison_probe":true,
            "skills_override":skills,"defensive_override":npc["defensive"],"attributes_override":npc["attributes"],
            "hitpoints":npc["skills"]["hp"],"progress_group":format!("npc_{source}")});
        if npc["weakness"].is_null() {
            w["clear_weakness"] = json!(true);
        } else {
            w["weakness_override"] = npc["weakness"].clone();
        }
        w
    }
    fn profile(&mut self, source: i32, kind: &str, reach: &str, styles: &[&str]) {
        let row = json!({"ids":[source],"kind":kind,"melee":reach,"allowed_styles":styles});
        if let Some(old) = self.profiles.insert(source, row.clone()) {
            assert_eq!(old, row, "Conflicting NPC rule {source}");
        }
    }
    fn probe(&mut self, mut w: Value, hp: u32, max: u32, note: &str) {
        w["hp_probe"] = json!(hp);
        w["hp_probe_max"] = json!(max);
        w["description"] = json!(note);
        self.windows.push(w);
    }
}
fn target(id: i32, style: &str) -> Value {
    json!({"id":id,"style":style})
}
fn npc_name(npc: &Value) -> String {
    let name = npc["name"].as_str().unwrap();
    let version = npc["version"]
        .as_str()
        .unwrap_or("")
        .replace("Normal mode, ", "")
        .replace("Hard mode, ", "")
        .replace(" (Normal)", "");
    if [
        "",
        "Normal",
        "Normal mode",
        "Hard",
        "Hard mode",
        "Hard Mode",
    ]
    .contains(&version.as_str())
    {
        name.into()
    } else {
        format!("{name} · {version}")
    }
}
fn group(name: &str, targets: Vec<Value>, note: &str) -> Value {
    json!({"name":name,"aliases":[],"targets":targets,"note":note})
}
fn hit_styles(w: &mut Value, styles: &[&str]) {
    w["hit_rule"] = json!({"always_max":false,"allowed_styles":styles});
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).ok_or(
        "Provide the public NPC catalog at revision 74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f",
    )?;
    let old = rows("assets/encounter-mechanics.json", Some("windows"))
        .into_iter()
        .map(|w| (w["id"].as_i64().unwrap() as i32, w))
        .collect();
    let mut b = Builder {
        npcs: serde_json::from_slice(&fs::read(input)?)?,
        windows: vec![],
        profiles: BTreeMap::new(),
        old,
    };
    let all = ["Melee", "Ranged", "Magic"];
    let mut groups = vec![];

    // Nex: vulnerable body, observed-HP phase probes, explicit protection/healing states.
    b.profile(11278, "standard", "any", &all);
    let mut base = b.window(11278, 11278, "Nex · vulnerable body", "Ranged", None);
    base["comparison_alternative"] = json!(true);
    base["description"] = json!(
        "Vulnerable body at recorded stats and 250 Defence floor. This is not a complete phase sequence. Selected Smoke/Ice incoming scenarios remain separate; transitions, siphon healing, curses, team contribution and full-trip supplies are not simulated."
    );
    b.windows.push(base);
    let mut nex = vec![];
    for (i, (name, hp)) in [
        ("Smoke", 3400),
        ("Shadow", 2720),
        ("Blood", 2040),
        ("Ice", 1360),
        ("Zaros", 1180),
    ]
    .into_iter()
    .enumerate()
    {
        let id = 200000 + i as i32;
        let mut w = b.window(
            id,
            11278,
            &format!("Nex · {name} vulnerable"),
            "Ranged",
            None,
        );
        w["comparison_alternative"] = json!(true);
        b.probe(w,hp,3400,"Vulnerable Nex at a declared phase HP point. Current HP may be changed up to 3,400 because healing/threshold overshoot can cross nominal boundaries. Zaros starts from the 680+500 healing reference. This is outgoing damage only: transition immunity, siphon/curses, movement, team contribution, incoming attacks and complete supplies are separate.");
        nex.push(target(id, "Ranged"));
    }
    for (id, label, hp, immune, deflect) in [
        (200005, "Zaros · Deflect Melee", 1180, false, true),
        (200006, "Blood Siphon · attacks heal Nex", 2040, true, false),
        (200007, "Protected transition", 2000, true, false),
    ] {
        let mut w = b.window(id, 11278, &format!("Nex · {label}"), "Ranged", None);
        w["comparison_alternative"] = json!(true);
        w["invulnerable"] = json!(immune);
        if deflect {
            hit_styles(&mut w, &["Ranged", "Magic"]);
        }
        b.probe(w,hp,3400,"Explicit special state. Deflect Melee blocks the melee comparison; use ranged/Magic. Protected transitions deal no HP damage. During Blood Siphon, attacks heal Nex instead, so this shows zero outgoing HP damage and does not quantify healing or retaliation. Timing, overshoot, incoming damage and full-fight supplies are not simulated.");
    }
    for (i, id) in [11283, 11284, 11285, 11286].into_iter().enumerate() {
        b.profile(id, "standard", "any", &all);
        let name = b.npc(id, None)["name"].as_str().unwrap().to_owned();
        let w = b.window(id, id, &format!("{name} · vulnerable"), "Ranged", None);
        b.probe(w,500,500,"One vulnerable Nex mage. Recorded -2 flat armour adds two to each accurate physical hitsplat, including supported bolt procs; misses remain zero. Mage activation, freezes, spawn counts, travel and full incoming sequences are excluded.");
        nex.push(target(id, "Ranged"));
        let mut protected = b.window(
            200010 + i as i32,
            id,
            &format!("{name} · protected"),
            "Ranged",
            None,
        );
        protected["comparison_alternative"] = json!(true);
        protected["invulnerable"] = json!(true);
        b.probe(protected,500,500,"Mage before activation: no outgoing HP damage, including bolt effects. This does not predict the activation sequence or incoming attacks.");
    }
    b.profile(11294, "standard", "any", &all);
    let reaver = b.window(11294, 11294, "Blood Reaver · Nex chamber", "Ranged", None);
    b.probe(reaver,150,150,"One chamber reaver at recorded stats. Spawn counts, remaining-HP transfer to Nex, healing the player and incoming damage are not simulated.");
    groups.push(group("Nex",nex,"Group ranged gear baseline: five vulnerable body HP points and four activated mages. Curses, siphon and protected states are separate comparisons. This does not assess solo viability, team damage share, movement, full incoming attacks or whole-trip supplies."));

    // Chambers: reproducible party-stat reference scales, retained in each target key.
    let routes = rows("assets/raid-routes.json", None);
    let cox = routes.iter().find(|r| r["id"] == "cox").unwrap();
    let cox_targets: Vec<Value> = cox["rooms"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["required"] == true || r["default"] == true)
        .flat_map(|r| r["targets"].as_array().unwrap().clone())
        .collect();
    let mut cox_ids: BTreeSet<i32> = cox_targets
        .iter()
        .map(|t| t["id"].as_i64().unwrap() as i32)
        .collect();
    cox_ids.extend([7533, 7568, 7543]);
    let mut cox_maps = BTreeMap::new();
    for (party, cm, offset) in [
        (1, false, 0),
        (3, false, 201000),
        (5, false, 201100),
        (1, true, 202000),
        (3, true, 202100),
    ] {
        let mut map = BTreeMap::new();
        for (index, source) in cox_ids.iter().copied().enumerate() {
            let id = if offset == 0 {
                source
            } else {
                offset + index as i32
            };
            map.insert(source, id);
            let npc = b.npc(source, None);
            let name = npc_name(&npc);
            let (kind, reach, styles) = match source {
                7540 | 7543 => ("tekton", "any", vec!["Melee", "Magic"]),
                7568 => ("crystal", "any", vec!["Melee", "Magic"]),
                7550 => ("olm_magic", "any", all.to_vec()),
                7552 => ("olm_melee", "any", all.to_vec()),
                7551 => ("olm_head", "any", all.to_vec()),
                7584 => ("ice_demon", "any", all.to_vec()),
                7569 => ("guardian", "any", vec!["Melee"]),
                7530 | 7533 => ("standard", "none", vec!["Ranged", "Magic"]),
                _ => ("standard", "any", all.to_vec()),
            };
            b.profile(source, kind, reach, &styles);
            let style = cox_targets
                .iter()
                .find(|t| t["id"] == source)
                .and_then(|t| t["style"].as_str())
                .unwrap_or(if source == 7568 {
                    "Stab"
                } else if source == 7543 {
                    "Crush"
                } else {
                    "Magic"
                });
            let label = format!(
                "{name} · {}{party}-player reference",
                if cm { "CM · " } else { "" }
            );
            let mut w = b.window(id, source, &label, style, None);
            w["raid_scale"] = json!({"kind":"cox","party_size":party,"challenge":cm,"highest_combat":126,"highest_hp":99,"average_mining":99});
            w["comparison_alternative"] = json!(id != source);
            if source == 7533 {
                w["comparison_sources"] = json!([7530]);
                w["comparison_alternative"] = json!(true);
            }
            if source == 7568 {
                w["comparison_sources"] = json!([7566]);
                w["comparison_alternative"] = json!(true);
            }
            if source == 7543 {
                w["comparison_sources"] = json!([7540]);
                w["comparison_alternative"] = json!(true);
            }
            w["description"] = json!(format!(
                "Recorded {} reference: {party} player(s), highest party Combat 126, highest HP 99, average Mining 99 for Guardian HP. Stat/HP scaling, CM exceptions and Olm hand/head scaling follow the pinned calculator. Guardian damage separately uses your base Mining and pickaxe tier. Tekton/crystal ranged immunity, Olm off-style reductions, Ice demon fire/demonbane exceptions and Vespula/portal reach apply. This is one vulnerable outgoing probe, not room counts, portal knockdown cycles, Olm turns/phases, healing, travel, incoming damage or a complete raid workload.",
                if cm { "Challenge Mode" } else { "normal-mode" }
            ));
            b.windows.push(w);
        }
        cox_maps.insert((party, cm), map);
    }
    for (name, party, cm) in [
        ("Chambers of Xeric", 1, false),
        ("Chambers of Xeric (3-player)", 3, false),
        ("Chambers of Xeric: Challenge Mode", 1, true),
    ] {
        let map = &cox_maps[&(party, cm)];
        let targets = cox_targets
            .iter()
            .map(|t| {
                target(
                    map[&(t["id"].as_i64().unwrap() as i32)],
                    t["style"].as_str().unwrap(),
                )
            })
            .collect();
        groups.push(group(name,targets,"Reviewed vulnerable-room gear reference with party maximum Combat 126 / HP 99 and average Mining 99. Guardian damage uses the selected player's Mining. Portal, crystals, team scales and CM comparisons are available separately. Room counts, Vespula portal/knockdown sequence, Vanguard resets, Olm turns/healing, supplies and full incoming damage remain unverified."));
    }

    // Theatre: canonical catalog NPCs supply stats; explicit colour states supply restrictions.
    let normal = [
        (8360, "Melee"),
        (8359, "Slash"),
        (8354, "Melee"),
        (8387, "Melee"),
        (8340, "Melee"),
        (8369, "Melee"),
        (8372, "Melee"),
        (8374, "Melee"),
    ];
    let hard = [
        (10822, "Melee"),
        (10813, "Slash"),
        (10807, "Melee"),
        (10867, "Melee"),
        (10772, "Melee"),
        (10847, "Melee"),
        (10850, "Melee"),
        (10852, "Melee"),
    ];
    for (is_hard, party, offset) in [(false, 3, 0), (false, 5, 203100), (true, 3, 0)] {
        let bosses = if is_hard { &hard } else { &normal };
        let mut targets = vec![];
        for (index, (source, style)) in bosses.iter().copied().enumerate() {
            let id = if offset == 0 {
                source
            } else {
                offset + index as i32
            };
            let npc = b.npc(source, None);
            let name = npc_name(&npc);
            let p1 = [8369, 10847].contains(&source);
            let nylo = [8354, 10807].contains(&source);
            b.profile(
                source,
                if p1 { "verzik_p1" } else { "standard" },
                "any",
                &all,
            );
            let mut w = b.window(
                id,
                source,
                &format!(
                    "{name} · {}{party}-player{}",
                    if is_hard { "Hard · " } else { "" },
                    if nylo { " · melee colour" } else { "" }
                ),
                style,
                None,
            );
            w["raid_scale"] = json!({"kind":"tob","party_size":party});
            w["comparison_alternative"] = json!(id != source);
            w["readiness_exempt"] = json!(p1);
            if nylo {
                hit_styles(&mut w, &["Melee"]);
            }
            w["description"] = json!(
                "Vulnerable Theatre body at the declared party scale: normal/hard HP uses (max(3, party size)+3)/8 of the catalog's five-player HP. Nylocas colour states accept only their matching style; wrong-style healing/reflection is not quantified. Verzik phase 1 uses a separate per-splat random limiter (0–10 melee, 0–3 ranged/Magic), with Dawnbringer loan probes separately available. Dawnbringer is omitted from the bank-gear rating. Bloat exposure, crab/exhume healing, mazes, webs, transitions, incoming attacks and full raid supplies are not simulated."
            );
            b.windows.push(w);
            targets.push(target(id, style));
            if nylo {
                for (j, colour) in ["Ranged", "Magic"].into_iter().enumerate() {
                    let alt = if is_hard {
                        204000
                    } else if party == 5 {
                        203110
                    } else {
                        203000
                    } + j as i32;
                    let mut w = b.window(
                        alt,
                        source,
                        &format!(
                            "Nylocas Vasilias · {}{party}-player · {colour} colour",
                            if is_hard { "Hard · " } else { "" }
                        ),
                        colour,
                        None,
                    );
                    w["raid_scale"] = json!({"kind":"tob","party_size":party});
                    hit_styles(&mut w, &[colour]);
                    w["comparison_alternative"] = json!(true);
                    w["description"] = json!(
                        "Only the matching colour/style can damage the boss. Wrong styles are blocked because they heal and/or reflect rather than progressing the kill. This is a vulnerable colour probe with party-scaled HP; colour duration, timing, small nylocas waves, healing/reflection and incoming attacks remain unverified."
                    );
                    b.windows.push(w);
                    targets.push(target(alt, colour));
                }
            }
        }
        let name = if is_hard {
            "Theatre of Blood: Hard Mode"
        } else if party == 5 {
            "Theatre of Blood (5-player)"
        } else {
            "Theatre of Blood"
        };
        groups.push(group(name,targets,"Bank-owned tribrid gear baseline at the declared 3-player minimum (solo/duo also use 3-player HP) or 5-player reference. Nylocas colours, Verzik phase-1 limiters and vulnerable bodies are reviewed. Raid-loaned Dawnbringer is omitted from gear readiness and has separate normal/single-special probes. Exposure, heals, waves, special energy, timing, incoming damage and whole-trip supplies are not simulated."));
    }
    for (source, offset) in [(8369, 203200), (10847, 204200)] {
        for special in [false, true] {
            let mut w = b.window(
                offset + i32::from(special),
                source,
                &format!(
                    "Verzik phase 1 · Dawnbringer {} · 3-player",
                    if special {
                        "single special"
                    } else {
                        "normal attack"
                    }
                ),
                "Magic",
                None,
            );
            w["raid_scale"] = json!({"kind":"tob","party_size":3});
            w["comparison_weapon"] = json!(22516);
            w["comparison_alternative"] = json!(true);
            w["readiness_exempt"] = json!(true);
            hit_styles(&mut w, &["Magic"]);
            if special {
                w["fixed_damage"] = json!([75, 150]);
            }
            w["description"] = json!(
                "Explicit raid-loaned Dawnbringer probe: the comparison supplies this weapon and uses no bank ownership or spell runes. Normal attacks use the pinned powered-spell formula, equipment Magic damage and final halving, with guaranteed accuracy and no Verzik limiter. One special is an independent guaranteed 75–150 roll ignoring damage bonuses. The special's nominal weapon cooldown is a comparison only; 35% energy, transfers, regeneration, pillars and total phase timing are not simulated or packed as owned gear."
            );
            b.windows.push(w);
        }
    }

    // Archive: HP is an explicit probe, not an invented team-scaling formula.
    for (source, style) in [
        (16659, "Magic"),
        (16652, "Crush"),
        (16640, "Crush"),
        (16641, "Magic"),
        (16667, "Magic"),
        (16665, "Magic"),
        (16684, "Magic"),
        (16643, "Melee"),
        (16645, "Melee"),
        (16647, "Melee"),
        (16673, "Melee"),
        (16675, "Magic"),
        (16677, "Ranged"),
        (16666, "Melee"),
        (16670, "Ranged"),
        (16678, "Ranged"),
        (16679, "Ranged"),
        (16680, "Ranged"),
        (16681, "Melee"),
    ] {
        let npc = b.npc(source, None);
        let name = if source == 16659 {
            npc["name"].as_str().unwrap().into()
        } else {
            npc_name(&npc)
        };
        b.profile(
            source,
            "standard",
            if source == 16659 { "flying" } else { "any" },
            &all,
        );
        let mut w = b.window(
            source,
            source,
            &format!(
                "{name} · {}",
                if source == 16659 {
                    "flying"
                } else if source == 16640 {
                    "Smorks HP 100–75%"
                } else {
                    "vulnerable recorded stats"
                }
            ),
            style,
            if source == 16659 {
                Some("Flying")
            } else {
                None
            },
        );
        if source == 16640 {
            w["defensive_override"]["flat_armour"] = json!(8);
            w["defensive_override"]["magic_resistance"] = json!(0);
        }
        let hp = (npc["skills"]["hp"].as_u64().unwrap() as u32).min(500);
        b.probe(w,hp,30000,"Recorded Archive defences/attributes and elemental weakness. This uses a declared current-HP point (500 for large bodies, catalog HP for small adds), not an assumed team size or kill workload. Set current HP to the observed value for HP-dependent bolts. Team HP scaling/allocation, route transformations, add counts, full timing, incoming attacks and supplies remain unverified. Bork's default armour is scoped to Smorks HP 100–75%; other armour/stun states are separate.");
    }
    let mut ground = b.window(
        205000,
        16659,
        "Archive Guardian · grounded",
        "Crush",
        Some("Grounded"),
    );
    ground["melee_reach"] = json!("any");
    ground["comparison_alternative"] = json!(true);
    b.probe(ground,500,30000,"Grounded recorded stats, with unrestricted melee reach, changed defences and the same fire weakness. Current HP is an explicit point, not team-scaled total HP. Transformation timing, flight/ground HP transfer, incoming attacks and whole-room supplies are unverified.");
    let mut xarth = b.window(
        205030,
        16684,
        "Xarth · phase 2 vulnerable body",
        "Crush",
        None,
    );
    xarth["comparison_alternative"] = json!(true);
    b.probe(xarth,500,30000,"Phase-2 vulnerable body at recorded defences and 80% Demonbane effectiveness. Phase 1 uses the separate ranged/Magic route suggestion; phase 2 adds a melee gear screen. These are ordinary damage comparisons, not a protection-rotation or shield model. Corpse spawns, water/ice cast-speed changes on corpses, healing, pillars, fire-spell side effects, enrage, incoming damage and complete supplies are excluded.");
    for (i, (label, armour, resistance, stun)) in [
        ("Smorks HP 75–50%", 10, 2, false),
        ("Smorks HP 50–25%", 12, 4, false),
        ("Smorks HP 25–0%", 14, 6, false),
        ("stunned", 0, 0, true),
    ]
    .into_iter()
    .enumerate()
    {
        let mut w = b.window(
            205010 + i as i32,
            16640,
            &format!("Bork · {label}"),
            "Crush",
            None,
        );
        w["defensive_override"]["flat_armour"] = json!(armour);
        w["defensive_override"]["magic_resistance"] = json!(resistance);
        if stun {
            for key in ["stab", "slash", "crush"] {
                let n = w["defensive_override"][key].as_i64().unwrap();
                w["defensive_override"][key] = json!(n - 100);
            }
        }
        w["comparison_alternative"] = json!(true);
        b.probe(w,500,30000,"Pinned Smorks-HP armour state: physical flat armour and Magic resistance apply to each accurate splat. Stun instead lowers all three melee defence bonuses by 100 at the recorded unempowered armour. HP is a comparison point; empowerment/stun uptime, cannonballs, team scaling, incoming damage and full supplies are not simulated.");
    }
    for (id, label, explosion, overhead) in [
        (205020, "phase 4 · no curse", None, ""),
        (205021, "phase 4 · Deflect Melee", None, "Melee"),
        (205022, "phase 4 · Deflect Ranged", None, "Ranged"),
        (
            205023,
            "phase 4 explosions · 2-tick opportunity",
            Some(true),
            "",
        ),
        (
            205024,
            "phase 4 explosions · ordinary cooldown",
            Some(false),
            "",
        ),
    ] {
        let mut w = b.window(id, 16677, &format!("Ocellus · {label}"), "Melee", None);
        w["comparison_alternative"] = json!(true);
        if !overhead.is_empty() {
            let allowed: Vec<_> = all.into_iter().filter(|s| *s != overhead).collect();
            hit_styles(&mut w, &allowed);
            if overhead == "Melee" {
                w["default_style"] = json!("Ranged");
            }
        }
        if let Some(speed) = explosion {
            w["ocellus_explosions"] = json!(speed);
        }
        b.probe(w,500,30000,"Recorded phase-4 body. Deflected styles are blocked; retaliation/healing are not quantified. During explosions each accurate splat becomes floor(2*damage/normal weapon ticks) plus an independent 0/1 coin flip; misses remain zero. The timed opportunity variant compares a 2-tick expected cooldown, while the other retains ordinary cooldown. HP is explicit. Curse/explosion uptime, movement, team HP, incoming attacks, burn and whole-fight supplies are not simulated.");
    }
    // Existing room route chooses Ezekiel/Moia/Xarth; alternative rooms retain their own probes.
    let archive = routes.iter().find(|r| r["id"] == "tfa").unwrap();
    let archive_targets = archive["rooms"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["required"] == true || r["default"] == true)
        .flat_map(|r| r["targets"].as_array().unwrap().clone())
        .collect();
    groups.push(group("The Fractured Archive",archive_targets,"Recorded-defence tribrid gear screen for the default Guardian/Ezekiel/Moia/Xarth/Ocellus route. Large bodies use explicit 500-HP outgoing points; this does not infer team HP scaling or a raid kill time. Other forks, Guardian grounding, Bork armour/stun and Ocellus curses/explosions have separate comparisons. Transformations, team contribution, adds, incoming sequences and full-trip supplies remain unverified."));

    // Expert ToA: a fixed, shareable solo-300/path-zero reference, retaining reviewed hit models.
    let toa = rows("assets/encounter-groups.json", None)
        .into_iter()
        .find(|g| g["name"] == "Tombs of Amascut")
        .unwrap();
    let mut expert = vec![];
    for (i, t) in toa["targets"].as_array().unwrap().iter().enumerate() {
        let source = t["id"].as_i64().unwrap() as i32;
        let style = t["style"].as_str().unwrap();
        let id = 206000 + i as i32;
        let name = npc_name(&b.npc(source, None));
        let mut w = b.window(
            id,
            source,
            &format!("{name} · solo 300 · {style} window"),
            style,
            None,
        );
        if let Some(old) = b.old.get(&source) {
            for field in ["damage_model", "hit_rule", "resource"] {
                if !old[field].is_null() {
                    w[field] = old[field].clone();
                }
            }
        }
        if source == 11789 {
            hit_styles(&mut w, &[style]);
        }
        w["raid_scale"] = json!({"kind":"toa","party_size":1,"invocation":300,"path_level":0});
        w["comparison_alternative"] = json!(true);
        w["description"] = json!(
            "Fixed Expert reference: solo, raid level 300, path level 0. HP/rounding and the 2.2 defence-roll multiplier apply; ToA Fang/Shadow behavior and the reviewed Wardens damage models are retained. Akkha windows accept only the chosen vulnerable style. This is not an arbitrary Expert invocation/team model, protection rotation, core-down/transfer sequence, movement, incoming scenario or full-trip supply estimate."
        );
        b.windows.push(w);
        expert.push(target(id, style));
    }
    groups.push(group("Tombs of Amascut: Expert Mode",expert,"Solo Expert gear reference at exactly raid level 300 / path 0, with tribrid Akkha vulnerable-style probes and reviewed Wardens hit models. Invocation-specific mechanics, other path/team levels, protections, core transfers, timing, incoming attacks and complete supplies are not simulated."));

    let managed: BTreeSet<_> = b
        .windows
        .iter()
        .map(|w| w["id"].as_i64().unwrap())
        .collect();
    write_rows(
        "assets/encounter-mechanics.json",
        Some("windows"),
        |w| !managed.contains(&w["id"].as_i64().unwrap()),
        &b.windows,
    );
    let sources: BTreeSet<_> = b.profiles.keys().map(|id| i64::from(*id)).collect();
    let mut profiles = vec![];
    for mut p in rows("assets/monster-rules.json", Some("profiles")) {
        p["ids"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !sources.contains(&id.as_i64().unwrap()));
        if !p["ids"].as_array().unwrap().is_empty() {
            profiles.push(p);
        }
    }
    profiles.extend(b.profiles.values().cloned());
    write_rows(
        "assets/monster-rules.json",
        Some("profiles"),
        |_| false,
        &profiles,
    );
    let names: BTreeSet<_> = groups
        .iter()
        .map(|g| g["name"].as_str().unwrap().to_owned())
        .collect();
    write_rows(
        "assets/encounter-groups.json",
        None,
        |g| !names.contains(g["name"].as_str().unwrap()),
        &groups,
    );
    let mut mechanics: Value =
        serde_json::from_slice(&fs::read("assets/encounter-mechanics.json")?)?;
    mechanics["unverified_modes"]
        .as_array_mut()
        .unwrap()
        .retain(|v| !names.contains(v.as_str().unwrap()));
    let text = fs::read_to_string("assets/encounter-mechanics.json")?;
    // That array was formatted on one line; replace by key boundaries, preserving all other data.
    let start = text.find("\"unverified_modes\": ").unwrap() + "\"unverified_modes\": ".len();
    let end = start + text[start..].find(']').unwrap() + 1;
    fs::write(
        "assets/encounter-mechanics.json",
        format!(
            "{}{}{}",
            &text[..start],
            mechanics["unverified_modes"],
            &text[end..]
        ),
    )?;
    let text = fs::read_to_string("assets/encounter-mechanics.json")?;
    for (id, percent) in [
        (7584, 115),
        (16665, 25),
        (16673, 80),
        (16677, 80),
        (16684, 80),
    ] {
        mechanics["demonbane_vulnerability"][id.to_string()] = json!(percent);
    }
    let marker = "\"demonbane_vulnerability\": ";
    let start = text.find(marker).unwrap() + marker.len();
    let end = start + text[start..].find('}').unwrap() + 1;
    fs::write(
        "assets/encounter-mechanics.json",
        format!(
            "{}{}{}",
            &text[..start],
            mechanics["demonbane_vulnerability"],
            &text[end..]
        ),
    )?;
    let mut browser: Value = serde_json::from_slice(&fs::read("assets/encounter-browser.json")?)?;
    for name in [
        "Chambers of Xeric (3-player)",
        "Theatre of Blood (5-player)",
    ] {
        if !browser["raids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == name)
        {
            browser["raids"].as_array_mut().unwrap().push(json!(name));
        }
    }
    fs::write(
        "assets/encounter-browser.json",
        format!(
            "{{\n  \"raids\": {},\n  \"groups\": {},\n  \"popular\": {}\n}}\n",
            browser["raids"], browser["groups"], browser["popular"]
        ),
    )?;
    let mut portraits: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read("assets/boss-portraits.json")?)?;
    portraits.insert(
        "Chambers of Xeric (3-player)".into(),
        portraits["Chambers of Xeric"].clone(),
    );
    portraits.insert(
        "Theatre of Blood (5-player)".into(),
        portraits["Theatre of Blood"].clone(),
    );
    fs::write(
        "assets/boss-portraits.json",
        serde_json::to_vec(&portraits)?,
    )?;
    let mut weapons: Value = serde_json::from_slice(&fs::read("assets/weapon-rules.json")?)?;
    weapons["families"]["dawnbringer"] = json!({"accuracy_bonus":11,"attack_types":["magic"],"denominator":3,"formula":"powered","numerator":1,"offset":-2,"style":"Magic","ticks":5});
    let definition = json!({"family":"dawnbringer","ids":[22516],"requirements":{}});
    let mut wr = weapons["weapons"].as_array().unwrap().clone();
    wr.retain(|w| !w["ids"].as_array().unwrap().contains(&json!(22516)));
    wr.insert(0, definition);
    write_rows("assets/weapon-rules.json", Some("weapons"), |_| false, &wr);
    let text = fs::read_to_string("assets/weapon-rules.json")?;
    if !text.contains("\"dawnbringer\":") {
        fs::write(
            "assets/weapon-rules.json",
            text.replacen(
                "  \"families\": {\n",
                &format!(
                    "  \"families\": {{\n    \"dawnbringer\":{},\n",
                    weapons["families"]["dawnbringer"]
                ),
                1,
            ),
        )?;
    }
    println!(
        "Refreshed {} raid/Nex attack states and {} encounter references from the pinned public catalog.",
        b.windows.len(),
        groups.len()
    );
    Ok(())
}
