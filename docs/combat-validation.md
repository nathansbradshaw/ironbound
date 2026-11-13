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

Ten additional golden loadouts come from the upstream `BasicRolls.test.ts` and `GeneratedTests.test.ts`, pinned to [`cc149e9`](https://github.com/weirdgloop/osrs-dps-calc/tree/cc149e9f428ecd239a793eb822784f23d1aa84b2/src/tests/calc): whip, Bowfa, trident, max-melee lance and Shadow with Salve or a task helmet. These include reference effective levels above 99 to represent boosted inputs; they do not imply those are base account levels.

## Updating after a game change

1. Reproduce the change with identical stats, gear, target version, stance, prayers and options in the reference calculator. Its defaults can differ: the Wiki currently enables the Kandarin diary bonus.
2. Save the source revision, inputs and independent expected values in `tests/fixtures/wiki-reference.json`. Never regenerate golden values using Ironbound itself.
3. Change the smallest responsible rule or pure effect. Run the comparator and the full test suite. Investigate mismatches; do not loosen tolerances to make tests pass.
4. Check the resulting owned-bank loadout and switch/inventory constraints, then build Wasm and verify the browser before publishing.

## Where mechanics belong

- `assets/weapon-rules.json`: weapon families, level requirements, ammunition, proc rates, equipment and target-stat scaling parameters.
- Its ordered `target_bonuses` table selects mutually exclusive Salve/task bonuses. Task eligibility comes from pinned public Wiki monster metadata; task status is explicit per target, never inferred from an account or spread to a whole raid. DHCB/task damage shares a data-defined additive group; melee dragonbane still multiplies.
- `src/effects.rs`: pure reusable arithmetic, with explicit truncation boundaries. Target curves take parameters, not item IDs.
- `src/combat.rs`: ordered calculation stages: effective stats → base rolls/hits → equipment and target modifiers → accuracy → hit distributions/procs → expected damage and timing.
- `assets/encounter-mechanics.json`: encounter windows, demonbane vulnerability and NPC magic-defence exceptions. Unsupported phases retain their coverage gates.
- `src/recommendation.rs` / `src/optimization.rs`: choose complete owned sets using those calculations and practical constraints; never duplicate weapon formulas here.

New mathematical behavior still needs a tested Rust effect. Balance changes to existing parameters can stay in data. Validate new rule data before calculating; do not silently ignore unknown mechanics.

## Remaining limits

These 17 reference loadouts do **not** establish full Wiki-calculator parity. Blowpipe/dart selection, many other weapon/armour effects, full buff/debuff controls, and several raid phases remain outside verified coverage. Ruby bolts still use full target HP rather than a changing-HP kill simulation. The planner is a bounded search, not proof of global BIS.
