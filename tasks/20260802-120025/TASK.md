# Make nova_autopilot predicate-driven: a generic scripted state machine

- PRIORITY: 85
- TAGS: v0.10.0, tooling, autopilot, testing
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
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

- [ ] Inventory the hand-rolled scaffolding across `playable`, `broadside`,
      `lifeline`, `com_range`, `hud_range`, the `sections/` fleet, and the
      screenshot producers: beat booleans, `playing_since` offsets, panic
      guards, `self_completing` runways, `AutopilotLoop` reset handlers.
      Record which of those a predicate step subsumes.
- [ ] Design the step API from that inventory: named step, optional target
      state, advance predicate (`&World -> bool`), `on_enter` and per-frame
      actions, settle frames, per-step deadline, and terminal outcome
      (finish / fail / loop). Keep `hold` as sugar over the elapsed predicate
      and keep the existing plugin shape buildable in one chain.
- [ ] Provide the predicate vocabulary the examples actually need: elapsed,
      state-is, resource/component observation, entity-count, and
      combinators (`and`, `or`, `not`). Vocabulary comes from the inventory,
      not from speculation; Nova-specific predicates stay in the caller.
- [ ] Add POINTER input synthesis alongside the existing keyboard poking:
      move the cursor to a window position, click/press/release a mouse
      button, and drag - written so the game sees the same events a real
      pointer produces (window cursor position + `CursorMoved` +
      `ButtonInput<MouseButton>`), inside the same post-`InputSystems` slot.
      This is what lets `ui/` examples drive real UI (buttons, the NOVA OS
      computer, the RTT screen). Keyboard and mouse only; gamepad and touch
      wait for a caller that needs them.
- [ ] Implement in `nova_autopilot` on the current completion protocol. A step
      deadline expiry is an ABORT that names the step, its elapsed time, and
      the last observed state; the runway/self-completing special case
      collapses into the ordinary per-step deadline.
- [ ] Make looping first-class: a declared loop point plus a reset hook, so
      `loop_while_pending` restarts a labeled step (not just the last hold)
      and the example no longer needs an `AutopilotLoop` reader plus a
      reload-gate poll to know the scene is live again.
- [ ] Migrate the existing examples that the fleet rebuild does not replace,
      and delete the per-example completion guards the driver now owns.
- [ ] Document script authoring (rustdoc + the dev wiki automation page) with a
      before/after of one migrated script.

## Definition of Done

- A step advances on an observed predicate, not on wall-clock, and the same
  script is timing-independent under a stalled clock.
  (test: `step_advances_only_when_its_predicate_holds`)
- `hold` still works and is implemented as the elapsed predicate.
  (test: `hold_is_sugar_for_the_elapsed_predicate`)
- A step that never satisfies its predicate aborts naming the step, its
  elapsed time, and the observed state; it never exits success.
  (test: `stalled_step_aborts_naming_the_step`)
- A declared loop point restarts at that step and fires its reset hook while
  other collectors are pending, then finishes as soon as they clear.
  (test: `loop_point_restarts_at_the_labeled_step_and_resets`)
- Entry/per-frame actions still land inside the real input pipeline
  (`just_pressed` survives into `Update`).
  (test: `step_actions_run_after_input_collection`)
- A synthesized click at a window position reaches the widget under it, the
  same as a real pointer. (test: `click_at_position_hits_the_widget_under_it`)
- At least two migrated examples complete headlessly through the new driver.
  (cmd: `nix develop --command cargo run -p nova_probe -- run playable`)

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
  compatibility aliases for the old shape beyond `hold`.
- Related backlog: `20260803-094601` (per-test timeout), `20260803-114158`
  (harness rustdoc nits).
