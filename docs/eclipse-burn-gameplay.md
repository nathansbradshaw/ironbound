# Eclipse burn transitions: inspected gameplay

Reviewed 13 November 2026. This is component evidence, not a fitted whole-fight
worksheet. **Current-patch confirmation remains open.** The directly inspected
Eclipse source was published on 31 March 2024; do not turn that into a claim of
2026 gameplay verification or apply one boss's behavior to another.

## Vorkath acid: retain existing Eclipse burns

Source: [Fairly Tender — OSRS Eclipse atlatl vs Vorkath](https://www.youtube.com/watch?v=zcq4mX-iyJU).
The equipment is shown near [0:09](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=9s):
Eclipse helm, chestplate, tassets and atlatl. Playback metadata was inspected at
1× speed; the following points were examined visually, including frame stepping
around the acid exit. Commentary is supporting context, not the damage evidence.

| Video position | Visible observation | What it establishes |
| --- | --- | --- |
| [2:25](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=145s) | Acid launch; recent Ranged XP counter reads 9,209,594. | Boundary location and last attack context. |
| [2:30](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=150s) | Acid pools/fireballs, walking player, displayed boss HP 496/750. | A sample within acid after the earlier attack's flight. |
| [2:35](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=155s) | Player still walking, displayed boss HP 492/750; creator explains not attacking during this phase. | Continued damage without new atlatl attacks. |
| [2:40](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=160s) | Acid remains active; a positive sunburst burn hitsplat of 2 appears on Vorkath. | Existing Eclipse burn is not cleared on acid entry and is not wholly suppressed during acid in this recording. |
| [2:42.5](https://www.youtube.com/watch?v=zcq4mX-iyJU&t=162s) | Acid pools have disappeared; a positive burn hitsplat is visible before the next Ranged XP drop/projectile impact. | Existing burn survives acid exit. |

The HUD's HP values are client displays, not independent server-HP measurements.
Acid pool healing and later special attacks prevent treating net HP change as an
exact burn total. These sparse observations do not measure every pulse, infer an
exact stack count, validate the four-tick ordering, or fit an entire kill.

The old default cleared stacks at both acid boundaries. That contradicts the
observed persistence, so acid entry/exit now retain existing stacks and their
clock independently of the unverified-boundary toggle. The existing half-damage
acid transform remains; the visible 2 does not independently determine its exact
rounding or starting stack count. Expiry, death and fresh-target rules still apply.

The current [Vorkath strategy page](https://oldschool.runescape.wiki/w/Vorkath/Strategies)
has a burn-nullification footnote attached to **burning claws**, not Eclipse.
That is not evidence that Eclipse stacks clear. [Burn rules](https://oldschool.runescape.wiki/w/Burn)
also distinguish normal claw burns from strong Eclipse burns. Neither fact
eliminates the need for a recent Eclipse-specific repeat before calling this
current-patch verified.

## Remaining boundaries

| Boundary | Evidence status and model behavior |
| --- | --- |
| Vorkath freeze/spawn | The same source was inspected near 3:02.5–3:07.5, but an identified live stack could not be followed cleanly through the whole interval. Immunity is modeled; clear versus age remains an explicit assumption. |
| Yama transitions/intermissions | No suitable full-Eclipse burn trace established in this review. Keep the clear/retain scenario control; do not infer it from Vorkath. |
| Wardens shield/core, skulls and enrage | No suitable full-Eclipse burn trace established. The inspected [Gnomonkey budget guide](https://www.youtube.com/watch?v=S4Z7Mjjdt4I&t=3090s) has other damage sources and does not establish a known Eclipse stack through the inspected skull interval. A phase label or an atlatl alone is insufficient. Keep the scenario control. |

To close these gaps, record full Eclipse with an identified live burn immediately
before a boundary, stop direct attacks before projectile flight can overlap it,
and capture the entire transition plus the first post-transition pulse. Declare
patch date, speed, target identity, special attacks and other damage sources.
Distinguish zero-damage pulses, expiry and target replacement from stack clearing.
Synthetic simulations and ordinary non-Eclipse guide footage cannot supply this
missing observation.

Regression tests are deliberately separate: fixed acid pulses check persistence
at both boundaries, preserved cooldown, combined pulse reduction, natural expiry
and immediate stopping at death. Simulator revision 3 invalidates older fitted
worksheets after this timeline change.
