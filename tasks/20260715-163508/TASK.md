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

## Plan (20260715)

Design (building on 142906's landed cache; read its TASK.md + REVIEW.md
first - especially R1.4's IDB commit caveat and R1.7's enablement note):

- MODULE: `nova_assets::portal` - client-side only, no UI. `PortalConfig {
  base_url }` resource: native default
  `https://alexjercan.github.io/nova-protocol/mods`, `NOVA_PORTAL_URL` env
  override; wasm derived from `window.location` (game at /play/ -> sibling
  /mods/), `?portal=` query override. URL join = simple
  `format!("{base}/{path}")` with a trailing-slash normalize.
- TRANSPORT: a small `PortalTransport` trait (fetch bytes by URL with a
  completion callback) - `EhttpTransport` (new dep `ehttp`, workspace-fresh:
  verify the current version at build; native ureq / wasm fetch under one
  API) + test transports (fs-serving mock, failure injection). Held as an
  `Arc<dyn PortalTransport>` inside a resource so tests swap it.
- STATE MACHINES (event-driven for 142916): trigger events
  `FetchPortalCatalog`, `InstallPortalMod { id }`, `UninstallPortalMod { id }`;
  resources `RemoteCatalog` (Idle | Fetching | Ready(PortalCatalog) |
  Error(String)) and `InstallJobs` (per-id: Fetching{done,total} | Verifying |
  Committing | Failed(String); entry removed on success - `DownloadedMods` is
  then the truth). Channels (crossbeam) + poll systems bridge the async
  callbacks, mirroring 142906's hydration idiom.
- CATALOG: parse as `nova_mod_format::PortalCatalog`; REJECT unknown
  `schema_version` with a clear Error (never misparse). Entries keep catalog
  order for the UI.
- INSTALL (staged, sequential files for simple progress): fetch each
  `files[]` -> verify size + sha256 (`sha2` dep on nova_assets) -> hold ALL in
  memory -> commit via mod_cache (files first, index last; on wasm the commit
  runs in an IoTaskPool task and must respect R1.4's request-vs-commit caveat
  - await each put, treat any failure as install-failed and attempt cleanup) ->
  push the record + kick the bundle load through the EXISTING 142906 systems
  (DownloadedMods change -> load -> mark -> merge). Installs stay disabled.
  Guards: reject install when id is already downloaded OR shadows a shipped
  catalog id (mirror the cache-side rule); validate id/paths via the shared
  `is_safe_*` gates BEFORE any fetch (portal data is untrusted input).
- UNINSTALL: remove files + index entry, drop from `DownloadedMods`, AND
  strip the id from `EnabledMods` (+prefs via the existing save path) -
  resolving 142906's R1.7 note: a reinstall starts disabled, matching the
  documented default. Update the modding-ron-format.md sentence accordingly.
- TESTS (native): e2e with `tiny_http` (dev-dep) serving a
  nova_portal_gen-generated tree of the REAL webmods: FetchPortalCatalog ->
  Ready lists gauntlet -> InstallPortalMod -> files cached + record present +
  (enable) merge registers `gauntlet_run` -> UninstallPortalMod -> gone +
  EnabledMods stripped. Failure injection via the mock transport: corrupted
  byte (sha mismatch) -> Failed, NO files, NO index entry; truncated body
  (size mismatch) -> same; `schema_version: 999` -> RemoteCatalog Error;
  transport error mid-install (Nth file fails) -> staged discipline holds
  (nothing committed). Each failure test asserts the ABSENCE evidence via the
  cache API. Wasm gate: `cargo check --target wasm32-unknown-unknown -p
  nova_assets -p nova_core`.
- DOCS: modding-ron-format.md (portal client paragraph + the uninstall/
  enablement update); docs/mod-portal.md ("the game consumes it" section
  updates from future-tense to present for fetch/install); CHANGELOG (Added).

Steps:
- [ ] 1. Deps: `ehttp` (nova_assets, verify version), `sha2` (nova_assets),
  `tiny_http` (dev-dep). Lockfile staged with the manifest.
- [ ] 2. `nova_assets::portal`: PortalConfig (+ per-platform default/override),
  PortalTransport trait + EhttpTransport, channel plumbing.
- [ ] 3. Catalog fetch: FetchPortalCatalog observer/system -> transport ->
  parse + schema_version gate -> RemoteCatalog states.
- [ ] 4. Install/uninstall state machines with the staged commit + guards +
  EnabledMods strip; wire into the 142906 machinery (DownloadedMods mutation
  through the existing installed_set_changed path).
- [ ] 5. Tests per the plan above (e2e + failure injection); every new test
  states how it fails with its mechanism deleted.
- [ ] 6. Wasm gate + static review of the wasm-side commit path.
- [ ] 7. Docs + CHANGELOG.
- [ ] 8. Verify: fmt; check --workspace --all-targets; the nova_assets test
  targets; wasm gate. Full suite on CI.

## Notes

- Relevant files: crates/nova_assets/src/{mod_cache.rs,lib.rs} (cache API,
  installed_set_changed, hydration idiom), crates/nova_mod_format/src/lib.rs
  (PortalCatalog + PORTAL_SCHEMA_VERSION), crates/nova_portal_gen (test tree
  generator), tasks/20260715-142906/{TASK,REVIEW}.md (R1.4 commit caveat,
  R1.7 enablement).
- The UI (spinner, buttons, offline/stale catalog cache) is 142916's scope;
  this task's deliverable is the event/resource API it will bind to.
- ehttp is a NEW third-party dep: no bevy coupling, wasm+native under one
  API; if its current release fights the workspace (edition/wasm-bindgen),
  fall back to cfg-split ureq + web-sys fetch and record the substitution.
