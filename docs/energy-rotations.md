# Energy recovery and special rotations

Reviewed 13 November 2026. Equip charged Zorya's tome or Crimson kisten, then open
**Combat → Special attack burst → Compare a timed rotation**. This opt-in model
uses normal attacks between affordable specials with the selected weapon and
equipment. It does not change ordinary DPS, automatic gear ranking or full-fight
survival/supply estimates. Other special weapons remain unmodelled.

## Recovery clock

- Natural recovery adds 10% energy every 50 game ticks (30 seconds).
- A **worn** Lightbearer uses 25 ticks (15 seconds). Owning one does not accelerate
  the equipped setup. The comparison uses an available, level-usable, non-excluded
  owned ring and its actual stats, replacing the current ring.
- **Next energy pulse** declares the current timer; Fresh interval means a full
  interval remains. Spending energy does not reset partial progress.
- Recovery pauses at 100%. A full bar cannot bank an almost-completed pulse;
  spending from full starts a fresh interval. A pulse caps at 100%, restoring only
  the missing amount.
- Equipping Lightbearer preserves a pulse already at most 25 ticks away, otherwise
  reducing the wait to 25 ticks. Removing it always starts 50 ticks. Keeping the
  same ring, spending, or switching weapons does not itself reset this clock.
  The UI comparison applies one equip at the start, not repeated ring hopping.

The ring comparison is not an unconditional recommendation: extra specials must
compensate for the displaced ring's offensive stats, overkill and the chosen
window. Results use the same declared seed and trial count, but changing attack
choices means the random draws need not represent identical damage events.

## Rotation contract

The default is 512 reproducible trials, bounded to 4,096 trials and 500 ticks.
Damage samples the reviewed exact normal/special distributions. Zorya keeps a
successful-accuracy flag even when reductions produce zero damage; accurate
openers grant three follow-ups. Held follow-ups are consumed before another paid
activation, including when **Normal attacks only** disables new activations.
Every attempt consumes a held hit, including misses. Kisten samples its four-roll
mixture as one damage hit. [Burst formulas and restrictions](zorya-kisten.md).

The first action starts at tick zero. Each next action starts after its attack
cooldown; recovery at that tick happens before choosing an affordable special.
Only actions starting **before** the window's end are credited. A recovery pulse
exactly at the end contributes to final energy but cannot fund another action.
Damage is applied at action start; projectile flight and opener-to-follow-up
travel are excluded. Target death stops both attacks and the reported clock at
the killing action, retaining unused energy and buffs. HP-dependent NPC stats
are recalculated at current HP. The first seeded trial is available as a trace.

Outputs include effective damage capped by target HP, defeat chance with a Wilson
95% sampling interval, damage standard error, mean attack counts, actual energy
restored/spent/remaining and remaining empowerment. Sampling uncertainty describes
model noise, not uncertainty about the real game's mechanics. Inputs persist in
backups/share links and exported assumptions. Full-fight simulations remain
normal-attack models.

## Evidence and remaining work

Primary item rules: [Lightbearer](https://oldschool.runescape.wiki/w/Lightbearer),
[special energy](https://oldschool.runescape.wiki/w/Special_attack) and
[game ticks](https://oldschool.runescape.wiki/w/Game_tick). The current equip rule
includes the April 2024 fix that avoids delaying a nearly due pulse.

Pinned RuneLite revision
[`bb0fa42`](https://github.com/runelite/runelite/blob/bb0fa42b3d9329ffe31115f1f20042c455def2e3/runelite-client/src/main/java/net/runelite/client/plugins/regenmeter/RegenMeterPlugin.java)
corroborates 25/50-tick recovery, full-energy timer pausing and the equip/unequip
adjustment. This is client overlay source, not game-server code or a new gameplay
capture. Damage formulas retain the separately pinned calculator reference.

`tests/energy_rotations.rs` independently checks exact pulse boundaries, partial
progress, full-energy pauses/caps, equip/removal rules, five-minute activation
timelines, end-boundary ordering, held-hit priority, target death, accurate-zero
buffs, reproducibility, sampling against direct roll enumeration, real ring stat
loss, validation and isolation from normal calculations. Shared-link/export
coverage is in `tests/special_attacks.rs`.

Release checks: all 719 integrated Rust tests and 24 independent reference
loadouts pass, including the latest quest and Sailing changes. Optimized
WebAssembly, formatting and whitespace checks pass. Synthetic browser checks
confirm three versus five kisten activations over 500 ticks from full energy,
zero-energy recovery, the worn ring's 15-second interval, normal-only policy,
saved settings after reload, exported rotation notes, and Zorya's three held
two-tick attacks before a new paid opener. Ring comparisons include displaced
stats rather than assuming more specials always means more damage. No browser
application errors were reported. These are model/browser checks, not gameplay
calibration.

Remaining: other weapon specials, ordered multi-weapon swaps, travel/movement,
phase transitions and downtime, Death Charge/Surge/pool and other external
restores, charge consumption, incoming survival and actual gameplay timing
calibration. The bounded comparison does not establish whole-fight special DPS.
