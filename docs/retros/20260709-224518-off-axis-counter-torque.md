# Retro: Off-axis counter-torque for a damage-shifted single drive

- TASK: 20260709-224518
- BRANCH: feat/off-axis-counter-torque (local branch by user request, no
  sprout worktree; squash-merged to master as 878dc4b)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES on commit hygiene + doc staleness,
  round 2 APPROVE)

## What went well

- **The user decision came first and stayed cheap.** The task's step 1 was an
  explicit design fork (hard zero-lateral vs bounded drift); one question at
  flow start settled the objective before any code, and the decision is
  recorded in TASK.md, the LATERAL_PENALTY doc comment, and the design doc.
- **The second physics path earned its place.** All four solver unit tests
  passed while the feature was actually broken end to end: in the autopilot's
  world frame, one tick of bounded drift tilts the error direction, the
  recruit's forward coefficient goes epsilon-negative, and at full stick the
  demand equality has zero slack - the projection crushed the recruit to zero
  after the first tick. Only the autopilot physics test saw it. This is the
  torpedo-shootdown lesson (under-modeled test environments) paying off:
  integration coverage on BOTH input paths was written up front, not as an
  afterthought.
- **Instrumented debugging over guessing.** A temporary dump of the
  allocation coefficients per tick turned "peak recruit input 0.044" into
  "forward = -0.0017 against a zero-slack equality" in one run, and the fix
  (recruits bill their whole force vector to the penalty, forward = 0) fell
  out of the diagnosis. The model correction is documented in
  docs/retros/20260710-off-axis-counter-torque.md.
- **Seeding choice replaced a regularizer.** Seeding the solver at the
  primary-set uniform throttle makes the seed a stationary point on balanced
  ships, so idle laterals stay exactly dark with no effort-penalty term - old
  behavior preserved bit-for-bit, pinned by a dedicated unit test.

## What went wrong

- **R1.1: `git add -A` swept three pre-existing uncommitted user edits into
  the feature commit.** Root cause: this task ran on a local branch in the
  main checkout (user request) instead of the usual sprout worktree, and the
  commit habit assumed the isolation the worktree normally guarantees. The
  working tree was shared state and I staged it blind. Cost a history rewrite
  in the address round.
- **The first constraint model died exactly at the headline use case.** Signed
  forward coefficients for recruits looked more general on paper, but the
  boundary regime (full-stick demand = zero equality slack) is precisely the
  damage case the task exists for. The regime enumeration happened after the
  failing test rather than at design time.
- **The first autopilot test asserted the wrong invariant** (heading held vs
  world identity) for a maneuver whose correct behavior is to turn and chase
  the drift it created. One debug loop to realize the test, not the code, was
  wrong; the manual-path test is where the straight-heading claim belongs.

## What to improve next time

- **In no-worktree mode, treat the working tree as shared:** check
  `git status` before the first commit and stage explicit paths only - never
  `git add -A` / `git commit -a` in the main checkout.
- **Before implementing an optimizer change, enumerate the constraint's
  boundary regimes** (saturation, zero slack, empty sets) against the task's
  headline scenario; if the scenario lives on a boundary, design for the
  boundary first.
- When a physics test encodes "the ship should not rotate", ask whether the
  controller is *supposed* to rotate in that scenario before blaming the code.

## Action items

- [x] Retro written; commit hygiene lesson recorded here (proposed as an
  AGENTS.md commit-guidelines line - user's call, it is a global file).
- [ ] Playtest the recruited-lateral feel (drift magnitude vs straightness at
  LATERAL_PENALTY = 0.05); retune the constant if drift reads as sliding.
