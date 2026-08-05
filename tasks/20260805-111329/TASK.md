# menu_scenarios is killed by a signal in the ui smoke, roughly 1 run in 5

- PRIORITY: 83
- TAGS: v0.10.0, bug, examples, testing
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955

## Story

`menu_scenarios` is intermittently KILLED BY A SIGNAL during the ui smoke
category - not a panic, not a stall, no exit code at all:

```
thread 'ui_reach_playing_without_panic' panicked at tests/examples_smoke.rs:314:9:
example menu_scenarios exited with None
```

Observed 2026-08-05 on `DISPLAY=:99` (Xvfb 1280x720), 1 failure in 5 runs of
`nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui`.
The other 4 passed, and a 23-example suite-shaped round passed 23/23.

`exited with None` is `ExitStatus::code() == None`: the process died on a
signal. The captured stderr tail (48 KB) ends mid-scenario-load, in the racer
section configs and the integrity/collider observers, with no panic, no
autopilot stall and no completion deadline - so the process was still doing
useful work when it went. The pointer beats before it had already succeeded.

Found while closing `20260805-091151` (the driven-click flake); it is a
DIFFERENT fault - that one exits 1 with an explicit stall line - and was not
introduced by its fix.

The signal is now known: SIGSEGV in the NVIDIA driver, on the
`Async Compute Task Pool` thread, AFTER the run completes. A bevy
pipeline-compile task still in flight at exit drops the last `Arc<Device>` and
tears the Vulkan device down from a task-pool thread while the main thread is
exiting. `NOTES.md` carries the core-dump stack; `DECISION.md` chooses the fix:
compile pipelines synchronously in `AppBuilder::new`, so no compile task ever
owns a device reference and the race cannot occur - in the harness and in the
shipped game alike.

## Steps

- [ ] Add `render_plugin()` to `crates/nova_core/src/lib.rs`, beside
      `window_plugin()` and `log_plugin()`: returns
      `RenderPlugin { synchronous_pipeline_compilation: true, ..default() }`.
      Carry a NOTE comment naming the teardown race and this task ID - the
      value looks like a free perf knob and must not be flipped back blind.
      Import it as `bevy::render::RenderPlugin` - it is NOT in bevy's prelude
      (`bevy_render-0.19.0/src/lib.rs:74-95` exports only `ViewPlugin` and
      `WindowRenderPlugin` from that module).
- [ ] Wire it into the `DefaultPlugins` chain in `AppBuilder::new`
      (`crates/nova_core/src/lib.rs:92-98`) as a fourth
      `.set(render_plugin())`, so every app and example in the tree gets it -
      no feature gate, no env gate (`DECISION.md`, alternatives 1 and 2).
- [ ] `nix develop --command cargo fmt` and
      `nix develop --command cargo check --workspace --features debug`.
- [ ] Prove it against the failure rate: 30 consecutive `menu_scenarios` runs
      under `DISPLAY=:99`, each expected to exit 0. Baseline for the same loop
      is 2 failures in 20 (`NOTES.md`).
- [ ] Check the kernel log over that loop's window for `Async Compute` +
      `segfault` records; expect zero. Record the count in `NOTES.md`
      alongside the pass count and median run time, against the 2/20 and 8.0 s
      baseline.
- [ ] Run the ui smoke category once end to end:
      `nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui`.

## Definition of Done

- `AppBuilder::new` builds `DefaultPlugins` with synchronous pipeline
  compilation, unconditionally.
  (cmd: `grep -n 'synchronous_pipeline_compilation: true' crates/nova_core/src/lib.rs`)
- 30 consecutive `menu_scenarios` runs exit 0.
  (cmd: `nix develop --command bash -c 'for i in $(seq 30); do DISPLAY=:99 NOVA_AUTOPILOT=1 cargo run --quiet --example menu_scenarios --features debug >/dev/null 2>&1 || exit 1; done'`)
- The ui smoke category passes.
  (cmd: `nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui`)
- The after-numbers - pass count, kernel segfault count, median run time - are
  written into `NOTES.md` against the recorded baseline, because nothing else
  outlives the run. (manual: read `NOTES.md`)

## Notes

- Discovered facts, from `NOTES.md`: not OOM (kernel log has zero OOM
  records), not scenario loading (the truncated 48 KB stderr tail misled the
  brief; the full log shows the run finishing), not lavapipe/CI - every
  observed instance is the NVIDIA driver on the developer box.
- `menu_scenarios` is the only SELF-ENDING example in the tree, which is why
  it alone hits this: it exits ~7 s in with a compile still in flight, while
  every other example runs the autopilot's lifetime out and has long finished
  compiling. 218 runs of the other pointer-driving examples produced zero
  signal deaths.
- The flag is a no-op on wasm, macOS and non-`multi_threaded` builds
  (`bevy_render-0.19.0/src/render_resource/pipeline_cache.rs:805-836` compiles
  a second `create_pipeline_task` that always blocks). So the web build is
  unaffected by this change, and cannot regress from it.
- Assumption: `RenderPlugin`'s other two fields (`render_creation`,
  `debug_flags`) stay at their defaults; `..default()` preserves whatever
  `DefaultPlugins` would have used.
- RISK, stated rather than buried: against a 10% base failure rate a clean run
  of 30 has about a 4% chance of being a fluke. The kernel-log count is the
  second, independent reading - a run that segfaults without failing the
  process would still show up there - and the prototype already scored 30/30
  clean with 0 kernel segfaults before the decision was taken. If either
  reading is dirty, this plan is wrong, not unlucky.
- Base redness of the run-loop proof is the recorded 2/20 measurement from
  this task's own investigation today, not a fresh 30-run baseline; the loop
  costs ~4 minutes per pass and the baseline was measured with kernel-log
  corroboration.
- Out of scope on purpose (`DECISION.md`): `tests/examples_smoke.rs` still
  reports a signal death as `exited with None`. Filed separately as
  `20260805-114935` - naming the signal is worth an hour of triage but is a
  different change.
