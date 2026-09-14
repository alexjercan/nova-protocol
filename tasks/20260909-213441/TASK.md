# Remove hidden content flags and prove settings and picker delivery

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.14.0, testing, examples, ui, modding

## Goal

Remove both legacy authored `hidden` flags. A scenario's explicit
`menu_backdrop` role is the only reason it is omitted from the Scenarios
picker, and every installed mod is visible to the player who may need to
inspect or disable it. Add the remaining live proof for settings persistence
and for the picker starting the row the player clicked.

From the 2026-09-09 coverage map: `system_headless_rebind` proves only an
inert-store rebind in one app, and `bug_menu_picker` reaches Playing without
asserting that the selected scenario is the one that started. Rename that
range to `system_scenario_picker`: it now proves the picker subsystem rather
than only the pane-layout regression.

Owner (2026-09-09): "add more systems examples tests ... create systems
examples for all edge cases and bugs we find."

Rules: `examples/systems/README.md`. A new range gets its `[[example]]` block
and slugs on the roster in `crates/nova_probe_cli/tests/catalog_drift.rs`.
It belongs in the `ui` shard from `20260909-213100` once that shard exists.

## Remove `ScenarioConfig.hidden`

- [x] Delete `ScenarioConfig.hidden` from the Rust and strict RON format. Do not
      retain a serde alias, ignored compatibility field, or replacement generic
      visibility switch.
- [x] The picker lists every scenario except one with `menu_backdrop: true`.
      Campaign membership does not override this exclusion.
- [x] Lint a campaign that names a menu-backdrop scenario as an Error. A
      campaign member is a player-launchable chapter; a backdrop is not.
- [x] Previously hidden chained chapters become ordinary picker rows. Campaign
      members remain grouped in their authored order under their campaign
      header.
- [x] Migrate generated base content, webmods, examples, tests, comments and
      documentation. Menu backdrops keep `menu_backdrop: true` and no longer
      need a second flag.
- [x] Treat this as a format break. Old scenario RON that authors `hidden` no
      longer parses and must migrate. Mark the changelog entry `**(breaking)**`.

`menu_backdrop` excludes a scenario from the picker only. It does not make the
scenario unloadable by the ambience system, a direct scenario ID, a test, or a
tool.

## Remove `ModEntry.hidden`

- [x] Delete `ModEntry.hidden` from the installed-mod catalog format. Do not
      retain an ignored compatibility field or another generic invisible-mod
      switch.
- [x] Build the player-facing `ModCatalog` from every installed catalog entry.
      Base remains visible and locked on. Downloaded mods keep their existing
      behavior.
- [x] Remove startup logic that strips hidden non-base IDs from `EnabledMods`
      and remove the synthetic hidden-mod tests.
- [x] Keep dev and test content out of the shipped installed-mod catalog. Put it
      in its owning example or test fixture instead of installing content the
      player cannot see or disable.
- [x] Migrate catalog examples, constructors, comments and documentation. Treat
      an authored catalog `hidden` field as a format break and document it in
      the same migration and changelog work as the scenario field.

There is no shipped hidden mod to preserve. The retired screenshot-reel tooling
mod was the old use case. Rust's `#[doc(hidden)]`, UI node visibility, and
ordinary uses of the English word "hidden" are unrelated and remain.

## Picker delivery proof

Do not add `system_scenario_launch`. Rename the existing `bug_menu_picker` to
`system_scenario_picker` and extend its journey. Update its file, clap name,
self-description, `Cargo.toml` example block, probe roster and every repository
reference. The broader `system_` name is now correct because the range proves
both stable picker layout and scenario delivery, not one regression.

- [x] Record the non-default scenario row selected through a real pointer click.
- [x] Click the existing Play button through real pointer input.
- [x] Wait for the atomic scenario-load gate from `20260909-213559` to release;
      do not use an arbitrary frame delay.
- [x] Assert the resulting `CurrentScenario` ID is exactly the selected row ID.
- [x] Add the named outcome marker and roster slug.

Campaign grouping and hidden-member launch already have focused menu tests.
Campaign chaining is already proved by `system_outcomes`. Do not duplicate
those claims here.

## `system_settings_persist`

- [x] Build the shipped `editor_app` on an isolated temporary
      `SettingsStoreRoot` with explicit `SettingsStoreAccess::ReadWrite`, since
      `harness_env_active()` otherwise makes the store inert under
      `NOVA_AUTOPILOT`.
- [x] Through the real Settings UI, rebind `main_drive` to `J`, set master
      volume to `0.4`, and select the `Low` graphics preset. These values are
      test fixtures, not new defaults.
- [x] Let the normal save path write the store, inspect the persisted value,
      then drop the app and build a second app on the same root.
- [x] Assert the relaunched UI and live resources contain the saved values and
      the rebound physical key drives `main_drive` in gameplay.
- [x] Reset only the `main_drive` binding. Prove the persisted keybind override
      is removed rather than only changing the current `InputBindings`
      resource, while the saved `0.4` master volume and `Low` graphics preset
      remain unchanged.
- [x] Start another app from a manually authored older partial store and prove
      omitted fields load on their serde defaults.

This range proves the composed UI -> store -> new app -> gameplay path. Keep the
existing focused serialization and settings-widget tests; do not duplicate
their internal assertions without exercising that path.

## Work moved out of this task

Do not implement these ranges or their defects here merely because an earlier
version of this task listed them:

- `system_session_loop` is owned in full by `20260909-214629`, including reload,
  status ownership, one-shot startup, teardown, backdrop and fresh-session
  invariants.
- `system_pause_settings` is owned in full by `20260909-214629`, including
  Retry, shared Settings UI, focus/modal arbitration, responsive containment
  and window policy.
- `system_broken_mod` is dropped. Optional-mod recovery and dependency failures
  belong to `20260909-214629`; content lint, the scenario gate and refused-load
  presentation already have focused proof there and in existing tests.
- A new `system_scenario_launch` is dropped. Its only uncovered delivery claim
  is added while renaming `bug_menu_picker` to `system_scenario_picker` above.
- Atomic load and frame-zero behavior remain owned by `20260909-213559`.
- Pending-sever and other simulation teardown resources remain owned by
  `20260909-214706`.

These sprint tasks compose. Do not duplicate a sibling's implementation, add a
second policy, or restore a field another item removes.

## Proof

- Scenario format and picker tests prove all non-backdrops list, backdrops never
  list, and campaign membership cannot expose a backdrop.
- Content lint rejects a campaign that names a backdrop.
- Installed-mod catalog tests prove every installed entry reaches `ModCatalog`.
- Strict format tests reject or fail to compile fixtures that still author
  either removed field; all in-tree authored content is migrated.
- `system_scenario_picker` proves stable pane layout and that a real clicked
  row becomes the exact `CurrentScenario` after atomic loading.
- `system_settings_persist` proves settings survive app reconstruction, the
  rebound key drives the verb, reset clears its persisted override, and an
  older partial store receives defaults.
- Run only affected crate checks, content generation/lint, and relevant docs or
  web checks. Inspect generated output.
- Run the affected UI ranges three times in a row after the `ui` shard from
  `20260909-213100` is available. Until then, run the ranges directly and record
  the shard dependency rather than editing the CI split here.

## Documentation

Update `CHANGELOG.md`, scenario/mod format documentation, picker/player docs,
worked mod examples, and migration guidance. Explain that:

- scenario `hidden` is removed; use `menu_backdrop: true` only for an actual
  menu backdrop, while ordinary and chained scenarios are picker-visible;
- installed-mod `hidden` is removed; every installed mod is player-visible and
  dev-only content must not enter the shipped catalog.

## Done when

- Neither `ScenarioConfig` nor `ModEntry` has a `hidden` field.
- No in-tree scenario or installed-mod catalog authors either removed field.
- Menu backdrops are absent from the picker and ordinary scenarios are present.
- Every installed mod has a player-facing row.
- The picker launches the exact row clicked.
- `system_settings_persist` is green with isolated writable storage.
- Affected generated content, lint, checks, docs and probe ranges pass.

## Result

Branch `hidden-flags` off `ec035e45d`. Every checkbox is done.

- `c22e564a9` Remove the scenario hidden flag and list every scenario but a
  backdrop
- `98a3c334d` Give every installed mod a player-facing row
- `68285ef2c` Refuse a campaign that names a menu backdrop
- `c5354cc03` Migrate every authored scenario off the hidden flag
- `4322910fe` Document the removal of both hidden flags
- `bc5ea9b92` Prove the scenario picker starts the row the pointer clicked
- `8f824bcf7` Prove a setting changed in the UI survives the app
- `3bca56ea0` Fold the mod seeding branches that say the same thing

### Format break

`ScenarioConfig.hidden` and `ModEntry.hidden` are deleted outright. Both
structs keep `#[serde(deny_unknown_fields)]`, so a file that still authors
either field fails to parse. No alias, no ignored field, no replacement
visibility switch. `nova_menu::scenarios::picker_lists` now filters on
`!menu_backdrop` alone. `nova_assets::mod_set` builds `ModCatalog` from every
installed entry and no longer strips IDs out of `EnabledMods`.

Migrated in tree: the four `main_menu` Rust builders, the four generated
`menu_*.content.ron` files, `assets/mods/example/example.content.ron` and the
stale `assets/mods.catalog.ron` comment.

### Generated content

`cargo run content gen` then `git status --porcelain`: empty. The only
generated diff produced by this task was the dropped `hidden: true` line in
`assets/base/scenarios/menu_{approach,dock,drift,patrol}.content.ron`,
committed in `c5354cc03` from the edited Rust builders. No
`assets/base/**/*.content.ron` file was hand edited.

`cargo run content lint`: 0 error(s), 0 warning(s), 0 finding(s), 9
scenario(s) balance-audited, 1 acked, 14 creative map(s).

### Unit and integration checks

| Command | Result |
| --- | --- |
| `cargo test -p nova_scenario --lib loader:: lint::` | 167 passed |
| `cargo test -p nova_menu --lib scenarios` | 13 passed |
| `cargo test -p nova_mod_format --lib` | 12 passed |
| `cargo test -p nova_assets --test example_scenario` | 14 passed |
| `cargo test -p nova_authoring --lib campaign_membership` | 2 passed |
| `cargo test -p nova_editor --lib scenario` | 32 passed |
| `cargo test -p nova_probe_cli --test catalog_drift` | 2 passed |
| `cargo check --workspace --all-targets --keep-going` | clean |
| `cargo check -p nova-protocol --features debug --all-targets` | clean |
| `cargo fmt --all --check` | clean |

Clippy was run on the touched packages only, never as a sweep: the two new
examples plus `-p nova_menu -p nova_assets -p nova_scenario -p nova_mod_format
-p nova_editor -p nova_authoring -p nova_probe_cli --all-targets`. Clean apart
from the two pre-existing lints listed under Owner calls.

A full workspace `cargo test` and a Clippy sweep were deliberately skipped:
they OOM this box.

### Probe ranges

There is no `ui` shard: `20260909-213100` owns it and is still OPEN, so the CI
split was not edited here. Following `20260909-214629`, each affected range was
run directly three consecutive times on the final source (`3bca56ea0`).
Evidence: `probe-runs/3bca56ea0/<range>/{checks.json,report.html,run.log}`.

`system_scenario_picker`, three consecutive runs:

| Run | Verdict | run_end | Playing | invariants | log_clean | markers |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | OK 6/8 | 175 | 170 | 0 violations | PASS | 6 |
| 2 | OK 6/8 | 175 | 170 | 0 violations | PASS | 6 |
| 3 | OK 6/8 | 174 | 169 | 0 violations | PASS | 6 |

Five slugs, one of them emitted twice per run: `the row click selects the row`,
`two or more rows measured`, `the pane split holds across selections`, `the
played row is not the picker's default`, `the picker starts the row the player
clicked`.

`system_settings_persist`, three consecutive runs:

| Run | Verdict | run_end | Playing | invariants | log_clean | markers |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | OK 6/8 | 48 | 44 | 0 violations | PASS | 7/7 |
| 2 | OK 6/8 | 53 | 49 | 0 violations | PASS | 7/7 |
| 3 | OK 6/8 | 47 | 43 | 0 violations | PASS | 7/7 |

`process_exit` and `artifacts_loadable` PASS in all six runs.
`capture_simulated` and `fps_within_baseline` are N/A for both ranges.

Regression ranges: `system_menu_boot` OK, `system_ship_editor` OK.
`system_headless_rebind` reports UNPROBEABLE ("no timeline") because that range
never adds `NovaProbePlugin`, so `probe_marker` no-ops. Pre-existing, present
at `ec035e45d`, not caused here.

### Persistence evidence

The settings run logs the store it wrote before the app is dropped:

```
PersistedSettings { master_volume: 0.40000004, ..., graphics_quality: Low,
..., keybinds: {"main_drive": BindingSpec { keyboard: [Keyboard(KeyJ)],
gamepad: [Gamepad(RightTrigger)] }} }
```

`0.40000004` is a real pointer click: nova's slider tracks carry no
`SliderThumb` or `SliderPrecision`, so bevy's `TrackClick::Snap` maps the click
fraction of the track box straight onto the range. The click lands at
`rect.min.x + rect.width() * 0.4`.

Three apps run in one process against one temporary root. Phases 1 and 2 write
`timeline-edit.jsonl` and `timeline-relaunch.jsonl`; phase 3 carries
`NovaProbePlugin::default()` and owns the graded `timeline.jsonl`.
`ProbeTimeline::create` truncates and holds a flock, so a shared path would
have kept only the last app's markers. `artifacts_loadable` ignores the
siblings.

"Reset only the `main_drive` binding": the Settings UI offers a group-wide
`Reset Bindings` button only. `main_drive` is the run's only override, so that
button is a one-row reset in effect, and the assertions prove exactly that -
`keybinds` empty on disk, `0.4` and `Low` untouched.

### Supporting additions

- `AppBuilder::with_settings_store` in `crates/nova_core/src/lib.rs`. `build()`
  skips `SettingsStorePlugin::from_env()` when a store plugin is already
  present, and `with_game_plugins` suppresses the menu under test, so the range
  needs this to get a writable isolated store on the shipped app.
- `LOGGER_INSTALLED` atomic in `crates/nova_core/src/lib.rs` disables
  `LogPlugin` for every app after the first in a process.
- `EDITOR_SANDBOX_SCENARIO_ID` in `nova_scenario`, consumed by
  `nova_menu::scenarios::picker_lists` and aliased by
  `nova_editor::scenario::SANDBOX_ID`.
- `SYSTEMS_INVARIANTS` in `crates/nova_probe_cli/tests/catalog_drift.rs` moved
  from 324 to 333.

### Defects

`clippy::if_same_then_else` at `crates/nova_assets/src/mod_set.rs:284`,
introduced by `98a3c334d` when the `else if entry.decl.hidden` branch was
removed and the two remaining arms became identical. Found by the targeted
clippy run, fixed in `3bca56ea0` by folding them into one condition. All
affected tests reran green afterwards.

### Owner calls

1. The editor sandbox scenario. Removing `hidden` would have leaked a "Saved
   Range" row into the player-facing picker. It is not a menu backdrop, so
   `menu_backdrop` could not carry it. Resolved with one named ID exception,
   `EDITOR_SANDBOX_SCENARIO_ID`, placed in `nova_scenario` as the lowest shared
   crate. This is a named-ID exception rather than the generic visibility
   switch the task forbids, but it is still a second reason a row is omitted
   and the owner may prefer a different home for the sandbox scenario.
2. Two pre-existing clippy failures are out of this lane and were left alone to
   avoid a land conflict. Both are present at the branch point `ec035e45d`:
   `clippy::doc_lazy_continuation` at `crates/nova_gameplay/src/hash.rs:180`
   and `clippy::filter_next` at `crates/nova_editor/src/scenario.rs:2371`.
3. The probe-range rename got no changelog entry. It is internal tooling and
   `[Unreleased]` has no tooling section.

### Documentation

`CHANGELOG.md` gained two `**(breaking)**` entries under `[Unreleased]`, one in
Scenarios & Objectives and one in Modding & Mod Portal, plus a plain entry for
the campaign lint. Docs updated in `4322910fe`: `docs/development.md`,
`docs/scenario-system.md`, `web/src/create/author-a-scenario.md`,
`web/src/create/base-content.md` (table column `hidden` became `in the picker`,
values inverted), `web/src/create/campaigns.md`, `web/src/create/mod-files.md`
(catalog `hidden` row dropped), `web/src/create/publish-a-mod.md`,
`web/src/create/scenarios.md`, `web/src/wiki/scenarios.md`.
`crates/nova_debug/src/harness.rs` and `examples/systems/README.md` follow the
range rename.

### Dropped

Nothing in scope was dropped. The ranges listed under "Work moved out of this
task" were not implemented here, as instructed.
