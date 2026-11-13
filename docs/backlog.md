# Ironbound backlog

Work in priority order. A checked item has implementation and recorded verification;
browser, reference-calculator and gameplay evidence are recorded separately.

## 1. Preserve inventory edits

- [x] Keep manual slot choices and quantities when equipment or switch preferences change.
- [x] Retain valid items; report stock, capacity and required-switch conflicts instead of silently resetting the bag. Reset all edits explicitly, or restore one slot in its picker.
- [x] Regression checks cover gear changes, required switches and exported quantities.

## 2. Make save failures visible

- [x] Show persistent save status, including unavailable storage, failed writes and unreadable saved data.
- [x] Offer backup and retry; never label an unsuccessful save as saved. Preserve unreadable original data for download.
- [x] Verify the storage-blocked failure and recovery paths in the browser. The Rust QA server uses a real opaque origin; it does not mock Storage.

## 3. Make encounter coverage actionable

The detailed [monster mechanics backlog](monster-backlog.md) tracks each known
boss, raid, wave and attribute-specific gap, with separate outgoing and full-fight
acceptance criteria. Work through its shared outgoing rules before enabling
dedicated encounter simulations.

- [x] First monster batch: 20 reviewed outgoing NPC windows covering Zulrah,
  Kraken/cave kraken, Corp, Dagannoth Kings, KQ, Kree'arra, vulnerable Guardians,
  awake Duke and Sire vents/body. Hit caps, reductions, reach/immunity,
  per-hitsplat armour interactions and changing-HP ruby rules have independent
  regressions; full-fight follow-ups remain open in the detailed backlog.

- [x] Show which selected attack windows have supported outgoing calculations and which are packing only.
- [x] Explain remaining mechanics beside the encounter and in preparation/export notes. Count Wardens explicitly rather than omitting it from the supported fraction.
- [x] Vorkath: expose current-HP ruby bolts and an acid damage window with per-outcome rounding; verify finite-HP attack budgets with decreasing ruby procs.
- [x] Normal Yama: distinguish the 71-HP final shadow flare, boss HP stages and full-health flare pairs; retain the explicit paired Judge model.
- [x] ToA / Wardens: enable explicit obelisk, unprotected P2 shield, melee core and vulnerable P3 outgoing windows with pinned P2 observations, independent loadouts and core scaling checks.
- [x] Vorkath: add a complete seeded fight timeline with alternating six-attack special cycles, freeze/spawn immunity, acid damage/healing and explicit execution/consumption assumptions.
- [x] Normal Yama: carry boss HP through transitions and both Judges; simulate flare cadence, deadlines, regeneration, healing and final-phase workloads using editable execution assumptions.
- [x] ToA / Wardens: simulate the obelisk, shield rotations, repeated core exposure/transfer/downs, skull sets and enrage healing/Defence reset. Solo Wardens only; earlier raid rooms are excluded.
- [ ] Calibrate all three scenario timelines and incoming damage against actual encounter traces. Spatial pathing, mechanic-changing invocations/contracts and excluded effects are not gameplay-validated by deterministic tests. See [full-fight model assumptions](full-fight-simulation.md).
- [ ] Later coverage: Challenge/Hard/Expert scaling, full-wave encounters and other unsupported bosses. Catalog presence alone does not close a mechanics task.

## 4. Make trip assumptions editable

- [x] Replace the fixed 150-cast spell affordability/packing assumption with a bounded user choice used consistently by selection, packing, sharing, alternatives, switch comparisons and export. Rune stacks cover the worst-case mix of the total cast budget.
- [x] Keep the chosen budget in backups and named setups.
- [x] State that ammunition, charges and food/prayer counts are packing assumptions, not a kills-per-trip prediction.
- [x] Make the ammunition packing budget editable throughout selection, phase previews, carried-stock validation and both RuneLite export formats; keep 100 as the migration default. Increasing the budget resets the existing-loaded-darts confirmation.
- [x] Add changing-HP full-fight consumption scenarios, first-fight median/tail demand, packed-supply coverage and stock-conserving trip distributions. Preserve actual bag constraints; report shortages rather than silently adding supplies.
- [ ] Calibrate consumption against gameplay before presenting unqualified duration, survival or kill-count predictions.

## 5. Surface preparation unknowns

- [x] Let users explicitly confirm encounter access, spellbook/unlocks, charges/pages and trip supplies.
- [x] Reset confirmations when the underlying account, options or resulting packing list changes; preserve them in named setups/backups.
- [x] Keep unknowns visible without inventing quest or charge data from bank ownership. Confirmations never override real packing/level blockers and are excluded from public links.

## 6. Add named setups

- [x] Save, load, rename and remove multiple named configurations on this device (up to 64; replacing/removing requires confirmation).
- [x] Restore gear, inventory positions/quantities, trip assumptions and encounter settings against the current bank.
- [x] Include the library in private backups, exclude it from public links, and preserve existing saved accounts and independent comparison caps.
- [x] Regression checks cover migration, round trips, duplicate names and invalid entries.

## 7. Expand workflow verification

- [x] Add end-to-end model checks spanning owned bank → customized plan → named setup → public link / RuneLite export.
- [x] Run the full Rust suite, independent combat comparator, Wasm build and browser workflow checks for the initial workflow release (325 tests, 24 reference cases).
- [x] Verify merged main and GitHub Pages source revision for the initial workflow release: PR #23, source `a81fbe0147b11d18c2e0c5151f982e1bac685d00`. The succeeding live source `75c8940442ca036cb393b9a405097f0af71d16ec` includes it.
- [ ] Actual RuneLite Inventory Setups import, equipping and phase switches: requires a running client and human/gameplay verification; decoder checks do not close this item.

RuneLite verification blocker (13 November): RuneLite is installed but was not running. Computer access to launch it was denied. Actual client import/equip/switch checks remain open; do not replace them with model or decoder evidence.

## 8. Equipment mechanics

The [equipment mechanics backlog](equipment-backlog.md) tracks
incorrectly blocked rings, Barrows/Obsidian/Inquisitor integration, Moon sets,
Scythe/Soulreaper, defensive passives, enchanted bolts and special rotations.

## Release evidence

Full-fight follow-up: 463 integrated Rust tests, 13 focused fight/trip regressions, the 24 independent outgoing references and the release Wasm build pass. Disposable browser QA exercised all three encounter reports, both named Warden routes, positive completions and timeouts, actual carried-stock shortfalls, stale-result invalidation, reload persistence and report download. The final browser export replayed identical individual trials through the Rust offline command. Screenshot QA verified readable demand columns. Dedicated simulation profiles inside the shared Cargo target root prevent collisions with concurrent builds. These are implementation, formula and browser checks; actual gameplay calibration remains open. [Model scope and assumptions](full-fight-simulation.md).

Release integration: after the concurrent Moon-set and Wilderness changes, 489 integrated Rust tests and all 24 outgoing references pass. The 13 fight regressions also pass with resilient synthetic fixtures. Reports exported before the new burn metadata still replay identically; burn-enabled sets explicitly block simulation until their stack timing is modeled. The combined release Wasm build passes.

13 November 2026, workflow release (PR #23): all 325 integrated Rust tests and 24 independent combat references passed, along with the release Wasm build. Disposable browser QA verified named saving/loading/reload, a 73-rune manual slot surviving a gear change, cast-budget repacking, single-slot reset, four manual confirmations and invalidation, plus blocked-storage warning/retry and recovery without losing saved fixtures. The release is merged and deployed; later tooltip publications include it.

Ammunition follow-up (PR #26): the full suite and 24 combat references pass with separate verification profiles inside the shared Cargo target root. Five focused regressions cover small budgets, worn and internal ammunition, phase previews, legacy defaults, over-allocation and forged share budgets. Browser QA verified 37 bolts in the starting bag and phase preview, a 1,000-projectile shortage disabling export, and recovery after restoring 37. Merged and deployed source: `1775cd72e2d661c5b430820e9eac64d8c7e086ec`.

Encounter follow-up: all 381 integrated Rust tests and 24 reference cases pass, along with the release Wasm build. Eight focused regressions include 18 pinned Wardens P2 observations, two independently computed P2 loadouts, core HP scaling, explicit invulnerability/incoming gates, current-HP ruby/share round trips, finite-HP miss loops/overkill, acid rounding and Yama flare/stage workloads. Browser QA verified Vorkath's 750→250 HP change (ruby max 100→50 and a smaller remaining attack budget), acid damage, Yama's separate 71-HP final flare, blocked P2 melee, guaranteed P2 Magic/core melee and explicit phase limits. These are source/formula and browser checks; actual gameplay remains open. See [calculator validation](combat-validation.md) for sources and exact limits.

Reproduce browser storage failure with `rustc tools/browser-check.rs -o /tmp/ironbound-browser-check`, then run `/tmp/ironbound-browser-check dist PORT` using a verified-free port. Open `/?storage=blocked`; return to `/` to recover the ordinary origin. Stop the owned server afterward. Use a disposable preview origin; never seed or clear a real account for testing.
