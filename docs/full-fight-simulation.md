# Full-fight and consumption scenarios

Choose **Vorkath**, **Yama**, or **Tombs of Amascut**, then open **Combat → Full-fight & consumption simulation**. Set your execution assumptions and run the scenario. These are solo scenarios without teammate damage. For ToA this covers the complete **Wardens fight**, not the preceding rooms.

- **First-fight demand:** median and 90th-percentile duration, attacks, runes, projectiles, weapon charges, blowpipe scales, food and prayer doses.
- **Packed coverage:** how many first fights completed with the starting bag and declared loaded charges. Extra food/potions are demand forecasts; they never appear in your actual bag or export.
- **Trip distribution:** repeat fights with remaining HP, Prayer and supplies, stopping when the packed budget fails. No bank reset, drops, death recovery or looting replenishment. The trip cap censors the result: reaching five does not prove that six is impossible.
- **Recheck changes:** results become stale when gear, account, inventory or assumptions change. Re-run before relying on them. Scenario settings persist in named setups/backups and public setup links; results do not get silently attached to RuneLite exports.

The default inputs illustrate an execution scenario. They are **not measured timing or player performance**. Incoming HP/minute is a user-specified residual damage budget after protection, separate from failed mechanics. It is not an invented mix of unverified NPC attack styles. Check starting HP/Prayer and supplies at room entry, projectile recovery, loaded charges, delays, uptime and failure chances.

## Encounter timelines

| Encounter | Whole-fight state |
| --- | --- |
| Vorkath | Six five-tick normal-attack intervals; random first special, then alternating acid/freeze. Freeze prevents boss damage and spends Crumble Undead runes/cooldown. Acid uses the reviewed per-hit damage reduction, optional attack uptime, failed movement damage and pool healing. Ruby damage follows current boss HP and spends player HP only on a proc. |
| Normal Yama | Carries the same boss HP through soft two-thirds/one-third thresholds, half-damage transition attacks, two 400-HP Magic-first alternating Judges, travel/return delays and final defence. Flare pairs have individual charge deadlines, regeneration and remaining-HP-scaled healing/damage. Transitions clear flares. Final flares start at 71 HP; the final-phase spawn count can increase. |
| Wardens | Obelisk → alternating open Magic/Ranged shield windows → exposed melee cores, repeating until fivefold core transfer exhausts body HP. Core exposure uses 21/29/37 ticks at the documented HP bands. P3 queues skull sets at 80/60/40/20%, then enrage heals once and resets base Defence to 180. Earlier reductions are not automatically reapplied. |

Source rules: [Vorkath](https://oldschool.runescape.wiki/w/Vorkath), [strategy](https://oldschool.runescape.wiki/w/Vorkath/Strategies), [Crumble Undead](https://oldschool.runescape.wiki/w/Crumble_Undead); [normal Yama strategy](https://oldschool.runescape.wiki/w/Yama/Strategies), [Judge](https://oldschool.runescape.wiki/w/Judge_of_Yama), [flares](https://oldschool.runescape.wiki/w/Void_Flare), [Mark](https://oldschool.runescape.wiki/w/Mark_of_Darkness); [core](https://oldschool.runescape.wiki/w/Core), [P3/enrage](https://oldschool.runescape.wiki/w/Elidinis%27_Warden), [skulls](https://oldschool.runescape.wiki/w/Energy_Siphon). Reviewed outgoing distributions retain their [independent validation](combat-validation.md).

## What is assumed

The browser exposes every numeric scenario input, including uncertain special/transition timings, P2 prayer rotation, skull clearance time/damage, flare cadence/charge/healing/regeneration, final Yama autos allowed in melee range, glyph costs, residual damage, execution uptime and projectile loss. These knobs do not claim exact game scripts. Residual HP/min applies during exposed attack phases, not immune/travel windows. Acid failure is sampled per acid tick; spawn failure is sampled per spawn. Failed Yama glyph/crash scenarios draw up to 60 HP; failed Wardens hazards/skulls draw up to 40 HP. These representative mistake costs supplement residual HP/min and are not validated incoming attack distributions.

Each tick advances prayer with the actual equipped set's bonus. Food consumes actual immediate-healing bites with a conservative three-tick attack delay; unsupported delayed healing is not credited. Ordinary carried prayer/restore/Sanfew doses use their existing restoration formulas. Exhausted food is extended with 20-HP shark equivalents for the demand forecast; exhausted prayer uses ordinary prayer-potion doses. Such a run fails packed coverage even if its unlimited-demand continuation finishes. Prayer is off between fights. Mark recasts consume runes at its base-level duration, multiplied by five when a carried Purging staff is used; it has no attack delay.

Projectile loss is sampled from the explicit recovery input, and firing requires a remaining projectile. [Bolt racks](https://oldschool.runescape.wiki/w/Pay-to-play_Ranged_training) always consume one per shot; [corrupted Bowfa variants](https://oldschool.runescape.wiki/w/Bow_of_Faerdhinen) use no weapon charges. [Scythe](https://oldschool.runescape.wiki/w/Scythe_of_Vitur) and Arclight consume one charge on a damaging attack rather than per hitsplat. Other powered/self-ammunition weapons spend declared charges; blowpipes spend internal darts and scales. Rune cost is per actual cast after staff/tome rune supply. Charges are a declared starting budget per weapon, not an account fact. Recharging materials are not also subtracted from carried rune stacks; other melee-weapon degradation remains a manual check.

The model excludes special attacks, passive healing, boost decay, combo eating, projectile travel, armour degradation, tome pages, poison/venom and protection-potion expiry. Maintain the selected boosts/protection manually. Confliction, conditional attack-speed procs, burn-stack timing, Soulreaper stack buildup/HP recovery and Dharok's changing player-HP damage block simulation rather than borrowing sustained/fixed-state assumptions. Other supported armour damage distributions are retained. ToA is solo, level 0–150 without mechanic-changing invocations; contracts and mixed custom encounters remain unavailable. This is a whole-fight **scenario model**, not a full game emulator or a proven survival forecast.

## Verification and replay

`cargo test --locked --test full_fight` checks independently calculated timelines, alternating Judges, core transfer/exposure, skull/enrage counts, cooldown continuity, trip stock conservation, prayer/food/charge demand, flare deadlines, deaths/timeouts, bounded inputs, migration and reproducible seeds. These checks are separate from gameplay calibration.

**Download scenario results** includes the numerical model inputs and individual trials, without bank history or account identity. Replay the same inputs offline:

```sh
cargo run --locked --bin simulate_fight -- ironbound-fight-scenario.json
```

The seeded sampler is deterministic for a given version and numerical input. First-fight percentiles include completed fights only; completion counts retain deaths and one-hour timeouts. Do not interpret a completed-only median as the expected outcome of a failed attempt. Tick-one attacks and immediate damage are simulation conventions; final projectile flight and death animations are excluded.

Actual encounter traces, incoming attack mixtures, spatial pathing and human RuneLite/gameplay checks remain open validation work.

Use **Compare with recorded gameplay** for local, setup-matched comparisons, separate training/holdout errors and directly measured setting suggestions. See the [recording guide and public-source audit](gameplay-calibration.md). No matched gameplay datasets are checked in; importing or replaying a synthetic report does not calibrate an encounter.
