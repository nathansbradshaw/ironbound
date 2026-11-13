# Monster mechanics backlog

Reviewed against remote main on 13 November 2026. Catalog entries and outgoing
attack windows do not establish a complete encounter simulation. Check each
layer separately: outgoing damage, incoming damage, encounter timing/supplies,
independent references, browser verification, then gameplay verification.

## 1. Shared outgoing damage and restrictions

- [x] Zulrah outgoing: reroll hits above 50 into 45–50, including bolt procs;
  enforce reach and share one 500-HP comparison workload across selected forms.
- [ ] Zulrah full fight: actual rotations/HP allocation, venom, snakelings,
  travel, incoming damage and supplies.
- [x] Kraken / Cave kraken outgoing: ranged damage divided by seven with its minimum-hit
  rule; boss melee immunity; distinguish emerged targets from whirlpools.
- [ ] Kraken full fight: awakening, tentacles, incoming attacks, healing and supplies.
- [x] Corporeal Beast outgoing: halve unsupported weapon damage per outcome; preserve
  full magic, eligible stab weapons and ruby procs; apply diamond before Corp.
- [ ] Corporeal Beast full fight: dark core, actual drains, incoming attacks and timing.
- [x] Dagannoth Kings outgoing: reviewed ordinary windows for Rex, Prime and
  Supreme; preserve their actual defences without inventing style immunity.
- [ ] Dagannoth Kings full fight: overlapping aggro, Spinolyps, respawns and the
  combined incoming attack sequence.
- [x] Dagannoth Kings selected incoming: Supreme ranged, Prime magic and Rex
  slash scenarios with matching protection prayers. Full attack overlap remains open.
- [x] Kree'arra outgoing: reachable-style restrictions and ordinary outgoing damage.
- [ ] Kree'arra full fight: bodyguards, movement, chinchompas and incoming attacks.
- [x] Grotesque Guardians outgoing: Dawn reach restrictions, Dusk ranged/magic
  immunity, separate vulnerable windows and one comparison HP pool for Dusk.
- [ ] Grotesque Guardians full fight: actual phase thresholds, invulnerability,
  lightning, orbs/healing, transitions and incoming damage.
- [x] Kalphite Queen outgoing: both forms and supported Keris effects use
  recorded defences; overheads do not grant complete style immunity.
- [ ] Kalphite Queen full fight: attack mixtures, workers, prayer drain,
  movement, respawn and supplies.
- [x] Kalphite Queen selected incoming: ordinary stab accuracy, guaranteed
  ranged/magic accuracy, zero rolls and matching protection. No attack mixture is assumed.

## 2. Other dedicated boss models

- [x] Abyssal Sire outgoing: exposed respiratory minimum hits, successful
  Demonbane finishers, reach restrictions, four-vent workload and vulnerable
  body stages with explicit 425→210→140→0 comparison allocations.
- [ ] Abyssal Sire full fight: stun/shadow spells, protected vents, regeneration,
  transition reduction, overshoot/reset rules, spawns, movement and incoming damage.
- [x] Alchemical Hydra outgoing: four 275-HP stages, phase-bounded current HP
  and three before-chemical comparisons with per-hitsplat 75% reduction, including
  bolt procs and multi-hit weapons.
- [ ] Alchemical Hydra full fight: actual vent timing, wrong-vent attack boosts,
  transition overshoot, protection rotations, poison/fire and incoming damage.
- [x] Phantom Muspah unshielded outgoing: recorded form defences/air weakness,
  125 Defence floor, shared 723-HP comparison allocation and 127-HP final stand.
- [ ] Phantom Muspah full fight: prayer-shield damage, damage/four-hit form
  thresholds, freezes, step-back timing, teleport/spikes, healing and incoming damage.
- [x] Duke Sucellus outgoing: normal post-quest awake window with 70% Demonbane
  effectiveness; retain quest/awakened gates.
- [ ] Duke Sucellus full fight: preparation, poison, gaze, gas, enrage cadence,
  movement and incoming damage.
- [x] Vardorvis outgoing: exact normal HP-dependent Defence/Strength, stat-drain
  ordering/floor, current-HP comparisons and a finite-HP recurrence that recomputes
  the complete outgoing distribution/cooldown at every remaining HP.
- [ ] Vardorvis full fight: axes, head, entangle, lifesteal, movement and incoming
  damage. Quest and awakened variants require separate validation.
- [x] Leviathan outgoing: normal pre-enrage ranged/Magic, melee restriction,
  post-stun front cap and rear Magic double-damage comparison.
- [ ] Leviathan full fight: stun cast/initial free shot, rear ranged minima and
  bolt procs, special reductions, pathfinder/enrage, movement and incoming attacks.
- [x] Whisperer outgoing: normal body, per-hit special reductions outside/inside
  the Shadow Realm, earth weakness and a separate 140-HP enrage with reset stats
  and independent debuff assumptions.
- [ ] Whisperer full fight: special thresholds/uptime, sanity, souls/healing,
  tentacles, travel and incoming attacks; quest/awakened need separate validation.
- [x] Nightmare / Phosani outgoing: distinct base-team/solo shield and charge
  counters, protected bodies, totem Magic damage/accuracy, separate Phosani final
  HP, and maximum crush hits on Phosani husks/weakened parasite. Counters travel
  through comparisons/exports; ruby against shields/charge stays blocked.
- [ ] Nightmare / Phosani full fight: larger-team scaling, shield/ruby proc
  semantics, stat recovery, parasite emergence/healing, sleepwalkers, spawn counts,
  spores/flowers, movement and incoming damage. Confirm the totem accuracy model
  in gameplay; normal and solo models stay distinct.
- [ ] Nex: phase-specific HP, immunities, blood healing, minions, team scaling
  and attack mixtures. Existing Smoke/Ice incoming scenario is only partial.
- [x] Hespori outgoing: flowers-cleared and flowers-alive body comparisons,
  fire weakness, guaranteed Farming Guild flower finishers and 12-flower attack
  budget. Quest flower rules stay separate and blocked.
- [ ] Hespori full fight: exact flower thresholds/overshoot, travel, roots,
  poison, incoming attacks and full supplies; quest variant needs its own model.
- [x] Skotizo outgoing: zero through four active altars, per-hitsplat/bolt
  reductions, exact Arclight/Emberlight exceptions and active altar finishers.
- [ ] Skotizo full fight: random activation/cooldown sequence, travel, adds,
  attack mixtures, incoming damage and full supplies.
- [ ] Hueycoatl: body/tail damage limits, crush/earth rules, pillar bonus and team
  scaling; verify these independently of generic elemental weakness.
- [ ] Royal Titans: titan resistances, elemental-add accuracy, barriers, duo
  scaling/timing and incoming attacks.
- [ ] Doom of Mokhaiotl: delve scaling, shield/demonbane rules, special phases,
  movement and incoming damage.
- [ ] Shellbane Gryphon, Mad Angel and Maggot King: each needs its own sourced
  phase, scaling, immunity, add and incoming-damage fixtures.
- [ ] Sol Heredit and TzKal-Zuk: boss-specific specials, positioning/shield,
  adds/healers and incoming attacks, separately from their full waves.

## 3. Raids, waves and dedicated equipment

- [ ] CoX: room/team/CM scaling, Tekton, Ice demon, Vespula/portal, Vanguards,
  Vasa/crystals, Guardians/pickaxes, Muttadiles, Mystics, Shamans and Olm's
  hands/head, transitions, healing and full raid supplies.
- [ ] ToB: normal/entry/hard scaling; Maiden, Bloat, Nylocas, Sotetseg, Xarpus
  and Verzik phase/immunity/damage rules, including Dawnbringer.
- [ ] Fractured Archive: Archive Guardian, Ezekiel, Bork/Smorks, Moia, Dreadnoth,
  Xarth, Dagon'hai and Ocellus; verify each route's scaling, phases, adds and
  transformations before enabling its DPS.
- [ ] ToA: path/team/Expert scaling; Akkha protections; complete Wardens cycle,
  core transfer/downs, skulls and enrage. Reviewed outgoing Wardens windows are
  already merged; full-fight work is tracked in the main backlog.
- [ ] Fight Caves / Inferno / Colosseum: actual wave counts, spawns, reinforcements,
  protection sequences, healing, invocations and full-trip supplies.
- [ ] Gauntlet / Corrupted Gauntlet: encounter equipment, preparation resources,
  Hunllef protection rotations, tornadoes, floor hazards and supplies.
- [ ] Tempoross / Wintertodt / Zalcano: dedicated skilling/equipment/resource
  models; do not treat them as ordinary bank-combat optimisation.

## 4. Existing partial models and quest/attribute cases

- [ ] Normal Yama: actual flare counts, regeneration/healing, movement,
  transitions, glyphs and incoming damage; contracts require a separate model.
  Judge alternation, final shadow flare and outgoing boss stages are merged.
- [ ] Vorkath: complete six-attack/special cycle, spawn/freeze immunity,
  acid movement/healing and incoming damage. Current-HP ruby and acid outgoing
  windows are merged.
- [ ] Moons: curses, special phases, camp boosts and movement; normal chains,
  flat armour and Blood healing are implemented.
- [ ] Barrows: Dharok's changing incoming damage, Verac's bypass distribution,
  remaining set effects, prayer drain and tunnels; Guthan healing is partial.
- [ ] Zilyana / Graardor / Sarachnis: complete basic attack mixtures, adds,
  movement and specials; retain the existing selected incoming scenarios.
- [ ] Fragment of Seren, Slagilith and Tormented Demons: their damage limiters,
  weapon exceptions, protection/shield states and phase-specific effects.
- [ ] Vampyres, leafy and flying monsters: exact weapon/spell requirements and
  exceptions; a blanket attribute block is only a temporary coverage gate.

## Acceptance for each outgoing window

1. Pin public source revisions and exact NPC variants; reject unknown states.
2. Add independently calculated boundary, rounding, immunity and proc cases.
3. Route planning, comparisons, current-HP effects and exports through the same
   rules; expose unsupported full-fight assumptions beside the result.
4. Run integrated Rust tests, reference cases, Wasm and browser checks; merge,
   publish and verify the released revision. Record gameplay as a separate check.

## Evidence

13 November 2026: 20 reviewed outgoing NPC windows, implemented in Rust with
explicit source IDs, reach/immunity rules and per-hit damage transforms. The
integrated suite passes 441 tests, including 15 dedicated monster regressions
and all 24 existing independent combat references. The standalone reference
comparator and release Wasm build pass. Multi-hit checks retain Karil's,
Torag's and Scythe's separate hits through NPC caps/reductions; changing-HP ruby effects retain
the same reductions/rerolls in finite-HP expectations.

Browser verification on a disposable local origin: Zulrah ruby max 50 at full
HP and 40 at 200 HP, explicit missing full-rotation workload; Kraken blocked
melee and reduced ranged damage; Sire 4/5 basic windows with phase 1 still
unsupported and four 50-HP exposed systems; Duke's awake-only scope and reduced
Demonbane note. Incoming estimates stay explicitly unavailable for these newly
reviewed outgoing windows. Merge/publication evidence follows in the PR.

Gameplay, physical RuneLite import, player movement and full-fight supply
validation remain open. A checked outgoing item does not close its full-fight
follow-up.

Second batch, 13 November 2026: 11 additional outgoing states (seven Hydra,
three Muspah, one normal Vardorvis) and selected incoming scenarios for both KQ
forms and all three Kings. Thirteen focused regressions independently check
rounding, bolt procs, Scythe splats, phase bounds, interpolation, drain ordering,
finite-HP miss loops/overkill, source-ID Slayer assignments and incoming protection.
The integrated build retains Moon follow-up reductions and rejects delayed burn
in finite-HP and fight-timeline models that cannot account for it.

Browser checks on the merged preview: Hydra before-chemical ruby maximum 25,
electric-state HP 551–825 and lower-bound clamping; matching Slayer assignment
across chemical states; Vardorvis accuracy and finite-HP budget changing at
700 versus 350 HP; KQ magic accuracy 100% and zero protected HP damage; Muspah
three of four supported outgoing windows, the prayer shield still unavailable,
and a 127-HP final allocation. No browser console errors were observed.

Integrated with the merged armour, fight-simulator and wilderness work: 504 Rust
tests pass, alongside all 24 standalone reference cases and the release Wasm
build. Deployment verification is recorded in the release PR.

Third batch, 13 November 2026: nine reviewed outgoing states for Skotizo,
its active altar, Hespori and Farming Guild flowers, plus an explicit quest-flower
gate. Eleven new regressions check the 15%/25% altar reductions, per-splat and
bolt ordering, named sword exceptions, flower invulnerability, one-hit finishers,
burn exclusion, 12-flower workloads and unchanged default boss HP allocations.
All 515 integrated Rust tests, 24 standalone reference cases and the release
Wasm build pass, with the latest OSRS skin retained.

Browser preview: ordinary Skotizo damage drops to zero with four active altars;
the altar is a separate comparison. Hespori's living flowers block direct body
damage; its flower target guarantees a 10-damage hit and shows exactly 12 attack
commitments (28.8 seconds for the example four-tick weapon). No console errors.
This excludes travel, flower thresholds, activation sequences and incoming attacks.

Fourth batch, 13 November 2026: 17 reviewed outgoing states and three explicit
gates for Leviathan, Whisperer, normal Nightmare and Phosani. Fifteen independent
monster regressions cover caps, fractional reductions, bolts, Scythe/Moon hits,
enrage/debuff scope, distinct shield/charge counters, totem effects, crush minions,
variant gates, exports and unverified burn-pulse interactions. An additional
readiness regression requires both melee and Magic for the Nightmare screens.
Integrated with the current burn, calibration, public-recording, death-risk and
requirements work: 586 Rust tests and all 24 standalone references pass.

Browser preview: Leviathan stunned-front ruby maximum 10 and blocked melee;
Whisperer example Magic maximum 20 becomes 6 outside or 8 inside the realm,
with a separate 140→0 enrage budget; normal/Phosani shield starts are 2,000/400
and totem starts 300/200, each with twelve totems. Protected bodies deal zero;
a weakened parasite with a guaranteed 19 max still needs several hits for 55 HP.
No console errors. Totem accuracy follows the documented strategy interpretation
and still needs gameplay confirmation. Deployment evidence is recorded in the PR.
