# PROOF-CAMDEPS

Worktree: /home/alex/.cache/sprouts/nova-protocol/resumable-worlds (branch resumable-worlds)
All commands via `nix develop --command`, -j 8, no edits made.

## 1. cargo test -p nova_menu --lib leave

Command: `nix develop --command cargo test -j 8 -p nova_menu --lib leave`

```
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 191 filtered out; finished in 0.05s
```

PASS.

## 2. cargo test -p nova_menu --lib load_screen

Command: `nix develop --command cargo test -j 8 -p nova_menu --lib load_screen`

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.03s
```

Test: `tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one`
(crates/nova_menu/src/tests/load_screen.rs)

Panic (two systems, same cause):
```
thread 'TaskPool (18)' panicked at bevy_ecs-0.19.0/src/error/handler.rs:130:1:
Encountered an error in system `<Enable the debug feature to see the name>`:
Parameter `<Enable the debug feature to see the name>` failed validation: Resource does not exist
```
Backtrace names the two failing systems:
- `nova_ship::camera::chase::chase_camera_update_state_system` (crates/nova_ship/src/camera/chase.rs:219, `time: Res<Time>` at line 220)
- `nova_ship::camera::chase::apply_camera_resume_blend` (crates/nova_ship/src/camera/chase.rs:274, `time: Res<Time>` at line 276)

Root cause, by evidence:
- `git diff crates/nova_ship/src/camera/chase.rs` shows `apply_camera_resume_blend` is new in this
  branch and takes `time: Res<Time>` (the default/real clock). Its own doc comment says
  "`Time` is virtual here: a paused game holds the blend" -- the comment and the parameter type
  disagree; `Res<Time>` is the wall-clock resource, inserted only by `TimePlugin`/`MinimalPlugins`.
- `crates/nova_menu/src/tests/support.rs:65` builds the menu test app as bare `App::new()` plus
  explicit plugins; it never adds `TimePlugin`, so plain `Time` is never inserted (only
  `Time<Virtual>`, via `arm_save_fixture` at `crates/nova_world_base/src/test_support.rs:71`,
  `world.init_resource::<Time<Virtual>>()`).
- `WorldSaveTestPlugin` (crates/nova_world_base/src/test_support.rs:34-39, new file) always adds
  `ChaseCameraPlugin`, so this is the first fixture exercising these systems with a spawned
  camera controller (per task framing, the save fixture now spawns one). `load_screen.rs` is the
  first test in this run to combine `menu()` (no `TimePlugin`) with `WorldSaveTestPlugin` and
  `arm_save_fixture`.
- `chase_camera_update_state_system` (line 219-220) also reads `Res<Time>`, not `Res<Time<Virtual>>`;
  that parameter is unchanged by this diff (not in the diff hunk), so the same missing-resource
  gap was latent before this change and is now reached because the fixture spawns a camera
  controller.

Not fixed (out of scope): report only.

## 3. cargo test -p nova_menu --lib world_setup

Command: `nix develop --command cargo test -j 8 -p nova_menu --lib world_setup`

```
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 192 filtered out; finished in 0.02s
```

PASS.

## 4. cargo check -p nova_scenario -p nova_core --tests

Command: `nix develop --command cargo check -j 8 -p nova_scenario -p nova_core --tests`

```
Finished `dev` profile [optimized + debuginfo] target(s) in 23.57s
```

PASS. No errors. No warnings reported for any crate (only the pre-existing,
unrelated `proc-macro-error2` future-incompat notice, not file-scoped).
New-warning count in crates/nova_ship/src/camera, crates/nova_world_base/src/save,
crates/nova_menu/src: 0.

## 5. cargo check --example system_world_resume

First attempt without features failed fast:
```
error: target `system_world_resume` in package `nova-protocol` requires the features: `debug`
Consider enabling them by passing, e.g., `--features="debug"`
```
Example is registered (Cargo.toml:574-575, path `examples/systems/system_world_resume.rs`) but is
feature-gated on `debug` (consistent with CI's wasm job comment on debug-only harness examples).

Command actually used: `nix develop --command cargo check -j 8 --example system_world_resume --features debug`

```
Finished `dev` profile [optimized + debuginfo] target(s) in 0.68s
```

PASS. No errors, no new warnings.

## 6. wasm32 check

CI's wasm job (.github/workflows/ci.yaml:425-473) runs `cargo clippy --workspace --exclude
nova_probe_cli --exclude nova_channel --exclude nova_bench --target wasm32-unknown-unknown`.
Per this task's scope (affected checks only, no workspace-wide Clippy), ran `cargo check` scoped
to the three named crates instead, matching the task's literal instruction:

Command: `nix develop --command cargo check -j 8 -p nova_world_base -p nova_ship -p nova_menu --target wasm32-unknown-unknown`

```
Finished `dev` profile [optimized + debuginfo] target(s) in 19.82s
```

PASS. No errors, no new warnings. (Not run: full wasm32 Clippy pass with
`CLIPPY_CONF_DIR=ci/wasm-clippy`; that is workspace-wide Clippy and out of this task's scope.)

## Summary

| # | Check | Result |
|---|-------|--------|
| 1 | nova_menu --lib leave | PASS (9 passed) |
| 2 | nova_menu --lib load_screen | FAIL (1 failed, missing `Res<Time>` resource) |
| 3 | nova_menu --lib world_setup | PASS (8 passed) |
| 4 | nova_scenario + nova_core --tests check | PASS, 0 new warnings |
| 5 | system_world_resume example check | PASS (needs `--features debug`), 0 new warnings |
| 6 | wasm32 check (3 crates) | PASS, 0 new warnings |

Remaining uncertainty:
- Item 2's fix is not attempted here. The failure is reproducible and deterministic (not
  timing-sensitive): same two systems, same validation error, on every run of this exact test.
- Full wasm32 Clippy (the actual CI gate) was not run, per the no-workspace-Clippy constraint;
  `cargo check` on the three touched crates is a narrower proxy and could miss a Clippy-only wasm
  lint (e.g. the banned `Instant::now`/`SystemTime` calls in ci/wasm-clippy/clippy.toml).
- Did not inspect whether other nova_menu tests beyond `leave`/`load_screen`/`world_setup` also
  reach `WorldSaveTestPlugin` + `ChaseCameraPlugin`; only the three named filters were run.
