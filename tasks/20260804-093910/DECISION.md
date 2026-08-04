# Decision: scope of the example retirement and the screenshots reduction

- DATE: 20260804-093910
- STATUS: ACCEPTED
- TASK: 20260804-093910
- TAGS: examples, testing, probe

## Context

Retiring `examples/gameplay/` and `examples/ui/nova_os_rtt_poc.rs` and reducing
`screenshots/` to capture-only touches three boundaries the plan had to settle
before implementation: a `Cargo.toml` key another task also claims, a
`NOT_PROBED` entry a CLOSED task deliberately kept, and how far the beat-script
conversion should reach.

## Decision

1. **This task deletes `[package.metadata.nova_probe]`, not `094006`.**
   `20260804-094006` (absorb `perf/` into `stress/`) also lists "delete the
   `fps_exempt` KEY" in its Steps, and its DoD greps for its absence. Taking it
   here anyway: this task's DoD 1 greps `Cargo.toml` for `broadside`, and
   `fps_exempt = ["broadside"]` is a hit. Deleting only the string would leave
   `fps_exempt = []`, an empty orphan. The table has exactly one key and nothing
   reads `package.metadata.nova_probe` any more (`20260804-093855` deleted
   `parse_fps_exempt`/`load_fps_exempt` and their re-exports), so the whole
   table goes.

2. **`NOT_PROBED`'s `render_scale_shot` entry STAYS.** NOTES.md flagged it as
   possibly redundant once `screenshots/` left probe's `--all` wholesale. It is
   not redundant by decision: `20260804-093855` (CLOSED) recorded that
   `NOT_PROBED` is the per-EXAMPLE axis and `CATEGORY_POLICIES` the per-CATEGORY
   one, kept separate on purpose so the aggregate report can say which kind of
   decision excluded a run - and its spec fixtures now depend on a `NOT_PROBED`
   example living inside a PROBED category. Deleting the entry here would reopen
   a question that task closed. Out of scope; no change.

3. **The reduction converts beat scripts to driver steps, including
   `nova_os`.** The DoD proofs only force `orbit`/`juice`/`combat` (the
   `playing_since` holders). `screenshot_ui` and `screenshot_nova_os` are
   converted too, which is more work than the greps demand. Why: the Steps'
   phrase is "delete the per-example hacks the driver now owns", and `nova_os`
   holds the last hand-rolled `HarnessCompletion::done` and the last
   stage+wait-counter machine in `screenshots/`. Leaving it converts four files
   onto the driver vocabulary and leaves a fifth as the counter-example that
   invites the next writer to copy it. One review pass, one idiom.
   `render_scale_shot` is NOT converted: it has no beat script to convert (a
   single `NOVA_SHOT` capture) and no probe wiring to strip.

## Alternatives considered

- **Leave `fps_exempt` to `094006`** and delete only the `"broadside"` string
  here. Rejected: it leaves an empty orphaned table, and DoD 1 would still need
  the string gone, so the two tasks would both half-touch one key.
- **Delete the `NOT_PROBED` `render_scale_shot` entry** as NOTES.md suggested.
  Rejected: it reopens a question `20260804-093855` closed, and its spec
  fixtures depend on the entry.
- **Convert only the three producers the DoD greps force.** Rejected per
  decision 3 - a surviving stage machine is a template for the next writer.

## Consequences

- `094006`'s corresponding step becomes a verification, not an edit. Its DoD
  (`! rg -n 'fps_exempt|examples/perf' Cargo.toml crates tests`) stays
  satisfiable either way, whichever of the two lands first.
- Risk taken on `nova_os`: its per-beat `settle` frame counts are tuned so
  `save_to_disk` lands before the next beat navigates away. They are carried
  over verbatim as `predicate::frames(n)` rather than re-derived, and the
  `NOVA_REEL` PNG run in the DoD is what catches a regression - the smoke path
  alone would not.
- Realized during work: the same asynchrony bites the END of every converted
  producer. `capture_window` registers no completion collector, and the driver
  reports done the moment the last step ends, where the old scripts idled out a
  runway. Every converted producer therefore ends on a capture step whose
  `until` is the settle hold. See the task close-out.
