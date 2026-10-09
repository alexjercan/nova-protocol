# Proof: resumable-worlds example updates (World Name Field)

Worktree: `/home/alex/.cache/sprouts/nova-protocol/resumable-worlds`
(branch `resumable-worlds`, base `655100c29`). Read-only verification: no
tracked file edited, nothing staged.

## Setup

- Own `Xvfb :96 -screen 0 1920x1080x24` (PID 3178129, stopped by PID after
  all runs; did not touch the `sw-p6` worker's `Xvfb :97`).
- Forced lavapipe:
  `VK_ICD_FILENAMES=VK_DRIVER_FILES=/nix/store/cxwx1dp3zar22zzblfiv4813b728dyaq-mesa-26.1.5/share/vulkan/icd.d/lvp_icd.x86_64.json`
- `ALSA_CONFIG_PATH=/tmp/proof-alsa/asound.conf` (null pcm/ctl, mirrors CI
  no-audio).
- `NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900`.
- `RUST_LOG=info,nova_menu=debug,nova_world=debug,nova_world_base=debug,nova_assets=debug`.
- Launch: `nix develop --command cargo run -j 4 --features debug --example <name>`,
  from the worktree root, one run at a time (serialised, no concurrent GPU
  use).
- Logs: `/tmp/proof-logs/<name>.log` (not under the worktree; scratch only).

Judged by log markers (`autopilot: cycle complete, no panic`, named `PASS`/
step lines) and by listing `<profile>/worlds/`, not by exit code alone.

## Results

### system_open_world, run 1

Command: as above, example `system_open_world`.

- `AdapterInfo { name: "llvmpipe ... backend: Vulkan }` confirms lavapipe.
- Reached Playing: `open_world: Create started seed 2405986320`, `the world
  arms around the player`, `every bound weapon fired`, `Retry kept the
  seed`, ends `autopilot: cycle complete, no panic (t=40.2s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/system_open_world-3178173-1791472019342623715/`
  - `worlds/probe-world/{world.ron,world.lock,state.2.ron}` present.

### system_open_world, run 2 (repeat, proves no NameTaken / no shared folder)

Same command, run immediately after run 1 finished (sequential, same display).

- Same shape of log: seed printed, world armed, weapons fired, Retry kept
  the seed, `autopilot: cycle complete, no panic (t=38.2s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches. In particular no
  `NameTaken`/"exists" refusal on the second Create of "Probe World".
- Profile folder (distinct from run 1, different pid+nanos):
  `target/example-profiles/system_open_world-3179475-1791472124929860218/`
  - `worlds/probe-world/{world.ron,world.lock,state.2.ron}` present.
- Both folders hold their own independent `probe-world` world; neither
  shares a path or a lock with the other (each run's own
  `NOVA_CONFIG_ROOT`, confirmed by the two distinct `target/example-profiles/
  system_open_world-<pid>-<nanos>/` directories above).

**Verdict: PASS.** The repeat Create did not collide; the per-run sandboxed
`NOVA_CONFIG_ROOT` isolates the two worlds as designed.

### system_open_world_identity

- Reached Playing and ran its map/blip watch: `identity: PASS no blip
  carried a ship name over 108 map frames, 108 with the intact code and 108
  with the derelict code`. Ends `autopilot: cycle complete, no panic
  (t=36.1s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/system_open_world_identity-3181009-1791472249720498433/`
  - `worlds/probe-world/{world.ron,world.lock,state.1.ron}` present.

**Verdict: PASS.**

### system_session_loop

- Full step sequence observed: launch -> ESC -> Back to Main Menu (first,
  session-only pass with no world) -> New Game -> `name the world` -> create
  the world (press/release) -> combat/session -> ESC again -> **Back to
  Main Menu again** (this is the leave-save path for the created
  "Probe World") -> `the loop closed on the same counts`. Ends `autopilot:
  cycle complete, no panic (t=3.7s)`.
- Leave-save/teardown line at the second Back to Main Menu:
  `nova_world: the world on hand is gone, dropping 2 job(s), 15 prepared
  sector(s), 0 frozen sector(s) and 0 unsettled wait(s), retiring 0 live
  sector(s) and despawning 0 top-level body(ies)`. "0 unsettled wait(s)"
  means nothing needed the settling-frames wait in this short run; no stall,
  no timeout, no visible failure text was logged between the click and the
  next step (~1.2s).
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/system_session_loop-3183491-1791472523480230869/`
  - `worlds/probe-world/{world.ron,world.lock,state.2.ron}` present.

**Verdict: PASS, no regression observed.** The leave-save wait did not stall
or error in this run; the world was still written and locked correctly.
Caveat: this run's world had nothing unsettled (0 unsettled waits), so it
does not exercise the up-to-600-frame settling wait itself, only that the
wait path completes and leaves a good save when there is nothing to settle.

### system_menu_boot

- `menu_boot: clicked Create` -> `the menu tore down and gameplay state is
  up`. Ends `autopilot: cycle complete, no panic (t=6.2s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/system_menu_boot-3184235-1791472655581490657/`
  - `worlds/probe-world/{world.ron,world.lock,state.1.ron}` present.

**Verdict: PASS.**

### bug_failed_assets

- Uses its own per-run temp root inside `stage_broken_mods`
  (`/tmp/nova-failed-assets-<pid>/config`), not the shared
  `target/example-profiles` convention; this matches the stated exception.
- Step log shows `failed_assets: name the world` then `create the world`
  (widget up/aim/press/release), then `the recovered game reached
  gameplay`, then `both fatal presentations read back`. Ends `autopilot:
  cycle complete, no panic (t=2.0s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches (the example's
  own deliberately-broken-mod handling logs its fatal report through the
  named step lines above, not through the strings grepped for here).
- The temp root under `/tmp/nova-failed-assets-<pid>/` is removed by the
  process on exit/next run (`remove_dir_all` at the top of
  `stage_broken_mods`), so no post-hoc directory listing is possible; the
  `name the world` / `create the world` step lines are the evidence that
  Create ran against the World Name Field for this example.

**Verdict: PASS.**

### loop_world_start (screenshot capture example)

- Full sector-streaming walk completed: speed-before/after-burn checks,
  locked a generated ship, sector retirement and arrival, window slide.
  Ends `autopilot: cycle complete, no panic (t=73.0s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/loop_world_start-3185264-1791472814930002237/`
  - `worlds/probe-world/{world.ron,world.lock,state.3.ron}` present.

**Verdict: PASS.** (Headless output here proves the flow completed and
wrote state; it does not prove the captured frames look right - no visual
claim is made.)

### screenshot_menu (screenshot capture example)

- Step log: Settings capture, then `start a new game` -> `focus the seed
  field` / `type the seed` -> `capture the world setup window` -> `focus
  the name field` -> `the name field takes the keyboard` -> `type the world
  name` -> `create the world` (press/release) -> `reach the first flight`
  -> `Create started the typed seed`. Ends `autopilot: cycle complete, no
  panic (t=28.1s)`.
- `grep -n "ERROR\|panicked\|refused\|exists"`: no matches.
- Profile folder:
  `target/example-profiles/screenshot_menu-3186655-1791473027721138747/`
  - `worlds/probe-world/{world.ron,world.lock,state.1.ron}` present.

**Verdict: PASS.** (Same headless caveat as above: appearance of the
captured frames is not asserted here.)

## Summary

All 6 requested examples (8 process runs counting the `system_open_world`
repeat) reached their stated goal state (Playing after Create, or the
example's own named PASS condition) with no `ERROR`, `panicked`, `refused`,
or `exists` lines in any log, and each left a `probe-world` folder with
`world.ron` + a `state.N.ron` under its own sandboxed
`NOVA_CONFIG_ROOT/worlds/` (except `bug_failed_assets`, whose own
self-cleaning temp root is proven instead by its step log). The
`system_open_world` repeat shows two distinct profile folders, each with its
own independent `probe-world`, confirming no `NameTaken` collision and no
shared folder between runs.

## Skipped

- `lesson_*` capture examples: skipped per instructions (time-boxed, captures
  only).
- No comparison against `master`: not needed, no failure was observed.

## Remaining uncertainty

- `system_session_loop`'s leave-save exercised the "0 unsettled wait(s)"
  branch only; the up-to-600-frame settling wait path (D2 option a in
  `GATE.md`) itself is not exercised by this run, only that leaving through
  Back to Menu after a Create still produces a readable `world.ron` +
  `state.N.ron` with no error.
- Headless/lavapipe output proves state and log markers, not the visual
  appearance of the World Name Field or the modal; no screenshot was
  inspected pixel-by-pixel here.
- `bug_failed_assets`'s world folder was not directly listed (self-cleaning
  temp root); only the step-log evidence that Create ran is available.
