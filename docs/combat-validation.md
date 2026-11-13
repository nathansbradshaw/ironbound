# Calculator validation

**Run the independent checks:**

```sh
cargo run --locked --bin validate_combat
cargo test --locked --test wiki_parity
```

## Compared with the live Wiki calculator

Checked 13 November 2026 UTC at [the Wiki calculator](https://tools.runescape.wiki/osrs-dps/), deployed revision [`880ab3f`](https://github.com/weirdgloop/osrs-dps-calc/tree/880ab3f324ebea2644681d835448ab9d45c04f5c).

All combat stats 99, no boosts or prayers, no other equipment except the stated helmet. Bow/crossbow use Rapid; lance uses controlled Stab. These are uninterrupted attack estimates, not kill-time predictions.

| Weapon / ammunition | Target | Diary bonus | Wiki DPS | Ironbound DPS |
|---|---|---|---:|---:|
| Twisted bow / dragon arrows | Abyssal demon 415 | irrelevant | 0.480 | 0.480 |
| Twisted bow / dragon arrows | TzTok-Jad 3127 | irrelevant | 2.728 | 2.728 |
| Dragon hunter lance | Vorkath 8059 | irrelevant | 2.713 | 2.713 |
| Rune crossbow / diamond bolts (e) | Vorkath 8059 | off | 2.263 | 2.263 |
| Rune crossbow / diamond bolts (e) | Vorkath 8059 | on | 2.297 | 2.297 |
| Rune crossbow / diamond bolts (e), Slayer helmet (i), task on | Vorkath 8059 | off | 2.915 | 2.915 |
| Rune crossbow / diamond bolts (e), Slayer helmet (i), task off | Vorkath 8059 | off | 2.296 | 2.296 |

Attack rolls, defence rolls, accuracy and max hits also match. DPS tolerance is 0.0005 and accuracy tolerance 0.005 percentage points, because the live UI rounds those results. Integer metrics must match exactly.

Seven more live comparisons used deployed revision [`74958b1`](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f), checked the same day. All combat stats 99, no prayers or other equipment, Vorkath 8059. Blowpipe uses Rapid, whip uses accurate Slash. Each reduction below represents a successful effect already applied; the order is fixed to the reference calculator's order.

| Setup / assumptions | Attack roll | Defence roll | Max hit | Wiki DPS | Ironbound DPS |
|---|---:|---:|---:|---:|---:|
| Toxic blowpipe, dragon darts, unboosted | 10058 | 20070 | 20 | 2.098 | 2.098 |
| Same, ranging potion | 11280 | 20070 | 22 | 2.586 | 2.586 |
| Same, 1 DWH hit | 11280 | 14310 | 22 | 3.627 | 3.627 |
| Same, 1 DWH hit + 30 BGS damage | 11280 | 11610 | 22 | 4.470 | 4.470 |
| Trident of the seas, saturated heart, 1 DWH + 30 BGS | 9717 | 48336 | 32 | 0.671 | 0.671 |
| Same, Accursed sceptre effect | 9717 | 41344 | 32 | 0.785 | 0.785 |
| Abyssal whip, super combat, saturated heart, Accursed + 1 DWH + 30 BGS | 18834 | 18232 | 29 | 3.124 | 3.124 |

Ranging potion was also active in the trident/whip rows; it does not affect their outgoing rolls. The whip row also retained saturated heart. The full reproducible inputs, revisions and accuracy values are in the fixture.

Ten additional golden loadouts come from the upstream `BasicRolls.test.ts` and `GeneratedTests.test.ts`, pinned to [`cc149e9`](https://github.com/weirdgloop/osrs-dps-calc/tree/cc149e9f428ecd239a793eb822784f23d1aa84b2/src/tests/calc): whip, Bowfa, trident, max-melee lance and Shadow with Salve or a task helmet. These include reference effective levels above 99 to represent boosted inputs; they do not imply those are base account levels.

## Updating after a game change

1. Reproduce the change with identical stats, gear, target version, stance, prayers and options in the reference calculator. Its defaults can differ: the Wiki currently enables the Kandarin diary bonus.
2. Save the source revision, inputs and independent expected values in `tests/fixtures/wiki-reference.json`. Never regenerate golden values using Ironbound itself.
3. Change the smallest responsible rule or pure effect. Run the comparator and the full test suite. Investigate mismatches; do not loosen tolerances to make tests pass.
4. Check the resulting owned-bank loadout and switch/inventory constraints, then build Wasm and verify the browser before publishing.

## Where mechanics belong

- `assets/weapon-rules.json`: weapon families, level requirements, ammunition, proc rates, equipment and target-stat scaling parameters.
- `assets/combat-modifiers.json` / `src/modifiers.rs`: current-level boost profiles and ordered NPC reductions with source-NPC floors. Integer operations distinguish multiplying the remaining level from subtracting a rounded drain. Accursed takes precedence over Vulnerability; BGS spills across stats only after a stat reaches zero. Ayak changes a defence bonus, not a skill level. Encounter windows resolve to their source NPC for floors.
- Blowpipes declare compatible internal ammunition in their weapon family. Darts add strength, not accuracy, and ignore worn ammo. The planner reserves 100 darts outside the 28 bag slots, or records an explicit already-loaded confirmation. Export notes carry that preparation assumption; equipment slot 13 does not contain the darts.
- Its ordered `target_bonuses` table selects mutually exclusive Salve/task bonuses. Task eligibility comes from pinned public Wiki monster metadata; task status is explicit per target, never inferred from an account or spread to a whole raid. DHCB/task damage shares a data-defined additive group; melee dragonbane still multiplies.
- `src/effects.rs`: pure reusable arithmetic, with explicit truncation boundaries. Target curves take parameters, not item IDs.
- `src/combat.rs`: ordered calculation stages: effective stats → base rolls/hits → equipment and target modifiers → accuracy → hit distributions/procs → expected damage and timing.
- `assets/encounter-mechanics.json`: encounter windows, demonbane vulnerability and NPC magic-defence exceptions. Unsupported phases retain their coverage gates.
- `src/recommendation.rs` / `src/optimization.rs`: choose complete owned sets using those calculations and practical constraints; never duplicate weapon formulas here.

New mathematical behavior still needs a tested Rust effect. Balance changes to existing parameters can stay in data. Validate new rule data before calculating; do not silently ignore unknown mechanics.

## Remaining limits

These 24 reference loadouts (14 live and 10 upstream goldens) do **not** establish full Wiki-calculator parity. Many weapon/armour effects and several raid phases remain outside verified coverage. Boosts are peak assumptions: decay, drinking order, potion delay, venom and special-attack damage are excluded. Forgotten brew cannot be combined with melee/defence/raid boosts without an explicit drinking order. Incoming damage is withheld when a reduction changes the NPC's damage stat, because those max-hit transformations are not yet verified. Ruby bolts still use full target HP rather than a changing-HP kill simulation. The planner is a bounded search, not proof of global BIS.
