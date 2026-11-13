# Quest damage and gear readiness

All 23 quest entries have ordinary outgoing damage and gear ratings. **Quest bosses** lists them separately; **Hide quest bosses** excludes them from any category without changing gear or requirements filters. Clear filters restores the default list. Farmable post-quest versions retain their separate entries.

Ratings screen owned, usable equipment, runes, ammunition and combat levels. Missing flails or ice gloves produce **A struggle** with the missing item named. A stronger incompatible weapon cannot earn a good rating. Multi-body fights use the weakest body's DPS for a single valid style.

These are gear guidelines, not completion probabilities. The recommended levels and 1.5/3.0 DPS bands are planner policy. Quest access, player execution, movement, incoming attacks, food and prayer endurance remain separate checks. Missing combat levels produce **Not assessed**.

| Encounter | Ordinary outgoing rules |
| --- | --- |
| Lowerniel Drakan | Reviewed hallowed-flail route, 25% accuracy/damage, Efaritay's aid, 26–28 damage cap. Counters and dodges excluded. |
| Ranis Drakan | Ivandis flail, 20% rolled-hit damage, Efaritay's aid. Airborne/add phases excluded. |
| Vanstrom Klause | Blisterwood flail, 5% accuracy, 25% rolled-hit damage, Efaritay's aid. Lightning, blood-orb damage and the final phase excluded. |
| Fragment of Seren | Existing per-splat 22–24 limiter and Defence-based Magic defence. Healing beams and near-max-HP special excluded. |
| Galvek | Fire/Air/Water/Earth use 300 HP each. Air/Water block melee. Per-form debuffs remain separate. Dragonbane and bolt effects apply. |
| Sliske's wights | Four 200-HP bodies, excluding Ketla's clone. No recoil or leech-healing contribution assumed. |
| Surok | Cast spells; powered staves, melee and ranged blocked. Counter-spells excluded. |
| Fareed | Water Magic or physical damage with ice gloves. All held-weapon routes require owned gloves. No unarmed spell route or ammo-recovery prediction. |
| Kamil | Fire Magic or physical damage. Other Magic, including powered staves and Iban Blast, blocked. Cold stat drains excluded. |
| Quest Vardorvis | 500 HP; Defence 180→130 and Strength 210→280 with full-fraction interpolation before flooring. Existing current-Defence drain floor applies. |
| Quest Duke | 330 HP, awake body, 70% Demonbane effectiveness. Preparation excluded. |
| Quest Leviathan / Whisperer | Their own 720/660 HP and recorded quest defences. Leviathan blocks melee. Post-quest stun/rear/enrage assumptions are not imported. |
| Other quest entries | Recorded ordinary body stats and applicable dragon/demon/undead rules. Specials, travel and assisted damage excluded. |

Vampyre damage is applied to rolled hits, preserving flooring: Efaritay's 10% damage first, then the flail bonus. Its 15% accuracy follows the flail's accuracy modifier. Misses remain zero. The reviewed quest routes deliberately use the weapon available for that quest; other later vampyrebane routes are outside these baselines.

The Drakan Wiki guide specifies a 26–28 cap without the server's full roll implementation. This model preserves hits up to 28 and rerolls overflow uniformly to 26–28. That distribution is an explicit modelling assumption, not gameplay calibration. Counter-attack damage is excluded.

## Sources and checks

- [Pinned calculator rules and factual data](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f): `PlayerVsNPCCalc.ts`, `BaseCalc.ts`, `constants.ts`, `scaling/Vardorvis.ts` and `scaling/DefenceReduction.ts`.
- [The Blood Moon Rises](https://oldschool.runescape.wiki/w/The_Blood_Moon_Rises), [hallowed flail](https://oldschool.runescape.wiki/w/Hallowed_flail), [Ranis](https://oldschool.runescape.wiki/w/Ranis_Drakan), [Vanstrom](https://oldschool.runescape.wiki/w/Vanstrom_Klause), [Surok's fight](https://oldschool.runescape.wiki/w/While_Guthix_Sleeps#Scuffle_with_Surok), [Fareed](https://oldschool.runescape.wiki/w/Fareed), [Kamil](https://oldschool.runescape.wiki/w/Kamil) and [DT2 finale](https://oldschool.runescape.wiki/w/Desert_Treasure_II#Wight_Club).

`tests/quest_combat.rs` checks independent attack-roll arithmetic, rolled-hit flooring and cap boundaries, preserved probability mass, weapon/element restrictions, quest stat curves, valid owned candidates, exclusions, ratings for all 23 entries, and classification/filtering. Existing post-quest arithmetic tests remain in place.

Edit `assets/quest-bosses.json`, then refresh reviewed metadata with `cargo run --locked --bin quest_data`. The Rust tool preserves unrelated catalog rows. It does not fetch private account data or infer quest completion.
