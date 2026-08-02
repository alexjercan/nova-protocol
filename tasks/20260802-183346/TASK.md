# Port the single-shot screenshot driver into nova_autopilot

- STATUS: OPEN
- PRIORITY: 96
- TAGS: v0.10.0,tooling,autopilot,screenshot
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183340

## Story

Port the single-shot capture driver into `nova_autopilot::screenshot`:
`ScreenshotPlugin<S>` forces a window resolution, advances to a target state,
waits N settled frames, writes a PNG, and reports done. Armed by `NOVA_SHOT`; a
`WxH` value overrides the resolution. Stands down when the autopilot is also
armed, since both drive `NextState`.

## Steps

- [ ] Port the plugin, drive system, `WxH` parsing, the max-wait error exit, and
      the resolution pin, renaming the arming env to `NOVA_SHOT`. Replace the
      BCS inspector `DebugEnabled` reach-in with a caller-supplied overlay-hide
      hook so the crate keeps no game dependency.
- [ ] Port the `WxH` parser tests and add App-driven tests for settle-then-
      capture, the unreachable-state error exit, and the stand-down when
      `NOVA_AUTOPILOT` is also set.

## Definition of Done

- A `WxH` value sets the resolution; a bare toggle or a nonsense value does not.
  (test: `rejects_non_resolution_values`)
- The capture waits the configured settled frames before reporting done.
  (test: `screenshot_reports_done_after_settling`)
- An unreachable target state error-exits instead of hanging.
  (test: `unreached_target_state_error_exits`)
- The module suite is green.
  (cmd: `nix develop --command cargo test --lib -p nova_autopilot screenshot`)

## Notes

- Parent: `20260802-120019`. Depends on the completion port.
- Overlay hiding is Nova-specific (nova, inspector, wireframe `DebugEnabled`);
  it becomes a hook the `nova_debug` preset supplies.
- Source: `bevy-common-systems/src/debug/harness/screenshot.rs`.
