# Recent equipment audit

Reviewed 13 November 2026 against the pinned public equipment and calculator
revision [74958b1](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f).
Catalog presence is separate from combat support. These are normal-attack rules;
special attacks, charges consumed and whole-fight sustain remain separate work.

## Weapon additions

| Weapon | Supported behaviour | Remaining limits |
| --- | --- | --- |
| Twinflame staff | Six-tick standard casting; +10% accuracy and damage; elemental Bolt/Blast/Wave spells add a correlated second splat at 40% of the first. Infinite fire/water runes are subtracted from packing once per cast. | No extra hit for Strike, Surge or binding spells. Extra-hit guaranteed-max/P2 windows remain gated. No Ancient autocast. |
| Smoke battlestaff / mystic smoke staff | Standard casting, free air/fire runes, +10% accuracy/damage with correct Salve/task ordering. | Other spellbooks are not enabled. |
| Dragon hunter wand | Standard and supported Ice casts. Dragon accuracy +75% and damage +40% precede elemental weakness; Slayer rounding is retained. | No free runes, new Ancient secondary effects or unsupported spells are invented. |
| Eye of Ayak | Charged three-tick powered attacks, floor(Magic/3)-6; requires 83 Magic. | Uncharged variants cannot attack. Special damage/drains still require separate modelling; selected drains remain explicit successful-effect assumptions. |
| Zorya's tome | Charged three-tick powered attacks, floor(Magic/3)-10 before equipment damage; requires 85 Magic. | Two-handed; no Confliction passive. Special and empowered follow-up rotations remain excluded. |
| Scorching bow | Compatible arrows through dragon; four-tick Rapid, demon accuracy +30%, damage +30% additive with task damage. | Shackles, binding and special-attack burns remain excluded. |
| Burning claws | Normal slash/stab attacks and reviewed 5% demonbane. | Special hits, burns and energy are not credited to normal attacks. |
| Noxious halberd | Normal slash/stab attacks at five ticks; requires 80 Attack. | Venom, special attack and movement/reach benefits remain uncredited. |
| Arkan blade | Normal slash/stab attacks at four ticks; requires 60 Attack. | Special attack/burn and quest confirmation remain separate. |
| Crimson kisten | Normal crush/stab attacks at four ticks; requires 60 Attack and 85 Strength. | Its four-accuracy-roll special is not a normal attack and remains open. |
| Glacial temotli / sulphur blades / earthbound tecpatl | Two independently accurate damage halves, per-splat NPC armour/reductions and actual crush/slash/stab styles. Requires 55 Attack. | Multi-target positioning and encounter specials remain separate. |

## Armour and accessories

| Equipment | Review outcome |
| --- | --- |
| Oathplate / radiant Oathplate | Existing slash accuracy and Strength stats already participate in search. Added the missing 78 Defence gate for both versions. Cosmetic pieces do not imply an invented full-set passive. |
| Torva / sanguine Torva | Added 80 Defence gates for actual restored/wearable variants. Unobtainable catalogue entries are not newly enabled. |
| Masori / fortified Masori | Added 80 Ranged plus 30/80 Defence respectively. No obsolete pre-release Low Life mechanic. |
| Hueycoatl hide | Added 70 Ranged; body/coif also require 40 Defence. Existing Prayer/Magic defence stats remain individual piece contributions. |
| Sunfire fanatic | Added 60 Prayer and 40 Defence; uses actual Prayer and equipment stats without an invented set passive. |
| Rancour / rupture | Added the missing 90 Hitpoints requirement. Cosmetic Rancour shares the wear requirement. |
| Dizana quiver / blessed quiver / Dizana max cape | Charged variants add +10 Ranged accuracy and +1 Ranged strength only with compatible fired arrows/bolts. No charged passive for Bowfa/crystal bow, blowpipes, atlatl or bolt racks. Uncharged quivers retain only their ordinary stats. Max variants require 99 in all 24 supported skills. |
| Avernic treads / Confliction | Existing multi-skill / 90-HP gates retained. New one-handed Eye of Ayak receives Confliction's stationary accuracy model; two-handed Zorya does not. |
| Virtus / Moon / Justiciar | Existing joint-set search, supported Ice bonus and exact Moon variants retained. New weapons cannot borrow Bloodrager or Blue Moon follow-ups without the matching set weapon. |

## Still open

- Obligator's size bonus and damage rerolls need independently pinned ordering
  against Inquisitor, minimum hits and NPC limiters. The reference itself flags
  ordering uncertainty; generic mace damage must not substitute for its passive.
- Ascension crossbows, ascension-bolt variants and Seeking arrows require their
  actual ammunition/proc/minimum-hit model, not a generic crossbow alias.
- Zorya's empowered follow-ups, Crimson kisten's four-roll special, other specs,
  energy recovery/Lightbearer and charge consumption remain in the equipment backlog.
- Elemental fragments/amulets, sunfire rune minimum hits, remaining Ancient
  secondary effects, defensive interactions and dynamic Soulreaper stacks remain open.
- Quest/access checks and actual gameplay calibration remain distinct from item
  ownership, wear levels and independently verified outgoing arithmetic.

## Sources and evidence

Arithmetic: pinned [outgoing stages](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts),
[equipment composition and speed](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/Equipment.ts),
[two-hit membership](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/BaseCalc.ts)
and [item stats](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/cdn/json/equipment.json).
Wear requirements and item behaviour: [Oathplate](https://oldschool.runescape.wiki/w/Oathplate_armour),
[Torva](https://oldschool.runescape.wiki/w/Torva_armour),
[Masori](https://oldschool.runescape.wiki/w/Masori_armour),
[Hueycoatl hide](https://oldschool.runescape.wiki/w/Hueycoatl_hide_armour),
[Sunfire fanatic](https://oldschool.runescape.wiki/w/Sunfire_fanatic_armour),
[jewellery](https://oldschool.runescape.wiki/w/Jewellery),
[Eye of Ayak](https://oldschool.runescape.wiki/w/Eye_of_Ayak),
[Zorya's tome](https://oldschool.runescape.wiki/w/Zorya%27s_tome),
[Scorching bow](https://oldschool.runescape.wiki/w/Scorching_bow),
[elemental staves](https://oldschool.runescape.wiki/w/Elemental_staves),
[Attack requirements](https://oldschool.runescape.wiki/w/Attack/Weapons_table)
and [2025 dragon-wand changes](https://oldschool.runescape.wiki/w/Update:Summer_Sweep_Up:_Combat).

`tests/recent_equipment.rs` contains independent arithmetic, correlated-hit and
probability checks, finite-HP timing, elemental/Salve/task ordering, rune packing,
variant/skill boundaries, quiver exclusions, cosmetic equivalence and Moon-set
isolation. These fixtures are not gameplay or additional live-calculator captures.
