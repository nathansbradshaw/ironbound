# Ironbound

An OSRS-style static Ironman planner, implemented in Rust and WebAssembly. Start with a sample account or import your own GroupIron snapshot, choose **Barrows** or **Tombs of Amascut**, review preparation and supplies, and copy a RuneLite Inventory Setups export.

## Bring your account

1. Open **Your account** in the site.
2. Import a GroupIron member-array JSON response or an Ironbound backup.
3. Choose your character. Review skills, quest access and freshness.
4. Choose an encounter, adjust supplies, and review the packing list.
5. Use **Copy Inventory Setup**. In RuneLite Inventory Setups choose **Import setup**, paste and confirm. Enable Bank Tags for filtering/layouts.

### GroupIron connection status

The browser connector uses only `GET https://groupiron.men/api/group/{group_name}/get-group-data?from_time=1970-01-01T00%3A00%3A00.000Z`, with the group token in the Authorization header. Tokens are neither stored nor exported. HTTP redirects are rejected.

On 9 November 2026 the deployed GroupIron API's preflight did **not** return `Access-Control-Allow-Origin` for this site's origin. Direct browser connection is therefore blocked by the remote service's CORS configuration. The browser connector is available for retry after upstream changes, with an explicit notice. No proxy/backend is silently introduced.

Use the Rust snapshot importer instead:

```sh
cargo run --locked --bin site -- import-group 'Your Group Name'
```

Enter the group token at the hidden terminal prompt. The importer requests the same authenticated read endpoint and writes a sanitized snapshot to `.local/groupiron.json`. Upload that file in **Your account**. The header is sent through stdin to curl, never a command-line argument or credential file. Requires Rust and curl; macOS/Linux interactive terminals use `stty` to suppress echo. `.local/` is ignored by Git.

Authenticated live fetching still needs verification with an actual account token. Do not commit snapshots or tokens. The selected character's items are used by default. Shared storage is opt-in and counted once; other members' personal banks are never implicitly borrowed.

## What this version covers

- Barrows magic-first preparation, a ranged switch for Ahrim, rune quantities, a spade, food, prayer restoration, and an exit teleport.
- ToA starter packing for raid levels 0–150, all three styles, room references, a Keris option, core dagger and puzzle pickaxe. All room gear comes from the same carried pool.
- Exact item IDs, quantities, GroupIron's alphabetical XP mapping and bundled quest index. Missing observations remain unknown.
- A finite reviewed gear-priority catalog, level checks, item exclusions, fewer-switches mode, and capacity checks across equipment changes.
- Four starter progression milestones: encounter access, 70 Prayer, 63 Herblore and 75 Magic. Goals are scoped by selected character; pin up to three.
- Local persistence, JSON backup/restore, copy/download exports using the current `InventorySetupPortable` structure.

This is a **starter planner**, not a best-in-slot or exact-DPS optimizer. It does not simulate boss mechanics, spec energy, boost decay, incoming damage or supply consumption. High-level gear beyond the priority catalog is retained in imports but not silently treated as supported. Charged IDs do not reveal remaining charges. Potion storage is retained as doses but must be withdrawn/decanted and observed in the bank before packing. Hiscores, full diary decoding, craftable resources, group-wide allocation, UIM planning and Hardcore risk assessment are not implemented.

## Build

Requires Rust with `wasm32-unknown-unknown`, and wasm-bindgen-cli **0.2.127**.

```sh
cargo test --locked -j 2
cargo run --locked --bin site -- build
```

The static output is `dist/`. Deploy it on any static host supporting `.wasm` with `application/wasm`. There is no application backend, database, account registration, analytics or paid service.

For an optional local preview, inspect port ownership first and run `cargo run --bin site -- serve PORT` on an authorized, unoccupied port. Stop the owned server after testing. This project never takes over existing servers.

## Data and attribution

See [SOURCES.md](SOURCES.md) for upstream formats, asset attribution and model limitations. All planning, import/export logic and build/import tools are Rust; JavaScript in the deployed site is limited to the browser's module loader and wasm-bindgen-generated bindings.
