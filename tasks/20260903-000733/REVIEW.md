# Review: The unpushed v0.13.0 work: console, railgun, settings, meters, polish

- TASK: 20260903-000733
- RANGE: origin/master..HEAD (45 commits, HEAD 340d54ad), reviewed as five
  groups; see `TASK.md` for the commit lists
- BRANCH: master

## Round 1

- REVIEWER: craft, performance (static, no measurement slot), correctness,
  contracts. Red team and feel skipped on the owner's instruction.
- VERDICT: REQUEST CHANGES (1 blocker, 16 major, 39 minor). Nothing fixed
  in this round: the owner asked for the findings to be recorded only
  (2026-09-03). Each finding is tagged `easy` (a local change with an
  obvious shape) or `decision` (needs the owner's call on design or
  scope); the decisions are collected at the end.

The code under review is sound where it is new and leaky where it meets what
already existed. The meters migration crossed the seam exactly once on every
path checked and has no correctness finding; its one blocker is a release
contract, not code. The console is wired correctly into its own systems and
loosely into the shared ones: change detection, pause state, input-mode
claims and the run-cheat lifecycle each have a gap at the edge where the
console joins them. The railgun rake has one real edge bug at the fixed-step
boundary. Settings and polish are mostly documentation and test hygiene.

Findings:

### commands (fe92322a)

- [x] R1.1 (MAJOR, easy) `crates/nova_console/src/lib.rs:112-114`,
  `crates/nova_os_ui/src/terminal/input.rs:398` - `run_pending_commands`
  takes `NovaOsTerminal` through `DerefMut` every frame (`get_resource_mut`
  then `take_pending_command` on `None`), and `handle_terminal_keyboard` ends
  with `take_pending_close()` every frame. `resource_changed::<NovaOsTerminal>`
  is therefore true every frame and the three Paint gates at
  `crates/nova_os_ui/src/terminal/mod.rs:282-289` never skip:
  `rebuild_terminal_ui`, `rebuild_nova_os_footer_hints` and
  `reconcile_nova_os_header` rerun in flight, menu and editor, with the
  String allocations each carries. `shell.rs:256` documents the intent this
  breaks. Change: peek through `Deref` (`has_pending_command`,
  `has_pending_close`) and take only on `Some`; same peek for the channel
  drain at `lib.rs:120-123`.
- [x] R1.2 (MAJOR, decision) `crates/nova_menu/src/pause.rs:108-141`,
  `crates/nova_os_ui/src/terminal/shell.rs:670-673`,
  `crates/nova_menu/src/lib.rs:221-224` - `:` from the pause menu enters
  `PauseStates::NovaOs`; the overlay is torn down by `DespawnOnExit(Paused)`;
  Esc in the CRT sets `Unpaused` unconditionally. The player lands live with
  no pause menu, against the design record in task 20260827-120347 ("returns
  to the pause menu underneath"), and the state comment "Only ever entered
  from / exited to `Unpaused`" is false. The named freeze ledger in
  `crates/nova_gameplay/src/freeze.rs` was added so that this case does
  not unpause a paused player (`docs/architecture.md:246-250`), but
  `PauseStates` is one enum: the `Paused -> NovaOs` transition runs
  `OnExit(Paused)` (`crates/nova_menu/src/lib.rs:216-219`), which releases
  the `PauseMenu` hold and despawns the overlay before `OnEnter(NovaOs)`
  takes the `Terminal` hold, so the two owners never overlap and the
  ledger's `is_held_by`/`owners` have no live reader. Decision: record the
  state under the shell when `:` opens it and restore it on close (the
  pause hooks rebuild the overlay), or make "close returns to flight" the
  documented rule; then keep or cut the ledger to match.
- [x] R1.3 (MAJOR, easy) `crates/nova_console/src/cheats.rs:226-232`,
  `crates/nova_scenario/src/loader/lifecycle.rs:50-102,198-224` -
  `RunCheats::begin_new_run` is called only by `scenario load`. Retry
  (`pause.rs:471`), Next scenario (`world.rs:349`) and New Game
  (`menu_ui.rs:512`) trigger `LoadScenario` without it, and the loader's
  teardown resets the outcome but not the cheats: `cheats enable`, lose,
  Retry, `cheats status` still reads ARMED and MARKED. The inverse also
  fails: `scenario load` resets the mark before the loader runs, and the
  loader refuses a scenario with Error-level lint before any teardown, so
  the current run continues with `SuspendedSectionAmmo` on the guns, a
  clean status, and an `ok` ack. Change: move `begin_new_run` into
  `teardown_scenario_entities`, delete both resets from `scenario_load`,
  and read `ScenarioStartFailure` after the trigger to answer with an error.
- [x] R1.4 (MAJOR, decision) `crates/nova_console/src/lookup.rs:60-110`,
  `crates/nova_os/src/commands.rs:208-216,346-354` - `section <id>` and
  `ammo refill section <id>` resolve across every live ship, and section
  ids are per hull (`cargoa` and `cargoa_raider` both carry `turret_port`,
  `assets/base/ships/base.content.ron:405,549`). In every shipped playable
  scenario the command answers "'turret_port' names 2 sections; address one
  by the ship that holds it" and no grammar accepts a ship. The catalog
  examples name `player_turret_1`, an id no base content uses. Decision: a
  `<ship> <section>` (or `ship/section`) address, lookup scoped to one ship,
  real ids in the examples, and a two-ship test.
- [x] R1.5 (MAJOR, easy) `crates/nova_editor/src/lib.rs:257-262` -
  `escape_backs_out` and `backspace_steps_out` run under
  `in_input_mode(InputMode::Normal)` and `ExampleStates::Editor` only;
  nothing claims the keyboard while the CRT is open, so Backspace and
  Escape typed into the shell over the editor also step the editor out. The
  verb set at `:650-655` is gated on `PauseStates::Unpaused`; only this pair
  leaks. Change: add the same gate.
- [x] R1.6 (MAJOR, easy) `crates/nova_menu/src/pause.rs:116-134`,
  `crates/nova_menu/src/lib.rs:208` - `open_command_shell` checks `NovaOs`,
  an armed menu rebind and Loading, not `InputMode::Insert`
  (`crates/nova_ui/src/input_mode.rs:134-141`) or the editor's Bind claim
  (`crates/nova_editor/src/lib.rs:722-723`), so `:` typed into a focused
  editor text field (`inspector.rs:1194`) or during an editor rebind capture
  opens the shell. The task promised `:` stays text in a focused field.
  Change: `.run_if(in_input_mode(InputMode::Normal))` on the registration.
- [x] R1.7 (MAJOR, easy) `crates/nova_os/src/commands.rs:564-574`,
  `crates/nova_os/src/shell.rs:266-274` - `resolve_command` returns
  `UnexpectedArguments` for every arity miss and the headline picks
  "unknown subcommand ''" whenever the command has subcommands, so a bare
  `bind` prints `bind: unknown subcommand ''`. Change: branch on an
  under-run and print `arity.rejection()`.
- [x] R1.8 (MAJOR, easy) `web/src/create/actions.md:649,658` - the
  documented one-section `RefillAmmo` snippet does not parse: `section` is
  `Option<String>` (`crates/nova_scenario/src/actions/ship.rs:588-592`),
  every loader runs ron 0.12.2 with default extensions (no `implicit_some`
  anywhere in crates, assets or webmods), and a bare
  `section: "turret_dorsal"` fails with "Expected option". The lane proved
  it on a scratch copy of `assets/mods/example`: the doc's line dies under
  `content lint --target`, `Some("hull_front")` lints clean. The page's own
  rule (`web/src/create/reference.md:25`) and the sibling rows spell it the
  other way. Change: `section: Some("turret_dorsal")` and the `Some`
  spelling in the table row. Not a BLOCKER: no shipped content carries it.
- [x] R1.9 (MINOR, easy) `crates/nova_console/src/lib.rs:75,83`,
  `crates/nova_os_ui/src/terminal/spawn.rs:40-53` - `run_command_shell` is
  an exclusive `&mut World` system in `Update` with no run condition, a
  sync barrier every frame; `ensure_nova_os_spawned` runs every frame
  declaring `Option<ResMut<Assets<Image>>>` and the CRT material assets.
  Change: run conditions (pending work; `not(any_with_component::<NovaOsRootMarker>)`).
- [x] R1.10 (MINOR, easy) `crates/nova_os/src/terminal/edit.rs:203-204,255`,
  `crates/nova_os_ui/src/terminal/input.rs:234-262` - `pending_command` is
  an `Option` taken once per frame; two submits in one frame (the channel
  can stage two Enters on one tick) drop the first. Change: a `VecDeque`,
  or refuse the second submit.
- [x] R1.11 (MINOR, easy) `crates/nova_console/src/cheats.rs:212-232,165-166` -
  `scenario load` from the main menu loads under the menu and `scenario`
  then reports "main menu / idle"; `speed-cap <ship> inf` installs an
  infinite cap while `nan` is refused. Change: refuse outside Playing;
  require `is_finite`.
- [x] R1.12 (MINOR, easy) dead and duplicated API in the console path:
  `CommandSource::Shell` is never constructed
  (`crates/nova_os/src/commands.rs:1034-1053`) and forces
  `drain_answers_for` plus an `unreachable!` in
  `crates/nova_channel/src/apply.rs:511-514`; `CommandSpec::marks_run`
  (`commands.rs:76`) and `ShellKind::ALL` (`terminal/state.rs:54`) have no
  caller; `command_shell_specs` clones a whole `OnceLock<Vec>` per call
  (`commands.rs:368-384`); `extend_shell_scrollback` re-implements the
  `MAX_SCROLLBACK_ROWS` drain and falsifies the `after_scrollback_change`
  doc (`terminal/state.rs:468-483,498-500`).
- [x] R1.13 (MINOR, easy) `crates/nova_console/src/dispatch.rs:30-31`,
  `crates/nova_console/src/lib.rs:142-153` - `clear` and `close` go
  through the executor and are re-identified by string match in
  `answer_the_shell`; from the channel they ack `ok` and do nothing.
  Change: handle them in `submit_command`
  (`crates/nova_os/src/terminal/edit.rs:191`) and refuse them from the
  channel.
- [x] R1.14 (MINOR, easy) `crates/nova_console/src/lookup.rs:52-65`
  re-implements `live_ship_sections`
  (`crates/nova_scenario/src/actions/ship.rs:644-647`, whose doc says it is
  public for the console); `ammo_infinite` diffs `SuspendedSectionAmmo`
  presence before and after because `apply_infinite_ammo` returns `()`
  (`cheats.rs:68-83`); the same four-line `or_error` match appears seven
  times (`cheats.rs:64,103,133,183`, `inspect.rs:81,111,142`).
- [x] R1.15 (MINOR, easy) preludes and doc links: `crates/nova_console/src/lib.rs:66`
  imports `nova_os_ui::terminal::NovaOsSystems` by path (not in the
  prelude); `crates/nova_channel/src/apply.rs:197,592` write
  `nova_os::prelude::CommandClass` with the prelude imported; the link
  reference definitions at `crates/nova_os/src/commands.rs:16` and
  `terminal/state.rs:159` point at the docs.rs front page.
- [x] R1.16 (MINOR, easy) test coverage: `lookup::resolve`, `scenario_load`,
  the console volume bounds, bind sorting, and `actions::ship`
  `SetInfiniteAmmo`/`RefillAmmo` have no tests; no example range drives the
  command shell (the proof is `poc/drive_commands.py`).
- [x] R1.17 (MINOR, easy) `CHANGELOG.md:60-62` - "`help`, completion and
  the wiki catalog all generated from one registry" is not true: nothing
  reads or writes `web/src/wiki/commands.md` from `COMMAND_CATALOG`, and
  `docs/keeping-docs-in-sync.md:79` documents the page as hand-kept. The
  27 names, usage strings and classes do match today (re-derived by the
  lane). Change: say the wiki catalog is kept by hand, or add the check.
- [x] R1.18 (MINOR, easy) `docs/architecture.md:242-243` - "Both frozen
  variants enter only from `Unpaused` and exit back to it, never into each
  other" is false at HEAD (`Paused -> NovaOs` is live) and the bullet at
  `:246-250` says so. Settle it with the pause-exit decision above and
  list the real transitions.
- [x] R1.19 (MINOR, easy) the removed `infinite_ammo` flag survives in
  prose: `crates/nova_scenario/src/objects/spaceship.rs:52-53` promises
  "an infinite-ammo flag" on `PlayerControllerConfig`;
  `docs/guide-add-section.md:118-120` tells a contributor to add
  "infinite-ammo handling" following arms this commit deleted;
  `webmods/gauntlet/gauntlet.content.ron:135-138` keeps a four-line
  comment explaining a field that is no longer under it. Change: delete
  the three passages.

### railgun (afa27e08, 65d9d0e7, e1e5bc73, 532c1ef8, f9c8aa56)

- [x] R1.20 (MAJOR, easy) `crates/nova_gameplay/src/rounds.rs:764-766,887-913` -
  a body's arming point is forgotten at the fixed-step boundary. A body
  armed in an earlier step re-enters `collect_rake_contacts` with
  `armed_at` 0.0, and the rear cap admits a collider down to
  `-radius - sqrt(radius^2 - offset^2)` behind the step start, so the
  sphere charges a lateral it passed BEFORE the tip struck that body,
  against the rule at `:760-762`. With the file's own fixture: bore cell at
  the origin, a pod at (1, 0, 1.5), `spawn_lance_slug(.., Some(1.0))` from
  `RAKE_START_Z`; the step starting at z 1.3125 arms the bore cell and
  correctly skips the pod (`elapsed` 0 < `armed_at`); the next step charges
  it (`lead` -0.75, hypot 0.90 <= 1.0, `elapsed` 0 >= 0.0). At lance speed
  the leak fires whenever the arming hit lands within about 1.9 units of a
  step end, so the same shot into the same hull bites one more cell
  depending on the distance it was fired from. Change: carry the arming
  mark in the round's own frame (accumulated flight time on `RoundRake`)
  and compare the unclamped `clock + depth / pace` against it; clamp only
  for the sort key and the stop position. Pin it with
  `a_section_the_sphere_passed_before_the_hit_is_not_raked_on_the_next_step`.
  Not a BLOCKER: the extra bite lands on a hull the shot did hit, and it
  needs a section standing forward of the bore column inside the radius.
- [x] R1.21 (MINOR, easy) `crates/nova_ship/src/sections/railgun_section/wake.rs:290-297,346-350`,
  `crates/nova_gameplay/src/rounds.rs:683,824` - the wake never receives
  the slug's last segment. `advance_rounds` writes the hit and despawns the
  slug in one flush; `follow_railgun_wakes` then finds no slug and retires
  the emitter at the previous frame's anchor, and
  `count_railgun_wake_spawns` zeroes the spawn count on `retiring` before
  charging `covered`. A shot into a hull leaves no haze over the last
  render frame of flight (about 250 m at 60 fps, more when a frame spans
  several fixed steps). Neither bench fires into a target, so the feel pass
  could not see it. Change: an `On<Remove, RailgunSlugProjectileMarker>`
  observer that copies the final `Transform` onto the emitters and marks
  them retiring; charge `covered` once more before returning 0.
- [x] R1.22 (MINOR, easy) `crates/nova_gameplay/src/transient_light.rs:270-275` -
  `a_light_holding_a_slot_counts_against_the_next_flash` cannot fail:
  `World::trigger` queues the observer's command and never flushes, and
  `lit_count` reads through `query_filtered` before the flush, so the count
  is 0 either way. Delete the `CappedLight` arm of the filter at `:137` and
  the test still passes. Change: `app.update()` before the assertion, as
  its siblings do.
- [x] R1.23 (MINOR, decision) `wake.rs:157-185,227` - the two wake graphs
  are built on the first slug of a session (two `EffectAsset` graphs, two
  WGSL generations, six pipeline compiles on the shot frame; synchronous on
  web and macOS, late or absent on native for a 1.2 s slug). Documented as
  deliberate ("an app that never fires a lance builds nothing") and the
  same lazy shape as the torpedo blast. Decision: warm the pair at scene
  load when `budget.particles` is on, or accept the first-shot hitch.
- [x] R1.24 (MINOR, easy) `crates/nova_gameplay/src/rounds.rs:532-545,609,764-779,892-927` -
  per-step work in the sweep: `Collider::sphere(ROUND_RADIUS)` is rebuilt
  per round per step (once per system run before afa27e08);
  `rake.armed.clone()` per step; `TipWalk::found` pushes onto a heap Vec on
  every first hit including the narrow path; every armed body is walked in
  full for the rest of the slug's life. Change: build the sphere once in
  `advance_rounds` and lend it; iterate `armed` by reference; a fixed array
  like `rejected`; a per-body bound at arming.
  The first three landed as written. The fourth landed as a RETIREMENT rather
  than a bound: a body whose every collider is already charged can never
  contribute another contact - both halves of the sweep skip a charged
  collider - so it is dropped from `armed`. A bound taken at arming would have
  had to survive a body that accelerates, which this does not.
- [x] R1.25 (MINOR, easy) rules spelled out more than once: the "lit set"
  filter (`transient_light.rs:131-137`, `wake.rs:369-376`,
  `examples/playable/railgun_wake_bench.rs:879`); the haze's longest
  lifetime as a bare 1.3 (`wake.rs:296,473`) while the filament graph
  (`:541`) disagrees with the factor; the authored-or-narrow rake rule and
  the slug bundle copied into `examples/systems/system_railgun_lance.rs:994-1032`
  from `firing.rs:215`; `check_railgun_charge` now also validates
  `rake_radius` (`crates/nova_scenario/src/lint/ship.rs:222,234-245`);
  `wake.rs:53` imports past the prelude already in scope. Change: one
  `pub fn` per rule (`RailgunSectionConfig::rake() -> Option<Meters>`,
  a lit-slot query in `transient_light`), delete the import, rename the lint.
- [x] R1.26 (MINOR, decision) `wake.rs:207-233,290-298` - every shot
  spawns two fresh hanabi instances with their own GPU slabs (about 0.6 MB),
  freed about 1.9 s after impact. Rare at the shipped cadence. Keep unless
  a GPU-host set shows a spike; otherwise one emitter pair per lance.
  KEPT: the owner's call (2026-09-03). No code change; revisit only if a
  GPU-host set shows the spike.
- [x] R1.27 (MINOR, easy) creator and player docs behind the rake and the
  wake: `web/src/create/base-content.md:58` lists every authored railgun
  number except `rake_radius: Some(10.0)`, and a creator who authors from
  that table ships a needle (59 dps against 178 in the task's own bank);
  `web/src/wiki/sections/railgun.md:71-73` ends its visuals at the charge
  bolt and `web/src/wiki/settings.md:48` still lists only "torpedo and
  muzzle particle bursts" as what Low does not draw, while the wake is
  gated on `budget.particles` (`render.rs:153-165`) and the slug light on
  `transient_lights` (`wake.rs:367-372`). Change: the rake in the catalog
  row; one sentence for the wake and the light; "railgun wake" in the Low
  list.
- [x] R1.28 (MINOR, easy) `CHANGELOG.md:92-94` - the `rake_radius` Modding
  entry revises a `Railgun` kind the `[Unreleased]` Modding block never
  introduces (v0.12.0 has no railgun section), against the collapse rule.
  Change: one Modding entry that introduces the kind with its fields, and
  fold the rake into it.
- [x] R1.29 (MINOR, easy) `web/src/wiki/sections/railgun.md:50,56` asks for
  `assets/loops/loop-section-railgun.webm` and `scripts/capture-web-media.sh:46-66`
  has no row that can produce it, nor a pending slot the way
  `gen-web-screenshots.py:148-151` has for the four railgun stills; a full
  media pass fills every section page but this one and never reports the
  gap. Change: a `loop_vfx_range|loop-section-railgun||` row (the range
  fires the lance every pass) or a pending row.
- [x] R1.30 (MINOR, easy) `docs/environment-variables.md:143-145` - the
  example-local knob list omits `NOVA_VFX_RANGE_BARE_SLUG`
  (`examples/screenshots/loop_vfx_range.rs:104,215`, named in
  `CHANGELOG.md:161`); the page's own audit recipe then finds an
  unexplained `env::var` site and `tests/env_contract.rs:106` excludes
  examples. Change: add it to the line.

### settings (e920c49e, e9c9e3c6)

- [x] R1.31 (MAJOR, easy) `crates/nova_menu/src/tests/settings_store.rs:53-56,95-98`,
  `crates/nova_menu/src/tests/support.rs:69-103` - the two store tests set
  the process-wide `NOVA_CONFIG_ROOT` to a root holding a non-default store
  (look 300 percent, quality Low), and every `app()` fixture reads the env
  unguarded, so a co-scheduled panel test loads look 300 and fails. The
  lane reproduced it: `cargo test -p nova_menu --lib the_mouse_group_shows
  a_menuless_app_boots an_inert_store -- --test-threads=3` failed 183 of
  300 runs. The env restore at `:75,:128` is skipped on a failed assert (no
  Drop guard), and the shared fixture goes through `from_env`, so
  `NOVA_CAPTURE=1 cargo test` fails
  `a_setting_edited_just_before_quitting_is_still_saved`. Change: an
  explicit storage root on `SettingsStorePlugin`, `load_settings` and
  `save_settings` (`None` = env) with the store tests passing their own
  root and a Drop guard; or move the store tests to an integration binary
  under `crates/nova_menu/tests/`.

  TAKEN: the explicit root, as `SettingsStoreRoot` - a resource the plugin
  inserts from its own `root: Option<PathBuf>` field, with `None` meaning
  the platform store. `nova_assets::storage::platform_at` is the seam
  (native picks the root, wasm has one origin and ignores it). No fixture
  in `nova_menu` touches `NOVA_CONFIG_ROOT` now, the mutex and the `Once`
  are gone, and `app()` adds the store itself with `live: true` so the
  fixture no longer reads the process env either. Verified: the
  reproduction at `--test-threads=3` is 0 of 60 (was 183 of 300);
  `NOVA_CAPTURE=1` and `NOVA_AUTOPILOT=1` both run the full
  `-p nova_menu --lib` green.
- [x] R1.32 (MAJOR, easy or decision) `examples/playable/railgun_wake_bench.rs:706-712`,
  `crates/nova_menu/src/settings_store.rs:269-275` - a hand-run bench (no
  harness env) carries a live store; pressing G writes `GraphicsQuality`
  and `persist_settings_on_change` saves the bench's tier into the player's
  `settings.ron`. At v0.12.0 a `with_game_plugins` app had no store. Easy:
  the bench keeps its own tier resource. Decision: split load from save so
  saving exists only where a settings panel does.

  TAKEN, the easy road, and not the one written above: the bench adds
  `SettingsStorePlugin { live: false }` itself, which `AppBuilder`'s guard
  then honours. A tier resource of the bench's own would have left the
  reading dependent on the developer's saved preset, which a measurement
  tool must not be; inert closes both directions in one line. The
  load/save split stays OPEN - it is the only thing that would let a
  `with_game_plugins` app read the player's settings without being able to
  write them, and nothing here needed that yet.
- [x] R1.33 (MAJOR, easy) `web/src/wiki/settings.md:69-78`, `web/src/wiki/keybinds.md:3` -
  the Controls page does not know the MOUSE group exists, and the
  mouse-look default change (v0.12.0 Scale 0.001, now two thirds of that at
  the 200 percent default) has no player note. Change: a Mouse subsection
  (Look 100-300 percent default 200, RCS 100-500 default 100, Free Camera
  100-300 default 200; mouse only; live from the pause menu; no Reset on
  that page) and MOUSE in the group lists.
- [x] R1.34 (MAJOR, easy) `docs/environment-variables.md:47,49`,
  `docs/development.md:393-398,783-790`,
  `crates/nova_gameplay/src/settings.rs:132-138` - `NOVA_AUTOPILOT` and
  `NOVA_CAPTURE` gained a second effect (the store is inert: no load, no
  save, no window mode) and no page states it; `development.md:783-790`
  still says `XDG_CONFIG_HOME` lets probe read your `settings.ron`.
  `docs/architecture.md:211-219,:16` omits `SettingsStorePlugin` from the
  assembly order and says persistence is the menu's. Change: document the
  gate in both env rows, the SILENT paragraph and the `HARNESS_ENVS`
  docstring; qualify the probe profile paragraph; add the plugin row.
- [x] R1.35 (MINOR, easy) `crates/nova_menu/src/settings.rs:1541-1551` -
  `sync_sensitivity_slider` rewrites three Text labels every frame the
  MOUSE page is open, copying the volume slider's shape (`:1526-1536`).
  Change: `Changed<SliderValue>` as `crates/nova_ui/src/slider.rs:209-222`
  does, or compare before assigning.
- [x] R1.36 (MINOR, easy) `crates/nova_menu/src/settings_store.rs:238-244`,
  `crates/nova_perf_web/src/main.rs:31-49` - `from_env` cannot be inert on
  wasm (`var_os` is always `None`), so `perf_web` carries a live store and
  a stored blob on the same origin overwrites `?quality=` at startup; the
  "inert under a scripted run" rustdoc at `crates/nova_core/src/lib.rs:381-383`
  does not hold there. Change: `perf_web` passes `SettingsStorePlugin { live: false }`.
- [x] R1.37 (MINOR, easy) `examples/systems/system_headless_rebind.rs:23-26,165-176`,
  `examples/systems/system_headless_drag.rs:25-27` - the marker
  `outcome: the rebind store starts isolated` is vacuous: under
  `NOVA_AUTOPILOT` the store is inert and load never runs, so the overrides
  are always empty. No example range covers `MouseSensitivity` at all.
  Change: drop the marker and the `set_var` with a note, or assert the
  gate; extend `system_headless_drag` with a MOUSE beat, a Scale readback
  and a catalog row.

  TAKEN: the marker ASSERTS the gate instead of being dropped. A new
  `SettingsStoreLive` resource records what the plugin decided, the rebind
  range asserts it is inert (the empty override set is asserted with it,
  as the consequence it is), and the slug is now
  `the rebind run's settings store is inert`. Both examples' own
  `NOVA_CONFIG_ROOT` writes are gone - the gate is what isolates them, and
  it is now checked rather than hoped for. `system_headless_drag` gained
  four beats to CONTROLS -> MOUSE and a second drag with three readbacks:
  the slider percent, `MouseSensitivity`, and the live `Scale` on the
  flight rig. Run: 200% -> 228.04%, both new markers filed.
- [x] R1.38 (MINOR, easy) craft: the write path clamps the same value three
  times and the read a fourth (`crates/nova_input/src/sensitivity.rs:188-194,202-214`),
  with a comment at `:326-327` naming a store-load writer that does not
  exist; `settings_store.rs:122-124` and `:155-157` read one value two
  ways and one default is named three ways; the store plugin's systems
  live in the panel module and `settings_store.rs` imports them back
  (`settings.rs:1250-1420`); the harness-env sweep at `settings_store.rs:240-242`
  copies `crates/nova_gameplay/src/settings.rs:143-145`;
  `build_controls_tab` takes a tuple and destructures it
  (`settings.rs:802,810`); `crates/nova_input/src/lib.rs:36` imports past
  the module prelude; the store rationale is written four times
  (`crates/nova_core/src/lib.rs:378-389`) and the header of
  `tests/settings_store.rs:4` claims no example builds a menu while
  `system_outcomes.rs:66` does.
- [x] R1.39 (MINOR, decision) `crates/nova_menu/src/settings.rs:1318-1334` -
  the 15-frame debounced save can `write_atomic` (fsync) on the main
  thread in the first gameplay frames after Resume. Pre-existing path,
  now with one more resource behind it. Options: `IoTaskPool`, or flush on
  `OnExit(Paused)`.

  ACCEPTED (the owner's call): a NOTE sits where the save fires, naming
  the two fixes. The system moved to `settings_store.rs` with the rest of
  the store, so the note is at `persist_settings_on_change` there.

### units to meters (fe92322a..540f5834, f3952cf8, 40c068d2)

- [ ] R1.40 (BLOCKER, easy) `webmods/the-ledger/the-ledger.bundle.ron:38`,
  `webmods/gauntlet/gauntlet.bundle.ron:16` - both portal mods carry x10
  content under version strings that already shipped: the-ledger 1.26.0
  was bumped in a3c8ea71 inside tag v0.12.0, gauntlet 1.10.0 in 96c361f9
  inside v0.11.0. The portal's update decision is an exact string compare
  (`crates/nova_menu/src/portal.rs:245` prints "installed"; `:521` gates
  the Update button), and `DB_VERSION`/`PORTAL_SCHEMA_VERSION` are
  unchanged. A player who installed either mod on v0.12.0 opens v0.13.0,
  sees "installed", gets no Update, and the new build reads the cached
  old-unit file as meters: a speed trip at 8 m/s, picket zones 24 m across,
  rocks 2 m, area triggers that never fire. Deploy also overwrites the
  published `mods/the-ledger/1.26.0/` in place (`scripts/gen-portal.py:711`,
  `.github/workflows/deploy-page.yaml:111`), against "the portal keeps
  every published version". Change: bump to 1.27.0 and 1.11.0 with
  changelog entries (the-ledger's CHANGELOG tops out at 1.25.0 and has no
  1.26.0 entry); `assets/mods/example` ships in-tree and needs a bump for
  consistency only.
- [ ] R1.41 (MAJOR, easy) `CHANGELOG.md:83-85` - the breaking entry says
  "Every distance and speed in a mod or scenario is ten times its old
  number", which is false for build-grid geometry: colliders, link points,
  joint and mount offsets, exhaust cones and `sections[].position` stay in
  cells. A section author who follows it and x10s a collider gets an
  unusable part. Change: "... Build-grid geometry - colliders, link points,
  mount offsets, part poses - stays in cells." (191 chars).
- [ ] R1.42 (MINOR, easy) `crates/nova_ship/src/input/ai/acquisition.rs:87-88` -
  the rustdoc says the AI range is "Shorter than the player's
  TARGETING_MAX_RANGE (20 km)"; that constant is 20 000 engine units, 200
  km (`crates/nova_ship/src/input/targeting/contacts.rs:17`). Change: "(200 km)".
- [ ] R1.43 (MINOR, easy) `CHANGELOG.md:31-33` - the railgun rake entry is
  204 chars once joined (cap 200); raised by two lanes. Change: trim
  ("through everything in the line" to "through everything in line").
- [ ] R1.44 (MINOR, easy) authored layouts still written in world units and
  wrapped per field with `from_engine`: `crates/nova_editor/src/scenario.rs`
  (28 sites, e.g. `:229,:773,:857`), `examples/playable/carve_asteroids.rs`
  (18 sites, e.g. `:212,216,811,815`), `examples/screenshots/screenshot_damage_levels.rs:122,585`.
  Change: literal-only sites as `Meters(4000.0)` and the like; keep
  `from_engine` only for values derived from stage geometry.
- [ ] R1.45 (MINOR, easy) `METERS_PER_UNIT` is exported through the
  preludes (`crates/nova_events/src/units.rs:88`) against its own doc
  ("Nothing outside an engine boundary should name it");
  `crates/nova_authoring/src/base_content/sections/mod.rs:125-131`
  re-derives `from_engine` by hand; `examples/playable/railgun_wake_bench.rs:346-348`
  divides a density by the constant; `crates/nova_scenario/src/objects/light.rs:119-139,184`
  holds meters in a bare `Vec3` and wraps at use. Change: drop the
  constant from the preludes; delete the loop; type the tuple `Meters3`.
- [ ] R1.46 (MINOR, easy) hand-mirrored constants: `SURFACE_MARGIN` twice
  in `crates/nova_authoring/src/shakedown/tests/pins.rs:456-457,702-703`
  while `gravity.surface_margin` is in scope, and `Meters(200) * 30` at
  `:489` mirrors the private `BEACON_LOCK_SIGNATURE`; `TORPEDO_ENVELOPE`
  (`crates/nova_authoring/src/balance.rs:61-65`) mirrors the private
  `AI_TORPEDO_MAX_RANGE`; the default muzzle speed appears in two registers
  three lines apart (`crates/nova_ship/src/sections/turret_section/config.rs:300,305`);
  the cap readout format is duplicated (`crates/nova_console/src/cheats.rs:197-200`,
  `inspect.rs:383`) and "metres" survives at `cheats.rs:160,166`,
  `inspect.rs:381`. Change: export and derive; one `cap_label` fn.
- [ ] R1.47 (MINOR, easy) `crates/nova_editor/src/inspect.rs:936-941` vs
  `:983-988` - `quantity_inner` treats any one-scalar tuple struct as a
  quantity "whatever it is called" and `quantity_unit` labels every unknown
  wrapper "m". Change: `Option<&'static str>` from `quantity_unit`, or
  enumerate the types and fix the doc.
- [ ] R1.48 (MINOR, easy) stale comments and doc paths: the x10 attributed
  to "the shared player-facing distance policy" at
  `crates/nova_hud/src/torpedo_target.rs:440-441,449-450`,
  `crates/nova_os_ui/src/map/contacts.rs:116-117`, `map/tests.rs:20`,
  `crates/nova_scenario/src/objects/spaceship.rs:133,164` (130-column doc
  lines) while `crates/nova_ui/src/units.rs:7` now says nothing converts;
  `crates/nova_editor/Cargo.toml:27`; doctests in `nova_ui/src/units.rs`
  and `nova_events/src/units.rs` import `Meters` past the prelude.
- [ ] R1.49 (MINOR, decision) weapon reach and muzzle speed are re-derived
  from SI config every tick (`crates/nova_ship/src/input/point_defense/mod.rs:81-85`,
  `turret_section/aim.rs:357`, `input/ai/railgun.rs:229`,
  `input/ai/torpedo.rs:169`, `crates/nova_hud/src/bore_sight.rs:250-262`)
  against the task's own cache-at-spawn rule (`TorpedoBlast`,
  `FlightSpeedCap`), and `ScriptedCameraPose` converts every frame
  (`crates/nova_scenario/src/loader/mod.rs:635-641`,
  `crates/nova_menu/src/ambience.rs:195-201`). Sub-10 us at 500 mounts;
  consistency debt, not cost. Options: cache engine-side at spawn, or
  record the rule as "cache where the hot loop is a physics step".

### polish (b6aa4289, 9c6fa3d7, 5287cdf5, 718ebfd2, 2f5a8c75, 8882ec39)

- [ ] R1.50 (MAJOR, easy) `examples/playable/wfc_arena.rs:853,1365`,
  `examples/playable/wfc_arena/lobby.rs:592,603` - the replay instruction
  logged at every lobby-started match names a head that fields a different
  matchup. The lobby open drafts from head H and logs "`--seed H` fields
  this matchup again" (correct), then sets `next_seed` to max(drafted)+1;
  Start overwrites `roster.seed` with that cursor and `draft_roster` logs
  "`--seed max+1` fields this matchup again" although every slot is pinned.
  Running that seed drafts different hulls and different derelict dressing
  (`wfc_arena.rs:1245,1264,1295` key off `roster.seed`). The last line in
  the log, the one a player copies, is the wrong one; scripted runs skip
  the lobby and stay right. Change: log the head only where a slot-less
  draft walks from it, print the per-slot replay (`--ship team:style:seed`)
  or the original head at a pinned start, and keep the resolved head in its
  own field instead of reusing `roster.seed` as the lobby cursor.
- [ ] R1.51 (MAJOR, easy) `crates/nova_autopilot/src/completion.rs:170,188`,
  `docs/automation-harness.md:472-481`, `docs/environment-variables.md:48`,
  `CHANGELOG.md:178-180` - the run-level `NOVA_AUTOPILOT_DEADLINE`
  backstop, which this group's contract hands to every step without a
  `.deadline`, counts `Res<Time>`: the virtual clone, which the pause
  overlay and the ship computer pause (`crates/nova_menu/src/lib.rs:214-228`
  into `crates/nova_gameplay/src/freeze.rs:131`). A script that opens
  either in an app that carries `NovaMenuPlugin` and then waits without a
  deadline on an `until` that never holds hangs until a supervisor kills
  it, with nothing naming the step or the collector; the docs promise an
  error exit naming the laggards. `elapsed()` reads the same clock by its
  own documented design, but the page names the clock for `deadline` only.
  Pre-existing at v0.12.0, and every changed range that walks into a hold
  gives its waits a deadline, so no shipped script hits it today; the
  changelog entry claims the pause case closed, and for the documented
  default it is not. Change: `Time<Real>` in `completion_watch` (the test
  at `completion.rs:289` already claims wall time), and one sentence in
  the Deadlines section on which clock `elapsed` and the watcher read.
- [ ] R1.52 (MINOR, easy) `examples/systems/system_menu_boot.rs:117` -
  "menu_boot: reach the main menu" waits on `state_is(GameStates::MainMenu)`
  with no `.deadline`, the only world-predicate wait in the changed fleet
  without one; a boot that never reaches the menu holds the run until the
  outer `NOVA_AUTOPILOT_DEADLINE` kills it with no step named. Change:
  `.deadline(BOOT_SECS)`.
- [ ] R1.53 (MINOR, easy) `crates/nova_autopilot/src/autopilot.rs:254-262,597`,
  `examples/systems/system_headless_crt.rs:257-258,371` - `click_named`'s
  aim beat re-issues the whole hover every frame it is current (String
  clone, a fresh `QueryState`, `Window` marked changed, two `CursorMoved`
  messages), and if the node box vanishes mid-aim `resolve` warns once per
  frame for up to 20 s; the driver clones the current `Step` every driven
  frame, which lands in probe fps runs; `resolve_blip` clones the target
  code per poll and `pick_the_target` rebuilds and sorts the contact list
  per frame. Driven runs only. Change: build the hover once outside the
  closure and re-aim only when the centre disagrees with the pointer;
  `Arc<[Step]>` or a borrow.
- [ ] R1.54 (MINOR, easy) helpers not used where they exist: the two-beat
  click is still hand-spelled in `examples/playable/widget_zoo.rs:736-797`,
  `examples/systems/system_field_controls.rs:220-255`,
  `system_input_modes.rs` and `system_headless_pointer.rs:90-108` where
  `AutopilotPlugin::click_named` exists; three NOVA OS predicates are
  duplicated in four files (`examples/screenshots/shared/computer.rs:199-226`,
  `system_nova_os.rs:206-233`, `system_headless_crt.rs:398-401`);
  `Gestures::click` and `EditorGestures::click_a_widget` are identical
  wrappers with different deadlines (`ui_walk.rs:304-306`,
  `system_ship_editor.rs:3506-3511`); `REACT_SECS`/`LOAD_SECS` are local
  copies of the harness deadlines (`system_nova_os.rs:88-97`).
- [ ] R1.55 (MINOR, easy) docs: `crates/nova_autopilot/src/autopilot.rs:149`
  links the removed `AutopilotPlugin::hold` (dangling rustdoc link) and the
  module table at `:12` says each predicate takes only elapsed seconds
  (`:91` now also passes a frame index); `examples/playable/wfc_arena.rs:135-137`
  keeps a stale rand comment; `computer.rs:30-35`, `system_nova_os.rs:55-58`
  and `system_headless_crt.rs:55-59` import `nova_os_openness` by path;
  comments narrate history at `autopilot.rs:707-711` and
  `crates/nova_debug/src/harness.rs:145-147`.
- [ ] R1.56 (MINOR, easy) `docs/automation-harness.md:472,480`,
  `crates/nova_autopilot/src/autopilot.rs:454-461`,
  `crates/nova_autopilot/src/loops.rs:49-58,344` - "in-step REAL seconds"
  is rendered frames over the profile fps inside a loop capture:
  `LoopCapturePlugin` pins `TimeUpdateStrategy::ManualDuration`, which
  Bevy applies to `Time<Real>`, so a `.deadline(60.0)` at 30 fps is 1800
  rendered frames (about ten wall minutes on llvmpipe at 3 fps) and the
  default run backstop is 3600. Longer waits only, never a wrong verdict.
  Change: one sentence in the Deadlines section and the loops module doc.

Considered and NOT raised:

- Meters correctness: every runtime path checked crosses the seam exactly
  once on the engine side (blast radius, weapon reach and AI fire range,
  speed cap, gravity, beacon lock, scenario actions and objects, HUD and
  NOVA OS readouts, editor inspect and gizmos), every quantity line of the
  22 `*.content.ron` files is exactly x10 against fec6e441, and the web
  helpers keep SI constants. No finding.
- The lazy first-shot build of the wake graphs as a MAJOR: the rustdoc
  states the trade, the torpedo blast ships the same shape, and the owner
  measured. Recorded as a decision (R1.23) instead.
- The `infinite_ammo` removal as a compatibility break: a v0.12.0 file
  with the field still loads (no `deny_unknown_fields` anywhere in the
  loader crates; the lane re-added it to a scratch copy and linted clean),
  the CHANGELOG marks it breaking with the migration line, and the creator
  pages were updated.
- `rake_radius` as a format change: the `Railgun` kind did not exist at
  v0.12.0, so no migration is owed; the base RON `Some(10.0)` matches the
  builder through `#[serde(transparent)]`.
- The rake ignoring a body's angular velocity in its rest frame: both
  halves of the sweep did so before this group.
- The settings tests' shared fixture reading the env: folded into R1.31.
- `system_headless_crt`'s `not(the_target_is_selected())` being true while
  a blip is unresolvable: no despawn path in `shakedown_run` produces one.
- The autopilot's per-frame `Step` clone and `click_named` re-aim as more
  than MINOR: driven runs only, and the owner's probe sets were quiet.
- `nova_channel`'s `std::thread` and `Instant`: pre-existing, and the crate
  stays behind the `debug` feature so it never reaches the wasm bundle.
- The console crate's exclusive-system barrier as MAJOR: precedent exists
  in the tree; recorded MINOR (R1.9).
- `loop_cockpit` waiting on `elapsed(0.8)` after Tab opens NOVA OS, which
  R1.51's chain says can never advance: `AppBuilder` adds `NovaMenuPlugin`,
  and with it the clock hold, to the default app only
  (`crates/nova_core/src/lib.rs:394-398`), so a `with_game_plugins`
  example never pauses the clock when the CRT opens. The loop cuts; the
  hang needs the menu.

Verified (not taken on trust), in this session:

- R1.1: `crates/nova_console/src/lib.rs:112-114` takes the resource
  through `DerefMut` on every frame; the three Paint gates at
  `crates/nova_os_ui/src/terminal/mod.rs:282-289` run on
  `resource_changed::<NovaOsTerminal>`; `input.rs:398` takes the close
  flag every frame.
- R1.2: `open_command_shell` returns early only for `NovaOs`, an armed
  rebind and Loading, then sets `NovaOs`; `OnExit(Paused)` releases the
  pause hold and `DespawnOnExit(Paused)` tears down the overlay;
  `shell.rs:670-673` sets `Unpaused` unconditionally; task
  20260827-120347 records the intended landing as the pause menu.
- R1.3: `begin_new_run` has one caller outside tests; the loader's
  teardown resets the outcome only; `lifecycle.rs:198-224` returns before
  teardown on Error-level lint.
- R1.4: `turret_port` appears on both cargoa hulls in
  `assets/base/ships/base.content.ron:405,549`; `lookup::resolve` refuses
  a multi-hit with no ship qualifier in the grammar.
- R1.5 and R1.6: the editor pair's run conditions at `lib.rs:257-262`; the
  verb set's `PauseStates::Unpaused` gate at `:650-655`; the only
  `ClaimKeyboard` writers are the editor's Browse and Bind claims and
  nova_ui's Insert; `open_command_shell` is registered with resource
  guards only.
- R1.7: the `UnexpectedArguments` headline at `commands.rs:564-574` and
  the `bind reset` subcommand.
- R1.8: `actions.md:649` carries a bare string for an `Option<String>`
  field; ron 0.12.2 in `Cargo.lock`; no `implicit_some` in crates, assets
  or webmods.
- R1.20: the rear-cap arithmetic in `collect_rake_contacts` admits a
  collider down to `-radius - sqrt(radius^2 - offset^2)` behind the step
  start and `armed_before` bodies pass `armed_at` 0.0.
- R1.23: `RailgunWakeArt::handle` builds each graph on first use, by
  its own rustdoc.
- R1.32 and R1.36: `railgun_wake_bench.rs:707-711` writes
  `GraphicsQuality` on G; `SettingsStorePlugin::from_env` reads
  `std::env::var_os`, which is `None` on wasm.
- R1.40: the-ledger `1.26.0` was introduced by a3c8ea71, which
  `git tag --contains` places inside v0.12.0; `portal.rs:245` compares
  version strings for equality; `CHANGELOG.md:83-85` reads as quoted.
- R1.42: `TARGETING_MAX_RANGE` is 20 000 engine units at
  `contacts.rs:17`.
- R1.50: the lobby's Start writes `roster.seed = model.next_seed` and
  `draft_roster` logs that value as the replay head.
- R1.51: `completion_watch` reads `Res<Time>`; the freeze pauses
  `Time<Virtual>`; the autopilot commit's own message records a driven
  beat stalling behind the ship computer; `NovaMenuPlugin` is added to
  the default app only.

Verified by the lanes (tests run, reported in their transcripts):

- `cargo test -p nova_os --lib` 45, `-p nova_channel --lib` 12,
  `-p nova_scenario --lib actions::ship` 5, `-p nova_console --lib` 6.
- `cargo test -p nova_gameplay --lib rounds:: transient_light::` 35,
  `-p nova_ship --lib railgun_section` 18.
- `cargo test -p nova_menu --lib settings` 38, `-p nova_ship --lib
  sensitivity` 4, `-p nova_input --lib sensitivity` 3; the flake
  reproduction for R1.31 at `--test-threads=3` (183 of 300 failed).
- `cargo test -p nova_editor --lib inspect` 120, `-p nova_events --lib
  units` 10; `cargo test -p nova_autopilot --lib -- tests::` 76.
- `content lint` on base, the-ledger and gauntlet: 0 errors, 0 warnings,
  0 findings; a scratch copy of the example mod with `infinite_ammo`
  re-added lints clean, and with the R1.8 snippet fails as reported.
- `npm test` in `web/`: all suites pass.
- Every quantity line of the 22 content files diffed against fec6e441.
- Every `outcome:` literal in the 35 `examples/systems` ranges against
  the catalog (213 slugs, 0 mismatches); the web manifest link checker
  (0 wiki problems); the 27 wiki command rows against the registry.

NOT verified (each is a skip, not a pass):

- The play lanes: red team and feel, and any `--play` measurement. The
  owner ran these before the review.
- Any rendered example, screenshot, loop or probe run; the wake stopping
  short (R1.21) and the first-shot hitch (R1.23) are argued from the code.
- The workspace test suite and Clippy (standing instruction: CI only).
- `cargo check` of the tree in this session; the wasm build (trunk); the
  mdbook and web builds; `content gen` byte parity of the base RON.
- The `cfg(feature = "serde")` railgun round-trip tests,
  `content_ron_parity`, `tests/env_contract.rs`, and the nova_autopilot
  integration binary.
- Whether any shipped hull has a section standing forward of the bore
  column inside the rake radius (the R1.20 trigger).
- The hidden `file:line` anchors in `web/src/wiki/nova-os.md`.
- Pick-map occlusion of `click_named` targets against a post-layout
  reflow.

## Decisions for the owner

Everything tagged `easy` above has a local fix with an obvious shape.
These need a call first:

1. R1.2 - what closing the command shell over a paused game returns to.
   The design record says the pause menu; the code says flight; the
   ledger built for the overlap never sees one. Restoring the state under
   the shell is the smaller change and keeps the record true.
2. R1.4 - a ship qualifier in the section address. Without it `section`
   and `ammo refill section` cannot name a section in any shipped playable
   scenario. `<ship> <section>` is the smallest grammar; `ship/section`
   reads better in a prompt.
3. R1.23 - warm the wake graphs at scene load or keep the first-shot
   build. Cost is one hidden emitter pair per tier with particles on.
4. R1.32 - split the settings store's load from its save, so a
   `with_game_plugins` app can read the player's settings without being
   able to write them. The bench-local tier is the easy stopgap.
5. R1.39 - the debounced save's fsync on the main thread after Resume:
   move it to the IO pool, flush on `OnExit(Paused)`, or accept it.
6. R1.49 - the cache-at-spawn rule for SI config read in hot loops: apply
   it to weapon reach and muzzle speed, or narrow the rule to physics
   steps and record that.
7. R1.26 - one hanabi emitter pair per lance instead of two fresh
   instances per shot, only if a GPU-host set shows the spike.

## The owner's answers (2026-09-03)

1. R1.2 - Escape closes the terminal emulator only. It restores whatever
   surface was under it and never unpauses a paused player, so
   `NovaOsCloseTransition::return_to` records the state `:` covered.
2. R1.4 - `<ship> <section>`. A section is identified by its ship plus its
   own id.
3. R1.23 - warm the wake graphs at scene load.
4. R1.26 - two instances per shot are fine for now.
5. R1.32 - undecided. Taken as the review's own easy option, stated under
   the finding: the bench declares its store inert. The load/save split is
   still the owner's to make.
6. R1.39 - fine for now; leave a NOTE where the fsync lands.
7. R1.49 - derive only if changed.
