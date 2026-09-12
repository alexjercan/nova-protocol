# Remove hidden content flags and prove settings and picker delivery

- STATUS: OPEN
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

- [ ] Delete `ScenarioConfig.hidden` from the Rust and strict RON format. Do not
      retain a serde alias, ignored compatibility field, or replacement generic
      visibility switch.
- [ ] The picker lists every scenario except one with `menu_backdrop: true`.
      Campaign membership does not override this exclusion.
- [ ] Lint a campaign that names a menu-backdrop scenario as an Error. A
      campaign member is a player-launchable chapter; a backdrop is not.
- [ ] Previously hidden chained chapters become ordinary picker rows. Campaign
      members remain grouped in their authored order under their campaign
      header.
- [ ] Migrate generated base content, webmods, examples, tests, comments and
      documentation. Menu backdrops keep `menu_backdrop: true` and no longer
      need a second flag.
- [ ] Treat this as a format break. Old scenario RON that authors `hidden` no
      longer parses and must migrate. Mark the changelog entry `**(breaking)**`.

`menu_backdrop` excludes a scenario from the picker only. It does not make the
scenario unloadable by the ambience system, a direct scenario ID, a test, or a
tool.

## Remove `ModEntry.hidden`

- [ ] Delete `ModEntry.hidden` from the installed-mod catalog format. Do not
      retain an ignored compatibility field or another generic invisible-mod
      switch.
- [ ] Build the player-facing `ModCatalog` from every installed catalog entry.
      Base remains visible and locked on. Downloaded mods keep their existing
      behavior.
- [ ] Remove startup logic that strips hidden non-base IDs from `EnabledMods`
      and remove the synthetic hidden-mod tests.
- [ ] Keep dev and test content out of the shipped installed-mod catalog. Put it
      in its owning example or test fixture instead of installing content the
      player cannot see or disable.
- [ ] Migrate catalog examples, constructors, comments and documentation. Treat
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

- [ ] Record the non-default scenario row selected through a real pointer click.
- [ ] Click the existing Play button through real pointer input.
- [ ] Wait for the atomic scenario-load gate from `20260909-213559` to release;
      do not use an arbitrary frame delay.
- [ ] Assert the resulting `CurrentScenario` ID is exactly the selected row ID.
- [ ] Add the named outcome marker and roster slug.

Campaign grouping and hidden-member launch already have focused menu tests.
Campaign chaining is already proved by `system_outcomes`. Do not duplicate
those claims here.

## `system_settings_persist`

- [ ] Build the shipped `editor_app` on an isolated temporary
      `SettingsStoreRoot` with explicit `SettingsStoreAccess::ReadWrite`, since
      `harness_env_active()` otherwise makes the store inert under
      `NOVA_AUTOPILOT`.
- [ ] Through the real Settings UI, rebind `main_drive` to `J`, set master
      volume to `0.4`, and select the `Low` graphics preset. These values are
      test fixtures, not new defaults.
- [ ] Let the normal save path write the store, inspect the persisted value,
      then drop the app and build a second app on the same root.
- [ ] Assert the relaunched UI and live resources contain the saved values and
      the rebound physical key drives `main_drive` in gameplay.
- [ ] Reset only the `main_drive` binding. Prove the persisted keybind override
      is removed rather than only changing the current `InputBindings`
      resource, while the saved `0.4` master volume and `Low` graphics preset
      remain unchanged.
- [ ] Start another app from a manually authored older partial store and prove
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
