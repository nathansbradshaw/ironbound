//! Reviewed main-game upgrade examples. This is a reference progression, not a universal BIS proof.
use crate::planner::{self, Gear};
pub const KERIS_VARIANTS: [(i32, &str); 5] = [
    (25979, "Base: kalphite damage bonus and a rare triple hit."),
    (
        25981,
        "Breaching: adds accuracy against kalphites. Requires the actual upgraded weapon.",
    ),
    (
        27291,
        "Sun: accuracy benefit at low target HP in ToA, plus a utility special. Full-HP DPS excludes those conditional benefits.",
    ),
    (
        27287,
        "Corruption: a group-support special. Its special-attack benefit is not included in sustained DPS.",
    ),
    (
        30891,
        "Amascut: stronger stab and strength stats in ToA, with 15% kalphite damage instead of 33%. Requires the actual upgraded weapon and 80 Attack.",
    ),
];
pub fn tiers(style: &str, slot: usize) -> Vec<&'static Gear> {
    let names: &[&str] = match (style, slot) {
        ("Magic", 0) => &[
            "Ancestral hat",
            "Virtus mask",
            "Infinity hat",
            "Ahrim's hood",
            "Farseer helm",
            "Blue wizard hat",
        ],
        ("Magic", 1) => &["Imbued Zamorak cape", "Zamorak cape", "Ardougne cloak 4"],
        ("Magic", 2) => &[
            "Occult necklace",
            "Amulet of fury",
            "Amulet of glory",
            "Amulet of magic",
        ],
        ("Magic", 4) => &[
            "Ancestral robe top",
            "Virtus robe top",
            "Ahrim's robetop",
            "Mystic robe top",
            "Wizard robe",
        ],
        ("Magic", 7) => &[
            "Ancestral robe bottom",
            "Virtus robe bottom",
            "Ahrim's robeskirt",
            "Mystic robe bottom",
        ],
        ("Magic", 5) => &[
            "Elidinis' ward (f)",
            "Arcane spirit shield",
            "Elidinis' ward",
            "Mage's book",
            "Book of darkness",
            "Tome of Fire",
        ],
        ("Magic", 9) => &[
            "Tormented bracelet",
            "Barrows gloves",
            "Infinity gloves",
            "Mystic gloves",
        ],
        ("Magic", 10) => &[
            "Eternal boots",
            "Infinity boots",
            "Mystic boots",
            "Wizard boots",
        ],
        ("Magic", 12) => &[
            "Magus ring",
            "Seers ring (i)",
            "Ring of shadows",
            "Beacon ring",
        ],
        ("Ranged", 0) => &[
            "Masori mask (f)",
            "Masori mask",
            "Armadyl helmet",
            "Robin hood hat",
            "Archer helm",
        ],
        ("Ranged", 1) => &["Ava's assembler", "Ava's accumulator", "Ava's attractor"],
        ("Ranged", 2) => &["Necklace of anguish", "Amulet of fury", "Amulet of glory"],
        ("Ranged", 4) => &[
            "Masori body (f)",
            "Masori body",
            "Armadyl chestplate",
            "Black d'hide body",
            "Red d'hide body",
        ],
        ("Ranged", 7) => &[
            "Masori chaps (f)",
            "Masori chaps",
            "Armadyl chainskirt",
            "Black d'hide chaps",
            "Red d'hide chaps",
        ],
        ("Ranged", 5) => &[
            "Twisted buckler",
            "Dragonfire ward",
            "Odium ward",
            "Book of law",
            "Unholy book",
        ],
        ("Ranged", 9) => &[
            "Zaryte vambraces",
            "Barrows gloves",
            "Black d'hide vambraces",
        ],
        ("Ranged", 10) => &[
            "Pegasian boots",
            "Ranger boots",
            "Saradomin d'hide boots",
            "Snakeskin boots",
        ],
        ("Ranged", 12) => &["Venator ring", "Archers ring (i)", "Ring of shadows"],
        (_, 0) => &[
            "Torva full helm",
            "Neitiznot faceguard",
            "Helm of Neitiznot",
            "Berserker helm",
        ],
        (_, 1) => &[
            "Infernal cape",
            "Fire cape",
            "Mythical cape",
            "Ardougne cloak 4",
        ],
        (_, 2) => &[
            "Amulet of rancour",
            "Amulet of torture",
            "Amulet of fury",
            "Amulet of strength",
            "Amulet of glory",
        ],
        (_, 4) => &[
            "Torva platebody",
            "Bandos chestplate",
            "Fighter torso",
            "Rune platebody",
        ],
        (_, 7) => &[
            "Torva platelegs",
            "Bandos tassets",
            "Blood moon tassets",
            "Obsidian platelegs",
            "Rune platelegs",
        ],
        (_, 5) => &["Avernic defender", "Dragon defender", "Rune defender"],
        (_, 9) => &[
            "Ferocious gloves",
            "Barrows gloves",
            "Dragon gloves",
            "Rune gloves",
        ],
        (_, 10) => &[
            "Primordial boots",
            "Dragon boots",
            "Rune boots",
            "Climbing boots",
        ],
        (_, 12) => &[
            "Ultor ring",
            "Berserker ring (i)",
            "Bellator ring",
            "Ring of shadows",
        ],
        _ => &[],
    };
    if slot == 3 {
        return match style {
            "Magic" => [27275, 22323, 12899, 11905, 28585, 12658, 1381]
                .into_iter()
                .filter_map(planner::gear)
                .collect(),
            "Ranged" => [20997, 25865, 12926, 26374, 11785, 9185, 12788]
                .into_iter()
                .filter_map(planner::gear)
                .collect(),
            _ => [22325, 26219, 22324, 11889, 12006, 4151, 4587, 25979]
                .into_iter()
                .filter_map(planner::gear)
                .collect(),
        };
    }
    if slot == 13 && style == "Ranged" {
        return [11212, 892, 9242, 9243, 9144, 9143]
            .into_iter()
            .filter_map(planner::gear)
            .collect();
    }
    names
        .iter()
        .filter_map(|name| planner::equipment().iter().find(|g| g.name == *name))
        .collect()
}
