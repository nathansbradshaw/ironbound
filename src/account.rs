use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// GroupIron's wire order is alphabetical, with Sailing appended, not RuneLite's Skill enum.
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
    pub name: String,
    pub skills: Option<Vec<u64>>,
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
    pub fn selected(&self) -> Option<&Member> {
        self.members
            .iter()
            .find(|m| m.name == self.selected && m.name != "@SHARED")
    }
    pub fn parse(raw: &str, fetched_at: String) -> Result<Self, String> {
        if raw.len() > 2_000_000 {
            return Err("This import is too large (maximum 2 MB).".into());
        }
        let v: Value = serde_json::from_str(raw).map_err(|_| "Could not read JSON. Paste the raw GroupIron group-data response or an Ironbound backup.".to_string())?;
        let members: Vec<Member> = serde_json::from_value(if v.is_array() { v } else if v.get("members").is_some() { v["members"].clone() } else { return Err("Expected a GroupIron member array or a snapshot with members.".into()); }).map_err(|_| "Member data has an unsupported type. Skills must be XP arrays and item containers must be ID/quantity pairs.".to_string())?;
        if members.is_empty() || members.len() > 20 {
            return Err("Expected between 1 and 20 group members.".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for m in &members {
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
            source: "GroupIron import".into(),
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
    pub fn level(&self, skill: &str) -> Option<u32> {
        SKILLS
            .iter()
            .position(|s| *s == skill)
            .and_then(|i| self.skills.as_ref()?.get(i))
            .map(|xp| level_for_xp(*xp))
    }
    pub fn total_level(&self) -> Option<u32> {
        self.skills
            .as_ref()
            .map(|s| s.iter().map(|x| level_for_xp(*x)).sum())
    }
    pub fn quest(&self, name: &str) -> Option<u8> {
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
            1540, 1, 1359, 1,
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
