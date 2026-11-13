use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// Internal snapshot skill order. External hiscores are mapped by skill name.
pub const SKILLS: [&str; 24] = [
    "Agility",
    "Attack",
    "Construction",
    "Cooking",
    "Crafting",
    "Defence",
    "Farming",
    "Firemaking",
    "Fishing",
    "Fletching",
    "Herblore",
    "Hitpoints",
    "Hunter",
    "Magic",
    "Mining",
    "Prayer",
    "Ranged",
    "Runecraft",
    "Slayer",
    "Smithing",
    "Strength",
    "Thieving",
    "Woodcutting",
    "Sailing",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Member {
    #[serde(default)]
    pub sailing_owned: crate::sailing_owned::Fleet,
    pub name: String,
    #[serde(default)]
    pub item_names: BTreeMap<i32, String>,
    pub skills: Option<Vec<u64>>,
    #[serde(default)]
    pub level_overrides: BTreeMap<String, u32>,
    #[serde(default)]
    pub quest_overrides: BTreeMap<String, u8>,
    pub bank: Option<Vec<i64>>,
    pub inventory: Option<Vec<i64>>,
    pub equipment: Option<Vec<i64>>,
    pub shared_bank: Option<Vec<i64>>,
    pub rune_pouch: Option<Vec<i64>>,
    pub potion_storage: Option<Vec<i64>>,
    pub quests: Option<Vec<u8>>,
    pub last_updated: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub members: Vec<Member>,
    pub selected: String,
    pub fetched_at: String,
    pub source: String,
}

impl Snapshot {
    pub fn from_bank_tsv(raw: &str, name: &str, fetched_at: String) -> Result<Self, String> {
        if raw.len() > 2_000_000 {
            return Err("Bank exports must be smaller than 2 MB.".into());
        }
        let mut lines = raw.lines();
        if lines
            .next()
            .map(|s| s.trim_start_matches('\u{feff}').trim_end())
            != Some("Item id\tItem name\tItem quantity")
        {
            return Err("Expected RuneLite's Item id / Item name / Item quantity tab-separated bank export.".into());
        }
        let mut items = BTreeMap::<i32, u32>::new();
        let mut item_names = BTreeMap::new();
        for (i, line) in lines.enumerate() {
            if i >= 10_000 {
                return Err("Too many bank rows.".into());
            }
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<_> = line.split('\t').collect();
            if cols.len() != 3 {
                return Err(format!(
                    "Bank row {} needs exactly three tab-separated columns.",
                    i + 2
                ));
            }
            let id = cols[0]
                .trim()
                .parse::<i32>()
                .map_err(|_| format!("Invalid item ID at row {}.", i + 2))?;
            let q = cols[2]
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid quantity at row {}.", i + 2))?;
            if id <= 0 || id > 1_000_000 || q > i32::MAX as u32 {
                return Err(format!(
                    "Item ID or quantity is out of range at row {}.",
                    i + 2
                ));
            }
            if q > 0 {
                if cols[1].len() > 120 {
                    return Err("Item name exceeds 120 characters.".into());
                }
                item_names.insert(id, cols[1].trim().to_string());
                let entry = items.entry(id).or_default();
                *entry = entry
                    .checked_add(q)
                    .filter(|q| *q <= i32::MAX as u32)
                    .ok_or("Combined item quantities are too large")?;
            }
        }
        if items.is_empty() {
            return Err("No non-zero items were found in this bank export.".into());
        }
        let bank = items
            .into_iter()
            .flat_map(|(id, q)| [id as i64, q as i64])
            .collect();
        let name = if name.trim().is_empty() {
            "Imported Ironman"
        } else {
            name.trim()
        };
        Ok(Self {
            members: vec![Member {
                name: name.into(),
                item_names,
                bank: Some(bank),
                ..Default::default()
            }],
            selected: name.into(),
            source: "RuneLite bank export".into(),
            fetched_at,
        })
    }
    pub fn selected(&self) -> Option<&Member> {
        self.members
            .iter()
            .find(|m| m.name == self.selected && m.name != "@SHARED")
    }
    pub fn parse(raw: &str, fetched_at: String) -> Result<Self, String> {
        if raw.len() > 2_000_000 {
            return Err("This import is too large (maximum 2 MB).".into());
        }
        let v: Value = serde_json::from_str(raw).map_err(|_| {
            "Could not read JSON. Import a RuneLite bank table or an Ironbound backup.".to_string()
        })?;
        let members: Vec<Member> = serde_json::from_value(if v.is_array() {
            v
        } else if v.get("members").is_some() {
            v["members"].clone()
        } else {
            return Err("Expected an Ironbound snapshot with members.".into());
        })
        .map_err(|_| "Account data has unsupported field types.".to_string())?;
        if members.is_empty() || members.len() > 20 {
            return Err("Expected between 1 and 20 group members.".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for m in &members {
            if !m.sailing_owned.valid() {
                return Err("Invalid Sailing availability.".into());
            }
            if m.quest_overrides.len() > 500
                || m.quest_overrides
                    .iter()
                    .any(|(name, state)| name.len() > 100 || *state > 2)
            {
                return Err("Invalid quest confirmations.".into());
            }
            if m.item_names.len() > 10_000
                || m.item_names
                    .iter()
                    .any(|(id, name)| *id <= 0 || *id > 1_000_000 || name.len() > 120)
            {
                return Err("Invalid item labels in the backup.".into());
            }
            if m.level_overrides.len() > 24
                || m.level_overrides
                    .iter()
                    .any(|(s, l)| !SKILLS.contains(&s.as_str()) || !(1..=99).contains(l))
            {
                return Err(
                    "Local skill corrections must name a supported skill and a level from 1 to 99."
                        .into(),
                );
            }
            if m.name.trim().is_empty() || m.name.len() > 30 || !names.insert(m.name.to_lowercase())
            {
                return Err("Each member needs a distinct name of at most 30 characters.".into());
            }
            if let Some(skills) = &m.skills {
                if !(23..=24).contains(&skills.len()) || skills.iter().any(|x| *x > 200_000_000) {
                    return Err(format!(
                        "{}: expected 23 or 24 skill XP values, each at most 200 million.",
                        m.name
                    ));
                }
            }
            for pairs in [
                &m.bank,
                &m.inventory,
                &m.equipment,
                &m.shared_bank,
                &m.rune_pouch,
                &m.potion_storage,
            ]
            .into_iter()
            .flatten()
            {
                if pairs.len() % 2 != 0 || pairs.len() > 10_000 {
                    return Err(format!(
                        "{}: item containers must contain complete ID/quantity pairs.",
                        m.name
                    ));
                }
                for pair in pairs.chunks_exact(2) {
                    if pair[0] < -1
                        || pair[0] > 1_000_000
                        || pair[1] < 0
                        || pair[1] > i32::MAX as i64
                    {
                        return Err(
                            "Item IDs or quantities are outside the supported range.".into()
                        );
                    }
                }
            }
            if m.quests
                .as_ref()
                .is_some_and(|q| q.len() > 500 || q.iter().any(|s| *s > 2))
            {
                return Err("Unsupported quest state array.".into());
            }
        }
        let selected = members
            .iter()
            .find(|m| m.name != "@SHARED")
            .ok_or("No player was included in this snapshot.")?
            .name
            .clone();
        Ok(Self {
            members,
            selected,
            fetched_at,
            source: "Account import".into(),
        })
    }
    pub fn available(&self, include_shared: bool) -> BTreeMap<i32, u32> {
        let mut out = BTreeMap::new();
        if let Some(m) = self.selected() {
            for c in [&m.bank, &m.inventory, &m.equipment, &m.rune_pouch]
                .into_iter()
                .flatten()
            {
                add_pairs(&mut out, c);
            }
            if include_shared {
                // The same shared chest can be reported on several members: choose one snapshot, never sum copies.
                let dedicated = self.members.iter().find(|m| m.name == "@SHARED");
                let container = dedicated
                    .and_then(|s| s.bank.as_ref().or(s.shared_bank.as_ref()))
                    .or_else(|| m.shared_bank.as_ref())
                    .or_else(|| {
                        self.members
                            .iter()
                            .filter(|m| m.name != "@SHARED")
                            .max_by_key(|m| &m.last_updated)
                            .and_then(|m| m.shared_bank.as_ref())
                    });
                if let Some(c) = container {
                    add_pairs(&mut out, c);
                }
            }
        }
        out
    }
}

pub fn add_pairs(out: &mut BTreeMap<i32, u32>, pairs: &[i64]) {
    for p in pairs.chunks_exact(2) {
        if p[0] > 0 && p[1] > 0 {
            let q = out.entry(p[0] as i32).or_default();
            *q = q.saturating_add(p[1] as u32);
        }
    }
}

pub fn xp_for_level(level: u32) -> u64 {
    (1..level.min(100))
        .map(|n| (n as f64 + 300.0 * 2f64.powf(n as f64 / 7.0)).floor() as u64)
        .sum::<u64>()
        / 4
}
pub fn level_for_xp(xp: u64) -> u32 {
    (1..=99).rev().find(|l| xp >= xp_for_level(*l)).unwrap_or(1)
}
impl Member {
    pub fn apply_hiscores(&mut self, raw: &str) -> Result<(), String> {
        if raw.len() > 1_000_000 {
            return Err("Skill response is too large.".into());
        }
        let v: Value =
            serde_json::from_str(raw).map_err(|_| "The skill response is not valid JSON.")?;
        let mut levels = BTreeMap::new();
        if let Some(skills) = v["skills"].as_array() {
            for skill in skills {
                if let (Some(name), Some(level)) = (skill["name"].as_str(), skill["level"].as_u64())
                {
                    if let Some(key) = SKILLS.iter().find(|s| s.eq_ignore_ascii_case(name)) {
                        if (1..=99).contains(&level) {
                            levels.insert((*key).to_string(), level as u32);
                        }
                    }
                }
            }
        } else if let Some(skills) = v["latestSnapshot"]["data"]["skills"].as_object() {
            for (name, skill) in skills {
                if let (Some(key), Some(level)) = (
                    SKILLS.iter().find(|s| s.eq_ignore_ascii_case(name)),
                    skill["level"].as_u64(),
                ) {
                    if (1..=99).contains(&level) {
                        levels.insert((*key).to_string(), level as u32);
                    }
                }
            }
        } else {
            return Err("This response has no supported skill data.".into());
        }
        if [
            "Attack",
            "Strength",
            "Defence",
            "Ranged",
            "Magic",
            "Prayer",
            "Hitpoints",
        ]
        .iter()
        .any(|s| !levels.contains_key(*s))
        {
            return Err(
                "The response is missing one or more combat levels. Enter them manually instead."
                    .into(),
            );
        }
        self.level_overrides = levels;
        self.last_updated = v["latestSnapshot"]["createdAt"]
            .as_str()
            .or(v["updatedAt"].as_str())
            .map(str::to_string);
        Ok(())
    }
    pub fn level(&self, skill: &str) -> Option<u32> {
        if let Some(level) = self.level_overrides.get(skill) {
            return Some(*level);
        }
        SKILLS
            .iter()
            .position(|s| *s == skill)
            .and_then(|i| self.skills.as_ref()?.get(i))
            .map(|xp| level_for_xp(*xp))
    }
    pub fn total_level(&self) -> Option<u32> {
        SKILLS
            .iter()
            .map(|s| self.level(s))
            .collect::<Option<Vec<_>>>()
            .map(|s| s.iter().sum())
    }
    pub fn quest(&self, name: &str) -> Option<u8> {
        if let Some(state) = self.quest_overrides.get(name) {
            return Some(*state);
        }
        let map: BTreeMap<u32, Value> =
            serde_json::from_str(include_str!("../assets/quests.json")).ok()?;
        let ids: Vec<_> = map
            .iter()
            .filter(|(_, v)| v.get("hidden") != Some(&Value::Bool(true)))
            .collect();
        let index = ids
            .iter()
            .position(|(_, v)| v["name"].as_str() == Some(name))?;
        self.quests.as_ref()?.get(index).copied()
    }
}

pub fn demo() -> Snapshot {
    let mut skills = vec![xp_for_level(65); 24];
    for (s, l) in [
        ("Attack", 75),
        ("Strength", 78),
        ("Defence", 70),
        ("Ranged", 76),
        ("Magic", 77),
        ("Prayer", 60),
        ("Hitpoints", 78),
        ("Herblore", 58),
        ("Crafting", 70),
        ("Slayer", 72),
    ] {
        skills[SKILLS.iter().position(|n| *n == s).unwrap()] = xp_for_level(l);
    }
    let quests: BTreeMap<u32, Value> =
        serde_json::from_str(include_str!("../assets/quests.json")).unwrap();
    let states = quests
        .values()
        .filter(|v| v.get("hidden") != Some(&Value::Bool(true)))
        .map(|v| {
            if [
                "Priest in Peril",
                "Underground Pass",
                "His Faithful Servants",
                "Beneath Cursed Sands",
                "The Fremennik Isles",
                "Animal Magnetism",
                "Dragon Slayer I",
                "Lost City",
            ]
            .contains(&v["name"].as_str().unwrap_or(""))
            {
                2
            } else {
                1
            }
        })
        .collect();
    let m = Member {
        name: "Iron Adventurer".into(),
        skills: Some(skills),
        quests: Some(states),
        bank: Some(vec![
            1381, 1, 12658, 1, 9185, 1, 4587, 1, 25979, 1, 1215, 1, 10828, 1, 1712, 1, 10499, 1,
            2413, 1, 4091, 1, 4093, 1, 2503, 1, 2497, 1, 7462, 1, 3105, 1, 12954, 1, 3842, 1, 2550,
            1, 9242, 600, 9142, 800, 556, 5000, 560, 2000, 565, 1000, 562, 1000, 554, 5000, 563,
            400, 8007, 30, 952, 1, 385, 80, 2434, 12, 139, 8, 141, 6, 3040, 8, 2444, 8, 2440, 8,
            2442, 8, 12625, 4, 1099, 1, 1135, 1, 1169, 1, 1731, 1, 2570, 1, 11905, 1, 2491, 1,
            1540, 1, 1275, 1,
        ]),
        last_updated: None,
        ..Default::default()
    };
    Snapshot {
        members: vec![m],
        selected: "Iron Adventurer".into(),
        fetched_at: "Example account · not connected".into(),
        source: "Demo".into(),
    }
}
