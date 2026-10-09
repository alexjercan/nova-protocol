# PROOF-P6FRAMES

Rendered proof of `examples/systems/system_world_resume.rs` (feature `debug`),
two full runs, own Xvfb, no source edits, nothing staged.

## Command (both runs, same env)

```
VK_DRIVER_FILES=/nix/store/vi9s3w1viw56plx4c8wbi52zgmklcv5i-graphics-drivers/share/vulkan/icd.d/lvp_icd.x86_64.json \
ALSA_CONFIG_PATH= DISPLAY=:77 NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900 NOVA_CAPTURE=1 \
nix develop --command cargo run -j 8 --features debug --example system_world_resume
```

Xvfb started with `Xvfb :77 -screen 0 1920x1080x24 &`, PID 3258883 recorded
before the runs, killed by that PID after (`ps -p 3258883` empty afterward,
`:150` left untouched). `./target` used (no `CARGO_TARGET_DIR` set).

## Run 1 (`p6-run.log`, 177 lines) - exit 0

Key lines:
- `world_resume: create phase wrote generation 2 for 'Probe World'`
- `autopilot: cycle complete, no panic (t=8.7s)` - once, not twice (see below)
- `world_resume load: the resumed ledger seam ran`
- `world_resume load: PASS every fixture value matched`
- No `stalled` line, no panic, no error-level line.
- Sandbox cleaned silently (no "would not clean up" warn), confirming the
  create phase's success path ran to its end and the spawned load phase
  returned a success exit status.

## Run 2 (`p6-run2.log`, 175 lines) - exit 0

Same pattern, same four key lines present once each:
- generation 2 written, `autopilot: cycle complete, no panic (t=9.4s)` once,
  resumed ledger seam ran, PASS every fixture value matched. No stall/panic.

## Discrepancy: "cycle complete, no panic" appears once per full round trip, not twice

The task brief and the example's own doc comment expect this line twice (once
per process). In both runs it appears exactly once, logged only by the create
phase. Reading `crates/nova_autopilot/src/autopilot.rs:676`, the line only
fires when the step list is exhausted; the load phase's final step
(`examples/systems/system_world_resume.rs:874`, `until(Arc::new(|_| false))`)
never becomes true by design - the script is deliberately racing the real
Pause-Exit `AppExit` against the harness's own deadline, and production wins
first in both runs, so the harness step driver never reaches a "steps
exhausted" state to log the line. This is consistent, not flaky (reproduced
identically in both runs), but it means the doc comment's "(twice, one per
process)" is inaccurate for the load phase as written. Not a correctness
regression in the save/load round trip itself - flagging per "judge
assertions... not exit zero or pilot prose."

## Frames (copied from run 2, the last run, to `p6-frames/`)

- `world_resume-status-line.png`: in-game view of the player ship, HUD top
  right reads "39 fps", version string, and "World saved" in green under it.
  No Loading overlay. Matches the expected description.
- `world_resume-load-screen.png`: the Load list showing two rows - "broken-
  world" selected and refused, with the right-hand detail pane reading
  "unreadable: world.ron: No such file or directory (os error 2)" and a
  greyed-out/disabled "Load" button, and "Probe World" listed below it.
  Matches the expected description.
- `world_resume-leave-overlay.png`: **does not show the leave overlay.** It is
  visually almost identical to `world_resume-status-line.png` (same ship,
  same "World saved" line, no darkening, no "Saving world..." text, no
  translucent panel). Checked two ways, not just by eye:
  - ImageMagick `compare -metric AE` against the status-line shot shows only
    scattered background/HUD-counter diffs (parallax stars, fps digit), not a
    concentrated overlay-shaped region.
  - Whole-image grayscale mean brightness: status-line 0.0974873 vs
    leave-overlay 0.0974677 - effectively identical. `sync_leave_overlay`
    (`crates/nova_menu/src/leave.rs:215-231`) spawns a full-screen node with
    `BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))`, which would darken
    the mean substantially if it had rendered into this frame. It did not.
  - The log confirms the shot was taken in the right step, right after the
    "Leave Overlay" UI node's presence gated the preceding click
    (`p6-run2.log:73-84`): `click Back to Main Menu: release` ->
    `shoot the leave overlay` -> `nova capture: world_resume-leave-overlay.png`
    -> `Screenshot saved` within ~0.4s. The predicate `ui_node_present` was
    satisfied (an ECS check), but the overlay evidently had not been
    composited into the rendered frame yet when the screenshot was taken, or
    is not rendering under lavapipe at all.
  - This is a real visual gap: the PNG does not prove the leave overlay
    appears to a player, even though the ECS-level gate the harness used
    passed. Flagging per "Headless output cannot prove appearance" - this is
    exactly that case, now demonstrated with a frame instead of just a log
    line. I did not attempt a fix (read-only verification).

## Cleanup

- No PNGs left at the worktree root; the three above are the only contents
  of `tasks/20261007-090756/p6-frames/`.
- No `target/example-profiles/system_world_resume-*` sandbox directories were
  left by either run (both create phases reported no cleanup warning).
- Xvfb :77 (PID 3258883) stopped; `:150` was not touched.

## Remaining uncertainty

- The leave-overlay rendering gap was only pixel-verified on run 2 (run 1's
  PNGs were not retained for comparison before this was noticed); both runs'
  logs show the identical step-timing pattern, so it is very likely the same
  in run 1, but that is inferred from logs, not independently re-verified
  pixel-for-pixel.
- Whether the missing overlay is a lavapipe-only rendering artifact or also
  reproduces on a hardware GPU is unknown; only lavapipe was run here.
