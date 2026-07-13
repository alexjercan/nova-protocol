# Investigate Entity-despawned command error on menu to game transition

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.5.0,bug

## Goal

The v0.5.0 web console shows, right when Play is hit (menu -> game
transition, before the fatal render error tracked in 20260713-175415):

    WARN Encountered an error in command `<...>`: Entity despawned: The
    entity with ID 891v0 is invalid; its index now has generation 1.

Some system queues a command against a menu-lifetime entity that is despawned
by the time the command applies. Non-fatal, but it is a real lifecycle bug
(or a missing `queue_handled`). Root-cause it and fix or document.

## Steps

- [x] Reproduce natively: run the editor app, enter Playing from the main
      menu (and from editor Play), with `bevy_ecs::error` logs visible;
      confirm whether the warn fires and on which transition.
      OUTCOME: does NOT reproduce natively - see Record.
- [x] Bisect the offending system: N/A, no native reproduction to bisect
      (the debug build names the command, but no error ever fired).
- [x] Fix the root cause: N/A without a reproduction; the sibling fixes
      (20260713-175415/175416) remove the fatal errors from the same
      transition, and the warn must be re-checked on the fixed web build.
- [x] Add a regression pin: examples/13_menu_newgame.rs drives the shipped
      New Game boot flow (and NOVA_MENU_PATH=editorplay the editor Play
      path) under the autopilot with the ECS fallback error handler swapped
      to panic, and joined HARNESSED_EXAMPLES in tests/examples_smoke.rs -
      any command error on these transitions now fails CI.
- [x] Record the evidence rig and close as documented-not-reproducible.
- [x] CHANGELOG.md entry if a code fix lands: no gameplay code changed, so
      no entry (the new example is test infrastructure).

## Notes

- Generation 1 at index 891 means the entity was spawned and despawned once;
  the reuse happens quickly during the menu teardown / gameplay spawn burst.
- Observed on wasm (Firefox) in the v0.5.0 build; not confirmed native.
- Independent of the two render fixes (20260713-175415, 20260713-175416);
  scheduled after them.

## Record

Evidence rig (exact, per record-the-exact-rig): the shipped app via
`editor_app(true)` (examples/13_menu_newgame.rs; during investigation a
throwaway `99_menu_newgame_probe.rs`, same content), built with
`--features debug` (bevy/track_location; the log filter includes
bevy_ecs=warn, so command errors would print with names), run under
`Xvfb :99` with `BCS_AUTOPILOT=1`; the autopilot clicks the real menu
buttons via `Activate` triggers and each run reached Playing (delivery
guard: "nova harness: reached Playing" + click markers asserted per run).

Results: menu -> Sandbox 1/1 clean (09_editor), menu -> New Game
(shakedown_run load) 6/6 clean, menu -> Sandbox -> create ship -> editor
Play 3/3 clean ("clicked Play" marker present). Zero "Encountered an
error in command" lines in any log (logs:
$CLAUDE_JOB_DIR/tmp/{newgame_probe,probe_1..5,editorplay_1..3}.log for
this session; the rig reproduces them from the example alone).

Pin proof (would-it-fail-without-it): with a deliberate stale-entity
command injected (spawn+despawn, then insert on the stale id next frame),
the pinned run aborts - exit 134, log shows "Entity despawned: The entity
with ID 950v0 is invalid; its index now has generation 1" - the exact
error shape from the web log. Sabotage applied after committing the pin
(commit 3db870a) and reverted via git checkout.

Conclusion: the warn is real on wasm (the user's log) but not reproducible
natively on any of the three transitions in 10 harnessed runs. Plausible
remaining explanations: wasm-specific frame pacing / slower HTTP asset
loads interleaving the teardown differently, or an interaction with the
now-fixed fatal render crash's shutdown cascade. Residual action routes to
the user's next web playtest: with 20260713-175415/175416 deployed, check
whether the warn still appears; if it does, the browser log now has a
native CI pin waiting to catch any regression that becomes reproducible,
and a fresh task should capture the new (post-fix) log context.

Reflection: extending the investigation into a permanent smoke example
turned a null result into durable coverage - the shipped New Game boot
flow had none. What could have gone better: the editorplay probe phase
was written before checking that 09_editor already had button-driving
machinery to copy; reading it first saved time and should have been the
first move, not the second.
