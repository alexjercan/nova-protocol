# Make nova_autopilot predicate-driven: a generic scripted state machine

- PRIORITY: 85
- TAGS: v0.10.0, tooling, autopilot, testing
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120019

## Story

`AutopilotPlugin` today is a list of `(state, seconds)` holds plus one
per-frame closure. Wall-clock is the only advance condition, so every example
re-implements the same scaffolding by hand: a script resource of booleans, a
`playing_since` offset, `if t > 0.5 && !script.raised` beats, a hand-rolled
panic guard, and a runway `hold(Loading, 30.0)` that has nothing to do with
`Loading`. Under llvmpipe a wall-clock window can collapse into a few frames,
so the examples that matter already wait on observed state by hand
(`playable` waits for a live `CombatLock`).

Generalize the driver: a step advances when its PREDICATE over the world holds,
and elapsed time is one predicate among many. `.hold(state, secs)` becomes
sugar for "enter this state, advance after N seconds". Steps get names, entry
and per-frame actions, per-step deadlines, and a loop point, so a script reads
as its beats and a stall names the beat that stalled instead of dumping a
boolean tuple.

This is the seam the rebuilt example fleet (`20260802-120029`) and `nova_probe`
sit on, so the looping/deadline hacks probe needs become driver features
instead of per-example workarounds.

## Steps

- [x] Add `crates/nova_autopilot/src/predicate.rs`: `pub type Predicate = dyn
      Fn(&World) -> bool + Send + Sync` plus the vocabulary the inventory
      below actually consumes - `elapsed(secs)` (seconds IN STEP),
      `frames(n)`, `in_state::<S>(s)`, `resource_where::<R>(f)`,
      `any_entity::<F: QueryFilter>()`, `and(a, b)`, `not(a)`. No `or`, no
      `entity_count`: no caller in this task needs them.
- [x] Add `crates/nova_autopilot/src/input.rs`: world actions
      `press_key`/`release_key`, `move_cursor(pos)` (writes the primary
      `Window::cursor_position` AND a `CursorMoved` message, the pair a real
      pointer produces), `click_at(pos, button)` and
      `press_mouse`/`release_mouse`. No `drag`: no caller. All are plain
      `Fn(&mut World)` so they compose as `on_enter`/`each` actions.
- [x] Rewrite `crates/nova_autopilot/src/autopilot.rs` around a step list:
      `Step { name, enter: Option<S>, on_enter, each, until, deadline }` built
      by `AutopilotPlugin::step(name) -> StepBuilder` with
      `.enter/.on_enter/.each/.until/.deadline/.add()`. `hold(state, secs)`
      becomes sugar for `step("hold:<state>").enter(state).until(elapsed(secs))`.
      `input(f)` stays as sugar meaning "attach `f` as the `each` action of
      every step" (every current caller is a single-step script, so its
      run-relative `elapsed` and the new step-relative one coincide).
      `autopilot_drive` walks the list: on entry set `NextState` + run
      `on_enter`, each frame run `each(world, step_elapsed)`, advance when
      `(until)(world)`, report `completion::AUTOPILOT` done after the last
      step.
- [x] Per-step deadline in the same driver: `deadline` unset means no per-step
      bound (the run-level `NOVA_AUTOPILOT_DEADLINE` watcher stays the
      backstop). Expiry writes `error!("autopilot: step `{name}` stalled after
      {step_elapsed:.1}s (run {elapsed:.1}s, state {state:?})")` then
      `AppExit::error()` and never reports done - the abort path
      `self_completing` used to own. DELETE `self_completing`.
- [x] Replace `loop_while_pending` with `loop_from(name)` + `on_loop(f)` in
      `autopilot.rs`: at the last step, while
      `HarnessCompletion::others_pending(AUTOPILOT)`, jump the cursor back to
      the step named by `loop_from`, run `on_loop`, write `AutopilotLoop`
      (kept - `crates/nova_probe/src/capture.rs` reads it), zero the clocks.
      Keep the existing early-finish: once nothing else is pending mid-cycle,
      report done immediately. DELETE `loop_while_pending`.
- [x] Re-export every new public item from `crates/nova_autopilot/src/lib.rs`'s
      prelude (`tests/prelude.rs` fails otherwise) and update the crate docs'
      step-model description and env table prose.
- [x] Rewrite `crates/nova_autopilot/examples/driven_app.rs` as the reference
      script (named steps, a predicate, an `on_enter` action) and extend
      `crates/nova_autopilot/tests/autopilot_example.rs` to assert the run
      logs the step names and that a deliberately unsatisfiable step
      error-exits naming that step.
- [x] Add the Nova-typed predicates to `crates/nova_debug/src/harness.rs`
      (they may not live in the bevy-only crate): `scenario_variable_is`,
      `section_gone(id)`, `player_ship_present()` - exactly what the three
      rewritten scripts below need, built on `resource_where`/`any_entity`.
      Keep `nova_autopilot()` as the Nova preset, now returning a step-shaped
      plugin.
- [x] Rewrite the three scripts that carry the full scaffolding onto predicate
      steps and delete their `playing_since` offsets, beat booleans, panic
      guards and completion guards: `examples/sections/com_range.rs` (drop
      `guard_script_completion`), `examples/ui/hud_range.rs`,
      `examples/gameplay/playable.rs` (its `on_autopilot_loop` reader becomes
      `on_loop`; the probe reload-gate calls stay in that hook).
- [x] Migrate the remaining `self_completing`/`loop_while_pending` callers at
      the construction site only - one named step carrying the existing
      closure as `each`, ending on the script's own done-report, with the old
      runway seconds as its `deadline` - and delete their `Last`-schedule
      guards: `examples/gameplay/broadside.rs`,
      `examples/gameplay/lifeline.rs`, `examples/gameplay/scenario.rs`,
      `examples/screenshots/screenshot_nova_os.rs`,
      `examples/ui/menu_scenarios.rs` (drop `guard_run_completion`). The
      six pure `hold`+`input` callers need no edit; confirm with a build.
- [x] Document script authoring in `web/src/wiki/dev/automation-harness.md`
      with a before/after of the `com_range` script, and check
      `web/src/wiki/dev/keeping-docs-in-sync.md` for any other page the
      renamed API touches.

## Definition of Done

- A step advances on an observed predicate, not on wall-clock: with the clock
  advanced but the predicate false the driver stays on the step, and with the
  clock stalled but the predicate true it advances.
  (test: `step_advances_only_when_its_predicate_holds`)
- `hold` still works and is built from the elapsed predicate over a named
  step. (test: `hold_is_sugar_for_the_elapsed_predicate`)
- A step that never satisfies its predicate inside its deadline error-exits
  naming the step, its in-step elapsed time and the observed state, and never
  reports the collector done.
  (test: `stalled_step_aborts_naming_the_step`)
- A declared loop point restarts at that named step and fires its reset hook
  while other collectors are pending, then finishes as soon as they clear.
  (test: `loop_point_restarts_at_the_labeled_step_and_resets`)
- Entry and per-frame actions still land inside the real input pipeline, so a
  synthesized key press is still `just_pressed` when `Update` reads it.
  (test: `step_actions_run_after_input_collection`)
- A synthesized click at a window position leaves the world in the state a
  real pointer would: primary-window `cursor_position`, a `CursorMoved`
  message at that position, and a `just_pressed` mouse button.
  (test: `click_at_position_reaches_the_widget_under_it`)
- The old shape is gone from the tree, aliases included.
  (cmd: `grep -rn --exclude-dir=tasks --exclude-dir=.git --exclude-dir=target --include='*.rs' -E 'self_completing|loop_while_pending' .`)
- The rewritten scripts carry no hand-rolled beat offsets or completion
  guards.
  (cmd: `grep -rn --include='*.rs' -E 'playing_since|fn guard_script_completion|fn guard_run_completion' examples/sections/com_range.rs examples/ui/hud_range.rs examples/gameplay/playable.rs examples/ui/menu_scenarios.rs`)
- The crate's own end-to-end app drives a real `DefaultPlugins` run through
  named steps to a clean exit, and an unsatisfiable step in that same real app
  error-exits naming it.
  (test: `driven_app_logs_its_step_names_and_a_stalled_step_aborts`)
- Three migrated examples complete headlessly through the new driver.
  (cmd: `nix develop --command cargo run -p nova_probe -- run com_range,hud_range,playable`)

## Notes

- The crate stays `bevy`-only, deliberately: the owner's long-term intent is a
  standalone autopilot crate, and every `nova_*` dependency makes that
  extraction harder. Pointer synthesis is generic Bevy input, so it belongs
  here; anything naming a Nova type does not.
- No serialized automation DSL. Rust scripts need direct world access for real
  input and scenario predicates; a predicate is a closure, not data.
- The crate owns sequencing, predicates, deadlines, diagnostics, looping, and
  completion. Scenario-specific actions and predicates stay in examples behind
  the `nova_debug::harness` adapter.
- Migration is atomic per the epic's Nova-first decision: rename/replace, no
  compatibility aliases for the old shape beyond `hold` and `input`, both of
  which are constructors over the step model rather than shims.
- BREADTH, not splittability: deleting `self_completing` breaks eight example
  binaries in the same commit that adds the step model, so driver and callers
  cannot land separately without a broken intermediate tree. The offsetting
  cut is that only three scripts are REWRITTEN; the other five change at the
  plugin-construction site and `20260802-120029` rebuilds their content.
- Inventory behind the migration split (`grep` over `examples/`):
  `self_completing` - broadside, lifeline, com_range, hud_range,
  screenshot_nova_os, menu_scenarios. `loop_while_pending` - playable,
  scenario. `playing_since` - com_range, hud_range, playable plus
  screenshot_combat/juice/orbit (those three are pure `hold`+`input`, so
  their offset is harmless and stays for `20260802-120029`). Per-example
  completion guards - `guard_script_completion` (com_range, broadside,
  lifeline, hud_range), `guard_run_completion` (menu_scenarios).
  Untouched pure `hold`+`input` callers: screenshot_combat, screenshot_juice,
  screenshot_orbit, screenshot_ui, screenshot_reel, torpedo_section.
- The `nova_probe` run proof is vacuous on the base branch, not green: the
  migrated sources do not compile against the old driver, so the command only
  becomes meaningful once the step model exists. The absence greps above are
  the red-on-base pins (33, 24 and 6 hits respectively at plan time).
- Per-step deadlines must stay summable under the run-level
  `NOVA_AUTOPILOT_DEADLINE` that `crates/nova_probe/src/bin/probe/native/env.rs`
  sizes from the fps window, or the generic hang detector wins the race and
  the named-step diagnostic is lost. Documented, not enforced in code - no
  caller needs the check yet.
- Decide the pointer test's depth: drive a real `bevy_ui` widget headlessly if
  `UiPlugin` + the picking backend run without a render device, or defer to
  asserting the observable pointer state (window `cursor_position` +
  `CursorMoved` + `just_pressed`) and record the reduction in DECISION.md.
- Related backlog: `20260803-094601` (per-test timeout), `20260803-114158`
  (harness rustdoc nits).

## Close-out

**What and why.** `AutopilotPlugin` is now a list of named steps, each
advancing when a predicate over the world holds. Wall-clock is one predicate
(`elapsed`) among `frames`, `state_is`, `resource_where`, `any_entity`, `and`,
`not` plus any closure. Steps carry `enter`, `on_enter`, `each` (step-relative
elapsed), `until` and an optional `deadline`; `loop_from(name)` + `on_loop(f)`
replace `loop_while_pending`, and the per-step deadline replaces
`self_completing`. The point is diagnosis: a run logs its beats and a stall
error-exits naming the beat that stalled, where the old driver could only
report that a runway expired. `nova_autopilot::input` adds the gestures a
script needs to be predicate-driven at all (`press_key`, `release_key`,
`press_mouse`, `release_mouse`, `move_cursor`, `click_at`), and
`nova_debug::harness` adds the Nova-typed predicates (`scenario_variable_is`,
`section_gone`, `player_ship_present`, plus the migration-only
`script_reports_done`). `com_range`, `hud_range` and `playable` are rewritten
onto beats; five more callers move at the construction site; six pure
`hold`+`input` callers are untouched.

**Alternatives.** Recorded in DECISION.md: keeping `self_completing` to land
the driver alone (rejected - two ways to end a run, and no compiling
intermediate tree either way), splitting driver from callers, shipping the
wider NOTES vocabulary, and run-relative `each` elapsed.

**Difficulties and diagnosis.** Two, both found by running rather than
checking.

- `set_mouse_button` was referenced but never written, so the crate did not
  compile. It survived an earlier `cargo check ... | tail` because the pipe
  swallowed the exit code - the AGENTS.md `set -o pipefail` rule, learned
  again.
- The real-widget pointer test failed until the gestures wrote `WindowEvent`
  as well as the concrete messages. `bevy_picking::input::mouse_pick_events`
  reads `WindowEvent` and tracks the cursor from those events alone, so a
  click that wrote only `CursorMoved` + `ButtonInput` resolved at the origin
  and the "click the button" beat stalled. Writing both is what `bevy_winit`
  does for a real device.

**Evidence.**

- `cargo test -p nova_autopilot --lib`: 30 passed, including all five DoD lib
  tests.
- `cargo test -p nova_autopilot --test autopilot_example` under `DISPLAY=:99`:
  2 passed - the real `DefaultPlugins` run through named steps, the
  unsatisfiable-step abort, and the click that reaches a live `bevy_ui` widget.
- `cargo test -p nova_autopilot --test prelude`: 3 passed (every new public
  item is re-exported).
- `cargo check --workspace --all-targets` and `cargo fmt --all --check`: clean.
  Remaining warnings are pre-existing `ambiguous_import_visibilities` in
  `nova_gameplay`.
- Both absence greps are empty; `cargo run -p nova_probe -- run
  com_range,hud_range,playable` completes.

**Reflection.** The plan's inventory was the thing that made a breadth-heavy
migration tractable - every deleted mode had a named caller list before any
code moved, so the commit had no surprises in it. What the plan could not
predict was the picking-backend detail, and the only reason it surfaced is
that the DoD asked for the pointer proof against a real widget rather than
against the state a click writes. The cheaper assertion would have passed on a
click that never reached anything. Worth repeating: when a proof can be
written against the real consumer, write it there.
