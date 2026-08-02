# Port the scripted autopilot driver into nova_autopilot

- STATUS: OPEN
- PRIORITY: 97
- TAGS: v0.10.0,tooling,autopilot
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183340

## Story

Port the scripted state driver into `nova_autopilot::autopilot`:
`AutopilotPlugin<S>` holds a `(state, seconds)` timeline, runs a per-frame input
closure after `InputSystems`, and reports done to the completion protocol.
Keeps `self_completing` (script owns the finish; an expired runway aborts) and
`loop_while_pending` (repeat the cycle while other collectors are pending,
announced by the `AutopilotLoop` message). Armed by `NOVA_AUTOPILOT`.

## Steps

- [ ] Port the plugin, driver system, and `AutopilotLoop`, renaming the arming
      env to `NOVA_AUTOPILOT` with no BCS alias.
- [ ] Add App-driven tests: the runway advances states on its timeline, the
      input closure's press survives `InputSystems` into `Update`, an expired
      self-completing runway error-exits, and a looping cycle resets its clock
      and finishes as soon as other collectors clear.

## Definition of Done

- The timeline advances the state machine and reports done once.
  (test: `autopilot_drives_the_timeline_and_reports_done`)
- A `just_pressed` poke from the input closure is visible to game systems.
  (test: `input_closure_press_survives_input_collection`)
- A self-completing script that never reports done aborts instead of passing.
  (test: `expired_self_completing_runway_error_exits`)
- Looping restarts the cycle while other collectors are pending and stops the
  moment they clear. (test: `loop_while_pending_resets_and_finishes_early`)

## Notes

- Parent: `20260802-120019`. Depends on the completion port.
- The `.after(InputSystems)` ordering is load-bearing: input collection clears
  `just_pressed` every frame.
- Source: `bevy-common-systems/src/debug/harness/autopilot.rs`.
