# Whole-number scrubbing, and an editable scenario root

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.12.0, editor, scenario

## Goal

Two corrections from round 6 of research record `20260815-231945`
(`POLISH-REVIEW-2.md`), both found while reviewing `c8b432cf`. Neither was in
that round's ranked queue: the first was dropped between round 5 and round 6,
the second was carried only by half.

1. A whole-number field in the inspector drags by 0.1 and snaps back.
2. A scenario cannot author its own starting sky, and the editor's scenario
   root has no editable field at all.

## Part 1: a whole-number field scrubs by whole numbers

`snapped()` rounds any whole value (`nova_editor/src/inspect.rs:1565-1567`),
but `walked()` takes the drag step from `field_spec(&path)` and falls back to
`FREE_STEP` (0.1) when nothing is declared (`:510-513`). One drag step on an
undeclared integer therefore moves 0.1 and rounds straight back to where it
started.

The live victim is `ScatterObjectsConfig.count: u32`
(`nova_scenario/src/actions/spawn.rs:271`) - how many rocks a belt scatters,
reachable from the script editor, and the editor authors belts itself
(`nova_editor/src/scenario.rs:855`). A builder dragging it today sees nothing
move.

Three declarations have already worked around this one field at a time: `SEED`
carries a comment explaining it, and `AMMO_CAPACITY` / `RELOAD_AMOUNT` were
given `step: 1.0` by hand in `c8b432cf`. The rule belongs in the type, not in
the table.

`number_at()` already derives whole-ness from the type (`inspect.rs:1543-1558`);
the fact simply never reaches the row.

- Add `WHOLE_STEP` and a `walked_number(..., whole)` beside `walked()`. It
  raises `nudge` to at least `WHOLE_STEP` for a whole type. A declaration may
  raise the step further; it can no longer set one below 1.
- Route both `RowValue::Number` sites through it: the leaf in `walk()`
  (`:956`, which has the reflected value) and the scalar arm of `walk_option()`
  (`:1094`, which has the payload type path - add a `whole_type()` beside the
  existing `number_type()`).
- Enforce at ROW CONSTRUCTION, not in `nudge_field`. `row.nudge` is what the UI
  turns into `DragRule.step` (`ui/inspector.rs:1345,1576`) and what the tests
  read, so one truth.
- Say in the `FieldSpec::step` doc that the floor for a whole type is
  structural.

Keep `step: 1.0` on `SEED`, `AMMO_CAPACITY` and `RELOAD_AMOUNT`. They read as
intent, and `SEED`'s comment also explains its `Limit`.

Out of scope: an upper bound for `MAX_SCATTER_COUNT`. `Limit` has no ceiling
variant and that is its own decision.

## Part 2: the scenario root is a node like every other node

`1000.0` is the skybox brightness in four places and no scenario can change it:
`nova_scenario/src/loader/lifecycle.rs:264` hardcodes it into
`PendingSkyboxSwap`, `actions/view.rs:148` keeps a private
`DEFAULT_SKYBOX_BRIGHTNESS` whose doc says by hand that it matches the loader,
and `nova_ship/src/camera/skybox.rs:79` is the component default. Brightness IS
reachable mid-mission through `SetSkybox` (`view.rs:172`), which no content
calls - the machinery exists and the starting value is the missing half. The
two shipped cubemaps differ in exposure, which is what made this visible.

Underneath it: `ScenarioNode` is a bare marker (`nova_editor/src/node.rs:83`)
and `scenario_rows()` returns three read-only counts
(`inspect.rs:1877-1888`). The scenario root has never had an editable field, so
every scenario-level value the editor writes is a constant - the sky is
`DEFAULT_SKY`, the description is a literal in `range_scenario()`, the name is
`SAVED_RANGE.name`.

Owner direction: everything at scenario level should be editable there.

### Runtime

- `nova_ship::camera::skybox`: one `pub const DEFAULT_SKYBOX_BRIGHTNESS`, used
  by `Default for SkyboxConfig` and exported from the prelude. `view.rs` drops
  its private copy and imports it, so the comment claiming the two match
  becomes true by construction.
- `ScenarioConfig` gains `skybox_brightness: f32`, serde-defaulted with
  `skip_serializing_if`, following the `hidden` / `menu_backdrop` precedent
  (`loader/mod.rs:184-201`). Old RON parses unchanged and generated base
  content stays byte-identical until a scenario sets one. `ScenarioConfig::new`
  fills the default.
- `lifecycle.rs:264` reads the field instead of the literal.

### Editor

- `ScenarioNode` gains fields and a `Default`, exactly as `ShipNode` carries
  its own (`node.rs:131-155`). `With<ScenarioNode>` filters are unaffected; the
  handful of bare spawn sites take `::default()`.
- The root carries `name`, `description`, `cubemap` and `skybox_brightness`.
- `scenario_rows` walks them beside the three counts. The cubemap needs no new
  machinery: `asset_sort` already returns `AssetSort::Image` for
  `AssetRef<Image>` (`inspect.rs:1219-1222`) and the picker sources its files
  from the declared bundle resources (`asset_index.rs`).
- A `SKYBOX_BRIGHTNESS` spec, unit `lx` and step `50.0`, matching `ILLUMINANCE`
  and `INTENSITY`.
- `EditTargets::edit`'s `FieldRoot::Config` arm gains a scenario-root branch
  (`ui/inspector.rs:2247`). Today that arm falls through to `objects.get_mut`
  and returns `GRIP_GONE`.
- `range_scenario()` (`scenario.rs:515`) takes the values; the save path
  (`bundle.rs:107`) and the Play hand-off both pass the document's, so a change
  shows on Play without saving.
- Open lifts them back onto the root, so they round-trip like every other
  authored field.

### Deliberately out

- `id`. It is the mod install slot, and `bundle.rs:57-63` already decided there
  is one, so the read-only rule stays structural rather than becoming a check.
- `hidden`. A property of the BUILD TARGET, not of the document: `SANDBOX` must
  always be hidden and `SAVED_RANGE` must not (`bundle.rs:73-84`).
- `menu_backdrop`. The lint makes a poseless backdrop an Error, so setting the
  flag means authoring a `SetCamera` too. That is a feature, not a checkbox.
- `thumbnail`. Needs an image the editor cannot produce yet.
- `watches`. The variables DSL, which the script editor owns.
- Brightness VALUES for the four `cubemap_alt` scenarios. They need a live
  render to judge, and round 6 item 8 - explicit zero ambient plus generated
  skybox IBL - changes every brightness judgement. Land the mechanism now and
  tune content after that baseline.

## Proof

Landed 2026-08-30. Every item below passes.

- `a_whole_field_scrubs_by_one_with_nothing_declared_about_it` - `count` on a
  scatter action, nothing declared, scrubs 8 -> 9.
- `a_declared_fractional_field_keeps_its_own_step` - `Seed` still 1.0, a
  turret's `Fire Rate` still 0.05.
- `a_scenario_lights_its_own_sky` (`loader/lifecycle.rs`) - an authored 250 lx
  reaches `PendingSkyboxSwap`; a scenario authoring none gets
  `DEFAULT_SKYBOX_BRIGHTNESS`.
- `thumbnail_and_hidden_default_when_absent_and_round_trip_when_present`
  extended - legacy RON parses, 250 lx round-trips, a defaulted file writes no
  key. `content -- gen` leaves `assets/base/**/*.content.ron` byte-identical.
- `the_scenario_root_is_typed_into_like_any_other_node` - the root's rows are
  typed into and the write lands on `ScenarioNode`; `Skybox Brightness` wears
  its `lx` unit.
- `the_range_settings_the_builder_authored_survive_the_file` - name, blurb, sky
  and brightness go out to the file and come back onto the root.
- `the_scenario_node_counts_what_the_document_holds` and the probe contract row
  updated: the root reads counts first, then what it authors.
- LIVE: `system_ship_editor` under Xvfb, new beat `editor: the root authors the
  range it stands on` - in the running editor the root's panel carries Ships,
  Objects, Name, Cubemap and Skybox Brightness, reading
  `base/textures/cubemap.png` at 1000 lx.

Not proven: clicking the cubemap row's picker in a live run. The example stalls
before it could be reached, at a beat that ALSO stalls on clean master (see
below), and the picker itself is covered by
`a_file_row_offers_the_bundles_files_and_marks_one_they_do_not_ship`. The
cubemap row reaches that picker by its type, naming no field.

## Found alongside, not fixed

`system_ship_editor` stalls at `editor: give it J`: the turret's Key row arms
the capture (the `press key` chip goes up) but the synthesized J never reaches
the row. Confirmed pre-existing - the same run stalls at the same beat with
this work stashed, on master at `87657898`. Not this task's lane.

## Docs

- `docs/scenario-system.md:26` lists the `ScenarioConfig` fields.
- One `[Unreleased]` entry under Scenarios & Objectives. Not breaking: the
  field is serde-defaulted and old files parse.

## Also record

`POLISH-REVIEW-2.md` gets a "considered and declined" note for round 5's
terminal point 4, echoing NovaOS command results to the HUD after close. Of the
five terminal-only capabilities, autopilot already echoes through the AP chip,
`reload` shows in the ammo pips, rebinding shows in the key chips, and the log
and objective list are pure reads. Exactly one has an invisible effect, `ship
repair`, and that is round 6 item 9 - own-ship integrity on the HUD - which is
already queued. No generic pattern is worth building. Owner decision,
2026-08-30.
