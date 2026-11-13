# Obligator and ammunition audit

Reviewed 13 November 2026 against OSRS Wiki calculator revision
[74958b1](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f),
confirmed as its current HEAD during this audit. Normal attacks only.

## Obligator

- Both exact charged/uncharged variants require 85 Attack and are two-handed.
- Combat's **Obligator attack** setting selects Thrust accurate (+3 Attack),
  Thrust aggressive (+3 Strength), or Smash (+3 Strength).
- Charged Thrust takes five ticks. One accuracy check precedes damage selection;
  on success it keeps the best of two uniform inclusive damage rolls, plus one
  roll for each of stab/slash defence bonuses strictly higher than crush.
  Equal defence bonuses give no extra roll. Misses do not reroll accuracy.
- Size 2 multiplies maximum damage by 120/100; size 3+ uses 140/100, floored
  before rolling. Accurate zeros become one after the best roll; ordinary flat
  armour, Corp halving and supported NPC caps follow. Finite-HP timing consumes
  the resulting probability distribution, rather than an average-damage shortcut.
- Smash takes three ticks and receives neither size nor damage-roll passives.
  Uncharged Thrust likewise receives neither passive and remains five ticks.
  The mode survives private backups/share links and appears in setup notes.

Charged Thrust with Inquisitor armour is deliberately unavailable for scoring:
the reference marks the relative size/armour rounding order as unresolved.
Encounter minimum-hit bounds (including Wardens P2) are likewise gated. This is
partial support with explicit reasons, not evidence of those unverified paths.
Multi-target positioning, charge consumption and actual gameplay calibration
remain outside this normal-attack model.

## Ascension crossbows

- Requires 80 Ranged, occupies both hands and fires one bolt every two ticks on
  Rapid, or every three ticks with the Accurate stance's +3 Ranged.
- Accepts only Ascension bolts, diamond/onyx Ascension bolts and their enchanted
  variants. Ordinary crossbows cannot fire them; Ascensions cannot fire ordinary
  or dragon bolts. No invented ruby Ascension bolt is accepted.
- Plain and unenchanted tipped bolts use their catalog strength without a proc.
- Diamond enchantment: 5% per shot, or 5.5% with Kandarin; independent of normal
  accuracy, replaces the damage roll with 0–floor(max × 115/100), then converts
  accurate zero to one.
- Onyx enchantment: 5.5%, or 6.05% with Kandarin, conditional on successful
  accuracy; replaces damage with 0–floor(max × 120/100). Undead block the effect.
  Healing, overheal, ammunition retrieval and sustained supply consumption remain
  excluded. These proc rates are half the ordinary crossbow rates.
- Charged Dizana bonuses apply to these fired bolts. Owned shot budgets, ammo
  selection, export quantities and NPC reductions reuse the reviewed paths.

## Seeking arrows

Compatible supported bows accept Seeking arrows up to their actual arrow tier,
including dragon only for the existing dragon-capable families. Catalog +20
accuracy participates normally. Successful damage 0–2 is raised to three before
NPC reductions; misses stay misses. This is a clamp (3,3,3,3,4...), not a reroll
or a uniform 3–max range. Seeking broad arrows retain 50 Ranged / 55 Slayer gates.
Crafting/quest access and unsupported bows remain separate from owned ammunition.

## Evidence

Independent tests in `tests/obligator_ascension.rs` enumerate every possible
Obligator damage-roll tuple and compare the resulting probabilities, then check
stance/charge/size boundaries, flat armour and NPC reductions. Ammunition tests
derive diamond/onyx mixtures, diary factors, undead immunity, accuracy-clamp
probabilities, small maximum hits, incompatible pairs, quivers, shot packing,
exports, share settings and finite-HP cooldown timing.
Ordinary attacks retain their established probability representation, while
exact JSON float parsing preserves new distributions in saved calibration inputs.

Primary implementations: [outgoing damage stages](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts),
[bolt effects](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/dists/bolts.ts),
[passive activation](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/BaseCalc.ts),
[compatibility/speed](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/Equipment.ts),
[stances](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/utils.ts).
Item/access cross-checks: [Obligator](https://oldschool.runescape.wiki/w/Obligator),
[Ascension crossbows](https://oldschool.runescape.wiki/w/Ascension_crossbows),
[crossbow ammunition table](https://oldschool.runescape.wiki/w/Crossbows),
[Seeking arrows](https://oldschool.runescape.wiki/w/Seeking_arrow).
