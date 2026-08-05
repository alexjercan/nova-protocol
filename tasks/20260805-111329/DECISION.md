# Decision: compile pipelines synchronously in the app builder

- DATE: 20260805-114019
- STATUS: ACCEPTED
- TASK: 20260805-111329
- TAGS: rendering, testing, examples

## Context

`menu_scenarios` dies of SIGSEGV in about one run in five, inside the NVIDIA
driver, AFTER the run has completed successfully - every completion sentinel
is in the failing run's own log. A core dump (`NOTES.md`) shows the faulting
thread is a bevy pipeline-compile task on `AsyncComputeTaskPool`: as it
finishes it drops the last `Arc<Device>` and tears the Vulkan device down from
a task-pool thread while the exiting main thread is doing the same. The race
needs a compile still in flight at exit, which is why only the one SELF-ENDING
example in the tree hits it - it exits about 7 s in, while the others run out
the autopilot's lifetime and have long finished compiling.

The forces: the smoke suite's whole value is that it runs the REAL app, so a
fix that makes the examples render differently from the game weakens what a
green suite means; and the fix must be provable against a 10% failure rate,
not a single passing run.

## Decision

Set `RenderPlugin { synchronous_pipeline_compilation: true }` in
`AppBuilder::new`, alongside the assets, log and window plugins it already
customises through `.set(...)`.

With no compile task there is no second owner of the device, so the teardown
race cannot occur - on any driver, in the harness and in the shipped game
alike. It fixes the mechanism rather than the symptom, in the one place the
tree already funnels plugin configuration through, and it introduces no new
concept: the switch is bevy's own
(`bevy_render-0.19.0/src/render_resource/pipeline_cache.rs:811`).

Built from scratch today the same call holds. The alternative default buys
warm-up smoothness the game has never been measured to need, and pays for it
with a shutdown race that has cost this repo real triage time twice.

Prototyped before deciding: 30/30 clean with zero kernel segfaults, against a
2/20 baseline, and no run-time cost (7.0 s median vs 8.0 s).

## Alternatives considered

- **Gate it behind the `debug` feature.** Every smoke example is built
  `--features debug` and the release game is not, so the game would keep async
  compilation. Rejected: the suite would then green-light a rendering
  configuration the shipped binary does not use, which is exactly the property
  the examples exist to protect.
- **Gate it on `NOVA_AUTOPILOT`.** Narrowest blast radius. Rejected for the
  same reason, plus it would make `nova_core` read an env var owned by
  `nova_autopilot` - a layering the tree does not have today.
- **Hold the exit until `PipelineCache` reports nothing in `Creating`.**
  Leaves rendering untouched. Rejected: a new exit-gating mechanism reaching
  into the render sub-app, for a narrower guarantee - it closes only the window
  it knows about, and any other task holding a device reference at exit
  reopens the race.
- **`std::process::exit` after a successful run.** Two lines, skips every
  destructor, so no Vulkan teardown and no crash. Rejected: it stops observing
  the race rather than fixing it, and would hide every future teardown bug in
  the game.
- **Do nothing.** The suite keeps failing one run in five with a message that
  names nothing (`exited with None`). Deferring costs a re-triage every time
  someone new hits it, and trains its readers to re-run until green - which is
  how a real failure gets waved through.

## Consequences

Easier: the ui smoke suite becomes trustworthy again; a signal death in it
now means something is actually wrong. The game gains a shutdown that cannot
race a compile task, so quitting during load is safe by construction.

Harder: pipeline compilation now blocks the thread that asks for it. In the
examples this is unmeasurable, but in real gameplay a shader compiled on first
appearance stalls that frame instead of skipping the item - so a first
encounter with an unseen material can hitch where it previously did not. If
that ever shows up in a frame-time capture, the honest fix is to warm the
pipelines at load, not to put the race back.

Unclaimed and deliberately out of scope: `tests/examples_smoke.rs` still
reports a signal death as `exited with None`. Naming the signal would have
turned an hour of triage into one line, and it stays worth doing on its own.
