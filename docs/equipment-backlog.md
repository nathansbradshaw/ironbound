# Equipment mechanics backlog

Requested 13 November 2026. Catalog presence and ordinary stats do not establish
support for a passive, set effect, special attack or trip model. Formula,
browser, live-calculator and gameplay evidence remain separate.

## 1. Incorrectly blocked Berserker rings

- [x] Remove the unsupported-passive gate for imbued IDs 25264 and 26770.
- [x] Verify equal stats and outgoing results with the other imbued variant.

## 2. Barrows and Amulet of the damned

Integrated from [PR #36](https://github.com/nathansbradshaw/ironbound/pull/36).
See [armour coverage](armour-sets.md) for exact support and limitations.

- [x] Verify weapons, active variants and complete-set discovery.
- [x] Verify Dharok health scaling, Verac bypass, Torag twin hits, Karil/Ahrim
  amulet effects, Guthan healing potential and amulet prayer bonuses.
- [x] Keep unsupported incoming effects, PvP drains, overhealing and reflect
  timing explicit. Potential healing is not a complete sustain simulation.
- [ ] Model Torag's amulet Defence bonus, reflected damage and dynamic NPC drains.

## 3. Obsidian and Berserker necklace

- [x] Integrate PR #36's joint weapon/armour/necklace discovery and verified
  ordering, partial sets, variants, ownership and switch constraints.

## 4. Inquisitor armour and mace

- [x] Integrate PR #36's current piece contributions, crush-only application,
  set search and mace support against the pinned independent reference.

## 5. Moon equipment

- [x] Add Dual macuahuitl normal melee, Eclipse atlatl normal ranged and Blue
  Moon spear elemental casts, with requirements, ammo and damage-stat selection.
- [x] Verify macuahuitl dependent hit rolls and per-splat armour/acid rounding.
- [x] Model Blood Moon's expected attack timing and jointly discover owned sets.
- [x] Pack the chosen Atlatl dart budget; retain exact active armour variants.
- [x] Gate full Eclipse sets while their burn effect is incomplete.
- [ ] Eclipse burn stacks, damage-over-time timing and finite-HP transitions.
- [ ] Blue Moon melee, binding spells and spell/melee follow-up attacks.
- [ ] Macuahuitl guaranteed-hit and special-phase transforms; specials remain open.

## 6. Scythe, Soulreaper and other weapons

- [x] Scythe: charged variants, target-size hit count, independent accuracy,
  per-hit damage floors/armour, acid rounding and finite-HP distributions.
- [x] Soulreaper: explicit bounded held stacks, prayer ordering and sustained
  normal attacks. Stack build-up/loss, self-damage, healing and specials are excluded.
- [ ] Independently verify Soulreaper boost combinations and dynamic stacks.
- [ ] Audit remaining catalog weapons by family. Never accidentally enable
  uncharged, broken, cosmetic or incompatible variants.

## 7. Defensive passives

- [x] Elysian: proc probability and per-hit floors for ordinary unprotected hits.
- [x] Full Justiciar: style-specific defence and per-hit reduction in that scope.
- [x] Dinh's: explicit settled Block assumption, reduction and attack lockout.
- [x] Rondache: flat reduction; equipment penalties remain in defensive stats.
- [ ] Extend these passives to chained hits, prayer chip, boss healing and
  specials only after independently verifying their ordering.
- [ ] Dinh's activation delay and defence-based offensive scaling.
- [ ] Spectral prayer drain, reflected damage and healing need explicit
  scenarios. Basic-hit estimates are not whole-fight survival or trip length.

## 8. Enchanted bolts

- [x] Opal, pearl, dragonstone and onyx ordinary-hit distributions, diary/Zaryte
  modifiers, attribute immunities, misses and accurate-zero ordering.
- [x] Preserve ruby changing-HP and diamond regression coverage.
- [ ] Verify the new effects' flat-armour and guaranteed/special-phase transforms;
  these combinations remain gated.
- [ ] Jade stun, topaz drains, sapphire prayer effects and emerald poison.
- [ ] Onyx healing, ruby self-damage and actual ammunition consumption.

## 9. Specials, energy and rotations

- [x] Add normal-attack-only coverage and exported preparation notes, including
  held Soulreaper stacks, Block lockout, onyx healing and Lightbearer limits.
- [x] Distinguish manually confirmed successful reductions from the probability,
  damage, energy and time needed to land them.
- [ ] Inventory special weapons separately from normal-attack support.
- [ ] Add one independently verified special distribution at a time, starting
  with weapons whose normal attacks are already supported.
- [ ] Energy costs/recovery, Lightbearer, timing, switches and rotations before
  recommending special gear or claiming whole-fight DPS.

## Reference and verification

Pinned upstream calculator revision
[`74958b1`](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f):
[outgoing stages](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts),
[bolts](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/dists/bolts.ts),
[incoming stages](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/NPCVsPlayerCalc.ts)
and [NPC sizes](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/cdn/json/monsters.json).
Rust implements the arithmetic independently. These are formula regressions,
not additional live-calculator captures or gameplay verification.

`cargo run --locked --bin equipment_data -- /path/to/monsters.json` refreshes
2,835 unambiguous factual sizes. Missing/conflicting sizes are withheld; attack
windows resolve to source NPCs. Never assume every target takes three Scythe hits.

`tests/equipment_effects.rs` covers rings, variants, target sizes, misses, finite
HP, multi-hit floors, Soulreaper prayer ordering, Moon attacks/timing, owned
search/ammunition, bolt immunities/diary/Zaryte, incoming floors, unsupported
scenarios and saved/shared settings. Existing armour and reference tests also apply.

## Release evidence

First follow-up: verification in progress. Release requires the integrated Rust
suite, independent comparator, Wasm build and browser checks, followed by remote
main and published source verification. Gameplay remains open.
