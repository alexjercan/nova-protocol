# Retro: frame-capture fps-exempt + category window defaults

- TASK: 20260719-233732
- BRANCH: fix/probe-partial-fps
- REVIEW ROUNDS: 1 (APPROVE, two NITs both addressed)

## What went well

- **Reproduce-first against the CURRENT tree crossed out most of the task
  before any code.** The bug playbook's first move - inspect the user's own
  probe-runs field data at git_sha f2663b00 - showed playable AND scenario now
  emit full `frametime.csv` (`fps=ok`); only broadside times out. The loop work
  had landed since the task was filed, so the whole partial-emit + yield
  pipeline the task specified had NO remaining caller. What could have been a
  capture.rs/stats.rs surgery became two small nova_probe pieces (a metadata
  flag + a window default). The ledger's `verify-stale-brief-against-tree`
  applied to a bug's FIX SCOPE, not just its premise.
- **Configurable-by-data beat a hardcoded list.** Putting `fps_exempt` in
  `[package.metadata.nova_probe]` (cargo's sanctioned tool-config slot, next to
  the `[[example]]` catalog) means future narrative examples opt out with one
  line and no code change - which is exactly what the user asked for.
- **Proved the fix on the exact failing command.** `probe run broadside --fps`
  end-to-end: exit 0, verdict OK, "fps pass skipped", correctness green, honest
  report note. A bug fix verified against the reported scenario, not a proxy.
- **Self-review caught two real things** (a cosmetic `[n/N]` label that counted
  the skipped pass, and a plain-run showing an exempt note for a pass nobody
  requested) - both cheap, both fixed in the same cycle.

## What went wrong

- The first scope question (AskUserQuestion) was rejected: I led with the
  three-way scope FORK before explaining the mechanism. Root cause: I offered
  the decision before making sure the user shared the model that broadside
  "runs long but produces few frames" and that cycling already fixed the
  others. Re-explaining plainly, then re-asking, resolved it in one exchange.

## What to improve next time

- When reproduce-first RE-SCOPES a task, lead the checkpoint with the mechanism
  (what the evidence shows, in plain terms) and only then present the scope
  fork - the user cannot adjudicate a fork whose premise they have not yet been
  walked through.

## Action items

- [x] LESSONS.md: bump `verify-stale-brief-against-tree` to x3 (reproduce
  against the current tree can falsify a bug's FIX SCOPE, not just its
  premise) -> moves to Pending promotions.
- No follow-up code: the partial-emit machinery is deliberately not built
  (crossed out in TASK.md); re-file only if a non-cycling, non-exempt fps
  example ever appears.
