# The static mod portal

How remote mods are published and served (task 20260715-142900, spike
tasks/20260714-202515/SPIKE.md). The portal is STATIC files on the existing
GitHub Pages site - no server, no database - generated on every deploy, never
hand-maintained. The game consumes it in the Explore flow (tasks 142906/142916).

## Layout

Published by the deploy workflow next to the game:

```text
https://alexjercan.github.io/nova-protocol/
  play/                     the WASM game
  mods/
    catalog.json            PortalCatalog (JSON, schema-versioned)
    <id>/<version>/<files>  every file of each mod, verbatim
```

Sources live in the repo-root `webmods/` directory - OUTSIDE `assets/`, so
portal mods never ship inside the game build. Each subdirectory is one mod;
the directory name is its id (lowercase ascii, digits, `-`). A mod is the same
folder-bundle shape as an installed mod: exactly one `<name>.bundle.ron` at the
mod root (content paths relative to it, `meta` block required for publishing:
non-empty `name` and `version`). Only that root-level manifest is the entry
point; a `*.bundle.ron` nested in a subdirectory is published as a plain data
file and validated by neither gate. Declared content paths must resolve to
files INSIDE the mod directory (the generator checks membership in the set it
serves, so escaping `../` paths are rejected).

## The generator

`crates/nova_portal_gen` (engine-free: no bevy, builds in seconds) turns
`webmods/` into the portal tree:

```sh
cargo run --release -p nova_portal_gen -- \
    --source webmods --shipped assets/mods.catalog.ron --out site/mods
```

It validates what a manifest gate can - bundle parses, publishable meta, every
listed content file exists, well-formed unique ids that do not collide with
the SHIPPED catalog, dependencies resolve within the portal + shipped set -
computes per-file size + sha256, copies files under `<id>/<version>/`, and
writes a deterministic `catalog.json` (sorted entries and file lists; two runs
over the same source are byte-identical, test-pinned). Any validation failure
exits non-zero with a one-line reason, failing the deploy.

Whether the content actually LOADS is deliberately not the generator's job:
the `webmods_validation` integration test (crates/nova_assets/tests) drives
every webmods bundle through the real bevy loaders to recursive `Loaded` on
regular CI - the deep half of the publish gate, running where tests already
run.

## The wire schema (catalog.json)

Types in `crates/nova_mod_format` (shared verbatim with the game):
`PortalCatalog { schema_version, entries }`; each `PortalEntry` carries `id`,
`version`, `bundle` (the entry-point manifest within the mod dir), the full
`ModMeta`, `files: [{path, size, sha256}]` and `total_size`. JSON, not RON, so
the TypeScript site and a future server API can produce/consume the same
shape.

`schema_version` (currently 1, `PORTAL_SCHEMA_VERSION`) bumps on any breaking
wire change; clients must reject catalogs with an unknown version rather than
misparse. The per-file sha256 is what the game verifies after download,
before installing.

## Publishing a mod (today)

1. Add `webmods/<id>/` with a `<name>.bundle.ron` (content list + full meta,
   version non-empty) and its content files.
2. `cargo test -p nova_portal_gen` and `cargo test -p nova_assets --test
   webmods_validation` locally (or let CI run them).
3. Land on master; the next deploy publishes it.

## Local development

Generate into a scratch dir and serve it statically:

```sh
cargo run -p nova_portal_gen -- --source webmods --shipped assets/mods.catalog.ron --out /tmp/portal/mods
python3 -m http.server -d /tmp/portal 8000   # portal at http://localhost:8000/mods/
```

The game's portal base URL is a config (task 163508), so a dev build can point
at localhost.

## How installed mods are stored (game side)

A downloaded mod lands in the game's LOCAL MOD CACHE and is served back to the
asset server through the `mods://` source - native under
`dirs::data_dir()/nova-protocol` (files + a RON installed index), the web in
IndexedDB + localStorage. From there it loads and merges exactly like a
shipped mod. The full format and runtime flow live in
docs/modding-ron-format.md, section "Downloaded mods: the local cache + the
`mods://` source" (task 20260715-142906); the fetch/verify/install flow that
fills the cache from this portal is task 20260715-163508.

## The real server, later

The static portal IS the v1 API contract. A future service (the
Wesnoth-add-ons-server analog, done over plain HTTPS) serves the SAME
catalog.json shape - generated from a database instead of a folder scan - and
adds what static hosting cannot: third-party upload/publish with auth,
server-side validation (this generator's checks become the upload gate),
download counts, and search/pagination past the one-file catalog. The client's
base URL is already configurable, so switching is a config change, not a
rework. See the spike's option G discussion.
