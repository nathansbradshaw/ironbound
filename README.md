# Ironbound

A Rust/WebAssembly static **Ironman bossing planner**, styled as a classic 2007 OSRS fan site. No login or application server.

1. Open **Your account** and import your RuneLite tab-separated bank export.
2. Enter combat levels, or request a public Wise Old Man snapshot using your RuneScape name.
3. Choose **Barrows**, **Tombs of Amascut**, or build a group of up to 12 bosses/monsters.
4. Set a maximum number of changed items per switch and an inventory budget. Balance the encounter or keep one style’s full set.
5. Click an equipment slot to search all owned items in that slot, ordered by DPS or defence. Its upgrade scale also shows reviewed unowned options; track one as your bossing goal. Hover a slot to see its name and equipped item.
6. Export to RuneLite **Inventory Setups** or **Bank Tags**, or copy a setup link.

Quest checks and quest-confirmation controls are intentionally omitted. Gear eligibility uses owned items and skill levels. Only boss-specific preparation and upgrades are tracked. GroupIron integration has been removed.

## Private bank, shareable setup

Bank files are read in the browser and saved in local browser storage. They are never uploaded. Public skill lookup sends only the requested RuneScape name to Wise Old Man; its cached snapshot may be stale. Every visitor downloads the same public icon pack, so displaying owned items does not make bank-derived image requests. Unbundled items use a placeholder.

The URL saves the encounter, raid level, custom monster/style selection, equipped items, inventory quantities, carried switches and comfort constraints. They exclude the bank, account name, levels, quests, goals and credentials. The URL fragment is not sent to the static host. Opening someone else’s link shows its exact equipment without replacing the recipient’s bank. Reloading your own saved URL keeps the editor connected to your local account. **Rebuild using my items** returns to local recommendations.

Private backups contain account data and should not be shared publicly. `.local/`, `target/` and `dist/` are Git-ignored. Keep all real test banks under ignored local storage; never add them as fixtures or static assets.

## Combat calculations and equipment tiers

The Rust engine computes standard sustained attack DPS, accuracy, max hit, attack speed and equipment bonuses. It covers reviewed melee weapons, bows/crossbows, powered staves, Iban Blast, and Wind Blast/Wave, including air weakness, accurate-zero handling, Fang accuracy/damage, Keris kalphite procs, ruby/diamond bolts, Sanguinesti staff, and Tumeken’s shadow. ToA boss rooms use the selected invocation level, solo HP scaling and path level 0. Warden-specific damage models remain unsupported.

Defaults are unboosted with no offensive prayer or special attack. Melee uses accurate, ranged rapid, powered staves accurate and standard spells autocast. Bolt calculations use full target HP and no Kandarin diary bonus. These are uninterrupted attack-window estimates, not whole-encounter DPS or predicted kill times. Prayer, potion boosts, defence reductions, incoming damage, movement and eating are not simulated. Some monster mechanics and equipment effects remain unsupported; those combinations return an explanation instead of an invented result.

Armour and accessories use the full equipment catalog, with skill requirements and supported DPS comparisons. Default sets start from item stats and are refined by bounded single-slot DPS improvements. Slot pickers show all owned items for that slot, including items whose combat behaviours are not modelled. The reference scale contains reviewed main-game examples rather than an exhaustive universal BIS list.

Ownership uses exact item IDs, including weapon variants. A plain Keris partisan does not unlock breaching, sun or corruption variants. The weapon picker compares their supported passive damage and marks missing variants as unowned; conditional effects and specials are described separately. Boss preparation guidelines are advisory, while equipment and spell skill requirements are enforced.

The switch-budget search checks every pair of style sets, uses one shared carried pool, explores hybrid pieces, and reports per-target DPS against its unrestricted starting sets. Balanced mode minimizes combined HP/DPS across supported attack windows; individual phase weights let players prioritize particular fights. A focus mode keeps one style’s set intact. This bounded search retains a small candidate frontier; it **does not prove global BIS**. Supplies, tools and optional spec weapons occupy additional slots and the full packing list must still fit 28. Warden DPS, movement and supply consumption are excluded from the objective.

Inventory Setups preserves equipment, inventory positions and quantities. Bank Tags v1 CSV provides an item filter and does not preserve quantities. The Inventory Setups JSON was checked against the upstream Java portable decoder; an in-game import has not been verified.

## Build and test

Requires Rust with `wasm32-unknown-unknown` and wasm-bindgen-cli **0.2.127**.

```sh
cargo test --locked -j 2
cargo run --locked --bin site -- build
```

The static output is `dist/`. For a local preview, inspect port ownership and run `cargo run --bin site -- serve PORT` on an unoccupied port. Stop the owned server after testing.

Native private testing (no network request):

```sh
cargo run --locked --bin site -- private-check /path/to/bank.tsv /path/to/hiscores.json
```

This prints aggregate coverage only and saves its private snapshot under ignored `.local/`.

## GitHub Pages

The configured destination is GitHub Pages. Source belongs on `main`; generated static assets belong on `gh-pages`. When publication is authorized, verify locally, commit and merge, then:

```sh
cargo run --locked --bin site -- publish-pages
```

The Rust publisher requires a clean `main`, pushes without force, stages only `dist/`, preserves release-branch history, writes `source-commit.txt`, and removes its temporary staging directory. GitHub Pages must be enabled for the release branch. No Rust builds run in Actions.

## Attribution

See [SOURCES.md](SOURCES.md). OSRS game artwork © Jagex. This is an independent fan project. Application logic, combat calculations, import/export and build tools are Rust; JavaScript is limited to the browser module loader and wasm-bindgen-generated bindings.
