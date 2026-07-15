# Portal fetch + staged install/uninstall: ehttp client, sha256-verified commits, install events for the UI

- STATUS: OPEN
- PRIORITY: 15
- TAGS: modding,wasm

Spike: tasks/20260714-202515/SPIKE.md (options M, J)
Depends on: 20260715-142906 (the mod cache + mods:// source + installed-set
integration this installs INTO). Split out of 142906 at its plan time: the
cache half is shippable and testable without network code, and two smaller
branches review better than one.

Goal: the game can fetch the portal catalog and install/uninstall mods over
the wire, on native and wasm. Pieces: an `ehttp`-based fetch layer driven from
IoTaskPool tasks + a channel resource; `PortalConfig` base URL (native default
= the Pages portal URL, wasm default derived from `window.location`, dev
override via env var / query param); parse `catalog.json` as
`nova_mod_format::PortalCatalog` (reject unknown schema_version); STAGED
installs - fetch every `files[]` entry, verify size + sha256, hold in memory,
then commit through 142906's cache API (files, then index entry last) and
register into the installed set live; uninstall reverses it. Event-driven API
for the UI (142916): trigger events (`FetchPortalCatalog`, `InstallPortalMod`,
`UninstallPortalMod`) + state resources (`RemoteCatalog`:
idle/fetching/ready/error, per-install progress/result) so the menu wires
without knowing the transport. Tests: transport behind a small trait - a real
localhost e2e on native (tiny_http dev-dep serving a nova_portal_gen-generated
tree, real ehttp), plus mock-transport failure injection (bad hash, short
body, unknown schema_version, mid-install abort leaves no index entry). Wasm
compile-gated via the wasm32 target check; behavior statically reviewed.
