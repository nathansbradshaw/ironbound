//! Local bank item grouping. Unknown names remain available in Other.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Category {
    Food,
    Potions,
    Equipment,
    Runes,
    Ammunition,
    Teleports,
    Tools,
    Other,
}
impl Category {
    pub const ALL: [Self; 8] = [
        Self::Food,
        Self::Potions,
        Self::Equipment,
        Self::Runes,
        Self::Ammunition,
        Self::Teleports,
        Self::Tools,
        Self::Other,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Potions => "Potions",
            Self::Equipment => "Equipment",
            Self::Runes => "Runes",
            Self::Ammunition => "Ammunition",
            Self::Teleports => "Teleports",
            Self::Tools => "Tools",
            Self::Other => "Other",
        }
    }
}
pub fn category(id: i32, name: &str) -> Category {
    let n = name.to_lowercase();
    if n.ends_with(" rune") || n.ends_with(" runes") {
        return Category::Runes;
    }
    if [
        "potion",
        "brew",
        "restore",
        "antipoison",
        "anti-venom",
        "antidote",
        "sanfew",
        "serum",
        "combat mix",
        "energy mix",
        "stamina mix",
        " mix(",
        "antifire",
        "magic essence",
        "remedy",
        "balm",
    ]
    .iter()
    .any(|v| n.contains(v))
        || (n.starts_with("super ")
            && ["(1)", "(2)", "(3)", "(4)"]
                .iter()
                .any(|dose| n.ends_with(dose)))
    {
        return Category::Potions;
    }
    if n.contains("teleport") || n.ends_with(" tablet") {
        return Category::Teleports;
    }
    if n.contains("pickaxe")
        || n.ends_with(" axe")
        || [
            "spade",
            "hammer",
            "chisel",
            "tinderbox",
            "knife",
            "rope",
            "pestle and mortar",
        ]
        .contains(&n.as_str())
    {
        return Category::Tools;
    }
    if let Some(gear) = crate::planner::gear(id) {
        return if gear.slot == "ammo" && !n.contains("blessing") {
            Category::Ammunition
        } else {
            Category::Equipment
        };
    }
    if ["bolt", "arrow", "dart", "javelin", "chinchompa"]
        .iter()
        .any(|v| n.contains(v))
    {
        return Category::Ammunition;
    }
    if !n.starts_with("raw ")
        && !n.contains("uncooked")
        && !n.contains("burnt")
        && !n.contains("dough")
        && ([
            "shark",
            "cooked karambwan",
            "lobster",
            "swordfish",
            "tuna",
            "trout",
            "salmon",
            "monkfish",
            "anglerfish",
            "manta ray",
            "sea turtle",
            "dark crab",
            "bass",
            "sardine",
            "herring",
            "pike",
            "shrimps",
            "anchovies",
            "bread",
            "stew",
            "curry",
            "banana",
            "orange",
            "pineapple",
            "peach",
        ]
        .contains(&n.as_str())
            || ["pie", "pizza", "cake", "potato"]
                .iter()
                .any(|v| n.contains(v)))
    {
        return Category::Food;
    }
    Category::Other
}
