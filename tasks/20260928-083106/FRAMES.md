# Target inset frame capture - task 20260928-083106

## What was run

Worked only in the sprout worktree
`/home/alex/.cache/sprouts/nova-protocol/dual-lock-inset`. Did not touch
`/home/alex/personal/nova-protocol`. Did not edit `crates/` (verified with
`git status`: only the pre-existing uncommitted changes to `target_inset.rs`
and its siblings, from before this task, remained modified).

Backed up `examples/systems/system_hud_indicators.rs` to
`/tmp/system_hud_indicators.rs.bak` before editing, then restored it with `cp`
at the end (never `git checkout`/`git restore`). Final check:

```
$ git diff --stat examples/
(empty)
```

### Xvfb

Started `Xvfb :99 -screen 0 1280x720x24`, PID recorded to
`tasks/20260928-083106/xvfb.pid` (969561). Stopped it by that recorded PID at
the end of the task (`kill 969561`, verified gone with `ps -p`). No broad
process matching used.

### Build

```
cd /home/alex/.cache/sprouts/nova-protocol/dual-lock-inset
nix develop --command cargo check --features debug --example system_hud_indicators
```
Compiled clean (only a pre-existing `proc-macro-error2` future-incompat
warning, unrelated).

### The disposable instrumentation

The stock range already shoots `inset_shot.png` mid-run, with a live combat
lock (both `CombatLock` and `TravelLock` pointed at the target ship at that
point - `commit_lock` sets both). That frame alone does not show the inset's
travel-only path or its kill cam, so per the task I added three temporary
steps and two temporary helper functions, disposable and restored afterward:

- `temp_clear_combat_lock` / `temp_restore_combat_lock`, and a
  `.step("temp: switch to travel-only")` / `.step("temp: capture the
  travel-only inset frame")` / `.step("temp: restore the combat lock")`
  sequence shooting `inset_travel.png`.
- `.step("temp: capture the kill cam")` shooting `inset_killcam.png`, inserted
  right after the existing `"kill the target"` step (which already waits
  `elapsed(0.4)` before the next step starts, landing inside the 2s kill-cam
  linger, close to the requested ~0.3s mark).

First attempt placed the travel-only window immediately after "capture the
inset frame" (mid-dwell). That broke the range: clearing `CombatLock`
interrupted the section fine-lock dwell, so `assert_lock_indicators` failed
("the focus meter is still visible after the dwell") because the dwell timer
had been reset by the momentary lock loss. Moved the window there again but
after `assert_lock_indicators`, right before `"engage the goto"` - still
broke it, this time downstream: `assert_goto_indicators` panicked with "the
pinned component lock vanished", because pinning happens after my restore and
something in the fine-lock state machine still depended on lock continuity
through that stretch. Final placement: after `"assert the goto indicators"`
(the destination marker, closing speed, velocity sphere and pinned-marker
checks all already ran and don't depend on lock continuity afterward) and
before the layout-measurement steps. With that placement the full range,
including the trailing `assert_indicators_hid`, passed clean.

### Run

```
cd /home/alex/.cache/sprouts/nova-protocol/dual-lock-inset
env DISPLAY=:99 \
  VK_DRIVER_FILES=/nix/store/vi9s3w1viw56plx4c8wbi52zgmklcv5i-graphics-drivers/share/vulkan/icd.d/lvp_icd.x86_64.json \
  ALSA_CONFIG_PATH= \
  BEVY_ASSET_ROOT="$PWD" \
  NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=300 \
  NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=tasks/20260928-083106/frames \
  RUST_LOG=info,nova_ship=debug \
  nix develop --command cargo run --features debug --example system_hud_indicators
```

Exit status: `0`. Log line at the end: `hud range: PASS - indicators track
their anchors and hide when they die` (`tasks/20260928-083106/run.log`).
Ran only this one example, one GPU measurement at a time; no other Xvfb/GPU
run was active concurrently (checked `ps aux` for other `Xvfb`/game/bevy
processes before starting; unrelated `cargo test`/`cargo check` CPU-only jobs
were running in other sprouts but did not touch the GPU or DISPLAY :99).

## Artifacts

`tasks/20260928-083106/frames/`:

| file | source | notes |
| --- | --- | --- |
| `inset_shot.png` (1024x768) | stock capture beat, mid-dwell, live combat lock | |
| `inset_travel.png` (1024x768) | temp beat, combat lock cleared, live travel lock | |
| `inset_killcam.png` (1024x768) | temp beat, ~0.4s into the 2s kill-cam linger | |
| `screenshot.png` (1024x768) | `nova_screenshot`'s own trailing shot (unrelated to this task) | |
| `inset_shot_crop.png`, `inset_travel_crop.png`, `inset_killcam_crop.png` (340x340) | cropped top-right corner (`convert -crop 340x340+660+0`) | for eyeballing the panel without the full frame |
| `inset_shot_corner_tr.png`, `inset_travel_corner_tr.png` (200x200) | tighter crop on the panel's top edge (`-crop 200x200+790+30`) | for judging border color close-up |

`run.log` and `xvfb.log` are also in the task directory.

## Per-frame observations (judged by reading the PNGs, not from log text)

### `inset_shot.png` - combat lock, mid-dwell

- Caption: two lines, `HUD Target Ship - NEUTRAL` / `DST 1.50 km  CLS -0.0
  m/s`. Fits fully inside the panel, bottom-left, not clipped by the panel
  edge or border.
- Text color: pale off-white/gray - matches `FACTION_NEUTRAL_COLOR`
  (`nova_ui::theme::semantic::NEUTRAL`), consistent with the target ship's
  authored relation in this range (no allegiance set -> `Relation::Neutral`).
- Frame color: bright orange-red border, matching `INSET_BORDER_HOT_COLOR`.
  Correct: the combat lock is live, weapons are hot at this point in the
  script (raised stance held through the dwell stages).
- No NO-SIGNAL overlay, no DESTROYED ribbon. Target ship model fills the
  frame at a reasonable size.

### `inset_travel.png` - travel lock, combat lock cleared

- Caption: one name line + one readout line, `HUD Target Ship` / `DST 1.17
  km  CLS +236.0 m/s`. No relation tag - correct per
  `target_inset.rs`'s travel-slot caption (`InsetSlotType::Travel` path never
  adds a relation tag). Fits inside the panel, not clipped.
- Text color: quiet steel gray - matches `INSET_BORDER_SAFE_COLOR`, the
  color a travel caption always takes regardless of the target's own
  allegiance.
- Frame color: pale steel border (visibly different from the orange frame in
  `inset_shot.png` - confirmed side by side in the `_corner_tr` crops). No
  armed corner ticks visible. Correct: `WeaponsHot` is independent of which
  lock is shown, and by this point in the script the stance was released
  long before (`commit_lock`'s release beat), so the frame reads safe
  regardless of slot - this frame does not by itself prove the hot/safe
  split is slot-independent, only that a travel-only frame with weapons
  already safe renders in quiet steel as expected.
- Distance/closing numbers differ from `inset_shot.png` because real sim
  time and ship motion continued between the two captures (~4s apart in the
  script) - expected, not a defect.

### `inset_killcam.png` - kill cam linger

- No caption text is visible anywhere over the frame - confirmed blank,
  matching the "no travel caption sits over a kill-cam image" rule in
  `drive_inset_frame_state` (`q_kill_cam` non-empty forces `subject = None`).
- `DESTROYED` ribbon visible, centered, legible, amber/orange text on a dark
  backdrop band.
- Frame shows fragments/an explosion effect (hanabi particles), frozen final
  camera pose. Border color reads pale steel (safe), not hot - expected,
  since the target that triggered the kill cam is gone and `WeaponsHot`
  reflects the player's own stance/lock state, not the kill cam.

## Not verified / uncertain

- **Armed corner ticks in `inset_shot.png`**: the hot frame's border color
  change is clearly confirmed by the crop comparison, but the four small
  corner tick marks (`TargetInsetArmedTickMarker`) are hard to distinguish
  from the target ship's own hull plating at this crop/zoom in a still image;
  did not crop tightly enough on each of the four corners individually to
  positively confirm all four ticks render. The existing range's own
  assertion (`commit_lock`, mid-script, checks `tick_visibility ==
  Visibility::Inherited` on `TargetInsetArmedTickMarker`) already covers this
  as a correctness fact; this capture only adds the visual read of the border
  color swap, which is confirmed.
- **Non-zoomable combat lock -> NO-SIGNAL over a framable travel target**:
  not captured here. The task description states this behavior is already
  confirmed elsewhere (and `target_inset.rs`'s own unit test
  `a_non_zoomable_combat_lock_holds_no_signal_over_a_framable_travel_target`
  at line 1423 exercises it directly); no PNG was produced for it in this
  session, so it is not re-verified visually here.
- **Timing of the 2s kill-cam window**: only one point ~0.4s after the kill
  was sampled (the `"kill the target"` step's own `elapsed(0.4)`), not the
  full 2s window or its exact 2.0s close. No claim is made about the linger
  boundary from this single frame, consistent with "never assert timing from
  one run."
- **HiDPI (2x scale) rendering of the inset**: the range's own
  `"double the scale factor"` / `"assert the measured layouts at 2x"` steps
  ran and passed (logged: `hud range (2x): stack 172 px under a 142 px strip,
  inset 46 px under a 42 px status bar`, identical to the 1x reading), but no
  frame was captured at 2x scale in this session - only the logical-pixel
  layout math was checked by the range's own assertion, not a rendered
  image.
