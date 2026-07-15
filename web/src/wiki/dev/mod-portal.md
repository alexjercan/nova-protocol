# The static mod portal

> To make and publish your own mod, follow the guide
> [Make and publish a mod](../guide-make-a-mod/).

How remote mods are published and served. The portal is STATIC files on the
existing GitHub Pages site - no server, no database - generated on every
deploy, never hand-maintained. The game consumes it through the portal client
(`nova_assets::portal`): it fetches `catalog.json`, verifies and installs mods
into the local cache, and the mods menu UI binds on top.

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

The full publish-then-install flow ties these sections together:

```mermaid
sequenceDiagram
    participant Pub as Author
    participant Gen as nova_portal_gen
    participant Portal as Static portal
    participant Game as Game client
    participant Cache as Local cache

    Pub->>Gen: run generator over webmods/
    Gen->>Gen: validate + hash (sha256, size)
    Gen->>Portal: write catalog.json + bundle files
    Game->>Portal: fetch catalog.json
    Portal-->>Game: PortalCatalog (entries)
    Game->>Portal: download chosen bundle files
    Portal-->>Game: files
    Game->>Game: verify size + sha256 per file
    Game->>Cache: commit files-first, index-last
```

Types in `crates/nova_mod_format` (shared verbatim with the game):
`PortalCatalog { schema_version, entries }`; each `PortalEntry` carries `id`,
`version`, `bundle` (the entry-point manifest within the mod dir), the full
`ModMeta`, `files: [{path, size, sha256}]` and `total_size`. JSON, not RON, so
the TypeScript site and a future server API can produce/consume the same
shape.

`schema_version` (currently 1, `PORTAL_SCHEMA_VERSION`) bumps on any breaking
wire change; the game rejects catalogs with an unknown version rather than
misparse (`RemoteCatalog::Error`, test-pinned). The per-file size + sha256 is
what the game verifies as each file downloads, before anything is committed
to the cache.

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

The game's portal base URL is a config (`PortalConfig`): a native dev build
points at localhost with `NOVA_PORTAL_URL=http://localhost:8000/mods`, a web
build with a `?portal=http://localhost:8000/mods` query parameter. Without an
override, native uses the production Pages URL and the web derives the
sibling `mods/` tree from its own `window.location`.

## How installed mods are stored (game side)

A downloaded mod lands in the game's LOCAL MOD CACHE and is served back to the
asset server through the `mods://` source - native under
`dirs::data_dir()/nova-protocol` (files + a RON installed index), the web in
IndexedDB + localStorage. From there it loads and merges exactly like a
shipped mod. The full format and runtime flow live in the
[RON data format page](../modding-ron/), sections "Downloaded mods: the local
cache + the `mods://` source" and "The portal client" - the latter is the
fetch/verify/install flow that fills the cache from this portal: staged,
sequential downloads verified against the catalog's size + sha256 per file,
committed files-first-index-last only after everything checks out.

## The real server, later

The static portal IS the v1 API contract. A future service (the
Wesnoth-add-ons-server analog, done over plain HTTPS) serves the SAME
catalog.json shape - generated from a database instead of a folder scan - and
adds what static hosting cannot: third-party upload/publish with auth,
server-side validation (this generator's checks become the upload gate),
download counts, and search/pagination past the one-file catalog. The client's
base URL is already configurable, so switching is a config change, not a
rework.
