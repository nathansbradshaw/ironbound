# Armour set comparisons

The planner compares ordinary armour with joint equipment candidates. It refines each candidate's remaining slots, then uses the selected Recommended policy or custom weights. Owned stock, usable variants, levels, exclusions, pinned slots, ranged category and later switch/inventory limits still apply. This bounded search is not proof of global BIS.

| Equipment | Calculated and compared | Limits |
| --- | --- | --- |
| Void / Elite Void | Complete melee, ranged and magic sets, including mixed normal/elite pieces. | Matching helm and every required slot must be present. |
| Crystal | Crystal bow/Bowfa jointly with every owned armour subset. Each piece retains its own accuracy/damage contribution. | Active variants only. A pinned non-crystal helmet can remain; other weapons receive no crystal bonus. |
| Dharok | Complete armour **and greataxe**, with per-roll missing-health scaling. | Combat → Boosts, darts & debuffs → **Current Hitpoints (Dharok's)**. Blank means full health. This is a fixed health assumption, not a prediction of survival or healing. |
| Verac | Complete armour/flail, with a 25% guaranteed hit rolling 1 through base maximum + 1. Full set plus Amulet of the Damned also adds 7 Prayer. | Unsupported NPC protection/immunity scenarios remain gated. |
| Guthan | Complete armour/warspear and 25% potential healing. | Displayed recovery precedes overheal and remaining-target-HP caps. It is not subtracted from incoming damage or credited as guaranteed trip sustain. |
| Karil | Complete armour/crossbow, exact bolt racks and Amulet of the Damned's 25% extra half-damage hitsplat. | Run-energy/Agility debuffs are not PvM damage bonuses. |
| Ahrim | Complete robes/staff and Amulet of the Damned's 25% chance of 30% extra spell damage, with per-roll flooring. | Supported elemental spells only. Ancient spells and dynamic NPC Strength drains remain outside the model. |
| Torag | Complete armour/hammers and two independently rolled hits, with NPC flat armour applied to each. | Run-energy drain has no PvM damage bonus. The amulet's missing-health Defence bonus remains explicitly unavailable in incoming estimates. |
| Obsidian | Armour, compatible melee weapon and optional Berserker necklace jointly. Armour adds a floored 10% of the base roll/hit after Salve/task; necklace scales each damage roll by 20%. | Cape/shield do not complete the armour set. Other weapons receive no Obsidian bonus. |
| Inquisitor | All owned piece subsets; current crush multipliers are 0.5% helm, 1% body, 1% legs. | Crush only; the removed mace-specific multiplier is not applied. |

Stat-only combinations such as Bandos, Torva, Armadyl and Masori use their individual item stats. Moon set weapons/spells and full Justiciar's incoming reduction remain outside verified support: unsupported weapon calculations and Justiciar incoming damage are withheld rather than assigned an invented bonus. Virtus's Ancient-specific bonus cannot activate while Ancient spells are unsupported.

Barrows active degradation variants can mix within one brother's set. Broken pieces and another brother's pieces cannot complete it. Recoil and dynamic NPC stat drains are not added to sustained player DPS. Health assumptions travel with shared setups and export notes.

`tests/armour_sets.rs` checks independently derived per-roll arithmetic, joint discovery, complete-set requirements, finite-HP miss loops, guaranteed-max windows, per-hitsplat acid/armour rounding, and setup constraints. Existing Void, crystal, encounter and reference tests also remain required.

Reference: [Wiki calculator combat stages at 74958b1](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts), [Barrows equipment](https://oldschool.runescape.wiki/w/Barrows_equipment), [Amulet of the Damned](https://oldschool.runescape.wiki/w/Amulet_of_the_Damned), [Inquisitor changes](https://oldschool.runescape.wiki/w/Inquisitor%27s_armour#Changes), and [Obsidian armour](https://oldschool.runescape.wiki/w/Obsidian_armour), reviewed 13 November 2026.
