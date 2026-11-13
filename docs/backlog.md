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

- [x] Show which selected attack windows have supported outgoing calculations and which are packing only.
- [x] Explain remaining mechanics beside the encounter and in preparation/export notes. Count Wardens explicitly rather than omitting it from the supported fraction.
- [ ] Vorkath: validate changing-HP bolt effects and special-phase timing against independent references before offering encounter-duration/consumption estimates.
- [ ] Normal Yama: validate movement/transition timing, flare counts and incoming/special mechanics; retain the existing explicit paired Judge model.
- [ ] ToA / Wardens: research each phase, defence and damage rules, scaling and invocation limits; add independent fixtures before enabling each supported calculation.
- [ ] Later coverage: Challenge/Hard/Expert scaling, full-wave encounters and other unsupported bosses. Catalog presence alone does not close a mechanics task.

## 4. Make trip assumptions editable

- [x] Replace the fixed 150-cast spell affordability/packing assumption with a bounded user choice used consistently by selection, packing, sharing, alternatives, switch comparisons and export. Rune stacks cover the worst-case mix of the total cast budget.
- [x] Keep the chosen budget in backups and named setups.
- [x] State that ammunition, charges and food/prayer counts are packing assumptions, not a kills-per-trip prediction.
- [ ] Make the ammunition packing budget editable throughout selection, phase previews, carried-stock validation and both RuneLite export formats; keep 100 as the migration default.
- [ ] Follow-up: independently verified consumption and changing-HP encounter simulation before offering automatic trip-duration or kill-count estimates.

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
- [ ] Run the full Rust suite, independent combat comparator, Wasm build and browser workflow checks.
- [ ] Verify merged main and GitHub Pages source revision.
- [ ] Actual RuneLite Inventory Setups import, equipping and phase switches: requires a running client and human/gameplay verification; decoder checks do not close this item.

## Release evidence

13 November 2026: eight new workflow regressions pass; the pre-integration full suite passed 311 tests, and all 24 independent combat reference cases passed. Browser QA on a disposable origin verified named saving/loading/reload, a 73-rune manual slot surviving a gear change, cast-budget repacking, four manual confirmations and invalidation, plus blocked-storage warning/retry and recovery without loss of the saved fixtures. Integration with concurrent switch-comparison and encounter-summary changes is being verified before publication.

Reproduce browser storage failure with `rustc tools/browser-check.rs -o /tmp/ironbound-browser-check`, then run `/tmp/ironbound-browser-check dist PORT` using a verified-free port. Open `/?storage=blocked`; return to `/` to recover the ordinary origin. Stop the owned server afterward. Use a disposable preview origin; never seed or clear a real account for testing.
