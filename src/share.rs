//! Only encounter and carried items are shareable. Never serialize an account here.
use crate::planner::{self, Boss, Check, Options, Packed, Phase, Plan, Target};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: i32,
    pub quantity: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Switch {
    pub style: String,
    pub equipment: Vec<Option<i32>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    #[serde(
        default = "crate::efficient::default_tolerance",
        skip_serializing_if = "crate::efficient::is_default_tolerance"
    )]
    pub switch_tolerance: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended: Option<bool>,
    #[serde(default)]
    pub ranking: crate::ranking::Preferences,
    #[serde(default)]
    pub combat: crate::defence::Settings,
    #[serde(default)]
    pub constraints: Option<Constraints>,
    pub version: u8,
    pub boss: Boss,
    pub raid_level: u32,
    pub targets: Vec<Target>,
    pub equipment: Vec<Option<i32>>,
    pub inventory: Vec<Item>,
    pub switches: Vec<Switch>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraints {
    pub max_switches: usize,
    pub switch_slots: usize,
    pub focus_style: String,
    #[serde(default)]
    pub phase_priorities: std::collections::BTreeMap<i32, u32>,
}
impl Setup {
    pub fn from_plan(p: &Plan, o: &Options) -> Self {
        Self {
            switch_tolerance: o.switch_tolerance,
            recommended: o.recommended,
            ranking: o.ranking.clone(),
            combat: {
                let mut combat = o.combat.clone();
                if let Some(ammo) = &p.loaded_ammo {
                    combat.loaded_dart = Some(ammo.id);
                }
                combat
            },
            constraints: o.max_switches.map(|max_switches| Constraints {
                max_switches,
                switch_slots: o.switch_slots,
                focus_style: o.focus_style.clone(),
                phase_priorities: o.phase_priorities.clone(),
            }),
            version: 1,
            boss: o.boss,
            raid_level: o.raid_level,
            targets: if o.boss == Boss::Custom {
                o.targets.clone()
            } else {
                vec![]
            },
            equipment: p.equipment.clone(),
            inventory: p
                .inventory
                .iter()
                .map(|i| Item {
                    id: i.id,
                    quantity: i.quantity,
                })
                .collect(),
            switches: p
                .phases
                .iter()
                .map(|s| Switch {
                    style: s.style.clone(),
                    equipment: s.equipment.clone(),
                })
                .collect(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let fail = || "This shared setup is invalid or unsupported.".to_string();
        if self.constraints.as_ref().is_some_and(|c| {
            c.max_switches > 11
                || c.switch_slots > 20
                || !(c.focus_style == "Balanced"
                    || crate::rules::STYLES.contains(&c.focus_style.as_str()))
                || c.phase_priorities.len() > crate::rules::MAX_TARGETS
                || c.phase_priorities
                    .iter()
                    .any(|(id, w)| *w > 10 || crate::planner::monster(*id).is_none())
        }) {
            return Err(fail());
        }
        if self.switch_tolerance > crate::efficient::MAX_TOLERANCE_PERCENT
            || !self.ranking.weights.valid()
            || !self.combat.valid()
            || self.version != 1
            || self.raid_level > 150
            || self.targets.len() > crate::rules::MAX_TARGETS
            || self.targets.iter().any(|t| !t.valid())
            || (self.boss == Boss::Custom && self.targets.is_empty())
            || self.inventory.len() > 28
            || self.switches.len() > crate::rules::MAX_TARGETS
        {
            return Err(fail());
        }
        let expected = match self.boss {
            Boss::Barrows => 2,
            Boss::Toa => 5,
            Boss::Custom => self.targets.len(),
        };
        if self.switches.len() != expected {
            return Err(fail());
        }
        for set in
            std::iter::once(&self.equipment).chain(self.switches.iter().map(|s| &s.equipment))
        {
            if set.len() != 14 {
                return Err(fail());
            }
            if let Some(weapon) = set[3].and_then(crate::rules::weapon) {
                if !weapon.family.internal_ammo.is_empty()
                    && self
                        .combat
                        .loaded_dart
                        .is_some_and(|dart| !weapon.family.internal_ammo.contains(&dart))
                {
                    return Err(fail());
                }
            }
            for (slot, id) in set.iter().enumerate() {
                if let Some(id) = id {
                    let keys = [
                        "head", "cape", "neck", "weapon", "body", "shield", "", "legs", "",
                        "hands", "feet", "", "ring", "ammo",
                    ];
                    if !planner::equipment()
                        .iter()
                        .any(|g| g.id == *id && g.slot == keys[slot])
                    {
                        return Err(fail());
                    }
                }
            }
            if set[3].is_some_and(|id| {
                planner::equipment()
                    .iter()
                    .any(|g| g.id == id && g.two_handed)
            }) && set[5].is_some()
            {
                return Err(fail());
            }
        }
        if self.inventory.iter().any(|i| {
            if i.id == 0 && i.quantity == 0 {
                return false;
            }
            i.id <= 0
                || i.id > 1_000_000
                || i.quantity == 0
                || i.quantity > i32::MAX as u32
                || (!planner::is_stackable(i.id) && i.quantity != 1)
        }) {
            return Err(fail());
        }
        if self.switches.iter().any(|s| {
            ![
                "Magic",
                "Melee",
                "Ranged",
                "Stab",
                "Slash",
                "Crush",
                "All three styles",
                "Magic / Ranged",
            ]
            .contains(&s.style.as_str())
        }) {
            return Err(fail());
        }
        let mut pool = std::collections::BTreeMap::<i32, u32>::new();
        for (slot, id) in self.equipment.iter().enumerate() {
            if let Some(id) = id {
                *pool.entry(*id).or_default() +=
                    planner::equipped_quantity(slot, *id, self.combat.ammo_shots);
            }
        }
        for i in &self.inventory {
            let n = pool.entry(i.id).or_default();
            *n = n.checked_add(i.quantity).ok_or_else(fail)?;
        }
        for s in &self.switches {
            let mut carried = pool.clone();
            for (slot, id) in s.equipment.iter().enumerate() {
                if let Some(id) = id {
                    let n = carried.entry(*id).or_default();
                    *n = n
                        .checked_sub(planner::equipped_quantity(
                            slot,
                            *id,
                            self.combat.ammo_shots,
                        ))
                        .ok_or_else(fail)?;
                }
            }
            let slots: usize = carried
                .iter()
                .map(|(id, q)| {
                    if planner::is_stackable(*id) {
                        usize::from(*q > 0)
                    } else {
                        *q as usize
                    }
                })
                .sum();
            if slots > 28 {
                return Err(fail());
            }
        }
        if let Some(c) = &self.constraints {
            let sets: Vec<_> = std::iter::once(&self.equipment)
                .chain(self.switches.iter().map(|s| &s.equipment))
                .collect();
            if sets.iter().any(|a| {
                sets.iter()
                    .any(|b| crate::optimization::changes(a, b) > c.max_switches)
            }) {
                return Err(fail());
            }
            let slots =
                self.switches
                    .iter()
                    .flat_map(|s| {
                        s.equipment.iter().enumerate().filter_map(|(slot, id)| {
                            id.filter(|id| self.equipment[slot] != Some(*id))
                        })
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
            if slots > c.switch_slots {
                return Err(fail());
            }
        }
        Ok(())
    }
    pub fn fragment(&self) -> Result<String, String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Ok(format!(
            "#setup={}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
    }
    pub fn parse(hash: &str) -> Result<Self, String> {
        let raw = hash
            .strip_prefix("#setup=")
            .ok_or("No shared setup found.")?;
        if raw.len() > 40_000 || raw.len() % 2 != 0 || !raw.is_ascii() {
            return Err("Shared setup exceeds the supported format.".into());
        }
        let bytes = raw
            .as_bytes()
            .chunks_exact(2)
            .map(|c| {
                std::str::from_utf8(c)
                    .ok()
                    .and_then(|s| u8::from_str_radix(s, 16).ok())
                    .ok_or("Invalid shared setup encoding.")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let s: Self =
            serde_json::from_slice(&bytes).map_err(|_| "Could not read this shared setup.")?;
        s.validate()?;
        Ok(s)
    }
    pub fn options(&self) -> Options {
        Options {
            switch_tolerance: self.switch_tolerance,
            recommended: self.recommended,
            ranking: self.ranking.clone(),
            combat: self.combat.clone(),
            boss: self.boss,
            raid_level: self.raid_level,
            targets: self.targets.clone(),
            max_switches: self.constraints.as_ref().map(|c| c.max_switches),
            switch_slots: self
                .constraints
                .as_ref()
                .map(|c| c.switch_slots)
                .unwrap_or(12),
            focus_style: self
                .constraints
                .as_ref()
                .map(|c| c.focus_style.clone())
                .unwrap_or("Balanced".into()),
            phase_priorities: self
                .constraints
                .as_ref()
                .map(|c| c.phase_priorities.clone())
                .unwrap_or_default(),
            ..Default::default()
        }
    }
    pub fn plan(&self) -> Plan {
        let names: Vec<String> = match self.boss {
            Boss::Barrows => vec!["Five brothers".into(), "Ahrim".into()],
            Boss::Toa => ["Ba-ba", "Kephri", "Zebak", "Akkha", "Wardens"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Boss::Custom => self.targets.iter().map(Target::name).collect(),
        };
        Plan{ammo_shots:self.combat.ammo_shots,loaded_ammo:self.combat.loaded_dart.filter(|_|std::iter::once(&self.equipment).chain(self.switches.iter().map(|s|&s.equipment)).any(|set|set[3].and_then(crate::rules::weapon).is_some_and(|r|!r.family.internal_ammo.is_empty()))).map(|id|Packed{id,quantity:self.combat.ammo_shots,purpose:"Shared loaded-ammunition assumption; verify contents".into()}),optimization:None,equipment:self.equipment.clone(),inventory:self.inventory.iter().map(|i|Packed{id:i.id,quantity:i.quantity,purpose:"Shared setup".into()}).collect(),phases:self.switches.iter().enumerate().map(|(i,s)|Phase{name:names.get(i).cloned().unwrap_or_else(||format!("Switch {}",i+1)),style:s.style.clone(),equipment:s.equipment.clone(),note:"Shared equipment; ownership, levels and access have not been checked for your account.".into()}).collect(),checks:vec![Check{name:"Shared setup".into(),state:Some(true),detail:"Carried items and capacity checked. Confirm your own ownership, levels and access.".into()}],warnings:vec!["This link contains only the encounter, equipment and inventory. Its creator’s bank, identity, levels and unlocks remain private.".into()],missing:vec![],switches:self.inventory.iter().filter(|i|planner::equipment().iter().any(|g|g.id==i.id)).count(),supported:0,owned:0,label:"Shared setup; account requirements unverified".into(),spell:match &self.combat.spell_selection { crate::rules::SpellSelection::Spell(name)=>name.clone(), crate::rules::SpellSelection::Element(element)=>format!("{element} spells"), crate::rules::SpellSelection::Auto=>String::new() }}
    }
}
