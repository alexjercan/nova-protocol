# Review: probe multi-run + aggregate

- TASK: 20260719-210438
- BRANCH: feature/probe-all
- ROUND: 1

## What I tried to break

- **Aggregate dishonesty** (a green table over thin evidence): measured
  n/total is a mandatory column, the banner carries the SKIPPED-is-not-
  held note, overall = worst row with unknown verdicts ranked as FAIL
  (fail-closed severity, unit-pinned), and the live list e2e shows a
  wired 5/6 row next to an unwired 2/6 row - the thinness reads at a
  glance.
- **Silent exclusions**: NOT_PROBED is threaded through resolution into
  the manifest and rendered with reasons; category expansion records what
  it skipped. An explicit name still runs (with a printed note) - the
  operator outranks the default, the default outranks silence.
- **A dead run hiding**: rows come from each run's own checks.json; a run
  that produced none becomes an ERROR row carrying the message, and the
  sweep continues (continue-on-failure verified by code path; a FAIL row
  cannot abort the fleet). Re-render keeps the manifest's recorded row
  when a dir lost its checks.json - deleting evidence cannot upgrade a
  verdict.
- **Parser drift**: the drift test now CALLS nova_probe's parser (inline
  copy deleted); the parser is fail-closed (missing keys, uncategorized
  paths, duplicates, name/category collisions, discovery-on all ERROR)
  and the collision check is what keeps name-or-category resolution
  unambiguous forever.
- **Web regression**: `--platform web` bypasses catalog resolution (its
  positional is a scenario id) - pinned in parse tests and guarded
  against multi specs. Single-run and `--out` semantics unchanged (the
  ui e2e ran through the same run() as before).
- **Accidental fleet sweep**: bare `probe run` exits 1 with the catalog;
  `--all` is the only run-everything trigger; spec+--all contradicts.

## Findings

- R1.1 (NIT, accepted): a second multi run into the same base replaces
  the previous aggregate while older per-example dirs persist - the
  index's spec line says what it covers; recorded in the close-out.
- R1.2 (NIT, recorded for T2): ui rows measuring 2/6 are correct today
  and are precisely T2's backlog - the aggregate is the motivation
  artifact.
- R1.3 (accepted): `--all` itself was not e2e-run in-cycle (25-40 min
  cold); it is T2's exit gate by design, and every mechanism it uses
  (resolution minus NOT_PROBED: unit-pinned; sequential driver +
  aggregate: live-proven on category and list specs) is covered.

## Verdict

APPROVE - land after user testing, per the flow.
