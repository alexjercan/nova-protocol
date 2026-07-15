# Portal fetch + staged install/uninstall: ehttp client, sha256-verified commits, install events for the UI

- STATUS: CLOSED
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
- [x] 1. Deps: `ehttp` (nova_assets, verify version), `sha2` (nova_assets),
  `tiny_http` (dev-dep). Lockfile staged with the manifest.
  (ehttp 0.7.1 locked and compiles clean on BOTH targets - no ureq/web-sys
  fallback needed; sha2 0.10 to share nova_portal_gen's existing pin; plus
  serde_json (catalog.json decode), nova_mod_format (wire types - nova_modding
  only re-exports the RON half), nova_portal_gen as a dev-dep for the e2e
  tree, and the web-sys `Location` feature.)
- [x] 2. `nova_assets::portal`: PortalConfig (+ per-platform default/override),
  PortalTransport trait + EhttpTransport, channel plumbing.
  ADAPTED: the channel is `std::sync::mpsc` (Sender in the resource, Receiver
  behind a never-contended Mutex for Sync), not crossbeam - same idiom, zero
  new dependencies. The wasm location/query helpers are pure + cfg-independent
  so native unit tests pin them.
- [x] 3. Catalog fetch: FetchPortalCatalog observer/system -> transport ->
  parse + schema_version gate -> RemoteCatalog states.
  (The gate probes `schema_version` ALONE first, so an unknown version is
  reported as such even when the rest of the shape would or would not parse.)
- [x] 4. Install/uninstall state machines with the staged commit + guards +
  EnabledMods strip; wire into the 142906 machinery (DownloadedMods mutation
  through the existing installed_set_changed path).
  ADAPTED (recorded deviations): (a) uninstall's file half is a new
  `mod_cache::remove_mod(id)` primitive (native: remove_dir_all of the mod
  dir; wasm: delete-by-`<id>/`-prefix returning the keys for memory-Dir
  eviction) - the index record does not store file lists, and the whole-dir
  sweep also collects orphans an older install left; (b) install REQUIRES the
  shipped catalog loaded (conservative Failed otherwise) - the no-shadowing
  guard cannot be checked without it, and the portal UI only exists past
  Loaded; (c) `Verifying` is the last file's integrity pass (per-file verify
  happens inside Fetching for fail-fast) - on native it flips within one
  frame, on wasm `Committing` is the observable async stage; (d) install jobs
  carry a generation counter so a stale transport callback from a failed +
  retried job cannot feed its successor.
- [x] 5. Tests per the plan above (e2e + failure injection); every new test
  states how it fails with its mechanism deleted.
- [x] 6. Wasm gate + static review of the wasm-side commit path.
- [x] 7. Docs + CHANGELOG.
- [x] 8. Verify: fmt; check --workspace --all-targets; the nova_assets test
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

## Close-out (20260715)

### What shipped

- `nova_assets::portal` (new module, ~750 lines + doc): the portal CLIENT -
  no UI, only the event/resource API task 142916 binds to.
  - `PortalConfig { base_url }` resource, resolved once at plugin build:
    native = `https://alexjercan.github.io/nova-protocol/mods` with a
    `NOVA_PORTAL_URL` override; wasm = derived from `window.location` (game
    at `<root>/play/` -> sibling `<root>/mods`, so forks' Pages deploys work
    with zero config) with a `?portal=<url>` query override. The href/query
    helpers are PURE and cfg-independent so native unit tests pin the
    wasm-only behavior; URL join normalizes the trailing-slash seam.
  - `PortalTransport` trait (fetch bytes by URL, callback completion; non-2xx
    is an Err) held as `Arc<dyn ...>` in the `PortalClient` resource;
    `EhttpTransport` is production (ehttp 0.7.1 - verified: compiles clean on
    BOTH targets, no fallback needed); tests swap in mocks.
  - Trigger events `FetchPortalCatalog` / `InstallPortalMod { id }` /
    `UninstallPortalMod { id }` (the repo's observer idiom: `add_observer` +
    `commands.trigger`/`world.trigger`); state resources `RemoteCatalog`
    (Idle | Fetching | Ready(PortalCatalog) | Error(String)) and
    `InstallJobs` (per-id Fetching{done,total} | Verifying | Committing |
    Failed(String); entry REMOVED on success - DownloadedMods is then the
    truth). Callbacks post `PortalMsg`s into an mpsc channel; the
    `poll_portal_messages` Update system drains it and advances the machines
    (the 142906 hydration idiom).
  - CATALOG fetch: the schema gate probes `schema_version` ALONE before the
    full parse, so an unknown version reports AS unknown - never a misparse,
    never a silent half-parse of a same-shaped future catalog.
  - STAGED INSTALL: guards first (job-in-flight; catalog Ready + entry
    exists; `validate_entry` re-checks id/version/file paths/bundle via the
    shared `is_safe_*` gates BEFORE any fetch - wire data; already-downloaded
    and shadows-a-shipped-id rejections), then sequential per-file fetch with
    size + sha256 verification on arrival (fail fast), bytes held in memory,
    commit only after the LAST file verifies: native `install_local` (files
    first, index last), wasm an IoTaskPool task awaiting ONE IndexedDB
    transaction to its `complete` event (the R1.4-correct commit signal - a
    new `mod_cache::commit_mod_files` + `await_transaction`), then the index,
    then the live `mods://` memory-Dir inserts. Success pushes the record
    into `DownloadedMods` (bundle loaded via `mods://<id>/<bundle>`) and the
    EXISTING load/mark/merge machinery takes over; installs stay DISABLED.
    Failed native commits sweep partial files (best-effort `remove_mod`).
  - UNINSTALL: index record first (the index must never point at missing
    files - the inverse of the install order), files second (new
    `mod_cache::remove_mod`: native remove_dir_all, wasm prefix-delete
    returning keys for memory-Dir eviction), drop from `DownloadedMods`, and
    STRIP the id from `EnabledMods` (persisted by the existing change-gated
    save system) - resolving 142906's R1.7: a reinstall starts disabled.
  - `PortalPlugin` wires it all; `GameAssetsPlugin` adds it, so production
    apps get the client for free; test rigs add the plugin and swap the
    transport/config.
- `mod_cache` additions: `commit_mod_files` (wasm, tx-commit-awaited),
  `remove_mod` (both platforms, id-gated), `upsert_index_record` /
  `remove_index_record` (cfg-independent index helpers), all behind the
  existing `validate_file_op` gate; `remove_mod_at` unit-tested.
- Deps: ehttp 0.7.1, sha2 0.10 (nova_portal_gen's existing pin - one sha2 in
  the tree), serde_json 1, nova_mod_format (direct - nova_modding re-exports
  only the RON half); dev-deps tiny_http 0.12 + nova_portal_gen; web-sys
  grew `Location`. Lockfile staged with the manifest.
- Tests: 5 portal unit tests (URL derivation/override/join, schema gate,
  entry validation) + 1 mod_cache unit (remove_mod sweep) + a NEW
  `portal_install` integration binary (6 tests): the REAL-WIRE e2e
  (nova_portal_gen tree from the REAL webmods -> tiny_http on an ephemeral
  localhost port -> real EhttpTransport -> fetch/install/enable/uninstall
  with byte-identity + EnabledMods-strip asserts) and mock-transport failure
  injection (sha mismatch, size mismatch, schema_version 999 + install-
  without-catalog, mid-install transport failure, shadowing/double-install
  guards + a mock-success presence contrast), each failure asserting ABSENCE
  through the cache API (no files, no index entry, no cache dir, no record).
- Docs: modding-ron-format.md new "The portal client" section + the R1.7
  uninstall/enablement paragraph REWRITTEN (uninstall now strips the pref);
  mod-portal.md consumption sections moved to present tense (client exists,
  base URL config documented for native + web dev); CHANGELOG Added entry.

### Decisions and deviations (vs the plan)

- `std::sync::mpsc` instead of crossbeam: Sender is Send+Sync since Rust
  1.72, the Receiver sits behind a never-contended Mutex purely for resource
  Sync - same idiom, zero new dependencies.
- Uninstall enumerates nothing: the index record carries no file list, so
  the file half is a whole-dir/prefix sweep (`remove_mod`), which also
  collects orphans an older install may have left. Alternative (file lists in
  the index) rejected as a format change with no consumer.
- Install REQUIRES the loaded shipped catalog (conservative Failed when
  missing): the no-shadowing guard cannot run without it, and the portal UI
  only exists past Loaded, so this is not a real-flow limitation.
- `Verifying` is the LAST file's integrity pass; per-file verification
  happens inside `Fetching` (fail fast). On native the tail states flip
  within one frame; on wasm `Committing` is the observable async stage
  (documented on the enum).
- Install jobs carry a generation counter (stale callbacks from a failed +
  retried job are dropped); the `Committed` message deliberately does NOT -
  a commit is only in flight while its id is `Committing`, which blocks any
  same-id retry until the message lands (documented on `PortalMsg`).
- The e2e keeps the mod_cache_install rig idioms (env lock owning BOTH env
  vars, production `register_mods_source`, real gauntlet mod, the literal
  production merge condition) and adds `PortalPlugin` - the production
  observers/poll system are the code under test, not a reimplementation.

### Evidence (all from the worktree root, this branch)

- `cargo test -p nova_assets --lib`: 38 passed (32 pre-existing + 6 new).
- `cargo test -p nova_assets --test portal_install`: 6 passed (all new; the
  real-wire e2e runs a real tiny_http server + real ehttp GETs).
- `cargo test -p nova_assets --test mod_cache_install`: 7 passed;
  `--test demo_scenario`: 11 passed; `--test webmods_validation`: 1 passed.
- `cargo fmt --check` clean; `cargo check --workspace --all-targets` green
  (only the pre-existing proc-macro-error2 future-incompat note).
- Wasm gate: `cargo check --target wasm32-unknown-unknown -p nova_assets -p
  nova_core` green, no warnings - RE-RUN after the final edits with a forced
  nova_assets recheck (touch + rebuild) so the gate covers the shipped code.
  ehttp needed no fallback. The wasm commit path (commit_mod_files awaiting
  the transaction `complete`/`error`/`abort` events, per-op db.close, the
  IoTaskPool commit task inserting into the shared Dir, uninstall's
  prefix-delete + Dir eviction) is statically reviewed against web-sys 0.3;
  like 142906's IDB wrapper it is compile-gated, NOT runtime-tested here -
  its first runtime exercise is the web UI task / a manual web session.
- Would-it-fail sabotage runs (each: apply, test, revert via git checkout,
  re-verify green; baseline committed first):
  1. STAGED-COMMIT DISCIPLINE - store each verified file on arrival ->
     sha_mismatch, size_mismatch AND mid_install_transport_failure all
     FAILED on their absence asserts (3 tests). Reverted -> 6 passed.
  2. SCHEMA GATE deleted -> unknown_schema_version_is_rejected_not_misparsed
     FAILED ("must never become Ready") AND the lib unit
     decode_catalog_gates_on_schema_version FAILED. Reverted -> green.
  3. sha256 check disabled -> sha_mismatch FAILED (via the documented
     wrong-success timeout path, 60s). Reverted.
  4. size check disabled -> size_mismatch FAILED fast (the truncated body
     fell through to a sha-flavored message; the test asserts the message
     names the size check). Reverted.
  5. EnabledMods strip disabled -> the wire e2e FAILED on exactly the
     "uninstall strips the id from EnabledMods" assertion (the R1.7 pin).
     Reverted.
- content_ron_parity: not in the required list; untouched by this branch
  (fixed on master per the task brief). Full suite stays on CI.

### Difficulties

- Remarkably few at runtime - all 6 integration tests passed on their first
  execution. The up-front reading (142906's TASK/REVIEW + rig idioms, the
  repo's On<>/trigger observer examples, the ehttp 0.7.1 source for its
  exact fetch signature and Response fields, bevy's memory.rs for
  Dir::remove_asset) is what bought that.
- First-compile warnings drove three real design corrections: the poll
  system had leaked private types through a `pub fn` signature (privatized);
  the `Committed` message carried a job generation that nothing could ever
  disambiguate with (removed, with the invariant documented instead); the
  wasm-only pure helpers needed explicit dead-code allows with the "tested
  natively on purpose" rationale.
- `percent_decode` briefly had a garbled bounds condition from a typo
  (caught on the immediate re-read, before compiling).
- The sha-check sabotage takes its full 60s to fail: `pump_install_failure`
  detects a wrongly-successful install only by timeout. Documented on the
  helper; acceptable for a sabotage-only path.

### Reflection

- Designing the failure tests FIRST (mock transport + canned routes) made
  the sabotage matrix nearly free - every mechanism had a mock-side pin
  before the real-wire e2e existed. Do this again.
- The one-channel/one-drain shape (native commits post into the same channel
  their own drain is reading) kept the finalize logic in exactly one place
  across platforms; worth remembering as the pattern for future async+sync
  split flows.
- `pump_install_failure`'s timeout-as-failure path is slow when sabotaged;
  next time give wrong-success a fast detector (e.g. watch the job entry
  vanish) so sabotage loops stay tight.

## Review round R1 (20260715)

APPROVE-quality verdict (no BLOCKER/MAJOR); 4 MINOR/NIT findings addressed in
a follow-up commit, plus two recorded non-changes:

- R1.1 (MINOR, URL containment overclaim): the `is_safe_*` gates accept any
  `Component::Normal`, so a percent-encoded dot-dot (`%2e%2e`) passed local
  validation while being a real dot-dot segment to a WHATWG-decoding fetcher
  (the browser on wasm; many CDNs) - a hostile catalog could steer GETs above
  the portal base (same-origin, sha256-pinned bytes, but the comment claimed
  tree containment). Fixed: `validate_entry` now ALSO enforces the
  generator's published charset - ids/versions lowercase alnum + `-` + `.`
  (never dot-only), file path components ascii alnum + `-` + `_` + `.` (never
  dot-only, so no `%`/`?`/`#`/`\` anywhere) - and the comment states the two
  enforced boundaries (local Path containment + URL charset identity)
  precisely. New unit test `validate_entry_rejects_percent_encoded_and_off_
  charset_segments` (`%2e%2e` as version/id/path, `?` metachar, uppercase id
  rejected; mixed-case FILE paths stay allowed - only ids/versions are
  generator-lowercase). Sabotage-verified: is_url_safe_segment forced true ->
  the test FAILED; reverted -> green.
- R1.2 (MINOR, wasm uninstall/reinstall race): the detached removal task
  could delete a same-id reinstall's fresh IDB writes. Fixed: a cfg-
  INDEPENDENT `PendingRemovals` resource; wasm uninstall inserts the id
  before spawning and the task reports back through a new (wasm-gated)
  `PortalMsg::Removed` that the poll system clears; `on_install_portal_mod`
  rejects pending ids FIRST (before any catalog state) with a clear Failed
  reason. Native never fills the set (its removal is synchronous) and is
  unaffected. The guard is unit-tested natively against the REAL observer
  (`install_is_rejected_while_an_uninstall_removal_is_pending`, including
  the clears-after-Removed half); the wasm send/clear path is compile-gated
  + statically reviewed like the rest of the wasm side. Sabotage-verified:
  guard forced false -> the test FAILED; reverted -> green.
- R1.4 (MINOR, unbounded staging): anti-absurdity caps (documented as caps,
  not quotas) in `validate_entry`: 32 MiB/file, 256 files, 128 MiB summed
  declared size (saturating add). Unit test covers all three
  (`validate_entry_enforces_the_staging_caps`). A lying server can still
  send one oversized body (ehttp buffers before the size check rejects) -
  noted on the constants; the caps bound what the CATALOG can command.
- R1.5 (NIT): uninstall now removes the id's (necessarily Failed) job entry
  so it cannot outlive the mod it describes.
- R1.6 (NIT): duplicate `files[].path` rejected (set check); assert added to
  `validate_entry_rejects_hostile_catalog_data`.
- R1.8 (NIT): `await_transaction` doc now cross-references `await_request`'s
  losing-closure leak rationale (three one-shot closures, two leak per
  commit).
- R1.3 (no timeout/cancel): NOT changed here - deferred to the UI task
  142916 per the reviewer (recorded there); the module doc now states the
  wedge-on-never-firing-callback limitation and points at 142916.
- Reviewer's finding 7 (sync fs writes in Update): accepted repo idiom, no
  change.

Evidence after the round: `cargo test -p nova_assets --lib` 41 passed (3 new
unit tests); `--test portal_install` 6 passed; `--test mod_cache_install` 7
passed (mod_cache touched: R1.8 doc only); fmt clean; `cargo check
--workspace --all-targets` green; wasm gate re-run green (nova_assets
recompiled with the Removed/PendingRemovals wasm arms). Sabotage runs
(applied/reverted, targeted edits since the fixes were uncommitted): charset
gate forced-true -> R1.1 test FAILED; pending guard forced-false -> R1.2
test FAILED; both reverted, full suite green. The caps/duplicate checks
share `validate_entry` with their tests (same-function coupling; no
separate sabotage run - stated honestly).
