# Ironbound

**[Open the planner →](https://nathansbradshaw.github.io/ironbound/)**

An OSRS-style bossing planner for Ironmen. Find gear **you actually own**, keep switches manageable, and compare damage, defence and prayer use.

## Start here

1. Open **Your bank** and import your RuneLite bank export.
2. Enter your levels, or look up your public skills.
3. Pick an encounter and set your **maximum switches**.
4. Click a gear or inventory slot to change items. Export with **Copy Inventory Setup**.

**Gear** is your packing workspace. **Combat** has the numbers; **Preparation** has trip checks. Optional gear policies stay in **Gear options**. Below the setup, **Save bag space** keeps DPS loss, damage taken and prayer drain allowances independent.

Your account stays on that browser. Import it again when using a different device or the hosted site instead of localhost.

**Starting gear + packing list** is what you export. Choose a boss phase to preview its equipped gear and inventory together; return to **Start** to edit the packing list. **Explore example** opens a temporary demo; your saved bank stays safe.

**Saved setups** keeps named configurations on this device. Save a learning setup, change your gear, then load it again; ownership and levels are rechecked against your current bank. Private backups include the library. Public links exclude setup names and personal confirmations.

**Gear → Adjust supplies** sets your spell budget (default 150 casts) and ammunition per weapon (default 100 projectiles). Selection, packing, phase previews and exports use those counts. Rune stacks cover the worst-case mix of selected attack spells. Food and restore counts remain editable; charges and actual trip length need your check.

**Preparation** shows encounter calculation coverage and lets you confirm access, unlocks, charges and supplies. These are your confirmations, not data inferred from the bank. Account or plan changes require checking again. Gear and switch edits keep manual inventory choices; packing conflicts block export until resolved. **Reset inventory edits** restores automatic packing.

If browser saving fails, a persistent warning offers a private backup and retry. Unreadable saved data is preserved for download rather than replaced by an example account.

## What you can do

- **Use Recommended:** reviewed ToA, Moons, Vorkath and Yama roles guide your owned gear. Every encounter compares complete owned Void/Elite Void sets with other armour for melee, ranged and magic, using supported calculations and your priorities. Selecting Vorkath also compares complete melee/ranged loadouts, including Salve. **Why this gear? → Compare owned styles** reruns that comparison. Custom weights live under **Options → Advanced**.
- **Yama:** packs melee/magic switches, spell runes, poison control and boosts. Combat shows Purging staff/Demonbane damage, Void Flare maximum hits and the alternating Judge cycle. Toggle **Mark of Darkness active** to match your play. Normal fight only; movement, specials and contract mechanics are outside the estimate.
- **Plan a trip:** boss and raid templates, or your own group of monsters. Select your CoX rooms or choose each Fractured Archive fork.
- **Compare switch limits:** choose any two caps in **Compare switches**. The encounter overview shows effective DPS, attack time, prayer used and drain alongside bag space. Expand phase details only when needed; apply either plan.
- **Pack fewer switches:** set **DPS loss**, **Extra damage taken** and **Extra prayer drain** independently (0–10% each; defaults 3%, 0%, 0%). Suggestions update automatically and the limits travel with setup links. A longer fight can use more total prayer even at the same drain rate. Unknown damage stays labelled.
- **Compare gear:** outgoing DPS, supported incoming damage, defence and prayer drain.
- **Compare armour sets:** Void, crystal bow/Bowfa, Barrows and Obsidian combinations compete as full loadouts. Set Dharok's current Hitpoints in **Combat → Boosts, darts & debuffs**; blank means full health. [Supported bonuses and limits](docs/armour-sets.md).
- **Slayer task:** choose **On task / Off task** beside the encounter controls for each eligible target. Recommendations and comparisons include helmet/mask bonuses; Combat explains whether your gear receives them. Ranged/Magic require an imbued helmet or mask, and Salve bonuses do not stack with Slayer.
- **Skip Void:** turn off **Use Void armour** beside Gear approach to compare your other gear. This excludes normal/Elite Void and cosmetic variants while keeping your Slayer task settings. Turn it back on to compare Void again; the preference is included in saved setups and setup links.
- **Check the maths:** [live Wiki comparisons and update guide](docs/combat-validation.md). Run `cargo run --locked --bin validate_combat` to check the pinned reference loadouts.
- **Model your fight:** Combat → **Boosts, darts & debuffs**. Choose active boosts, loaded blowpipe darts and successful reductions for each target.
- **Quest bosses:** search Drakan, Seren, Galvek, Sliske or a quest title. Cards label each quest and rate your owned gear using its ordinary attack rules. Choose **Quest bosses** to see them, **Bosses** for farmable bosses, or **Gear & requirements → Hide quest bosses** to exclude them from any list. [Damage rules and rating limits](docs/quest-bosses.md).
- **See weaknesses:** Combat breaks ranged defence into Light / Standard / Heavy and magic into Air / Water / Earth / Fire. Compare supported owned spells, hit chance, max hit and DPS; the selected spell shows its base and elemental bonuses.
- **Try spells manually:** Combat → Magic. Click an element and a spell in the icon grid, or choose **Auto**. Your selection is saved and used for the Magic plan, rune packing and exports. Missing runes are flagged; **Auto** restores automatic choice.
- **Choose ranged ammunition:** Combat → Ranged → **Light / Standard / Heavy**. The plan chooses a compatible owned weapon and ammo using your shot budget. **Auto** restores automatic choice; missing gear or ammunition is flagged.
- **Choose prayers:** click icons in the **Prayer book** to toggle protection, a stat prayer and Preserve. Grey icons show unmet levels. Ranged stance uses **Rapid / Accurate** buttons.
- **Choose your switch limit:** this is a maximum. See how many changes the plan actually uses. Equal combat scores prefer lower prayer use, then fewer packed items. Numeric controls update while typing or using their arrows.
- **Why this item?** Hover or keyboard-focus gear for its role and up to two owned alternatives with signed trade-offs. Supplies show their packing purpose.
- **Pack your inventory:** click any slot, search owned items by type, and keep your chosen positions when exporting.
- **Compare supplies:** food by potential healing per slot; potions by restoration or boost within each role, with fuller bottles first on ties.
- **Plan longer trips:** weigh prayer efficiency against damage and see a prayer-supply estimate.
- **Track upgrades:** compare owned equipment with reviewed unowned options.
- **Take it into RuneLite:** export Inventory Setups or Bank Tags, or share a setup link.

## What is still incomplete?

The prioritized [development backlog](docs/backlog.md) records completed work, remaining mechanics and verification evidence.

**The catalog covers all 72 current hiscores bosses/modes. Their combat mechanics are not all fully supported.**

Coverage notes distinguish packing templates from supported calculations. CoX supports selecting rooms while keeping all Olm phases together. The Fractured Archive supports five-boss routes: Archive Guardian, one room at each of three forks, and Ocellus. Its editable starting styles and partial phase coverage are packing templates; outgoing DPS remains withheld for every Archive boss. ToB and ToA also have raid templates. Challenge/Hard/Expert scaling, full wave simulations and many special phases remain unfinished.

Moons includes defence-aware normal attacks, chained hits, flat armour and Blood Moon healing. Its curses, special phases, camp boosts and movement are not fully simulated. Unknown damage mechanics show **unavailable**, rather than zero.

Zulrah, emerged Kraken/cave kraken, Corporeal Beast, Dagannoth Kings, Kalphite
Queen, Kree'arra and vulnerable Grotesque Guardians have reviewed outgoing attack
windows. Their notes explain hit caps, reductions, reach and immunity rules.
Awake Duke uses reduced Demonbane effectiveness; exposed Sire vents use their
minimum-hit and successful Demonbane finisher rules, with separate body stages.
Incoming damage and full-fight mechanics remain partial; see the
[monster backlog](docs/monster-backlog.md).

Hydra comparisons distinguish neutralised forms from the 75% reduction before
the correct chemical. Muspah supports unshielded forms; its prayer shield remains
unverified. Vardorvis uses current-HP stats and recalculates its attack budget as
HP falls. KQ and Dagannoth Kings offer separate reviewed incoming attack scenarios.

Skotizo offers zero through four active-altar comparisons and the named sword
finishers. Hespori separates flower protection from its vulnerable body and shows
the twelve-flower attack budget; quest flowers keep their own coverage gate.

Leviathan and Whisperer have separate reviewed attack states; unsupported
pathfinder/enrage mechanics stay labelled. Nightmare/Phosani distinguish shield,
totem charge and physical HP, with separate encounter counts and Phosani minion
rules. These comparisons do not predict complete fights or supplies.

Hueycoatl separates body, head, protected head and capped tail, with optional
lit-pillar comparisons. Royal Titans separate close/withdrawn attacks and elemental
adds; their gear screen needs melee, ranged and Magic. These are outgoing windows;
team contribution, barriers, incoming sequences and full fights remain unverified.

Recommendations use a bounded search. They are **not a proof of global BIS** or a prediction of successful kills.

## Your bank stays private

Bank imports are processed and saved locally in your browser. They are never uploaded. Public skill lookup sends only your requested RuneScape name to Wise Old Man.

Share links contain the encounter, carried gear, quantities, switches and combat preferences. They exclude your bank, account name and levels. **Private account backups do contain account data—keep those private.**

<details>
<summary><strong>Calculation details</strong></summary>

[Recent equipment coverage](docs/recent-equipment.md) includes Twinflame, Eye of
Ayak, Zorya's tome, the dragon hunter wand, Scorching bow and newer melee weapons.
Charged Dizana quiver bonuses require compatible arrows/bolts. Oathplate, Torva,
Masori, Hueycoatl hide, Sunfire fanatic and recent jewellery use reviewed wear
requirements. Special attacks and the remaining equipment gaps stay labelled.

[Obligator and ammunition](docs/obligator-ascension.md) adds charged Thrust's
damage selection and size bonus, all three Obligator stances, Ascension crossbows
with their own bolt/proc rules, and Seeking arrow minimum hits. Choose the
Obligator stance in **Combat → Boosts, darts & debuffs**. Charged Thrust with
Inquisitor or encounter minimum-hit bounds remains explicitly unverified.

Weapon IDs map to shared families and reusable mechanics. Exact variants stay separate: owning a basic Keris partisan does not unlock any upgrade. Families supply attack types, stances, ammunition, requirements, rune recipes and effects to both the planner and calculator.

Defaults use unboosted levels. Selected boosts use peak current levels; wear requirements still use base levels. Supported prayers affect combat and drain. Reviewed outgoing calculations include melee, bows/crossbows, loaded blowpipes, powered staves, all 20 elemental Strike/Bolt/Blast/Wave/Surge spells and Iban Blast, with supported Fang, Keris, bolt and Shadow effects. Elemental damage tiers use base Magic; boosted levels unlock casts and affect accuracy. Only matching elements gain weakness bonuses. Charged Fire/Water/Earth tomes have their supported bonuses and rune supply; check pages manually. ToA assumes solo, path level 0 and invocation levels 0–150. Combat offers explicit Wardens obelisk, unprotected P2 shield, melee core and vulnerable P3 comparisons; the complete Wardens cycle remains unverified.

**Combat → Attack window & supply budget** lets you set current target HP and inspect finite-HP attack expectations, including misses, overkill and changing ruby-bolt procs. Vorkath has an optional acid attack window; normal Yama has a separate 71-HP final flare. These estimates exclude movement, specials, regeneration and supply recovery. Packing budgets and the overview's HP/DPS comparison are separate assumptions, not kills-per-trip predictions.

**Combat → Full-fight & consumption simulation** runs complete Vorkath, normal Yama and Wardens scenarios with editable execution assumptions. Compare median/tail demand and the fights covered by your carried supplies; download a replayable report. [Model assumptions and verification](docs/full-fight-simulation.md) explain the scenario inputs, excluded effects and outstanding gameplay calibration.

**Compare with recorded gameplay** downloads a setup-matched worksheet and compares your annotated recordings locally, with separate training/holdout errors and measured-input suggestions. [Recording guide](docs/gameplay-calibration.md). Actual gameplay calibration remains open for all three; public guide text and synthetic tests do not validate consumption or survival.

Blowpipes preload your chosen ammunition budget from owned compatible darts, or you can explicitly confirm at least that many darts are already loaded. Changing the budget requires confirming the contents again. These occupy no bag or worn ammo slot. Projectile recovery, charges and actual consumption need your check. Boost decay, potion timing, venom and special-attack damage are outside these estimates.

Scythe uses target-size hits. Soulreaper uses **Boosts, darts & debuffs → Soulreaper held stacks**. Initial Moon support includes Dual macuahuitl, expected Blood Moon timing, Atlatl Strength-based damage and Blue Moon elemental casts. Full Eclipse burns and Blue Moon follow-ups remain unavailable. [Equipment coverage and remaining work](docs/equipment-backlog.md).

Incoming values describe an explicitly selected attack scenario, not a full-fight average. Elysian, full Justiciar, settled Dinh's Block and Rondache reductions cover ordinary unprotected hits. Their interactions with chains, boss healing, prayer chip and specials remain unavailable. **Dinh's Block active** prevents attacks. Automatic defence priority uses your selected defence bonuses; incoming scenarios remain separate estimates.

Prayer duration assumes full starting prayer, continuous use and carried ordinary prayer/restore doses. It excludes flicking, regeneration, restoration modifiers and encounter-imposed drains. It is not a whole-trip or kills-per-trip prediction.

Recommended starts from owned, level-usable encounter gear. It compares supported attack windows and keeps your switch, stock and inventory limits. At Moons it includes normal chained attacks and Blood healing; special phases and full raid survival remain partial. Custom weights are optional. Monster groups support up to 64 attack windows.

No quest checks are used. Equipment and spell skill requirements still apply. Reference tiers are reviewed examples, not an exhaustive universal BIS list.

Inventory Setups preserves equipment and inventory positions/quantities; Bank Tags provides an item filter. Export JSON has been checked against the upstream portable decoder; an in-game import has not been verified.

</details>

<details>
<summary><strong>Build, test and publish</strong></summary>

Requires Rust, the `wasm32-unknown-unknown` target, wasm-bindgen-cli **0.2.127**, and `gzip`.

```sh
cargo test --locked -j 2
cargo run --locked --bin site -- build
```

When another checkout is building into the same Cargo cache, use the reusable verification profiles to keep package outputs separate:

```sh
cargo test --locked --profile verify-dev -j 2
cargo run --locked --profile verify-dev --bin site -- build verify-release
```

Both profiles use the existing target root; no per-check target directory is needed. Keep the same chosen profile through verification and release packaging.

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

**Boss bestiary:** open **Change encounter** below your encounter name. **For you** puts well-equipped fights first; filter by **Well equipped**, **Doable** or **A struggle** to find your next goal. Cards include real boss artwork, recorded target stats and the owned setup behind the rating. Choose **Not assessed**, then expand **Details & readiness** to see which phase, mode or equipment model is still missing. Ratings screen gear and levels; a good rating does not verify access or predict a kill.

Readiness now includes Brutus, Mimic, Skotizo, Corporeal Beast, Prime/Supreme and the full Dagannoth Kings group, Jad/Fight Caves, base ToA and hybrid Yama. Mixed-style encounters use the weakest required setup. ToA uses a solo raid-level-0 baseline; Yama screens melee on the boss plus melee/magic for the Judge and magic for flares. Wave endurance, fast flare clearing, full supplies and player execution need separate checks. Lunar Chests shares the Moons of Peril guideline.

**Gear and requirements are independent.** Each card shows **Gear: Well equipped / Doable / A struggle / Not assessed**, plus a separate access line. Kraken can show good gear while also showing **Requirements missing — Needs 87 Slayer**. Gear filters include locked bosses; **Requirements missing** finds known entry locks without hiding their gear scores. Meeting a Slayer level changes the access line, not the gear score. Unknown Slayer levels leave access unverified while supported gear calculations still run. **Slayer level met** checks that level only; quest access and current assignments remain unverified.

Open **Gear & requirements** to combine filters, for example **Well equipped** + **Known levels met**. Changing one keeps the other, search, category and Wilderness choices. **Known levels met** checks recorded Slayer requirements (including reviewed profiles with no Slayer gate); it does not verify quests, keys or tasks. Unreviewed or unknown level requirements stay separate. **Clear filters** resets both groups.

**Nervous about the Wilderness?** In **Boss bestiary**, choose **Avoid Wilderness** or **Wilderness only**. **Scaredy cat · 3 valuable items** rerates exposed bosses with three valuable pieces plus owned cheap gear. Choose a filler ceiling (default 10k GP per piece/ammo stack), expand a card to see the exact kit and full-kit comparison, then select the boss to keep that kit in the planner. **Return to full gear** removes the restriction. Price estimates use the displayed public snapshot; missing values consume a valuable slot. Confirm **Items Kept on Death** in game while unskulled with **PK Skull Prevention** on. Supplies, charges, fees and loot add risk.

**Kept items & death risk** appears after selecting a Wilderness boss or scaredy-cat kit. It uses the actual packed inventory, worn ammo and declared contents, with a GP breakdown for gear, supplies, runes, ammunition and charges. Protection ordering uses separate official-GE/reviewed death values; it does not reuse the cheap-gear threshold. Choose your skull/Protect Item scenario. For unreviewed values or ties, open **Complete risk details** and select the items shown as kept in-game, then enter missing base losses or remaining contents. **Known subtotal · incomplete** stays visible until all required values are resolved. Charge counts are actual starting contents, not simulator budgets. Declarations save privately, expire when the packed pool changes, and are never inherited by recipients of shared setups. Add entry fees, loot and unrepresented bag/pouch contents under **Other GP at risk**. This is estimated replacement/repair loss for standard-account Wilderness PvP, not a promise of protection or the PKer's loot value; UIM, Deadman and Trouver/grave mechanics remain outside its scope.

Barrows incoming comparisons now distinguish Dharok's NPC health from your own
armour-set health setting. In **Combat → Attack window & supply budget**, lower
**Current target HP** to update his incoming damage. Verac keeps a separate
protection-bypass branch; Protect from Melee reduces it but does not remove it.
These are selected attacks, excluding prayer drain, tunnels and complete trips.
