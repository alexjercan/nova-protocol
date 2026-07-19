# Review: probe surface close-out

- TASK: 20260719-211500
- BRANCH: feature/probe-closeout (stacked on T3)
- ROUND: 1

## What I tried to break

- **Capability loss**: `trace`'s only unique power was tabling a chrome
  trace from OUTSIDE a probe run dir - confirmed nothing in the repo does
  that (probe's own trace.json always lives in a run dir, where
  `--profile` renders and `probe report` re-renders the table).
  `aggregate_system_costs`/`render_top_table` stay in the lib, used by
  the report path. The aliases forwarded to `run` forms that all still
  exist - the pointed errors carry the exact commands.
- **perf_web mistaken-removal recurrence**: the header now leads with
  "THE WASM APP `--platform web` measures - not a CLI" and names the
  perf.html data-bin linkage plus both near-miss tasks. Wasm check
  proves it still builds.
- **Dead code left behind**: `Cmd::Run` had no producer after the
  aliases - removed with its dispatch arm and `trace_table`; the
  compiler confirms nothing dangles (80/80, zero warnings).
- **Release-notes dishonesty**: the Unreleased bullets advertised
  "aliases forward for one release" and `probe trace` - both reworded;
  v0.8.0's notes now describe only what it ships.
- **Stack integrity**: the post-T3 sync's TASK.md conflict was resolved
  to the LANDED version, and the auto-merged skill/CHANGELOG were
  re-read for meaning, not just merged.

## Findings

- R1.1 (NIT, accepted): the retired-alias error is one shared message
  for sweep/web/profile (naming all three forms) rather than
  per-alias - one signpost covering three roads is fine at this size.
- R1.2 (note): USAGE no longer mentions the retired verbs at all; only
  the error path does. Intentional - usage is for the living surface.

## Verdict

APPROVE - land per the stacked-flow authorization. This closes the probe
strand.
