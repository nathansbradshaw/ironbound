# Nex and raid comparisons

Outgoing damage and bank gear readiness are available for these references.
Gear ratings remain separate from known Slayer requirements. Quest, key, kill
count and task access are not inferred.
Raid screens compare usable owned specialist weapons and compatible ammunition
in the style's baseline armour, including ruby bolts and elemental casting
weapons. Exclusions and the declared shot/cast budgets still apply.

| Encounter | Readiness reference |
| --- | --- |
| Nex | Group ranged gear; five vulnerable body HP points and four activated mages |
| Chambers | Solo or three-player, maximum party Combat 126 / HP 99, average Mining 99 |
| Chambers CM | Solo, same party skill reference, CM scaling and exceptions |
| Theatre | Three-player minimum HP or five-player HP; owned tribrid gear |
| Theatre Hard | Hard-mode stats, three-player HP; owned tribrid gear |
| Fractured Archive | Default route, recorded defences, explicit 500-HP points for large bodies |
| Expert ToA | Exactly solo level 300, path 0; vulnerable style windows |

There are 194 declared raid/Nex attack states. Chambers also has five-player
normal and three-player CM data references; these are not additional bestiary
ratings. Alternative state buttons retain the selected raid scale. Preset target
IDs retain their scale when shared; room toggles preserve it too.
Older saved Archive routes with one Xarth entry retain their raid identity,
styles and room controls. Editing another fork does not silently add a phase;
newly selecting the Xarth fork includes both reviewed body windows.

## Reviewed rules

- **Nex:** body HP probes allow observed HP across nominal phase boundaries.
  Zaros defaults to 680+500 HP. Deflect Melee blocks melee; protected transitions
  and Blood Siphon give zero outgoing HP damage. Siphon healing is not quantified.
  Activated mages have -2 physical flat armour: +2 per accurate splat, including
  supported bolt effects. Accuracy misses remain zero. Protected mages take zero.
- **Chambers:** party skill/team scaling, CM flooring/clamps, Tekton and crystal
  exceptions, Olm hand/head HP and mage-hand Magic. Ranged immunity, off-style
  divisors and Ice demon fire/demonbane exceptions apply per splat. Vespula and
  its portal cannot be hit with melee. Guardian HP uses declared average party
  Mining; damage uses your base Mining and owned pickaxe tier. Readiness checks
  the pickaxe separately and asks for Mining when that level is missing.
- **Theatre:** minimum three-player and five-player HP scales, Hard-mode catalog
  stats, explicit Nylocas colours and Verzik phase-1 per-splat random limiters.
  Wrong Nylocas styles are blocked; healing/reflection is not quantified.
  Dawnbringer normal and one-special probes explicitly supply the raid loan.
  They do not assert bank ownership or add it to exports as owned equipment.
  The guaranteed 75–150 special ignores damage bonuses; its ordinary cooldown
  is a comparison only, excluding 35% energy cost, transfers and regeneration.
- **Archive:** recorded variants/weaknesses, flying/grounded Guardian reach,
  Bork armour and Magic resistance keyed to **Smorks' remaining HP**, the stun's
  -100 melee defence bonuses, and Ocellus phase-4 curses/explosion transforms.
  Explosion damage is `floor(2 * damage / ordinary weapon ticks) + uniform(0,1)`
  per accurate splat. Misses stay zero; ruby procs that roll zero retain the coin
  flip. The timed opportunity uses two expected ticks; the other probe retains
  ordinary cooldown. Archive team HP scaling is unverified: edit current HP to
  the observed value for HP-dependent attacks. Do not read 500 as total boss HP.
  Xarth has separate phase-1 Magic and phase-2 melee route suggestions at 80%
  Demonbane effectiveness; corpse healing/cast-speed changes are excluded.
  Ezekiel uses the pinned ordinary formula; reported special-state Magic caps
  are not inferred from it. Bork stun guarantees melee accuracy as described by the Wiki strategy;
  ranged and Magic retain normal accuracy. Ascension bolts and ordinary
  Eclipse burn buildup are supported on Bork, Smorks and Xarth body probes
  where applicable; flat armour leaves burn pulses unchanged. See
  [Archive DPS verification](archive-dps.md).
- **Expert ToA:** solo-300 HP/rounding and defence-roll scale, existing ToA
  Fang/Shadow behaviour, style-restricted Akkha and reviewed Wardens hit models.
  Legacy ToA's invocation controls retain their existing supported range.

These states deliberately withhold complete encounter attack budgets. They do
not establish phase uptime, team damage allocation, raid completion, incoming
attack sequences or whole-trip supplies. Nex's pre-existing selected Smoke/Ice
incoming scenarios remain separate. Special Eclipse burn timing and combined
unverified effect ordering stay gated. Entry Theatre, arbitrary Expert settings,
Archive team HP and full-fight/room mechanics remain follow-up work.

## Sources and refresh

The public Wiki calculator is pinned at
[`74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f`](https://github.com/weirdgloop/osrs-dps-calc/tree/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f):
[NPC variants](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/cdn/json/monsters.json),
[Chambers scaling](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/scaling/ChambersOfXeric.ts),
[Theatre scaling](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/scaling/TheatreOfBlood.ts),
[ToA scaling](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/scaling/TombsOfAmascut.ts),
[NPC transforms and attack speed](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/PlayerVsNPCCalc.ts),
and [phase states](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/scaling/Phases.ts).
Wiki encounter context:
[Nex](https://oldschool.runescape.wiki/w/Nex),
[Fumus](https://oldschool.runescape.wiki/w/Fumus),
[Dawnbringer](https://oldschool.runescape.wiki/w/Dawnbringer),
[Archive strategy](https://oldschool.runescape.wiki/w/The_Fractured_Archive/Strategies).

Download that revision's public NPC JSON, then run:

```sh
cargo run --locked --bin raid_data -- /path/to/public-monsters.json
cargo test --locked --test monster_raids
```

The Rust importer reads public NPC data and existing local route definitions;
it never reads account data or executes upstream code. Review source changes
before updating the pin. Independent arithmetic regressions cover scaling,
limiters, armour/bolt ordering, loan semantics, phase choices, workload guards,
room toggles and access-independent readiness.
