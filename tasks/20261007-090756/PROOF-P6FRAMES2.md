# PROOF-P6FRAMES2

Re-run of `examples/systems/system_world_resume.rs` (feature `debug`) after
the read-only-leave change. Same method as PROOF-P6FRAMES.md: own Xvfb on a
free display with recorded PID, forced lavapipe, `ALSA_CONFIG_PATH=` empty,
`./target`, `-j 8`, `nix develop --command`. One run (one GPU measurement).
No source edits, nothing staged.

## Command

```
VK_DRIVER_FILES=/nix/store/vi9s3w1viw56plx4c8wbi52zgmklcv5i-graphics-drivers/share/vulkan/icd.d/lvp_icd.x86_64.json \
ALSA_CONFIG_PATH= DISPLAY=:90 NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900 NOVA_CAPTURE=1 \
nix develop --command cargo run -j 8 --features debug --example system_world_resume
```

Xvfb started with `Xvfb :90 -screen 0 1920x1080x24 &`, PID 3261005 recorded
before the run, stopped by that PID after (`ps -p 3261005` empty afterward,
`:97` and `:150` left untouched).

## Run (`p6-run3.log`, 192 lines) - exit 0

Key lines, in order:
- `world_resume: create phase wrote generation 2 for 'Probe World'` (first
  arm, line 77) - the pre-leave disk assertion.
- `nova_world_base: the world save failed: cannot write state.3.ron:
  Permission denied (os error 13)` (line 85) - the leave save failing against
  the read-only folder, as the change intends.
- `world_resume: create phase wrote generation 2 for 'Probe World'` again
  (line 92) - the on-disk header re-asserted after the failed leave, still
  generation 2, unchanged by the failed write (`assert_probe_world_on_disk`
  runs again in the `the failed leave kept the last good save` step).
- `autopilot: cycle complete, no panic (t=9.3s)` once (create phase; the load
  phase's Exit still ends the process before its own script finishes, as
  PROOF-P6FRAMES.md already found and the doc comment still describes this
  way for the load phase).
- `world_resume load: the resumed ledger seam ran`
- `world_resume load: PASS every fixture value matched`
- No `stalled`, no `would not clean up`, no panic, no other ERROR-level line
  besides the expected save-failure one above.

The "wrote generation ... 2" line appearing twice matches the brief: it is
the create phase's own pre-leave and post-failed-leave disk assertions, not
the two-processes doc-comment line from the older proof (that was about
"cycle complete, no panic", unrelated here and still correct as one-per-round-
trip).

## Frames (copied to `p6-frames/`, root copies removed)

- `world_resume-status-line.png`: in-game ship view, HUD reads "42 fps",
  version string, "World saved" in green beneath it. No overlay. Matches.
- `world_resume-load-screen.png`: Load list with "broken-world" selected and
  refused (right pane: "unreadable: world.ron: No such file or directory (os
  error 2)", greyed Load button) and "Probe World" listed below. Matches.
- `world_resume-leave-overlay.png`: **now shows the leave overlay**, unlike
  the prior run (PROOF-P6FRAMES.md flagged it as not compositing under
  lavapipe). The frame shows: the background ship darkened behind a
  translucent panel; the panel titled "SAVE FAILED" in red; a detail line
  "cannot write state.3.ron: Permission denied (os error 13). The last save
  is kept."; a green "Try again" button and a red-bordered "Leave without
  saving" button; and a red status-line banner at the top reading "SAVE
  FAILED: cannot write state.3.ron: Permission denied (os error 13)",
  matching the log's error line. This resolves the earlier open question
  about whether the overlay renders at all under lavapipe - in this run, with
  the harness now waiting on `LEAVE_TRY_AGAIN_BUTTON` (an overlay-only node)
  before the shot instead of the earlier `BACK_TO_MENU_BUTTON`-gated click, it
  visibly composited. This is one rendered frame, not proof it renders every
  time; still, headless output alone would not have shown this - the frame
  was required to confirm appearance.

## Cleanup

- No PNGs left at the worktree root after copying to `p6-frames/`.
- No `target/example-profiles/system_world_resume-*` sandbox directories were
  left (the create phase's final cleanup ran without a "would not clean up"
  warning, and none were found by search after the run).
- Xvfb :90 (PID 3261005) stopped by PID; `:97` and `:150` (other owners' or
  prior sessions') were not touched.

## Remaining uncertainty

- One run only: per brief, no timing is asserted and this does not claim the
  leave overlay renders deterministically on every invocation under lavapipe,
  only that it rendered correctly in this run (a clear improvement on the
  prior run's non-render, verified by eye against the log's expected content
  rather than inferred from an ECS gate alone).
- Whether the overlay's prior non-render was fixed by this specific change
  (the new `LEAVE_TRY_AGAIN_BUTTON` gate before the shot) or was flaky timing
  in the old run is not isolated here; only a single before/after comparison
  (PROOF-P6FRAMES.md vs this run) is available, not a repeat set.
- Hardware-GPU rendering of the overlay was not checked; only lavapipe was
  run here, consistent with the rest of this task's proofs.
