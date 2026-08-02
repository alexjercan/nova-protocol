# Add a runnable nova_autopilot example with a headless integration test

- STATUS: OPEN
- PRIORITY: 94
- TAGS: v0.10.0,tooling,autopilot,testing
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183343

## Story

Prove the crate stands alone and give it the runnable-example seam later work
builds on: a `nova_autopilot` example that is a self-contained Bevy app with its
own three-state machine, driven end to end by the autopilot, plus an integration
test that runs it headless and asserts the exit and log lines. This is the
pattern `nova_probe` reuses for correctness and profiling runs.

## Steps

- [ ] Add `crates/nova_autopilot/examples/` with one minimal driven app
      (timeline, input closure, completion) that needs no Nova crate.
- [ ] Add an integration test that runs the example under `NOVA_AUTOPILOT` and
      asserts a success exit plus the cycle-complete line, skipping loudly with
      no display rather than failing.

## Definition of Done

- The example runs headless to a clean exit under the arming env.
  (test: `autopilot_example_completes_a_cycle`)
- The example builds as part of the crate's targets.
  (cmd: `nix develop --command cargo check -p nova_autopilot --examples`)
- The run is skipped, not failed, without a display.
  (cmd: `nix develop --command env -u DISPLAY -u WAYLAND_DISPLAY cargo test -p nova_autopilot --test autopilot_example`)

## Notes

- Parent: `20260802-120019`. Depends on the autopilot driver port.
- Repository rule: an example is not done until it has been RUN once; use Xvfb
  `:99` locally.
- Keep the example free of Nova types - it is the standing proof the crate has
  no hidden game coupling.
