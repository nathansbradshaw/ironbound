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
pub const FILTERS: [&str; 11] = [
    "For you",
    "Well equipped",
    "Doable",
    "A struggle",
    "Requirements missing",
    "Not assessed",
    "Popular",
    "Bosses",
    "Raids",
    "Groups",
    "All",
];
pub const CATEGORY_FILTERS: [&str; 6] = ["For you", "Bosses", "Raids", "Groups", "All", "Popular"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GearFilter {
    #[default]
    Any,
    WellEquipped,
    Doable,
    Struggle,
    Unknown,
}
impl GearFilter {
    pub const ALL: [Self; 5] = [
        Self::Any,
        Self::WellEquipped,
        Self::Doable,
        Self::Struggle,
        Self::Unknown,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "Any gear",
            Self::WellEquipped => "Well equipped",
            Self::Doable => "Doable",
            Self::Struggle => "A struggle",
            Self::Unknown => "Not assessed",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.label() == s)
    }
    pub fn matches(self, a: &Assessment) -> bool {
        self == Self::Any || a.readiness.label() == self.label()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RequirementsFilter {
    #[default]
    Any,
    LevelsMet,
    Missing,
    Unknown,
}
impl RequirementsFilter {
    pub const ALL: [Self; 4] = [Self::Any, Self::LevelsMet, Self::Missing, Self::Unknown];
    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "Any requirements",
            Self::LevelsMet => "Known levels met",
            Self::Missing => "Requirements missing",
            Self::Unknown => "Level unknown",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.label() == s)
    }
    pub fn matches(self, a: &Assessment) -> bool {
        match self {
            Self::Any => true,
            Self::LevelsMet => matches!(
                a.requirements.status,
                RequirementStatus::LevelMet | RequirementStatus::NoLevelGate
            ),
            Self::Missing => a.requirements.status == RequirementStatus::Missing,
            Self::Unknown => a.requirements.status == RequirementStatus::Unverified,
        }
    }
}
pub fn matches_filters(a: &Assessment, gear: GearFilter, requirements: RequirementsFilter) -> bool {
    gear.matches(a) && requirements.matches(a)
}

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementStatus {
    Missing,
    LevelMet,
    NoLevelGate,
    Unverified,
}
impl RequirementStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Missing => "Requirements missing",
            Self::LevelMet => "Slayer level met",
            Self::Unverified => "Access unverified",
            Self::NoLevelGate => "Access unverified",
        }
    }
    fn class(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::LevelMet => "level-met",
            Self::Unverified => "unverified",
            Self::NoLevelGate => "unverified",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Requirements {
    pub status: RequirementStatus,
    pub reason: String,
}
impl Default for Requirements {
    fn default() -> Self {
        Self {
            status: RequirementStatus::Unverified,
            reason: "Quest, task and other access requirements are not checked.".into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Assessment {
    pub readiness: Readiness,
    pub requirements: Requirements,
    pub reason: String,
    pub detail: String,
    pub wilderness: bool,
    pub preview: Option<crate::wilderness::Preview>,
}
impl Assessment {
    pub fn matches_filter(&self, filter: &str) -> bool {
        if filter == "Requirements missing" {
            self.requirements.status == RequirementStatus::Missing
        } else {
            self.readiness.label() == filter
        }
    }
}
fn unknown(reason: &str) -> Assessment {
    Assessment {
        readiness: Readiness::Unknown,
        requirements: Requirements::default(),
        reason: reason.into(),
        detail: String::new(),
        wilderness: false,
        preview: None,
    }
}
struct Profile {
    level: u32,
    prayer: u32,
    styles: &'static [&'static str],
    slayer: u32,
    note: &'static str,
}
fn profile(name: &str) -> Option<Profile> {
    let (level, styles, slayer, note): (_, &[&str], _, _) = match name {
        "Doom of Mokhaiotl" => (
            90,
            &["Ranged", "Melee", "Demonbane shield"],
            0,
            "Deep-delve ranged body, ordinary melee and the best usable owned demonbane shield style. No melee-punish bonus is assumed. Larvae, acid, beam timing, incoming attacks and full-trip supplies are not scored.",
        ),
        "Shellbane Gryphon" => (
            65,
            &["Melee"],
            51,
            "Post-quest melee baseline with a usable tortugan shield. Equipment weight, knockback counters, corrosion and whirlwinds are not scored; task and quest access remain unverified.",
        ),
        "Mad Angel" => (
            75,
            &["Melee"],
            0,
            "Post-quest ordinary damage baseline. Perfect counters, reflected blasts, enrage and incoming damage are not assumed.",
        ),
        "Maggot King" => (
            85,
            &["Ranged", "Melee"],
            0,
            "Ranged far-phase and melee nearby gear baseline. No fire burn or timed punish bonus is assumed. Larvae, healing, bile, phase timing and incoming damage remain unverified.",
        ),
        "Sol Heredit" => (
            90,
            &["Melee"],
            0,
            "Sol-only boss gear baseline. Colosseum waves, invocations, grapples, perfect parries, positioning and incoming damage are not scored.",
        ),
        "TzKal-Zuk" => (
            95,
            &["Ranged"],
            0,
            "Zuk-only exposed-window gear baseline. Inferno waves, moving shield uptime, adds, healers and incoming attacks are not scored.",
        ),
        "Inferno" => (
            95,
            &["Ranged", "Magic"],
            0,
            "Enemy-type gear baseline including Zuk. Wave counts, moving shield uptime, adds, healers, protection execution and full-trip supplies are not scored.",
        ),
        "Fortis Colosseum" => (
            90,
            &["Melee", "Ranged", "Magic"],
            0,
            "Enemy-type gear baseline including Sol. Wave counts, reinforcements, invocations, grapples, parries, positioning and incoming attacks are not scored.",
        ),
        "Fragment of Seren" => (
            80,
            &["Magic"],
            0,
            "Vulnerable-body Magic gear baseline with the per-splat limiter. Song of the Elves access, healing beams, clones, movement, near-max-HP special and full-fight supplies are not scored.",
        ),
        "Turoths" | "Kurasks" => (
            if name == "Turoths" { 55 } else { 65 },
            &["Melee", "Ranged", "Magic"],
            if name == "Turoths" { 55 } else { 70 },
            "Ordinary mob gear baseline with required leaf-bladed, broad ammunition or Magic Dart equipment. The best usable owned style is screened; Slayer access, task, superior encounters and full-trip supplies remain separate.",
        ),
        "Scurrius" | "Obor" | "Bryophyta" => (
            50,
            &["Melee", "Ranged", "Magic"],
            0,
            "Learn protection prayers and movement; keys may be needed.",
        ),
        "Brutus" => (
            40,
            &["Melee", "Ranged", "Magic"],
            0,
            "Post-quest baseline. Bring the cowbell amulet and food; charges, stomps and safespot movement are not scored.",
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
        "Dagannoth Prime" => (
            75,
            &["Ranged"],
            0,
            "Prime-only ranged baseline. Protect from Magic, room entry, the other Kings and Spinolyps still need planning.",
        ),
        "Dagannoth Supreme" => (
            75,
            &["Melee"],
            0,
            "Supreme-only melee baseline. Protect from Missiles, room entry, the other Kings and Spinolyps still need planning.",
        ),
        "Dagannoth Kings" => (
            80,
            &["Melee", "Ranged", "Magic"],
            0,
            "Tribrid baseline: melee for Supreme, ranged for Prime and magic for Rex. The weakest required setup determines the rating; overlapping aggro, respawns and sustained supplies are not scored.",
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
        "Moons of Peril" | "Lunar Chests" => (
            75,
            &["Melee"],
            0,
            "Defence matters heavily. Blood healing and special phases can change the outcome.",
        ),
        "Mimic" => (
            75,
            &["Melee", "Magic", "Ranged"],
            0,
            "Ordinary attacks against the Mimic body. Clue access, special attacks, freezing, candy minions and dodging stomps are not scored.",
        ),
        "Tormented Demons" => (
            80,
            &["Melee", "Ranged", "Magic"],
            0,
            "Bring two different attack styles for protection switches, protection prayers and prayer restoration. While Guthix Sleeps access is not checked. This is a shielded outgoing gear screen; unshielded attacks, phase uptime, incoming damage and full-fight supplies are not scored.",
        ),
        "Skotizo" => (
            80,
            &["Melee", "Ranged"],
            0,
            "Body damage assumes the altars are cleared. Bring a dark totem and a way to disable altars; Arclight/Emberlight can disable them in one hit. Altar timing, adds and running are not scored.",
        ),
        "Corporeal Beast" => (
            90,
            &["Melee", "Magic"],
            0,
            "Undrained body baseline with the reviewed Corpbane damage rules. Stab spears/halberds or a fang avoid the usual melee reduction. Team support, stat-drain specials, the dark core and incoming damage are not scored.",
        ),
        "TzTok-Jad" | "Fight Caves" => (
            75,
            &["Ranged"],
            0,
            "Ranged gear baseline, including Jad. Practise prayer switches and healer management; a rating does not assess surviving all 63 waves or having enough ammunition, prayer restoration and food for the run.",
        ),
        "Tombs of Amascut" => (
            80,
            &["Melee", "Ranged", "Magic"],
            0,
            "Solo, raid-level-0 gear baseline across all three required styles. The weakest setup determines the rating. Invocations, path/team scaling, puzzles, protection rotations and full-raid supplies are not scored; this is not an Expert-mode rating.",
        ),
        "Yama" => (
            85,
            &["Melee", "Magic"],
            0,
            "Hybrid baseline: melee on the boss and Judge, magic on the Judge and Void Flares. Both setups are required. Fast flare clearing, regeneration, poison control, glyphs, movement, duo support and contracts need separate checks.",
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
        "The Hueycoatl" => (
            80,
            &["Melee"],
            0,
            "Unboosted body/head/tail gear screen. Pillar uptime, team contribution, phase transitions, movement, incoming damage and full-trip supplies are not scored.",
        ),
        "The Royal Titans" => (
            70,
            &["Melee", "Ranged", "Magic"],
            0,
            "Both titans, their out-of-reach ranged windows and elemental adds need all three styles. Add counts, barriers, duo contribution, movement, incoming attacks and full-trip supplies are not scored.",
        ),
        "Nightmare" | "The Nightmare" => (
            90,
            &["Melee", "Magic"],
            0,
            "Base one-to-five-player shield/totem gear screen. Both melee and Magic are needed. Team support, protection, stat recovery, parasites, special attacks and full-trip supplies are not scored.",
        ),
        "Phosani's Nightmare" => (
            95,
            &["Melee", "Magic"],
            0,
            "Solo shield/totem gear screen. Both melee and Magic are needed; bring Sanfew serum or Relicym's balm for parasites. Maximum crush damage does not guarantee killing an add in one hit. Spores, movement, protection and full-trip supplies are not scored.",
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
        prayer: if matches!(name, "Nightmare" | "The Nightmare" | "Phosani's Nightmare") {
            70
        } else {
            43
        },
        styles,
        slayer,
        note,
    })
}
fn requires_all(name: &str) -> bool {
    matches!(
        name,
        "Barrows Chests"
            | "Zulrah"
            | "Phantom Muspah"
            | "Grotesque Guardians"
            | "Abyssal Sire"
            | "Dagannoth Kings"
            | "Tombs of Amascut"
            | "Yama"
            | "Nightmare"
            | "The Nightmare"
            | "Phosani's Nightmare"
            | "The Royal Titans"
            | "Doom of Mokhaiotl"
            | "Maggot King"
            | "Inferno"
            | "Fortis Colosseum"
    )
}

/// Build five independent owned sets once per account screening, not per boss.
/// Neutral unboosted settings prevent the current boss's debuffs or prayers from
/// leaking into unrelated encounters. Manual slot overrides do not constrain discovery.
pub fn assess_catalog(snapshot: &Snapshot, preferences: &Options) -> Vec<Assessment> {
    let mut assessments = assess_gear_catalog(snapshot, preferences);
    for (entry, assessment) in crate::encounters::catalog().iter().zip(&mut assessments) {
        if snapshot.selected().is_some() && profile(&entry.name).is_some_and(|p| p.slayer == 0) {
            assessment.requirements = Requirements {
                status: RequirementStatus::NoLevelGate,
                reason: "No recorded Slayer level requirement. Quest, key, task and other access checks remain unverified.".into(),
            };
        }
        let Some(minimum) = profile(&entry.name).map(|p| p.slayer).filter(|n| *n > 0) else {
            continue;
        };
        assessment.requirements = match snapshot.selected().and_then(|m| m.level("Slayer")) {
            Some(level) => Requirements {
                status: if level < minimum {
                    RequirementStatus::Missing
                } else {
                    RequirementStatus::LevelMet
                },
                reason: if level < minimum {
                    format!(
                        "Needs {minimum} Slayer (yours: {level}). Task and other access checks remain unverified."
                    )
                } else {
                    format!(
                        "{minimum} Slayer requirement met (yours: {level}). Task and other access checks remain unverified."
                    )
                },
            },
            None => Requirements {
                status: RequirementStatus::Unverified,
                reason: format!(
                    "Add your Slayer level to check the {minimum} Slayer requirement. Task and other access checks remain unverified."
                ),
            },
        };
    }
    assessments
}
fn assess_gear_catalog(snapshot: &Snapshot, preferences: &Options) -> Vec<Assessment> {
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
    let mut owned = snapshot.available(options.shared);
    owned.retain(|id, _| !options.item_excluded(*id));
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
        if !entry.bank_equipment { return unknown("This encounter needs its own equipment or skilling readiness model."); }
        if entry.targets.is_empty() { return unknown(if crate::encounters::mechanics().unverified_modes.contains(&entry.name) { "This mode needs its own scaling and readiness model; the base-mode rating does not apply." } else { "A verified target mapping is needed before gear readiness can be rated." }); }
        let guideline = profile(&entry.name);
        let mut incomplete: Vec<_> = entry.targets.iter().filter_map(|t| {
            let target = planner::monster(t.id);
            target.is_none_or(|m| crate::encounters::limitation(m).is_some()).then(|| target.map(|m| m.name.as_str()).unwrap_or("unmapped target"))
        }).collect();
        incomplete.dedup();
        if !incomplete.is_empty() {
            let mut a = unknown(&format!("Unreviewed phases: {}{}.", incomplete.iter().take(2).copied().collect::<Vec<_>>().join(", "), if incomplete.len() > 2 { format!(" and {} more", incomplete.len()-2) } else { String::new() }));
            a.detail = format!("Readiness needs reviewed calculations for: {}. Check the encounter coverage notes for the remaining mechanics.", incomplete.join(", "));
            return a;
        }
        let Some(p) = guideline else { return unknown("No reviewed readiness guideline yet. Open the gear plan where available."); };
        let mut entry_sets = sets.clone();
        if matches!(entry.name.as_str(), "Turoths" | "Kurasks") {
            for (style,set) in &sets {
                let candidates: &[i32] = match rules::base_style(style) {
                    "Melee" => &[20727,4158,11902],
                    "Magic" => &[4170,21255],
                    _ => &[],
                };
                for id in candidates.iter().filter(|id| owned.get(id).is_some_and(|q| *q>0) && planner::can_use(**id,&member)) {
                    let mut alternative = set.clone();
                    alternative[3] = Some(*id);
                    if planner::gear(*id).is_some_and(|g| g.two_handed) { alternative[5] = None; }
                    entry_sets.push((*style,alternative));
                }
                if rules::base_style(style) == "Ranged" {
                    for id in [4160,11875,21316,33601].into_iter().filter(|id| owned.get(id).is_some_and(|q| *q>=options.combat.ammo_shots) && planner::can_use(*id,&member) && set[3].is_some_and(|w| rules::ammo_ids(w).contains(id))) {
                        let mut alternative = set.clone();
                        alternative[13] = Some(id);
                        entry_sets.push((*style,alternative));
                    }
                }
            }
        }
        if entry.name == "Doom of Mokhaiotl" {
            // Generic winners may be unable to damage the shield. Screen each
            // owned, usable demonbane weapon in its matching style baseline.
            for (style, set) in &sets {
                for id in owned.keys().filter(|id| planner::can_use(**id, &member)
                    && rules::weapon(**id).is_some_and(|w| w.family.style == rules::base_style(style)
                        && (w.weapon.family == "demonbane" || crate::monster_rules::is_demonbane(**id, if w.family.style == "Ranged" {"standard"} else {"slash"}, "")))) {
                    let mut alternative = set.clone();
                    alternative[3] = Some(*id);
                    if planner::gear(*id).is_some_and(|g| g.two_handed) { alternative[5] = None; }
                    if rules::base_style(style) == "Ranged" {
                        alternative[13] = rules::ammo_ids(*id).iter().rev().find(|ammo| owned.get(ammo).is_some_and(|q| *q >= options.combat.ammo_shots) && planner::can_use(**ammo,&member)).copied();
                    }
                    entry_sets.push((*style, alternative));
                }
            }
        }
        if entry.name == "Shellbane Gryphon" {
            if !owned.get(&31398).is_some_and(|q| *q > 0) || !planner::can_use(31398, &member) {
                return Assessment { readiness: Readiness::Struggle, requirements: Requirements::default(), reason: "No usable owned tortugan shield for the protected melee baseline.".into(), detail: p.note.into(), wilderness: false, preview: None };
            }
            // The tortugan shield is worn on the back (cape slot), not in the
            // off-hand slot. Its offensive cost must be included in the score.
            for (_, set) in &mut entry_sets { set[1] = Some(31398); }
        }
        if matches!(entry.name.as_str(), "TzTok-Jad" | "Fight Caves") && member.level("Prayer").unwrap() < 43 {
            return Assessment { readiness: Readiness::Struggle, requirements: Requirements::default(), reason: format!("43 Prayer is recommended for this ranged baseline (yours: {}).", member.level("Prayer").unwrap()), detail: p.note.into(), wilderness: false, preview: None };
        }
        let mut choices = vec![];
        for required in p.styles {
            let shield_role = *required == "Demonbane shield";
            let mut best: Option<(f64, &str, i32, u32, usize)> = None;
            for (style, set) in &entry_sets {
                let actual_style = rules::base_style(style);
                if !shield_role && actual_style != *required { continue; }
                let Some(weapon) = set[3] else { continue; };
                let level = match actual_style { "Melee" => member.level("Attack").unwrap().min(member.level("Strength").unwrap()), "Magic" => member.level("Magic").unwrap(), _ => member.level("Ranged").unwrap() };
                let armour = [0, 4, 7, 9, 10].iter().filter(|slot| set[**slot].is_some()).count();
                let mut lowest = f64::INFINITY;
                let mut valid = true;
                for target in &entry.targets {
                    // Tormented Demons need any two styles. Score each against
                    // a compatible explicit overhead, rather than requiring Magic.
                    if entry.name == "Tormented Demons" && crate::monster_rules::style_limitation(target.id, required).is_some() { continue; }
                    // Mixed encounters use each recorded phase's intended style.
                    if entry.name == "Doom of Mokhaiotl" {
                        if (target.id == 100178) != shield_role || !shield_role && rules::base_style(&target.style) != *required { continue; }
                    } else if requires_all(&entry.name) && rules::base_style(&target.style) != *required { continue; }
                    let Some(monster) = planner::monster(target.id) else { valid = false; break; };
                    let spell = rules::spell_name_for_target(&member, set, &owned, &options.combat, monster, 0);
                    if actual_style == "Magic" && (!rules::casting_supported_for_style(&member, set, &owned, &options.combat, style) || rules::spell_requirement(&member, set, &owned, &options.combat, spell).is_some()) { valid = false; break; }
                    match combat::calculate_configured(&member, set, style, monster, spell, 0, &options.combat) { Ok(result) if result.dps.is_finite() => lowest = lowest.min(result.dps), _ => { valid = false; break; } }
                }
                if valid && lowest.is_finite() && best.as_ref().is_none_or(|b| lowest > b.0) { best = Some((lowest, *style, weapon, level, armour)); }
            }
            if let Some(choice) = best { choices.push(choice); } else if requires_all(&entry.name) { return Assessment { readiness: Readiness::Struggle, requirements: Requirements::default(), reason: format!("No usable owned {required} setup with supported damage. Check weapons, ammunition and runes."), detail: p.note.into(), wilderness: false, preview: None }; }
        }
        if choices.is_empty() { return Assessment { readiness: Readiness::Struggle, requirements: Requirements::default(), reason: "No usable owned setup with supported damage. Check weapons, ammunition and runes.".into(), detail: p.note.into(), wilderness: false, preview: None }; }
        if !requires_all(&entry.name) {
            choices.sort_by(|a,b| b.0.total_cmp(&a.0));
            if entry.name == "Tormented Demons" {
                if choices.len() < 2 {
                    return Assessment { readiness: Readiness::Struggle, requirements: Requirements::default(), reason: "Two supported owned attack styles are needed for protection switches.".into(), detail: p.note.into(), wilderness: false, preview: None };
                }
                choices.truncate(2);
            } else { choices.truncate(1); }
        }
        let mut readiness = Readiness::WellEquipped;
        let hp = member.level("Hitpoints").unwrap();
        let def = member.level("Defence").unwrap();
        let prayer = member.level("Prayer").unwrap();
        for (dps, _, _, level, armour) in &choices {
            let band = if *dps < 1.5 || *level + 15 < p.level || hp + 20 < p.level || *armour < 2 || prayer < 37 { Readiness::Struggle }
                else if *dps < 3.0 || *level < p.level || hp < p.level || def + 10 < p.level || *armour < 4 || prayer < p.prayer { Readiness::Doable }
                else { Readiness::WellEquipped };
            readiness = readiness.max(band);
        }
        let setup_reason = choices.iter().map(|(dps, style, weapon, _, _)| format!("{} · {style} · {dps:.1} basic DPS", planner::item_name(*weapon))).collect::<Vec<_>>().join("; ");
        let scope = match entry.name.as_str() {
            "Tombs of Amascut" => "Solo · raid level 0",
            "Yama" => "Melee-boss hybrid",
            "Corporeal Beast" => "Undrained baseline",
            "Fight Caves" => "Ranged gear baseline",
            "Nightmare" | "The Nightmare" => "Base 1–5-player gear",
            "Phosani's Nightmare" => "Solo shield/totem gear",
            "Doom of Mokhaiotl" => "Deep-delve gear · body and demonbane shield",
            "Sol Heredit" => "Sol-only gear · waves excluded",
            "TzKal-Zuk" => "Zuk-only gear · waves excluded",
            _ => "",
        };
        let reason = if scope.is_empty() { setup_reason } else { format!("{scope}: {setup_reason}") };
        let gaps = if readiness == Readiness::WellEquipped { "Levels and owned gear meet this screening baseline.".into() } else { format!("Guideline: {} offensive level / HP, {} Defence, {} Prayer; 3+ basic DPS and four armour slots for the top band. Yours: {} HP, {} Defence, {} Prayer.", p.level, p.level.saturating_sub(10), p.prayer, hp, def, prayer) };
        Assessment { readiness, requirements: Requirements::default(), reason, detail: format!("{gaps} {}", p.note), wilderness: false, preview: None }
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
    let location = match crate::wilderness::exposure(&entry.name) {
        crate::wilderness::Exposure::Combat => "<span class=\"wild-tag\">Wilderness</span>",
        crate::wilderness::Exposure::Route => "<span class=\"wild-tag\">Wilderness route</span>",
        crate::wilderness::Exposure::None => "",
    };
    let requirement_class = a.requirements.status.class();
    let requirement_status = a.requirements.status.label();
    let requirement_reason = esc(&a.requirements.reason);
    let (requirement_summary, requirement_details) = match a.requirements.status {
        RequirementStatus::Missing | RequirementStatus::LevelMet => (
            format!(
                r#"<div class="boss-requirements" aria-label="Entry requirements"><span class="requirement-badge requirement-{requirement_class}">{requirement_status}</span><p>{requirement_reason}</p></div>"#
            ),
            String::new(),
        ),
        RequirementStatus::Unverified | RequirementStatus::NoLevelGate => {
            (String::new(), format!("<p>{requirement_reason}</p>"))
        }
    };
    let kit=a.preview.as_ref().map(|p| {
        let mut html=String::from("<div class=\"wilderness-kit\"><strong>Exact screened kit</strong><ul>");
        for (slot,id) in p.equipment.iter().enumerate() {
            if let Some(id)=id {
                let quantity=crate::wilderness::gear_quantity(*id,crate::rules::DEFAULT_LOADED_SHOTS);
                let valuable=!crate::wilderness::disposable(*id,quantity,p.kit.filler_cap);
                html.push_str(&format!("<li>{}<span>{}{}<small>{} · {}</small></span></li>",crate::tooltip::item_icon(*id),esc(&planner::item_name(*id)),if quantity>1{format!(" ×{quantity}")}else{String::new()},planner::SLOTS[slot],if valuable{"Valuable / unpriced"}else{"Disposable filler"}));
            }
        }
        html.push_str("</ul><p>All other equipment slots are empty. Food, runes, charged contents and other supplies need a separate risk check.</p></div>");html
    }).unwrap_or_default();
    format!(
        r#"<article class="bestiary-card"><button class="encounter-choice bestiary-open" data-action="choose-encounter:{index}" {} aria-label="Plan {}"><span class="boss-portrait">{}</span><span class="boss-entry"><span class="boss-entry-heading"><strong>{}</strong><span class="boss-readiness readiness-{}">Gear: {}</span></span>{location}<span class="boss-reason">{}</span></span><span class="boss-open-arrow" aria-hidden="true">→</span></button>{requirement_summary}<details class="bestiary-notes"><summary>Details & readiness</summary>{requirement_details}<div class="boss-entry"><span class="boss-stats">{stats}</span><small>{context}</small></div>{kit}<p>{}</p><p>{}</p><small>{}</small></details></article>"#,
        if entry.targets.is_empty() || a.wilderness && a.preview.is_none() {
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

pub(crate) fn rate_setup(
    member: &crate::account::Member,
    name: &str,
    style: &str,
    set: &[Option<i32>],
    dps: f64,
) -> Option<Readiness> {
    let p = profile(name)?;
    if !p.styles.contains(&rules::base_style(style)) {
        return None;
    }
    let offence = match rules::base_style(style) {
        "Melee" => member.level("Attack")?.min(member.level("Strength")?),
        "Magic" => member.level("Magic")?,
        _ => member.level("Ranged")?,
    };
    let hp = member.level("Hitpoints")?;
    let def = member.level("Defence")?;
    let prayer = member.level("Prayer")?;
    let armour = [0, 4, 7, 9, 10]
        .iter()
        .filter(|slot| set[**slot].is_some())
        .count();
    Some(
        if dps < 1.5 || offence + 15 < p.level || hp + 20 < p.level || armour < 2 || prayer < 37 {
            Readiness::Struggle
        } else if dps < 3.0
            || offence < p.level
            || hp < p.level
            || def + 10 < p.level
            || armour < 4
            || prayer < 43
        {
            Readiness::Doable
        } else {
            Readiness::WellEquipped
        },
    )
}
/// Wilderness mode replaces the damage/gear assessment; a full-kit badge is never reused.
pub fn assess_wilderness(snapshot: &Snapshot, preferences: &Options, cap: u32) -> Vec<Assessment> {
    let mut results = assess_catalog(snapshot, preferences);
    for (entry, a) in crate::encounters::catalog().iter().zip(&mut results) {
        if crate::wilderness::exposure(&entry.name) == crate::wilderness::Exposure::None {
            continue;
        }
        a.wilderness = true;
        if a.readiness == Readiness::Unknown {
            a.detail = format!("{} {}", a.detail, crate::wilderness::CAUTION);
            continue;
        }
        if let Some(preview) =
            crate::wilderness::preview(snapshot, preferences, entry, cap, a.readiness)
        {
            a.readiness = preview.readiness;
            a.reason = format!(
                "3-item kit: {} · {} · {:.1} basic DPS. Full kit: {}.",
                planner::item_name(preview.equipment[3].unwrap()),
                preview.style,
                preview.dps,
                preview.full_readiness.label()
            );
            a.detail = format!(
                "{} Disposable gear ceiling: {cap} GP per piece or ammunition stack. Public price snapshot: {}. Unknown prices use a valuable slot. {}",
                profile(&entry.name).map(|p| p.note).unwrap_or_default(),
                crate::wilderness::prices().captured_at,
                crate::wilderness::CAUTION
            );
            a.preview = Some(preview);
        } else {
            a.readiness = Readiness::Unknown;
            a.reason="No supported three-item kit found. Check owned weapons, compatible ammunition and runes.".into();
            a.detail = crate::wilderness::CAUTION.into();
        }
    }
    results
}
