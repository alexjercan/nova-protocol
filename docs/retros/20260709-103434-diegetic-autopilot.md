# Retro: Diegetic autopilot (STOP + GOTO through real actuators)

- TASK: 20260709-103434
- BRANCH: feature/diegetic-autopilot (squash-merged as afd665a)
- REVIEW ROUNDS: 1 (APPROVE, 3 NITs recorded, none blocking)

See `tasks/20260709-103434/TASK.md` for what shipped and
`docs/2026-07-09-diegetic-autopilot.md` for the design. This cycle REPLACED
the velocity-servo flight assist merged the same morning (52b582d) after the
user rejected its model; that context is most of what this retro is about.

## What went well

- **The redirect was cheap because the process caught it at a checkpoint.**
  The user's feedback ("the computer should fly the ship through its real
  actuators, not invisible forces") arrived as a spike request with explicit
  open questions; four targeted AskUserQuestion items settled scrap-vs-salvage,
  v1 actions, breakout semantics, and UI scope in one round, and every
  recommendation was accepted. Salvage-not-revert kept the churn small: the
  spool, capability scan, HUD shell, input plumbing and test harness all
  survived into the new model.
- **Physics-level tests found the two real dynamics bugs before review.**
  The GOTO test refused to pass until the arrival plan budgeted flip time
  (the ship sailed through the standoff at 30+ u/s because a 180 costs real
  seconds of un-braked travel) and until completion waited for engine
  spool-down (the dying burn pushed the "arrived" ship ~2 u/s off station).
  Both are "autopilot feels drunk" bugs that a diff review would never catch.
  The instrument-with-a-trace-then-remove pattern (position/velocity/phase
  printed every 100 ticks) diagnosed both in minutes.
- **Implementation found a genuine simplification and the plan was updated,
  not silently diverged from.** The planned camera-mode machinery was
  unnecessary - gating the manual rotation copy with `Without<Autopilot>`
  makes the mouse camera-only automatically; only the disengage re-seed
  observer was needed. The TASK.md step was rewritten to match reality.

## What went wrong

- **A full cycle (spike, plan, work, review, merge) shipped a model the user
  did not want.** Root cause: the original task said "assisted-vs-Newtonian
  default ... confirm at plan time", and I "confirmed" it against the task's
  own written lean instead of asking the user. The second round proved the
  fix costs one AskUserQuestion call; the miss cost a cycle. For decisions
  about how the game *feels* or what the fantasy *is* - as opposed to how
  code is structured - written leans in a task are hypotheses, not
  confirmations.
- **Two mechanical slips in the flow choreography:** `sprout rm` was run
  while the shell stood inside the worktree being removed (killed the shell's
  cwd mid-command), and the squash-merge was first run from inside the
  worktree (a silent no-op - merging the branch into itself). Flow's "cd back
  to the main checkout" step is literal; both slips were caught immediately
  but wasted a round-trip each.

## What to improve next time

- When a task or spike marks a design call "confirm with user", that means an
  actual user interaction (AskUserQuestion), not a re-read of the task. Feel
  and identity calls especially: one question round is always cheaper than a
  redirected cycle.
- Run flow's merge/cleanup steps from the main checkout, never from inside
  the worktree being merged or removed.
- Keep writing physics tests that assert the *outcome of the maneuver* (ends
  at rest, arrives at standoff) rather than intermediate values - both bugs
  were only visible because the assertions were about the end state.

## Action items

- [ ] Next: 20260709-095043 (feel polish + playtest retune with the user) -
  it now also owns the autopilot constants (margin, flip lead, floor,
  standoff) and the R1.3 held-burn debounce question.
- [ ] 20260709-103454 (diegetic instruments) parked at v0.5.0 per roadmap.
