# Zorya and Crimson kisten special attacks

Reviewed 13 November 2026. Equip a charged Zorya's tome or Crimson kisten under
**Gear → Weapon**, then open **Combat → Special attack burst**. Normal attack DPS,
gear ranking, packing and full-fight simulations continue to use normal attacks.
The burst panel models one activation, not an indefinitely repeatable spec.

## Zorya's tome

The opener costs 50% energy, takes the existing three-tick powered-attack cooldown,
and multiplies the final attack roll by 250/100 and maximum damage by 160/100,
with separate integer floors after gear, prayer and task/Salve stages. Elemental
weakness does not apply to the powered spell. Its two-handed restriction excludes
shields and the one-handed Confliction passive. Only the charged catalog variant
and base 85 Magic requirement are accepted.

A successful opener accuracy roll grants three empowered attacks. A failed roll
grants none. Damage reduced to zero after a successful roll retains the successful
accuracy flag for buff application; this is the source-wording interpretation,
not a separate gameplay capture. Empowered attacks use the same accuracy/damage
boosts and take two ticks apiece. Every attempted empowered attack consumes one
held attack, including misses. They do not refresh themselves or spend more energy.

The exact probability calculation stops when the target is defeated and retains
unused empowerment. It reports effective damage capped at remaining HP, damage
before overkill, defeat chance, expected attempts, cooldowns and remaining buffs.
The normal estimate starts without a buff. **Zorya starting state** can instead
declare 1–3 already-held attacks; this costs no new energy and can compare those
attacks against a different target. It is a manually confirmed state, not a claim
that buffs survive any particular switch, reset, bank, pool or logout.

Follow-up calculations use each remaining-HP state, including the existing
HP-dependent NPC stat evaluator, within the same supported attack window.
Cooldown sums include the finishing cooldown. They exclude movement, projectile
travel, waiting for an opener projectile, target swaps and phase transitions.
Actual timing and charge consumption still require gameplay evidence.

## Crimson kisten

Brutal Swing costs 50% energy and makes **one damage hit** at the normal four-tick
cooldown. Four independent accuracy rolls determine its damage band:

| Successful rolls | Inclusive damage range before NPC reductions |
| --- | --- |
| 0 | Miss |
| 1 | floor(max × 70/100)–floor(max × 110/100) |
| 2 | floor(max × 90/100)–floor(max × 130/100) |
| 3 | floor(max × 110/100)–floor(max × 150/100) |
| 4 | floor(max × 130/100)–floor(max × 170/100) |

The chance of exactly k successful rolls is the independent binomial probability,
not four damage splats. Attack bonuses and stance use the selected supported
attack type (the current planner maps Crush to Accurate and Stab to Controlled);
target defence is always crush. Inquisitor modifiers therefore still
follow the selected crush style. Accurate zeros become one before ordinary flat
armour; NPC caps, rerolls and final division apply to the single resulting hit.
Reviewed guaranteed-maximum Phosani minions replace it with the final maximum.

**Source discrepancy:** the release announcement promised a final maximum minus
one when all four rolls succeed. The pinned calculator explicitly records that
this reduction is absent in observed game behaviour; the current Wiki item table
also gives the unmodified 130–170% band. This implementation follows that current
formula and does not silently add the announced minus one. It remains subject to
future game fixes; the tests establish the chosen arithmetic, not a new game test.

## Limits and persistence

- Available special energy is explicit. Less than 50% prevents a new activation;
  100% still previews only one activation. Held Zorya hits require no new energy.
- Invalid charge variants, unmet/unknown known wear levels, two-handed/offhand
  conflicts, invulnerable or unreviewed resource/boundary states are unavailable.
- Wardens P2/core, non-HP counters, encounter minimum-hit bounds and phase
  transitions, counterattack and scripted guaranteed-accuracy windows remain
  excluded from special-burst estimates. Supported ordinary
  reductions include Corp (full Magic, half non-Corpbane melee), Huey caps and
  per-hit Vorkath acid halving. These do not establish whole-fight special timing.
- Energy and held-buff assumptions save in private backups/share links and setup
  notes. They never alter normal DPS scores or automatically insert spec swaps.
- Recovery/Lightbearer, chained activations, resource replenishment, actual charge
  balances, complete rotations and special-weapon recommendations remain open.

## Sources and verification

Pinned primary reference revision
[74958b1](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f)
was confirmed as HEAD during this audit:
[outgoing accuracy/damage and kisten distribution](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts),
[attack timing and energy costs](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/Equipment.ts).
The reference marks Zorya partial because it does **not** model the follow-up
buff; that state machine is an independent Rust extension grounded in
[Zorya's item mechanics](https://oldschool.runescape.wiki/w/Zorya%27s_tome) and the
[official rewards blog](https://oldschool.runescape.wiki/w/Update:The_Fractured_Archive_-_Rewards_Poll_Blog).
Kisten cross-checks: [current item mechanics](https://oldschool.runescape.wiki/w/Crimson_kisten),
[release announcement](https://oldschool.runescape.wiki/w/Update:The_Blood_Moon_Rises_-_Out_Today!).

`tests/special_attacks.rs` independently checks all sixteen accuracy-roll patterns,
Zorya's full/failed opener branches, accurate-zero triggers, held-hit consumption,
death/overkill and a recursive probability oracle, stage floors, energy and skill
boundaries, caps/armour/halving, guaranteed maxima, persistence/export and isolation
from normal calculations. Synthetic fixtures are not gameplay calibration.

Final integrated verification: all 691 Rust tests passed, including 12 special
attack tests and the latest monster/Tormented Demon regressions. All 24 independent
reference fixtures, optimized WebAssembly, formatting and whitespace checks pass.

Disposable browser checks against Zombie (26) show normal Zorya DPS remaining
7.049 when the special budget changes. The opener and empowered previews have
41 maximum hits at three/two ticks. A paid activation reports 21.74 effective
damage, 34.35 before overkill and 50% spent; zero energy prevents activation.
Three explicitly held attacks remain usable with zero energy, persist after
reload and appear in Inventory Setups export notes. Kisten reports one attack,
34 maximum, a four-tick cooldown and 50% spent (21.99 effective / 29.38 uncapped
damage). No browser application errors were reported. The preview was stopped
after verification; actual gameplay and recovery/rotation gates remain open.
