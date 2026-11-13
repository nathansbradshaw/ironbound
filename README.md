# Ironbound

**[Open the planner →](https://nathansbradshaw.github.io/ironbound/)**

An OSRS-style bossing planner for Ironmen. Find gear **you actually own**, keep switches manageable, and compare damage, defence and prayer use.

## Start here

1. Open **Your account** and import your RuneLite bank export.
2. Enter your levels, or look up your public skills.
3. Pick an encounter and set your **maximum switches**.
4. Click a gear slot to compare alternatives. Export with **Copy Inventory Setup**.

Your account stays on that browser. Import it again when using a different device or the hosted site instead of localhost.

**Starting gear + packing list** is what you export. Choose a boss phase to preview its equipped gear and inventory together; return to **Start** to edit the packing list. **Explore example** opens a temporary demo; your saved bank stays safe.

## What you can do

- **Use Recommended:** reviewed ToA, Moons, Vorkath and Yama roles guide your owned gear. Selecting Vorkath compares complete melee/ranged loadouts, including Salve and full Void sets. **Why this gear? → Compare owned styles** reruns that comparison. Custom weights live under **Options → Advanced**.
- **Yama:** packs melee/magic switches, spell runes, poison control and boosts. Combat shows Purging staff/Demonbane damage, Void Flare maximum hits and the alternating Judge cycle. Toggle **Mark of Darkness active** to match your play. Normal fight only; movement, specials and contract mechanics are outside the estimate.
- **Plan a trip:** boss and raid templates, or your own group of monsters. Select your CoX rooms or choose each Fractured Archive fork.
- **Switch less:** limit changed items and carried gear; favour one style or phase.
- **Compare gear:** outgoing DPS, supported incoming damage, defence and prayer drain.
- **Check the maths:** [live Wiki comparisons and update guide](docs/combat-validation.md). Run `cargo run --locked --bin validate_combat` to check the pinned reference loadouts.
- **Model your fight:** Combat → **Boosts, darts & debuffs**. Choose active boosts, loaded blowpipe darts and successful reductions for each target. Numeric settings, including switch limits, update after a short typing pause.
- **Pack your inventory:** click any slot, search owned items by type, and keep your chosen positions when exporting.
- **Compare supplies:** food by potential healing per slot; potions by restoration or boost within each role, with fuller bottles first on ties.
- **Plan longer trips:** weigh prayer efficiency against damage and see a prayer-supply estimate.
- **Track upgrades:** compare owned equipment with reviewed unowned options.
- **Take it into RuneLite:** export Inventory Setups or Bank Tags, or share a setup link.

## What is still incomplete?

**The catalog covers all 72 current hiscores bosses/modes. Their combat mechanics are not all fully supported.**

Coverage notes distinguish packing templates from supported calculations. CoX supports selecting rooms while keeping all Olm phases together. The Fractured Archive supports five-boss routes: Archive Guardian, one room at each of three forks, and Ocellus. Its editable starting styles and partial phase coverage are packing templates; outgoing DPS remains withheld for every Archive boss. ToB and ToA also have raid templates. Challenge/Hard/Expert scaling, full wave simulations and many special phases remain unfinished.

Moons includes defence-aware normal attacks, chained hits, flat armour and Blood Moon healing. Its curses, special phases, camp boosts and movement are not fully simulated. Unknown damage mechanics show **unavailable**, rather than zero.

Recommendations use a bounded search. They are **not a proof of global BIS** or a prediction of successful kills.

## Your bank stays private

Bank imports are processed and saved locally in your browser. They are never uploaded. Public skill lookup sends only your requested RuneScape name to Wise Old Man.

Share links contain the encounter, carried gear, quantities, switches and combat preferences. They exclude your bank, account name and levels. **Private account backups do contain account data—keep those private.**

<details>
<summary><strong>Calculation details</strong></summary>

Weapon IDs map to shared families and reusable mechanics. Exact variants stay separate: owning a basic Keris partisan does not unlock any upgrade. Families supply attack types, stances, ammunition, requirements, rune recipes and effects to both the planner and calculator.

Defaults use unboosted levels. Selected boosts use peak current levels; wear requirements still use base levels. Supported prayers affect combat and drain. Reviewed outgoing calculations include melee, bows/crossbows, loaded blowpipes, powered staves, Wind spells and Iban Blast, with supported Fang, Keris, bolt and Shadow effects. ToA assumes solo, path level 0 and invocation levels 0–150; Wardens remain unsupported.

Blowpipes preload 100 owned compatible darts, or you can explicitly confirm contents already loaded. These occupy no bag or worn ammo slot. Check charges and trip supply yourself. Boost decay, potion timing, venom and special-attack damage are outside these estimates.

Incoming values describe an explicitly selected attack scenario, not a full-fight average. Unknown prayer interactions, special attacks, chip damage and defensive passives remain unverified. Automatic defence priority uses your selected defence bonuses; incoming scenarios remain separate estimates.

Prayer duration assumes full starting prayer, continuous use and carried ordinary prayer/restore doses. It excludes flicking, regeneration, restoration modifiers and encounter-imposed drains. It is not a whole-trip or kills-per-trip prediction.

Recommended starts from owned, level-usable encounter gear. It compares supported attack windows and keeps your switch, stock and inventory limits. At Moons it includes normal chained attacks and Blood healing; special phases and full raid survival remain partial. Custom weights are optional. Monster groups support up to 64 attack windows.

No quest checks are used. Equipment and spell skill requirements still apply. Reference tiers are reviewed examples, not an exhaustive universal BIS list.

Inventory Setups preserves equipment and inventory positions/quantities; Bank Tags provides an item filter. Export JSON has been checked against the upstream portable decoder; an in-game import has not been verified.

</details>

<details>
<summary><strong>Build, test and publish</strong></summary>

Requires Rust, the `wasm32-unknown-unknown` target and wasm-bindgen-cli **0.2.127**.

```sh
cargo test --locked -j 2
cargo run --locked --bin site -- build
```

Output: `dist/`. To preview locally, check port ownership first, then:

```sh
cargo run --locked --bin site -- serve 3040
```

Refresh the public icon catalog with `cargo run --locked --bin site -- assets-inventory`. Rebuild food profiles from the public RuneLite files linked in [SOURCES.md](SOURCES.md) with `cargo run --locked --bin consumables -- ItemStatChanges.java ItemID.java`.

Stop a task-owned server after testing, unless it is being kept open for the user.

Private native testing uses ignored `.local/` storage and makes no bank upload:

```sh
cargo run --locked --bin site -- private-check /path/to/bank.tsv /path/to/hiscores.json
```

Publish from a clean, verified `main` (or a detached revision exactly matching remote `main`), after building that same revision:

```sh
cargo run --locked --bin site -- publish-pages
```

Source lives on `main`; only generated public static files go to `gh-pages`. The publisher preserves branch history, checks the remote, writes a source revision marker and removes its temporary staging directory. GitHub Pages serves that branch's root. Rust builds run locally.

Never commit private banks, account snapshots or screenshots. `.local/`, `target/` and `dist/` are ignored.

</details>

## Credits

[Data sources and attribution](SOURCES.md). Game artwork © Jagex. Independent fan project, not affiliated with Jagex or RuneLite. App logic, calculations and tools are Rust/WebAssembly.
