# Archive outgoing DPS verification

Bork, Smorks and Xarth now have numeric ordinary outgoing comparisons. Combat
shows Bork's armour/stun states and Xarth's phase-2 body separately. These use
declared current-HP probes; the default 500 HP is not inferred team-scaled HP.

| Bork state | Physical armour per hitsplat | Magic resistance per hitsplat |
|---|---:|---:|
| Smorks 100–75% HP | 8 | 0 |
| Smorks 75–50% HP | 10 | 2 |
| Smorks 50–25% HP | 12 | 4 |
| Smorks 25–0% HP | 14 | 6 |
| Stunned | 0 | 0 |

The table follows the pinned Wiki calculator's
[phase transformer](https://github.com/weirdgloop/osrs-dps-calc/blob/74958b18f6559cf1bf58f7ac74e7a442a7ac8e2f/src/lib/scaling/Phases.ts).
Stunned Bork also has all three melee defence bonuses lowered by 100 and
guaranteed melee accuracy, as described by the
[strategy](https://oldschool.runescape.wiki/w/The_Fractured_Archive/Strategies).
Ranged and Magic keep normal accuracy; this does not make every style guaranteed.

Smorks have two physical flat armour and no Magic resistance. Physical armour
is applied to each successful splat, after weapon and bolt effects. Accurate
hits reduced to zero can still trigger Eclipse burns. Armour does not reduce
the burn pulses. Ascension plain, unenchanted tipped, diamond and onyx bolts
use the existing per-shot model with armour after each bolt's roll. Both stances
and Kandarin proc rates are covered; no ruby Ascension bolt is invented.

Xarth's unshielded body uses its recorded defences, 85% water weakness and 80%
Demonbane effectiveness. It does not inherit the ordinary Tormented Demon
shield, protection cycle or unshielded weapon-speed bonus. Eclipse buildup is
available for stationary body comparisons. Bork and Xarth retain Defence floors
of 180 and 102; Smorks can drain to zero.

`tests/archive_remaining_dps.rs` independently enumerates Bork mace/trident/
crossbow rolls, all stunned melee attack types, Smork armour, ruby/diamond
ordering, Scythe/Karil splats, Eclipse pulses and Xarth modifiers. Ascension
checks cover five ammunition variants, both stances, diary on/off and four
armour states. Every Archive route retains outgoing coverage.

These comparisons do not simulate Smork HP changes, stun uptime, cannonball
damage, team HP, movement, corpse healing, extra corpses from fire spells,
incoming attacks or complete supplies. Full-fight burn timing, HP workloads
and gameplay acceptance remain separate from the stationary outgoing DPS.
