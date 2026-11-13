# Public recordings we can use

Reviewed **13 November 2026**. These three sources have visually checked gameplay HUDs and useful component evidence. They are **not yet fully matched simulator worksheets**. Keep observations whose relevant conditions match; do not fit unsupported damage or prayer effects into an uptime or incoming-damage adjustment.

The [machine-readable evidence ledger](public-recordings.json) records inspection points, observed values and restrictions. It is a research ledger, **not** a file for the browser's worksheet importer. No defaults were changed and no holdouts were evaluated.

| Encounter | Recording and starting point | Useful evidence | Current restriction |
| --- | --- | --- | --- |
| Vorkath | [TheEdB0ys — 1 Hour of Vorkath (DHCB)](https://www.youtube.com/watch?v=mdXk3JWoH-Y&t=216s) | Gear/bonuses, visible food/potions, ammo stacks, normal/acid/spawn execution and an extended session for consecutive-attempt annotation. | Ruby-to-diamond switching and karambwan/possible combo eating need explicit treatment before full-model time/consumption comparison. |
| Normal Yama | [FCASandChill — full commentated solo](https://www.youtube.com/watch?v=_CYs_6GTOUY&t=2288s) | Both Judges, final phase, Mark handling, visible stats/supplies, and a client-reported completed-fight duration. | Thralls, lured Judge fireballs and Donofly/perfect-dodge timing are outside the current full-fight model. Release-week footage needs mechanic-specific patch review. |
| Wardens | [NibzScape — solo 150, full P2](https://www.youtube.com/watch?v=ViGwWEw2Gu4&t=3631s) | Solo/raid-level confirmation, first full-health core, repeated downs, visible supplies and explicitly documented invocations. | Overclocked I/II, Deadly Prayers, thralls, adrenaline/DDS specials and raid supplies prevent a default full-fight comparison. Core body HP/exposure is a separately usable component. |

## Vorkath: best extended-session source

The creator shows the setup near **0:47–2:46**. At **1:11.3**, the equipment panel shows +260 ranged attack, +146 ranged strength, +9 Prayer, and 852 ruby dragon bolts in the quiver. The first attempt's preparation is visible around **3:36**.

A speed spot-check at **4:06.3 → 4:26.3** has an in-game clock change of **2:10:34 PM → 2:10:54 PM**: 20 seconds in both. This supports normal-speed timing in that inspected interval, not an assertion that every minute of the upload is unedited. At **5:21.3**, the HUD shows 57/750 boss HP and a 848-bolt stack following the ammunition swap. Do not equate stack loss with shots fired.

Use this source first for a detailed phase/attack/food timeline. Reconstruct remaining base stats and rune-pouch contents where observable; leave unknowns unset. Reserve later uninspected trips before fitting, and obtain another session/creator for stronger holdout evidence.

## Yama: an actual recorded result

The full solo example starts at **38:08**; Mark is prepared near **38:15**. At **43:19.4**, the client displays **4:53.40**, Yama **0/2500**, player HP **84**, Prayer **20**, and base 99 Attack/Strength/Defence/Magic in the restore panel. The two inspected game clocks advance **00:18:09 → 00:23:13**, matching the 304-second video-position difference.

The reported duration is **293.40 seconds**, or 489 nominal 0.6-second ticks. It is a **game-counter observation**, not automatically `Usage.ticks`: first-attackable/challenge and killing-hit boundaries must be reconciled. The later bank outro is another capture; do not use its supplies as this attempt's endpoint.

Judge sequences are near **39:40–40:10** and **41:35–42:07**. Use their independently observable transitions/overheads while accounting for the creator's lured fireballs. Do not use the entire kill to calibrate a model that omits those hits or thralls.

## Wardens: use phase-specific evidence

The source shows gear at **1:57**, inventory at **4:36** and invocations at **7:41**. The visible party/settings screen confirms one member and level 150. The creator identifies Deadly Prayers and Overclocked I/II; those are material for prayer and P3 timing.

At **1:01:37.7**, Elidinis' body bar shows **5180/5180** while the first core is attackable. This is a HUD cross-check of the existing level-150 core workload fixture; a plugin-supplied maximum is not an independent server-HP measurement. Annotate ejection/return frame boundaries and damage before treating it as a measured exposure duration; **no exact 21/29/37-tick duration was measured here**.

Creator-labeled full-phase chapters are **P1 56:18**, **P2 1:00:31**, and **P3 1:07:29**. Tutorial explanations separate these chapters. Do not subtract their timestamps to invent a continuous room duration or assume inventories match across cuts. P3 footage has not been fully annotated.

## Sources screened out of timing work

[TheEdB0ys' shorter Vorkath guide](https://www.youtube.com/watch?v=CKw7yqPhnMQ&t=1372s) explicitly describes speeding up its full-kill footage. Its wall-clock playback duration is unsuitable without a verified rate conversion. The previously checked [Theoatrix walkthrough](https://www.youtube.com/watch?v=0Pf80fdOlv8&t=183s) crops the inventory/stat HUD at the inspected points. Keep those as explanatory references.

## Next annotation batch

1. Record events, resource actions and exact loadout conditions from the selected windows; distinguish first fights from carry-over fights.
2. Put component observations in an appropriate phase comparison, or extend the model for the explicitly missing effects before producing full-fight worksheets. Unknown quantities remain unknown.
3. Preassign untouched holdouts before fitting. Same-session examples are correlated; a success-selected tutorial does not establish survival probability.

The [calibration guide](gameplay-calibration.md) retains the exact-setup requirements for full-fight comparisons. **Accepted fully matched fight worksheets remain zero for all three.** The progress here is three inspected sources and concrete gameplay observations, not a calibrated survival or trip forecast.
