# Monster mechanics backlog

Reviewed against remote main on 13 November 2026. Catalog entries and outgoing
attack windows do not establish a complete encounter simulation. Check each
layer separately: outgoing damage, incoming damage, encounter timing/supplies,
independent references, browser verification, then gameplay verification.

## 1. Shared outgoing damage and restrictions

- [ ] Zulrah: reroll hits above 50 into 45–50, including bolt procs; allow only
  reachable attacks; share the 500-HP workload across forms. Rotations, venom,
  snakelings, travel and incoming damage remain separate work.
- [ ] Kraken / Cave kraken: ranged damage divided by seven with its minimum-hit
  rule; boss melee immunity; distinguish emerged targets from whirlpools.
- [ ] Corporeal Beast: halve unsupported weapon damage per outcome; preserve
  full magic, eligible stab weapons and ruby procs; apply diamond before Corp.
  Dark core, drains and incoming attacks need separate verification.
- [ ] Dagannoth Kings: reviewed ordinary outgoing windows for Rex, Prime and
  Supreme; preserve their actual defences without inventing style immunity.
  Overlapping aggro, Spinolyps, respawns and incoming damage remain open.
- [ ] Kree'arra: reachable-style restrictions and ordinary outgoing damage;
  bodyguards, movement and incoming attacks remain open.
- [ ] Grotesque Guardians: Dawn reach restrictions, Dusk ranged/magic immunity,
  separate vulnerable windows and a shared HP workload for Dusk's forms.
  Lightning, orbs/healing, transitions and incoming damage remain open.
- [ ] Kalphite Queen: both forms, protection/accuracy rules, respawn and incoming
  attacks; reference-check Keris interactions before enabling outgoing damage.

## 2. Other dedicated boss models

- [ ] Abyssal Sire: respiratory systems, stun/shadow spells, transition reduction,
  immunities, spawns, movement and final-phase damage.
- [ ] Alchemical Hydra: chemical weakening, 75/50/25% phase budgets, vents,
  protection rotations, poison/fire and incoming damage.
- [ ] Phantom Muspah: melee/ranged forms, shield prayer damage, reductions,
  step-back timing, teleport/spike phases and incoming damage.
- [ ] Duke Sucellus: preparation versus awake combat, reduced demonbane
  vulnerability, enrage cadence, movement and incoming damage.
- [ ] Vardorvis: current-HP-dependent stats, defence reductions, axes, head,
  entangle and incoming damage; recompute distributions as HP changes.
- [ ] Leviathan: reach restrictions, stun/rear bonuses, enrage and incoming damage.
- [ ] Whisperer: specials, shadow realm, enrage, sanity and incoming damage.
- [ ] Nightmare / Phosani: shield/HP phases, magic totem multiplier, husks,
  sleepwalkers, parasite and incoming damage; normal and solo models stay distinct.
- [ ] Nex: phase-specific HP, immunities, blood healing, minions, team scaling
  and attack mixtures. Existing Smoke/Ice incoming scenario is only partial.
- [ ] Hespori: flower invulnerability, roots and incoming damage.
- [ ] Skotizo: altar reductions, Arclight interactions, adds and incoming damage.
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

Initial inventory recorded. Implementation verification is pending.
