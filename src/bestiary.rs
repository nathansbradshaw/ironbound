//! Conservative, local gear-and-level screening. These bands are planner policy,
//! not kill probabilities; movement, access, supplies and player execution matter.
use crate::{
    account::Snapshot,
    combat,
    encounters::Entry,
    planner::{self, Boss, Options},
    rules,
};
use std::{collections::BTreeMap, sync::OnceLock};
pub const FILTERS: [&str; 9] = [
    "For you",
    "Well equipped",
    "Doable",
    "A struggle",
    "Popular",
    "Bosses",
    "Raids",
    "Groups",
    "All",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Readiness {
    WellEquipped,
    Doable,
    Struggle,
    Unknown,
}
impl Readiness {
    pub fn label(self) -> &'static str {
        match self {
            Self::WellEquipped => "Well equipped",
            Self::Doable => "Doable",
            Self::Struggle => "A struggle",
            Self::Unknown => "Not assessed",
        }
    }
    pub fn class(self) -> &'static str {
        match self {
            Self::WellEquipped => "ready",
            Self::Doable => "doable",
            Self::Struggle => "struggle",
            Self::Unknown => "unknown",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Assessment {
    pub readiness: Readiness,
    pub reason: String,
    pub detail: String,
}
fn unknown(reason: &str) -> Assessment {
    Assessment {
        readiness: Readiness::Unknown,
        reason: reason.into(),
        detail: String::new(),
    }
}
struct Profile {
    level: u32,
    styles: &'static [&'static str],
    slayer: u32,
    note: &'static str,
}
fn profile(name: &str) -> Option<Profile> {
    let (level, styles, slayer, note): (_, &[&str], _, _) = match name {
        "Scurrius" | "Obor" | "Bryophyta" => (
            50,
            &["Melee", "Ranged", "Magic"],
            0,
            "Learn protection prayers and movement; keys may be needed.",
        ),
        "Barrows Chests" => (
            60,
            &["Magic", "Ranged"],
            0,
            "Bring a spade and prayer restoration. Tunnels and prayer drain still matter.",
        ),
        "Giant Mole" | "Sarachnis" | "Hespori" | "Amoxliatl" => (
            65,
            &["Melee"],
            0,
            "Check access and encounter supplies before travelling.",
        ),
        "Dagannoth Rex" => (
            60,
            &["Magic"],
            0,
            "Assumes the isolated safespot method; reaching the room remains dangerous.",
        ),
        "Crazy Archaeologist" | "Deranged Archaeologist" | "Scorpia" => (
            65,
            &["Magic"],
            0,
            "Check poison protection and escape supplies; some encounters are in the Wilderness.",
        ),
        "King Black Dragon" => (
            70,
            &["Ranged", "Melee"],
            0,
            "Dragonfire protection is essential; the route crosses the Wilderness.",
        ),
        "Calvar'ion" | "Vet'ion" | "Spindel" | "Venenatis" | "Artio" | "Callisto" => (
            75,
            &["Melee", "Ranged", "Magic"],
            0,
            "Wilderness risk and boss-specific immunities need a manual check.",
        ),
        "Chaos Elemental" | "Chaos Fanatic" => (
            75,
            &["Ranged"],
            0,
            "Wilderness risk, positioning and equipment disruption still matter.",
        ),
        "Vorkath" => (
            80,
            &["Ranged", "Melee"],
            0,
            "Needs Dragon Slayer II, dragonfire and venom protection, and a spawn solution.",
        ),
        "Zulrah" | "Phantom Muspah" => (
            80,
            &["Magic", "Ranged"],
            0,
            "Both styles are screened. Rotations, protection and phase mechanics need practice.",
        ),
        "Moons of Peril" => (
            75,
            &["Melee"],
            0,
            "Defence matters heavily. Blood healing and special phases can change the outcome.",
        ),
        "General Graardor" | "K'ril Tsutsaroth" | "Kalphite Queen" => (
            85,
            &["Melee", "Ranged"],
            0,
            "Solo baseline; minions, alternative methods and team size change difficulty.",
        ),
        "Commander Zilyana" | "Kree'Arra" => (
            85,
            &["Ranged"],
            0,
            "Solo baseline; kiting, minions and stamina are not scored.",
        ),
        "Duke Sucellus" | "Vardorvis" => (
            85,
            &["Melee"],
            0,
            "Quest access and precise movement matter even with strong equipment.",
        ),
        "The Leviathan" => (
            85,
            &["Ranged"],
            0,
            "Quest access, prayer switches and the enrage phase need practice.",
        ),
        "The Whisperer" => (
            85,
            &["Magic"],
            0,
            "Quest access, sanity and movement are outside this screening.",
        ),
        "Grotesque Guardians" => (
            75,
            &["Melee", "Ranged"],
            75,
            "Requires a gargoyle task and brittle key; both styles are screened.",
        ),
        "Abyssal Sire" => (
            85,
            &["Melee", "Magic"],
            85,
            "Requires an abyssal demon task; stun and transition mechanics need review.",
        ),
        "Kraken" => (75, &["Magic"], 87, "Requires a kraken task and 87 Slayer."),
        "Cerberus" => (
            85,
            &["Melee"],
            91,
            "Requires a hellhound task; ghost prayers and supplies are not scored.",
        ),
        "Thermonuclear Smoke Devil" => (
            85,
            &["Melee", "Magic"],
            93,
            "Requires a smoke devil task and 93 Slayer.",
        ),
        "Alchemical Hydra" => (
            85,
            &["Ranged"],
            95,
            "Requires a hydra task; protection and phase positioning still matter.",
        ),
        "Araxxor" => (
            85,
            &["Melee"],
            92,
            "Requires an araxyte task; venom protection and movement still matter.",
        ),
        _ => return None,
    };
    Some(Profile {
        level,
        styles,
        slayer,
        note,
    })
}
fn requires_all(name: &str) -> bool {
    matches!(
        name,
        "Barrows Chests" | "Zulrah" | "Phantom Muspah" | "Grotesque Guardians" | "Abyssal Sire"
    )
}

/// Build five independent owned sets once per account screening, not per boss.
/// Neutral unboosted settings prevent the current boss's debuffs or prayers from
/// leaking into unrelated encounters. Manual slot overrides do not constrain discovery.
pub fn assess_catalog(snapshot: &Snapshot, preferences: &Options) -> Vec<Assessment> {
    if !planner::missing_combat_levels(snapshot).is_empty() {
        return crate::encounters::catalog()
            .iter()
            .map(|_| unknown("Add all seven combat levels to assess readiness."))
            .collect();
    }
    let options = Options {
        boss: Boss::Custom,
        shared: preferences.shared,
        excluded: preferences.excluded.clone(),
        exclude_void: preferences.exclude_void,
        recommended: Some(false),
        ..Default::default()
    };
    let owned = snapshot.available(options.shared);
    let member = planner::member_with_stock(snapshot.selected().expect("checked levels"), &owned);
    let sets: Vec<_> = ["Stab", "Slash", "Crush", "Ranged", "Magic"]
        .into_iter()
        .map(|style| {
            (
                style,
                planner::equipment_for_style(snapshot, &options, style),
            )
        })
        .collect();
    crate::encounters::catalog().iter().map(|entry| {
        let Some(p) = profile(&entry.name) else { return unknown("No reviewed readiness guideline yet. Open the gear plan where available."); };
        if !entry.bank_equipment || entry.targets.is_empty() { return unknown("This encounter needs its own equipment or mode model."); }
        if p.slayer > 0 && !member.level("Slayer").is_some_and(|level| level >= p.slayer) {
            return Assessment { readiness: Readiness::Struggle, reason: format!("Needs {} Slayer (yours: {}).", p.slayer, member.level("Slayer").map(|v| v.to_string()).unwrap_or_else(|| "unknown".into())), detail: p.note.into() };
        }
        // All mapped windows must support outgoing estimates before an overall label.
        if entry.targets.iter().any(|t| planner::monster(t.id).is_none_or(|m| crate::encounters::limitation(m).is_some())) { return unknown("Phase calculations are incomplete; readiness is not rated yet."); }
        let mut choices = vec![];
        for required in p.styles {
            let mut best: Option<(f64, &str, i32, u32, usize)> = None;
            for (style, set) in &sets {
                if rules::base_style(style) != *required { continue; }
                let Some(weapon) = set[3] else { continue; };
                let level = match *required { "Melee" => member.level("Attack").unwrap().min(member.level("Strength").unwrap()), "Magic" => member.level("Magic").unwrap(), _ => member.level("Ranged").unwrap() };
                let armour = [0, 4, 7, 9, 10].iter().filter(|slot| set[**slot].is_some()).count();
                let mut lowest = f64::INFINITY;
                let mut valid = true;
                for target in &entry.targets {
                    // Mixed encounters use each recorded phase's intended style.
                    if requires_all(&entry.name) && rules::base_style(&target.style) != *required { continue; }
                    let Some(monster) = planner::monster(target.id) else { valid = false; break; };
                    let spell = rules::spell_name_for_target(&member, set, &owned, &options.combat, monster, 0);
                    if *required == "Magic" && rules::spell_requirement(&member, set, &owned, &options.combat, spell).is_some() { valid = false; break; }
                    match combat::calculate_configured(&member, set, style, monster, spell, 0, &options.combat) { Ok(result) if result.dps.is_finite() => lowest = lowest.min(result.dps), _ => { valid = false; break; } }
                }
                if valid && lowest.is_finite() && best.as_ref().is_none_or(|b| lowest > b.0) { best = Some((lowest, *style, weapon, level, armour)); }
            }
            if let Some(choice) = best { choices.push(choice); } else if requires_all(&entry.name) { return Assessment { readiness: Readiness::Struggle, reason: format!("No usable owned {required} setup with supported damage. Check weapons, ammunition and runes."), detail: p.note.into() }; }
        }
        if choices.is_empty() { return Assessment { readiness: Readiness::Struggle, reason: "No usable owned setup with supported damage. Check weapons, ammunition and runes.".into(), detail: p.note.into() }; }
        if !requires_all(&entry.name) { choices.sort_by(|a,b| b.0.total_cmp(&a.0)); choices.truncate(1); }
        let mut readiness = Readiness::WellEquipped;
        let hp = member.level("Hitpoints").unwrap();
        let def = member.level("Defence").unwrap();
        let prayer = member.level("Prayer").unwrap();
        for (dps, _, _, level, armour) in &choices {
            let band = if *dps < 1.5 || *level + 15 < p.level || hp + 20 < p.level || *armour < 2 || prayer < 37 { Readiness::Struggle }
                else if *dps < 3.0 || *level < p.level || hp < p.level || def + 10 < p.level || *armour < 4 || prayer < 43 { Readiness::Doable }
                else { Readiness::WellEquipped };
            readiness = readiness.max(band);
        }
        let reason = choices.iter().map(|(dps, style, weapon, _, _)| format!("{} · {style} · {dps:.1} basic DPS", planner::item_name(*weapon))).collect::<Vec<_>>().join("; ");
        let gaps = if readiness == Readiness::WellEquipped { "Levels and owned gear meet this screening baseline.".into() } else { format!("Guideline: {} offensive level / HP, {} Defence, 43 Prayer; 3+ basic DPS and four armour slots for the top band. Yours: {} HP, {} Defence, {} Prayer.", p.level, p.level.saturating_sub(10), hp, def, prayer) };
        Assessment { readiness, reason, detail: format!("{gaps} {}", p.note) }
    }).collect()
}

pub fn portrait(entry: &Entry) -> String {
    static ICONS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/boss-portraits.json")).expect("boss portraits")
    });
    icons.get(&entry.name).map(|data| format!(r#"<img src="data:image/png;base64,{data}" alt="" width="96" height="96" loading="lazy">"#)).unwrap_or_else(|| "<span class=\"portrait-missing\" aria-label=\"Artwork unavailable\">?</span>".into())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
/// Pure markup also used by native verification; no account data enters images.
pub fn card(entry: &Entry, index: usize, a: &Assessment) -> String {
    let stats = entry
        .targets
        .first()
        .and_then(|t| planner::monster(t.id))
        .map(|m| {
            let hp = m
                .skills
                .get("hp")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into());
            let def = m
                .skills
                .get("def")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into());
            let magic = m
                .skills
                .get("magic")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into());
            format!(
                "<span>♥ {hp} HP</span><span>⛨ {def} Defence</span><span>✦ {magic} Magic</span>"
            )
        })
        .unwrap_or_else(|| "<span>Recorded stats unavailable</span>".into());
    let context = if entry.targets.len() > 1 {
        "First recorded phase · base stats"
    } else {
        "Recorded target · base stats"
    };
    format!(
        r#"<article class="bestiary-card"><button class="encounter-choice bestiary-open" data-action="choose-encounter:{index}" {} aria-label="Plan {}"><span class="boss-portrait">{}</span><span class="boss-entry"><span class="boss-entry-heading"><strong>{}</strong><span class="boss-readiness readiness-{}">{}</span></span><span class="boss-stats">{stats}</span><small>{context}</small><span class="boss-reason">{}</span></span><span class="boss-open-arrow" aria-hidden="true">→</span></button><details class="bestiary-notes"><summary>Readiness & encounter notes</summary><p>{}</p><p>{}</p><small>{}</small></details></article>"#,
        if entry.targets.is_empty() {
            "disabled"
        } else {
            ""
        },
        esc(&entry.name),
        portrait(entry),
        esc(&entry.name),
        a.readiness.class(),
        a.readiness.label(),
        esc(&a.reason),
        esc(&a.detail),
        esc(&entry.note),
        esc(entry.coverage())
    )
}
