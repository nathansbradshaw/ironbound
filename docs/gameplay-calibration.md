# Compare recorded fights

**Gameplay calibration remains open for Vorkath, normal Yama and Wardens.** Three [public recording sources](public-recordings.md) now have visually verified component observations, including a real Yama result and level-150 Warden body HP. No fully matched fight worksheets are checked in. Simulator QA exports and test fixtures remain synthetic; they are not calibration samples. Component evidence does not prove whole-fight time, consumption or survival.

## Use the browser

1. Select the encounter and pack the exact setup used in the recording.
2. Open **Combat → Full-fight & consumption simulation**. Set the room-entry HP/Prayer, execution assumptions and loaded charges, then run it.
3. **Download gameplay worksheet**. Keep the exported numerical inputs unchanged.
4. Annotate actual attempts using the format below. Include deaths and escapes. Reserve later attempts as **holdout** before choosing settings.
5. Open **Compare with recorded gameplay** and choose the worksheet. Comparison runs locally; recordings are not fetched or uploaded. Evidence is session-only, outside saves and public setup links. Download the comparison to keep it.

Changing the setup or assumptions makes the comparison stale. A different worksheet setup is rejected. Descriptive model notes may differ; equipment/distributions, supplies, prayer, execution inputs and route must match. The worksheet's model revision must also match. There is no automatic “calibrated” badge or automatic application of suggestions.

## Recording rules

Use an unedited, normal-speed first fight with visible equipment, stats, boosts, prayers and starting/ending supplies. Count server ticks where possible; otherwise declare video timing uncertainty. Start at the first attackable tick and end at the killing hit, not the loot or death animation. Room-entry HP/Prayer and declared charge budgets must be known. Do not infer doses from net Prayer change or food from net HP change.

Use the same solo rules and setup as the worksheet. Set `setup_verified: false` if any are unknown. Add unsupported effects to `exclusions`; those attempts remain visible as skipped. Examples include special attacks, thralls, lifesteal, boost decay, combo eating, venom, potion-expiry gaps, speed-changing procs, teammate damage and invocation differences. Footage with unverified gear is a research lead, not a matched consumption baseline.

For repeated attempts, split training and holdout **before** fitting. The tool detects duplicate IDs and overlapping spans from the same reference, including common YouTube timestamp/share aliases. It cannot authenticate a recording, detect every URL alias, prove independence or remove selection bias. Use consecutive attempts from multiple sessions, not only uploaded successful kills. Do not relabel an inspected holdout as independent after tuning.

## Public-source audit · 13 November 2026

| Encounter | Public evidence reviewed | What remains to measure |
| --- | --- | --- |
| Vorkath | [Wiki fight overview](https://oldschool.runescape.wiki/w/Vorkath) and [strategy](https://oldschool.runescape.wiki/w/Vorkath/Strategies): six normal attacks between alternating specials, spawn immunity, acid reduction and damage/healing. [Theoatrix's own walkthrough](https://www.theoatrix.net/post/vorkath-guide-osrs) describes ranged and melee kills, utility casting and changing ammunition. Its text does not provide an exact tick/supply trace for a single matched setup. | Acid setup/end boundaries, spawn/cast/recovery timing, attack opportunities, prayer reactivation, ruby self-cost, net ammo loss and first-fight supply use. A ruby-to-diamond swap is outside the current single-weapon-window policy. |
| Normal Yama | [Wiki strategy](https://oldschool.runescape.wiki/w/Yama/Strategies), [Judge](https://oldschool.runescape.wiki/w/Judge_of_Yama) and [flares](https://oldschool.runescape.wiki/w/Void_Flare): two transitions/Judges, reactive prayer, guaranteed hits and final flares. These references also identify mechanics beyond the current scenario. | Stage-specific flare cadence, individual charge deadlines/regeneration, glyph visits, Judge travel/return, Mark recasts and final melee autos. Judge damage from lured fireballs, perfect-dodge haste and poisoned/burning flares need separate modeling; do not fit them away with uptime. |
| Wardens | [Wiki core](https://oldschool.runescape.wiki/w/Core), [Elidinis](https://oldschool.runescape.wiki/w/Elidinis%27_Warden) and [strategy](https://oldschool.runescape.wiki/w/Tombs_of_Amascut/Strategies): fivefold transfer and 21/29/37-tick exposure bands are source rules already tested separately. | Exact route/invocations, shield rotation across downs, core ejection/return boundaries, successful skull clearance/damage and enrage. Record only this room; preceding-raid supply loss is not Wardens consumption. |

No source text above supplies an independently annotated, matched full-fight dataset with both fitting attempts and untouched holdouts. No global timing, incoming-damage, food or kill-count defaults were tuned from those pages. Wiki search excerpts were available during this audit; direct retrieval of several full pages was blocked. The creator's [Vorkath video](https://www.youtube.com/watch?v=0Pf80fdOlv8&t=183s) was inspected around 3:03 and 3:24: those walkthrough frames crop out the inventory/stat HUD, so they cannot establish matched supply consumption. No quantitative tick measurements were taken from it.

The [public-recording follow-up](public-recordings.md) replaces title-only leads with inspected HUDs, declared conditions and specific observations. Its [evidence ledger](public-recordings.json) is not a browser-importable worksheet. The 150-invocation ToA source has Overclocked I/II and Deadly Prayers; its core/body observations can be used separately from incompatible P3/prayer comparisons. The inspected Yama source uses thralls and Judge self-damage. [Theoatrix's Yama explanation](https://www.theoatrix.net/post/osrs-yama-guide-mechanics-strategies-gear) also describes conditional flare specials and Judge self-damage. The current periodic flare interval remains an editable scenario proxy; matching its average is insufficient to validate the real conditional cycle.

## Worksheet format

The downloaded file contains `version`, `model_revision`, `inputs` and an empty `observations` array. Each observation has the fields below. **These numbers illustrate the schema only; they are not a recorded kill.** Do not import this example as evidence.

```json
{
  "id": "session-a-attempt-01",
  "split": "training",
  "evidence": {
    "kind": "video",
    "reference": "URL-or-local-recording-path",
    "recorded_on": "2026-11-13",
    "start_seconds": 12.0,
    "end_seconds": 84.0,
    "uncertainty_ticks": 2,
    "notes": "Unedited span; first attackable tick through killing hit."
  },
  "setup_verified": false,
  "exclusions": [],
  "outcome": "completed",
  "ticks": 120,
  "metrics": {},
  "timings": {},
  "exposed_ticks": null,
  "residual_damage": null,
  "eligible_opportunities": null,
  "used_opportunities": null
}
```

`split` is `training` or `holdout`. `kind` is `video` or `client_log`. `outcome` is `completed`, `died`, `teleported` or `timeout`. Give a distinct ID and exact source span per attempt. The annotator, not the tool, verifies the claimed source and date. Worksheet files are bounded to 4 MB and 256 attempts. Each attempt is at most 6,000 ticks, matching the simulation's one-hour safety limit. Longer clips require investigation rather than truncation into a fake complete attempt.

### Optional measurements

Omit unobserved keys. Explicit zero means **observed zero**. Unsupported keys and incomplete measurement pairs are rejected.

| `metrics` key | Unit / meaning |
| --- | --- |
| `attacks`, `casts`, `shots` | Count; shots fired includes recovered ammunition |
| `food_bites`, `prayer_doses` | Actual bites or doses consumed, not inventory slots |
| `prayer_points`, `damage_taken`, `ruby_damage` | Points/HP; damage taken includes self-cost and mechanics |
| `resource:9242` | Units of item 9242 lost/consumed, not all shots fired; any positive item ID is allowed |
| `charge:12899`, `scales` | Per-weapon charges or blowpipe scales consumed |
| `crumble`, `mark`, `acid`, `freeze`, `judges`, `flares`, `exploded`, `core_downs`, `skulls`, `enrage` | Cast/occurrence counts |
| `healing` | Boss HP healed |
| `phase:Acid` | Total observed ticks in the named phase; use the exact phase label from the scenario report |

The required `ticks` field supplies total duration; do not duplicate it under `metrics`. Phase totals may aggregate repeated occurrences. Do not use a single core's duration as total core time. Count consumed loaded charges separately from carried recharge materials. With unsupported resupply, passive healing or ammo switching, mark the attempt excluded.

Training observations can additionally provide:

- `timings`: named lists of **individual interval lengths in ticks**, not absolute timestamps or aggregate phase times. Vorkath accepts `acid_ticks`, `freeze_ticks`; Yama accepts `judge_travel`, `flare_interval`, `flare_charge`, `flare_regen`; Wardens accepts `prayer_rotation`, `skull_ticks`. Measure the same event boundaries as the scenario, annotate uncertainty, and investigate stage-dependent differences rather than pooling them blindly.
- `exposed_ticks` **and** `residual_damage`: time and HP lost in exposed attack phases, excluding ruby cost, glyphs, flare explosions, poison and failed mechanics. Suggested incoming HP/min = `100 × total residual HP / total exposed ticks`. Unknown damage attribution means both stay null.
- `eligible_opportunities` **and** `used_opportunities`: count complete attack slots with a reachable/vulnerable target while attack-ready, excluding eating, utility casts and travel. Count misses as attacks used. Suggested uptime = `100 × total used / total eligible`. Elapsed ticks are not attack opportunities.

Timing suggestions use the observed training median. Incoming and uptime suggestions weight exposure/opportunity counts; they do not average per-fight percentages. Suggestions show sample/run counts and ranges. Out-of-bounds values are reported, not clamped. No settings change automatically. Freeze duration is an execution-dependent observation, not a universal boss constant.

### Read the comparison

For each observed metric the report gives the model's 10th/50th/90th percentiles, median-minus-observation error and predictive interval coverage. Positive error means overprediction. Timing uncertainty widens only timing comparisons. Aggregate training and holdout results stay separate; mean absolute error uses the same units as the metric.

Failed and escaped attempts contribute to completion counts with a Wilson 95% interval, but their censored time/consumption does not enter completed-fight comparisons. A model with no completed simulations cannot produce completed-only comparisons. Low samples and success-selected footage cannot establish survival accuracy. Interval coverage is descriptive; it is not a confidence interval or a pass/fail threshold.

Suggestions do not re-fit and validate on the same evidence. After changing inputs, generate a new worksheet and collect **new prospective holdouts**. Keep the original file for history. Formula/timeline changes must increment `simulation::MODEL_REVISION`; old worksheets must be reviewed and explicitly regenerated rather than silently reused.

## Offline Rust commands

```sh
cargo run --locked --bin calibrate_fight -- template ironbound-fight-scenario.json > gameplay-worksheet.json
# Annotate observations from actual recordings in the worksheet, then:
cargo run --locked --bin calibrate_fight -- compare gameplay-worksheet.json > gameplay-comparison.json
```

The command never contacts RuneLite or any network service and recomputes predictions from numerical inputs rather than trusting exported trial results. Synthetic regression fixtures test arithmetic, bounds, split handling, censoring and setup matching. They do not close the gameplay-calibration backlog.
