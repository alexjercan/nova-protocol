# Review: fleet wiring + build-profile schema

- TASK: 20260719-210443
- BRANCH: feature/probe-fleet-wiring
- ROUND: 1

## What I tried to break

- **Vacuous wiring** (plugins added but not measuring): refuted by the
  gate itself - every wired row shows run_completed with a CLOSED bracket
  and invariants counted over real frames (287-306 per run), and
  spot-reads across four categories confirmed the details. An inert-line
  mistake would have shown SKIPPED "no timeline" rows; there are none
  outside render_scale_shot.
- **Invariant false positives at scale**: the task PREDICTED them; the
  fleet produced zero. The bounds are engine-level (health, finiteness,
  10x soft-cap absurdity), not scenario-tuned - that generality held.
- **Measurement-window preemption** (the known --fps-vs-self-ending
  interaction): perf_baseline's fix makes exit ownership EXCLUSIVE by
  construction (`!perf_armed()` gates the autopilot), and the --fps
  non-regression run proved the capture still owns its window. The
  skill documents the residual case (--fps on broadside's self-ending
  script) as an operator concern.
- **Schema drift**: v3 append into a v2 file is REFUSED (not silently
  mixed); v1/v2 parse with profile "unknown" (pinned); the fps run's
  live CSV shows the 18-column row with `dev` recorded and the report
  badge rendering. The web row hardcodes `release` because trunk builds
  release by construction (commented at the site).
- **Comment-splitting inserts** (the reorg's lesson): the staged-anchor
  script initially split four explanatory comments from their statements;
  caught by reading each insertion's context and moved above the comment
  runs before any commit.

## Findings

- R1.1 (fixed in-round, the gate's catch): perf_baseline had no exit
  path without the capture - the spike's "runs fine headless" assumption
  was false. Conditional harness added; aggregate all green.
- R1.2 (NIT, recorded): perf/ rows in a plain --all now measure 5/6 via
  the autopilot path, which slightly overstates "perf coverage" - the
  fps row itself still needs --fps. The aggregate's fps column shows
  skip on plain runs, so the surface is honest.
- R1.3 (accepted): the smoke suite does not gain perf_baseline (its
  NOT_SMOKED reason "probe owns it" still holds; adding it would spend
  CI seconds to duplicate what probe --all now covers).

## Verdict

- VERDICT: APPROVE - land per the user's standing stacked-flow authorization.
