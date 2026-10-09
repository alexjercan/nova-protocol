# SW-LOAD: load_screen.rs + save_status.rs

Scope: `tasks/20261007-090756/SLICE4.md` sections "load_screen.rs" and
"save_status.rs".

## Files changed

- `crates/nova_menu/src/load_screen.rs` (new)
- `crates/nova_menu/src/save_status.rs` (new)
- `crates/nova_menu/src/menu_ui.rs`: added the native-only "Load" button under
  New Game, and the native-only Load panel spawn (list-beside-details, same
  shape as the Scenarios panel) in `setup_menu_ui`. No other line touched.
- `crates/nova_menu/src/lib.rs`: added the two native-only `mod` lines, the
  native-only `use` lines for the new items, `init_resource` for
  `WorldListings`/`SelectedWorldSlug`, the chained `refresh_load_list` /
  `refresh_load_details` registration (gated `in_state(MainMenu)`), and
  `sync_save_status_line` (gated `in_state(Playing)`). No other line touched.
- `crates/nova_menu/src/tests/load_screen.rs` (new)
- `crates/nova_menu/src/tests/mod.rs`: added `#[cfg(not(target_arch =
  "wasm32"))] mod load_screen;`.

Did not touch `world_setup.rs`, `pause.rs`, `leave.rs`, `nova_core`,
`nova_world_base`, or the Cargo.toml dev-dependencies table (the main worker
had already added `tempfile` and `nova_world_base`'s `test-support` feature;
I used both, added nothing further).

## Names added

`load_screen.rs`: resources `WorldListings(Result<Vec<WorldListing>,
WorldRefusal>)` (manual `Default` -> `Ok(Vec::new())`, since `Result` can't
carry a blanket impl) and `SelectedWorldSlug(Option<String>)`; markers
`LoadPanel`, `LoadWorldList`, `LoadWorldRow { slug }`, `LoadWorldDetails`,
`LoadWorldButton`; handlers `on_load_screen`, `on_load_back`,
`on_load_world_row_select`, `on_load_world`; systems `refresh_load_list`,
`refresh_load_details`; private helpers `spawn_load_note`, `spawn_load_row`.

`save_status.rs`: marker `SaveStatusLine`; system `sync_save_status_line`;
private helper `status_text`.

No new pub items beyond what the spec named; no items beyond the spec's list.

## Design notes / deviations

- `on_load_screen` treats `WorldsRoot(None)` as `WorldListings(Ok(vec![]))`;
  `refresh_load_list` checks `WorldsRoot` directly (not through
  `WorldListings`) to draw the "no folder" line, so the no-root and
  empty-root messages stay distinct without a third enum variant.
- `refresh_load_list` / `refresh_load_details` run on
  `resource_changed::<WorldListings>` / `.or_else(resource_changed::<
  SelectedWorldSlug>)`, not a custom "dirty" predicate like the Scenarios
  screen's `Added<ScenariosList>` trick - the spec asked for plain
  `resource_changed`, and nothing here has a live registry to watch.
- `on_load_world`'s `Err` path mutates the matching row's `header` in
  `WorldListings` to the fresh refusal (rather than a separate "last load
  error" slot), which is what re-triggers both refresh systems via
  `resource_changed` and greys that row's Load button on the same frame.
- Per the spec's fallback clause: `LoadedSectionPacks` has no `Default`, so
  every system here takes it as `Res`, not `Option<Res>`. The test fixture
  (`nova_world_base::test_support::arm_save_fixture`) supplies it.
- `WorldSaveHeader.player_sector` is a `nova_world::SectorCoord`, a type
  `nova_menu` cannot name (not re-exported by `nova_world_base`'s prelude, and
  `nova_world` is not a dependency of this crate). Per the spec's own
  fallback clause, the test's second ("refused") world is written with
  `create_world` only, no header at all, so it refuses as `Unreadable`
  rather than the `Catalog` mismatch the task text sketched. This still
  exercises the identical `Err` branch (list row, greyed Load button) with
  no further Rust API available to construct a differently-cataloged header
  without that type.

## Test

`load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one` in
`tests/load_screen.rs`. Builds the menu rig, points `WorldsRoot` at a
tempdir, arms `nova_world_base::test_support::arm_save_fixture` (catalog
digest 1, 300-credit player) and runs the real production save pipeline
(`WorldSaveTestPlugin`) to completion for one "Good World", drops its lock,
creates an empty "Bad World" folder (no header), presses "Load Button",
asserts both rows exist, asserts the bad row's "Load World Button" carries
`InteractionDisabled` and the good row's does not, presses Load, and asserts
`WorldSaveSession` (named "Good World"), `ResumedWorld`, `GameMode::NewGame`,
`NewGameScenario(None)` and `GameStates::Playing`.

```
running 1 test
test tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 198 filtered out; finished in 0.02s
```

## Mutation

Removed `InteractionDisabled` from the details pane's refused-world Load
button spawn. Re-ran the test:

```
---- tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one stdout ----
thread '...' panicked at crates/nova_menu/src/tests/load_screen.rs:104:5:
a refused world's Load button is greyed
test result: FAILED. 0 passed; 1 failed; ...
```

Reverted the edit (via `Edit`, not a `git checkout`); the test passes again
and `cargo fmt -p nova_menu -- --check` is clean.

## Checks run

- `nix develop --command cargo test -j 4 -p nova_menu --lib load_screen` ->
  pass (1 test).
- `nix develop --command cargo check -j 4 -p nova_menu --tests` -> clean, no
  warnings.
- `nix develop --command cargo fmt -p nova_menu` then `-- --check` -> clean.

No compile errors were seen in files I do not own; nothing to report there.
