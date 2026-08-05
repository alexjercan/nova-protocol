# Notes: menu_scenarios segfaults during teardown after a successful run

## Problem Statement

Running the ui example smoke suite fails about one time in five, and the
failure names nothing: `example menu_scenarios exited with None`. There is no
panic, no stall, no assertion message - the process is simply gone.

The run itself was fine. Every completion sentinel the suite looks for is
present in the failing run's own log, including the last line the example ever
prints. The process dies AFTER all its work is done, while it is shutting down.

Who feels it: anyone running the suite locally, human or agent. A green suite
is the signal that the examples still work, and a signal that is wrong one run
in five is not a signal - it trains its reader to re-run until it passes, which
is exactly how a real failure gets waved through. It cost this repo an hour
already: it surfaced while closing an unrelated task and had to be triaged from
scratch before that task could be believed.

CONFIRMED with the user 2026-08-05: fix the crash at its source, not the report
of it.

Not this:

- Not the driven-click flake (`20260805-091151`). Different fault, already
  fixed; that one exits 1 with an explicit stall line.
- Not the smoke harness's diagnostics. Reporting "killed by SIGSEGV" instead of
  "exited with None" would have saved the triage, and it is worth doing, but it
  is a separate concern from stopping the crash. Left unclaimed on purpose.
- Not a CI failure. CI renders with lavapipe (`.github/workflows/ci.yaml:43`);
  every observed instance of this crash is inside the NVIDIA driver on the
  developer box.
- Not scenario loading. The brief guessed the process died mid-scenario-load
  because the captured stderr TAIL ended there; the tail is truncated to 48 KB
  and the full log shows the run finishing normally.

## Context

### What actually happens, from the core dump

`coredumpctl` caught the crash (PID 648290, 2026-08-05 11:27:06). The faulting
thread is `Async Compute Task Pool`, and its stack is unambiguous:

| Frame | What it says |
| --- | --- |
| `PipelineCache::start_create_compute_pipelines` | a pipeline-compile task, spawned on `AsyncComputeTaskPool` |
| `Arc<CoreComputePipeline>::drop_slow` -> `compute_pipeline_drop` | the task is finishing and dropping its pipeline |
| `Arc<ShaderModule>::drop_slow` -> `Arc<Device>::drop_slow` | that drop releases the LAST reference to the wgpu Device |
| `wgpu_hal::vulkan::CommandEncoder::drop` | so the device teardown runs here, on the task thread |
| `libnvidia-glcore.so.595.84 + 0xfa5800` | SIGSEGV |

So: the app exits while a pipeline is still compiling. The main thread drops
its half of the render resources, the compile task drops the other half, and
whichever loses the race destroys the Vulkan device from a task-pool thread
while the driver is still being used elsewhere. The NVIDIA driver segfaults.

### Surfaces

| Where | Why it is in play |
| --- | --- |
| `crates/nova_core/src/lib.rs:92-98` | the single funnel: every app and example builds `DefaultPlugins` here, already customised through `.set(...)` for assets, log and window |
| `examples/ui/menu_scenarios.rs` | the only SELF-ENDING example in the tree - it exits the moment its script finishes, roughly 7 s in |
| `tests/examples_smoke.rs:285-320` | asserts `output.status.success()` and reports `.code()`, which is `None` for a signal death |
| `bevy_render-0.19.0/src/render_resource/pipeline_cache.rs:805-822` | `create_pipeline_task` spawns on `AsyncComputeTaskPool` unless told to be synchronous |
| `bevy_render-0.19.0/src/lib.rs:133` | `RenderPlugin::synchronous_pipeline_compilation` is the existing switch |

### Measured

| Run | Result |
| --- | --- |
| 20 x `menu_scenarios`, unmodified, `DISPLAY=:99` | 2 SIGSEGV (rc 139), 18 pass - 10% |
| Both failures | matching `Async Compute T[...]: segfault ... in libnvidia-glcore.so.595.84` in `journalctl -k`, one core dump each |
| Failing run's log vs a passing run's log | identical to the last line: `harness completion: all collectors done, exiting` |
| `journalctl -k` since 2026-08-04 | 8 of these segfaults, 0 OOM kills |

The OOM hypothesis in the brief is DISPROVEN: the kernel log has no OOM
records at all, and the signal is SEGV, not KILL.

### Why only this example

`menu_scenarios` is the only example that ends itself; the rest run out the
autopilot's lifetime and exit tens of seconds later, by which time every
pipeline has long since compiled. The race needs a compile still in flight at
exit, and only the early exit provides one. This matches the record: 218 runs
of the other pointer-driving examples during `20260805-091151` produced zero
signal deaths.

Assumption: the shipped game is exposed to the same race in principle (a
player quitting during the first seconds of load), but nobody has observed it
and this task does not go looking.

### Constraints

- User, this session: fix the crash at the source, not the reporting of it.
- User, standing: no full `cargo test` or `cargo clippy` locally; CI owns both.
- Any fix must be provable against a 10% failure rate, so "it passed once" is
  not evidence - a run count in the dozens is the minimum.
- The examples must keep exercising the REAL app. A fix that makes the smoke
  suite run a different app than the game is not a fix.

### Unknowns

- Whether lavapipe has the same race. It very likely does at the wgpu level;
  it just does not segfault. Does not block: the fix removes the race, not the
  driver's reaction to it.
- Whether this ever bites a player on quit. Does not block - see the
  assumption above.

## Ideas

### 1. Compile pipelines synchronously in the app builder

`RenderPlugin { synchronous_pipeline_compilation: true }`, set once in
`AppBuilder::new` alongside the three plugins it already customises. No task
is ever spawned, so there is no second owner of the device at exit and the
race cannot occur - on any driver, in the harness and in the game alike.

Cost: pipeline compilation blocks the thread that asks for it. For the
examples this is invisible (measured below), and for the game it trades a
hitch-free first frame for hitches spread over the first seconds. That cost is
real and is the reason ideas 2 and 3 exist.

Placed first because it removes the mechanism rather than the symptom, it is
about five lines in the one place the tree already funnels plugin
configuration through, and it needs no new concept - the switch is bevy's.

### 2. Idea 1, gated to debug builds

Same change, behind `#[cfg(feature = "debug")]`. Every example in the smoke
suite is built `--features debug`; a release build of the game is not, so it
keeps async compilation and its smoother warm-up.

Cost: the shipped configuration is no longer the tested one. That is the exact
thing the last constraint above warns about, and it is why this places second
rather than first - it buys back a frame-time cost nobody has complained about
by weakening the suite's meaning.

### 3. Idea 1, gated on `NOVA_AUTOPILOT`

Same change, applied only when the autopilot is driving. Narrowest blast
radius of the three.

Cost: `nova_core` would have to read an env var owned by `nova_autopilot`,
which is a layering it does not have today, and it shares idea 2's flaw -
driven runs would render through a different pipeline path than real ones.

### 4. Wait for in-flight pipeline compiles before exiting

Hold the exit until `PipelineCache` reports nothing in `Creating`. Fixes the
race without touching how the game compiles anything.

Cost: needs a new exit-gating mechanism reaching into the render sub-app, and
it only closes the window it knows about - any other task holding a device
reference at exit reopens it. Strictly more machinery than idea 1 for a
narrower guarantee.

### 5. Exit the process without unwinding

`std::process::exit` once the app has returned success: skip every destructor,
so no Vulkan teardown, so no crash. Two lines.

Cost: it does not fix the race, it stops observing it, and it would hide every
future teardown bug in the game - including ones that lose data. Recorded
because it is the tempting one, and rejected for that reason.
