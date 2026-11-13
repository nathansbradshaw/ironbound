# Equipment mechanics backlog

Requested 13 November 2026. Catalog presence and ordinary item bonuses do not
establish support for a weapon, passive, set effect, special attack or trip model.
Keep implementation, independent formula checks, browser checks and gameplay
evidence separate. Unsupported mechanics must remain explicit.

## 1. Incorrectly blocked Berserker ring variants

- [ ] Remove the unsupported-passive gate from Berserker ring (i) IDs 25264 and 26770.
- [ ] Verify equal stats and outgoing results with the other imbued variants.

## 2. Barrows and Amulet of the damned

Already owned by the active `codex/armour-set-comparisons` workstream; preserve its
checkout and integrate its merged results rather than duplicating changes.

- [ ] Verify weapon support, exact active variants and complete-set discovery.
- [ ] Verify Dharok health scaling, Verac bypass, Torag twin hits, Karil/Ahrim
  amulet effects, Guthan healing potential and amulet prayer bonuses.
- [ ] Keep unsupported incoming effects, PvP drains, overhealing and reflect
  timing explicit. Do not claim a complete sustain simulation.

## 3. Obsidian and Berserker necklace

Owned by `codex/armour-set-comparisons`.

- [ ] Jointly discover weapon, armour and necklace; check additive/multiplicative
  ordering, partial sets, exact variants and ownership/switch constraints.

## 4. Inquisitor armour and mace

Owned by `codex/armour-set-comparisons`.

- [ ] Verify current piece contributions, crush-only application, full-set
  behaviour and mace support against a pinned independent reference.

## 5. Moon equipment

- [ ] Add Dual macuahuitl, Eclipse atlatl and Blue Moon spear attack rules,
  requirements, ammunition and damage-stat selection.
- [ ] Verify macuahuitl dependent hit rolls and per-splat armour.
- [ ] Model Blood Moon attack timing, Eclipse burns and Blue Moon spell/melee
  follow-ups only with independent fixtures; ordinary armour stats do not cover
  these effects. Gate incomplete set combinations.
- [ ] Compare complete owned sets and carry their real ammunition/rune needs.

## 6. Scythe, Soulreaper and other unsupported weapons

- [ ] Scythe: exact active variants, target-size hit count, independent accuracy,
  per-hit damage floors and armour; verify finite-HP calculations.
- [ ] Soulreaper: explicit bounded stacks, boost/prayer ordering and sustained
  normal attacks. Keep stack build-up, self-damage, healing and specials separate.
- [ ] Audit remaining catalog weapons by family; add reviewed rules without
  accidentally enabling uncharged, broken, cosmetic or incompatible variants.

## 7. Defensive passives

- [ ] Elysian: proc probability, per-hit reduction/floors and expected damage.
- [ ] Full Justiciar: style-specific defence contribution and per-hit rounding.
- [ ] Dinh's: explicit Block assumption and its attack lockout; defence-based
  offensive scaling remains a separate weapon task.
- [ ] Rondache: flat reduction and its equipment penalty ordering.
- [ ] Spectral prayer drain, reflect, healing and encounter-specific chip/special
  interactions need explicit scenarios; do not turn basic-hit results into
  full-fight survival or trip-length claims.

## 8. Enchanted bolts

- [ ] Add opal, pearl, dragonstone and onyx damage distributions, diary/Zaryte
  modifiers, attribute immunities, misses and per-hit armour ordering.
- [ ] Track jade stun, topaz stat drains, sapphire prayer effects and emerald
  poison separately; direct-hit DPS cannot establish those encounter effects.
- [ ] Preserve ruby changing-HP and diamond regression coverage; onyx healing,
  ruby self-damage and actual ammunition consumption remain separate tasks.

## 9. Specials, energy and rotations

- [ ] Inventory supported and unsupported special weapons separately from
  normal-attack support. Add explicit coverage/preparation notes.
- [ ] Add one independently verified special-attack distribution at a time,
  starting with already supported normal-attack weapons.
- [ ] Model energy costs/recovery, Lightbearer, timing, switching and rotations
  before recommending special-attack gear or claiming whole-fight DPS.
- [ ] Retain the distinction between a manually confirmed successful reduction
  and the probability, damage, energy and time needed to land that reduction.

## Release evidence

Pending the first equipment follow-up. Every release must pass relevant Rust
regressions, the independent comparator, Wasm build and browser checks; verify
remote main and the published source revision. Gameplay remains separate.
