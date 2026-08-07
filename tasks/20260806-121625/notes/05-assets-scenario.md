# nova_assets and nova_scenario

## nova_assets - 16,702 src LOC / 32 files (27,476 incl. tests)

### Module map

| Module | LOC | Owns |
| --- | --- | --- |
| `collections.rs` | 426 | `BootAssets`/`GameAssets` preload collections + publish systems |
| `plugin.rs` | 178 | `GameAssetsStates` machine, scheduling |
| `merge.rs` | 701 | bundle flatten/overlay into `GameSections`/`GameScenarios` |
| `sections.rs` | 483 | **built-in ship-section content + balance constants.** Name says "sections", contents are authored game data |
| `scenario.rs` + `scenario/` | ~4,500 | **built-in scenario scripts.** Misnamed against the `nova_scenario` crate - this is content, not the engine |
| `mod_set.rs`, `mod_refs.rs`, `mod_cache.rs`, `mod_prefs.rs`, `persist.rs` | 2,700 | the modding/storage stack |
| `portal/` | 1,866 | network client - `mod.rs`, `install.rs`, `catalog.rs`, `config.rs`, `transport.rs` |
| `lint_walk.rs`, `balance.rs`, `content_report.rs`, `scenario_generation.rs`, `bin/content.rs` | 2,600 | **the authoring toolchain. Zero game runtime** - stated at `lint_walk.rs:1-5` |

`lib.rs:20-45` declares 16 modules spanning loading, modding, networking and an
authoring CLI. Three crates' worth.

### Size outliers

`scenario/shakedown/mod.rs` 1,405 (plus 1,744 lines of tests under
`shakedown/tests/`), `mod_cache.rs` 1,178 (index RON + native FS + hand-rolled
IndexedDB + `mods:/` asset source - four concerns), `mod_refs.rs` 954,
`balance.rs` 897, `lint_walk.rs` 855, `portal/install.rs` 824.

### The `content` CLI is in the wrong crate

`src/bin/content.rs:210`. It needs only `scenario_generation`, `lint_walk`,
`balance`, `content_report`. It drags `clap` into the game's asset crate and
forces `#[doc(hidden)] pub` escape hatches (`lib.rs:34-46`) to reach otherwise
private surface.

### Authoring is split across two crates

One feature, two homes: reference/geometry/ship checks in
`nova_scenario/src/lint/` (2,000 LOC); balance + walk + report + CLI in
`nova_assets` (2,600 LOC).

### wasm gates - 9 files

| File | Guards | Removable by a `Storage` trait? |
| --- | --- | --- |
| `persist.rs` | file-RON vs `localStorage`; `load_from`/`save_to` native-only (`:137,:144`) | **Yes** |
| `mod_cache.rs` (22 gates) | mods dir path, file read/write/delete, index IO, `ModsSourceDir` memory `Dir` (`:404`) | Mostly. The asset-source half (FileAssetReader vs memory `Dir`) is a real Bevy platform difference and stays |
| `mod_set.rs:256-302` | sync native load vs async hydrate/poll pair | **No** - sync-vs-async is control flow, not IO. Only removable if native also goes async |
| `plugin.rs:8,23,85-87`, `lib.rs:50,57` | consequences of the above | disappear when `mod_set` unifies |
| `portal/config.rs:31,41` | env var vs `window.location` | **Yes** - config-source trait |
| `portal/catalog.rs:140,206` | last-good catalog cache backend | **Yes** - same trait as `persist` |
| `portal/install.rs:481-576` | commit path + wasm-only system params | partly; sync/async again |
| `portal/mod.rs:99-271` | uninstall + `PendingRemovals` wasm-only params | same class |

Net: ~5 of 9 files are pure IO-backend gates a `Storage` trait erases.
**`transport.rs:17` already proves the pattern works** - copy it.

This is the one item that makes currently-untestable wasm paths testable
natively. `persist.rs:149-153` admits the wasm backend is guarded by static
review only, and no CI job compiles it.

~~Implied: the never-compiled wasm paths have probably rotted.~~
**Corrected 2026-08-07.** Measured, not assumed:
`cargo check -p nova_assets -p nova_probe --target wasm32-unknown-unknown`
exits 0 with 7 warnings and **no errors**, pulling in all 14 workspace crates.
`persist.rs`, `mod_cache.rs` and the whole `portal/` stack type-check on wasm
today. See `09-clippy-and-lints.md`.

What survives the correction:

- The `Storage`-trait extraction is still justified - by **testability and
  gate removal**, not by a latent-breakage argument. Type-checking is not
  behavior, and no test runs the wasm backend.
- The 7 warnings are one cluster: the whole of `crates/nova_probe/src/report.rs`
  is dead on wasm and wants a `cfg(not(target_arch = "wasm32"))` gate like its
  siblings in `lib.rs:82-109`.

**And it now pairs with a defect.** The atomic-write fix (finding 2 in
`11-review-assets-scenario.md`) touches `persist.rs:91`,
`mod_cache.rs:521`, `portal/catalog.rs:197` and `bin/content.rs:103` - the
same four files. "Write atomically" belongs in the `Storage` trait as a
contract rather than repeated as a convention. Do the two together.

### Coupling

No `nova_events` bypass (used only in tests). Deep-path imports are rare and
legitimate: `collections.rs:12 use nova_ui::font::UiFont`, `bin/content.rs:40`.
Real coupling is `nova_gameplay::prelude` in 16 files - unavoidable for content
builders, and it disappears if base content moves out.

### Doc rot

Module docs carry task ids that will rot: `portal/mod.rs:3` ("142906"),
`Cargo.toml:15`.

## nova_scenario - 14,678 LOC / 30 files

### Module map

| Module | LOC | Owns |
| --- | --- | --- |
| `objects/` | 4,400 | spawnables |
| `loader/` | 2,900 | parse/register/lifecycle/clock/trackers |
| `lint/` | 2,100 | author-time checks |
| `world.rs` | 767 | `NovaEventWorld` |
| `render_scale.rs` | 523 | **the odd one out** - a Bevy render-target lever with no scenario vocabulary. Here only because it scales "the scenario view" |
| `filters` | 417 | config vocabulary |
| `variables` | 280 | |
| `actions` | 207 | |
| `events` | 59 | |

`loader/` bundles parsing, lifecycle, clock and trackers. **Trackers are not
loading** - `loader/trackers.rs:1-2` emits gameplay-derived events.

### Size outliers

`objects/asteroid.rs` 1,082 (697 code + noise/mesh gen + gravity wells),
`actions/spawn.rs` 1,035 (394 code, despawn+spawn+scatter), `lint/scenario.rs`
992, `loader/lifecycle.rs` 948, `lint/ship.rs` 929.

### The one real coupling violation in the workspace

Scenario writes `nova_gameplay` HUD state directly, bypassing both the prelude
and `nova_events`:

- `world.rs:138-144` `nova_gameplay::hud::readout::HudReadoutFormat`
- `actions/mission.rs:512,534,554` `HudReadouts`

Fix: route through `nova_events` with one new event kind. Two files, and the
mission tests get rewritten.

### Other

- `fixtures.rs` exists in both `lint/` and `loader/` and **ships in the non-test
  build**.
- Two module heads use `///` on the first item rather than `//!`
  (`loader/mod.rs:1`, `objects/mod.rs:1`), so the doc attaches to the wrong node.
- ~~Four `unreachable!()` in `lint/`~~ **WITHDRAWN 2026-08-07.** All four
  (`ship.rs:443,769,772`, `scenario.rs:712`) are inside `#[cfg(test)] mod tests`
  (opened at `ship.rs:314` and `scenario.rs:529`). They are test assertion
  helpers, not a production lint path. No risk.

## Crate boundary

Direction is clean: `nova_assets -> nova_scenario`, no back-edge. Only doc
mentions the other way (`content_report.rs:2`, `lint_walk.rs:423`).

What leaks is authoring, and base content living in an asset crate.

## Testability

Tests-in-src dominates. Five files carry more test than code: `spawn.rs`
641/1,035, `lifecycle.rs` 576/948, `lint/ship.rs` 616/929, and the shakedown
scenario has 1,744 test lines.

Content builders are testable only by running the whole app
(`shakedown/tests/walk.rs:991` drives a full `App`), so a one-line balance tweak
costs a scenario walk.

Everything behind a wasm gate is **untestable by construction** and compiled by
no CI job.

## Defects found in this scope - added 2026-08-07

Full detail in `11-review-assets-scenario.md`; ranked and deduplicated in
`16-findings-master.md`. Summarised here because they change how this note's
ranked improvements should be read.

**Framing:** mod content is **untrusted input**. It arrives from a remote
portal catalog and from files the player may have edited. A panic, OOM or
stack overflow reachable from mod data is a defect, not an upheld invariant.

| Cluster | Sites (re-verified 2026-08-07) |
| --- | --- |
| **Mod data loss** | `mod_cache.rs:593,104,117` (`read_index_at(root).unwrap_or_default()` folds a corrupt index into an empty Vec, then persists it); `mod_cache.rs:521` + `persist.rs:91` + `portal/catalog.rs:197` + `bin/content.rs:103` (non-atomic `std::fs::write`); `nova_mod_format/src/deps.rs:104` (duplicate ids report a phantom cycle) |
| **Unbounded untrusted input** | `actions/spawn.rs:317` (uncapped `ScatterObjectsConfig::count`, field at `:244`); `portal/catalog.rs:71` + `transport.rs:31` (unbounded catalog body, parsed twice); `nova_mod_format/src/deps.rs:25` (unbounded recursion over an untrusted graph); `variables.rs:66` + `filters.rs:164` (both DSLs `Box`-recursive with no depth limit, and 7 fires on an installed-but-*disabled* mod) |
| **Gate coverage** | `merge.rs:214` (undeclared-ref violations recorded for Scenario content only, so a bad Section merges anyway); `mod_refs.rs:75` (`self://` gets no component validation) |
| **Determinism** | `objects/binding_input.rs:83` iterates a `HashMap` straight into the serde output that writes `input_mapping:` into generated `assets/base/**/*.content.ron`. Stable only because bevy uses `FixedState`. A bevy bump reshuffles every generated scenario file at once and `content_ron_parity` fails on a diff nobody authored. `lint_walk.rs:380` is the same class |

The determinism item is a direct threat to the content pipeline's only
integrity gate, and interacts with the `base-content-ron-is-generated` rule
(never hand-edit those files). `BTreeMap` or a sorted-key `serialize_map` is
cheap insurance.

### Came back clean in this scope

Recording these because they were the suspected areas:

- **Path traversal / zip-slip is unusually well done.** `is_safe_id` /
  `is_safe_rel_path` (`mod_cache.rs:134,142`) reject every non-`Normal`
  component, are applied in the shared `validate_file_op` **before** the cfg
  dispatch, and re-applied at the fs boundary in each `*_at`.
- **Overlay precedence does not depend on `HashMap` iteration order.**
  `merge_bundles` consumes an explicitly ordered `Vec`; `topological_order`
  re-scans `ids` in input order each round.
- HTML report escaping (`content_report.rs:332`) escapes `&` before `<`/`>`.
- `Collider::sphere` with a RON-authored zero/negative/NaN radius is degenerate
  geometry, not a panic (verified against `avian3d-0.7.0`/`parry3d-0.27.0`).

## Ranked improvements

1. Extract the authoring toolchain (`lint_walk`, `balance`, `content_report`,
   `scenario_generation`, `bin/content`, plus `nova_scenario/src/lint`) into
   `nova_content_tools`. Cost: new crate, rework the `#[doc(hidden)]`
   re-exports, move CI gate tests.
2. Introduce a `Storage`/blob-store trait mirroring `PortalTransport`. Cost: 6
   files. Removes ~5 files' gates and makes the wasm path testable natively.
3. Move base content (`sections.rs`, `scenario/**`, ~5,000 LOC) into
   `nova_content`. Cost: nova_assets loses its `nova_gameplay` dep in 16 files;
   assembly order changes.
4. Route scenario -> HUD through `nova_events`. Cost: 2 files, one event kind.
5. Split `mod_cache.rs` into index / blob-store / asset-source, and lift
   `render_scale` out of nova_scenario into nova_core or nova_gameplay.
