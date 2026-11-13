# Ironbound backlog

Work in priority order. A checked item has implementation and recorded verification;
browser, reference-calculator and gameplay evidence are recorded separately.

## 1. Preserve inventory edits

- [ ] Keep manual slot choices and quantities when equipment or switch preferences change.
- [ ] Retain valid items; report stock, capacity and required-switch conflicts instead of silently resetting the bag.
- [ ] Regression checks cover gear changes, required switches and exported quantities.

## 2. Make save failures visible

- [ ] Show persistent save status, including unavailable storage, failed writes and unreadable saved data.
- [ ] Offer backup and retry; never label an unsuccessful save as saved.
- [ ] Verify the failure and recovery paths in the browser.

## 3. Make encounter coverage actionable

- [ ] Show which selected attack windows have supported outgoing calculations and which are packing only.
- [ ] Explain remaining mechanics beside the encounter and in preparation/export notes.
- [ ] Track deeper encounter work explicitly: Wardens, raid scaling, full-wave encounters and unsupported bosses need independent mechanics research and reference fixtures before enabling calculations.

## 4. Make trip assumptions editable

- [ ] Replace the fixed 150-cast spell affordability/packing assumption with a bounded user choice used consistently by selection, packing, sharing and export.
- [ ] Keep the chosen budget in backups and named setups.
- [ ] State that ammunition, charges and food/prayer counts are packing assumptions, not a kills-per-trip prediction.
- [ ] Follow-up: independently verified consumption and changing-HP encounter simulation before offering automatic trip-duration or kill-count estimates.

## 5. Surface preparation unknowns

- [ ] Let users explicitly confirm encounter access, spellbook/unlocks, charges/pages and trip supplies.
- [ ] Reset confirmations when the underlying plan changes; preserve them in named setups/backups.
- [ ] Keep unknowns visible without inventing quest or charge data from bank ownership.

## 6. Add named setups

- [ ] Save, load, rename and remove multiple named configurations on this device.
- [ ] Restore gear, inventory positions/quantities, trip assumptions and encounter settings against the current bank.
- [ ] Include the library in private backups, exclude it from public links, and preserve existing saved accounts.
- [ ] Regression checks cover migration, round trips, duplicate names and invalid entries.

## 7. Expand workflow verification

- [ ] Add end-to-end model checks spanning owned bank → customized plan → named setup → public link / RuneLite export.
- [ ] Run the full Rust suite, independent combat comparator, Wasm build and browser workflow checks.
- [ ] Verify merged main and GitHub Pages source revision.
- [ ] Actual RuneLite Inventory Setups import, equipping and phase switches: requires a running client and human/gameplay verification; decoder checks do not close this item.

## Release evidence

Pending implementation.
