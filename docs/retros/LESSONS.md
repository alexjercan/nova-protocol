# Lessons ledger

One line per recurring lesson; /compound appends new lessons or bumps
counts, linking the retros that earned them. A lesson at three or more
occurrences moves to Pending promotions for the user to fold into
AGENTS.md or a skill; promoted lessons stay listed with their promotion
date so counts keep history. Seeded 2026-07-11 from the corpus at 104
retros.

## Process lessons

- `diagnostic-first` (x5): trace the exact reported scenario before
  theorizing a mechanism; the trace has beaten the hypothesis list every
  time it has been tried. 20260709-125640, 20260711-103527,
  20260710-231930, 20260711-121701, 20260711-125225.
- `fail-first-regression-ab` (x6, PROMOTED 2026-07-11 -> work skill): a
  bug-fix regression is proven by failing it against the pre-fix
  behavior and recording the numbers. 20260711-103527 (7.1 rad/s -> 0),
  20260710-231931 (4.26 -> 0), 20260710-231930, 20260710-231928
  (54 px -> sub-pixel), 20260710-231929, 20260711-121711 (20 u drift).
- `delivery-guards-on-null-assertions` (x4, PROMOTED 2026-07-11 ->
  review skill): "nothing happens" tests need proof the stimulus fired.
  20260710-231931 (R1.1 MAJOR), 20260710-231930, 20260711-121701,
  20260711-121711.
- `verify-first-plan-steps` (x3, PROMOTED 2026-07-11 -> plan skill):
  plan steps encoding mechanisms/formulas/orderings must cite the
  verifying file or be phrased verify-first. 20260710-231931 (torque
  -blind burn), 20260710-231930 (wrong overshoot algebra),
  20260711-121701 (balancer chatter on a single-engine ship); related
  ordering cases in 20260710-231928.
- `landing-no-cd` (x3, PROMOTED 2026-07-11 -> flow skill): the
  squash-merge is its own command, no `cd`, `pwd` first, from the main
  checkout. 20260709-160753, diegetic-autopilot retro, 20260711-125225
  (near-miss).
- `record-the-exact-rig` (x3): evidence notes and falsification closes
  record the rig (systems run, command path, components) or they
  mislead the next session. 20260709-125640 (origin), 20260711-121701,
  20260711-125225.
- `commit-before-sabotage` (x1 + scar, PROMOTED 2026-07-11 -> work
  skill): commit the fix before A/B sabotage; file-level `git checkout`
  restores the branch base, not your uncommitted work. 20260710-231930
  (~250 lines lost and redone).
- `production-faithful-rigs` (x2, PROMOTED 2026-07-11 -> work skill):
  clock/schedule test rigs must mirror production scheduling components;
  a clean trace on a non-faithful rig is not evidence. 20260711-103527,
  20260710-231930.
- `presence-vs-behavior-tests` (x2): component-exists assertions stay
  green while the behavior regresses; assert the behavior.
  20260709-160753 (R1.2), applied in 20260710-231931.
- `sweep-then-delete` (x2): grep for consumers BEFORE deleting or
  slimming a symbol/readout. 20260711-000547, 20260711-125226.
- `does-the-old-element-survive` (x2): when a design adds an element
  overlapping an existing one, the spike/plan must ask what happens to
  the old one. 20260711-000547, 20260711-125226.
- `one-cargo-test-filter` (x2): `cargo test a b c` errors after the slow
  compile; one substring filter or a module prefix per run.
  20260709-155922, 20260709-155920.
- `data-source-over-schedule-fight` (x2): when a fix seems to need
  ordering after something that is itself ordered after the consumer,
  change WHERE the data comes from (compose fresh poses) instead of
  fighting the schedule. 20260710-231928, 20260710-231929.
- `if-feasible-must-be-answered` (x1): a plan's "if feasible" hedge is a
  question the implementation answers explicitly - feasible-and-done or
  infeasible-because. 20260709-160753.
- `discrete-not-continuous-filters` (x1): compensating a frame-stepped
  filter takes the steady state of the actual update equation, not its
  continuous limit; keep regression bounds tight enough to tell them
  apart. 20260711-121711.
- `dependency-fix-first-reruns-symptom` (x1): after landing a dependency
  fix, re-run the original symptom against it before interpreting old
  traces or planning further fixes. 20260709-125640.
- `spike-fix-record` (positive pattern, PROMOTED 2026-07-11 -> spike
  skill): multi-task spikes keep a living fix-record section as the
  family's single source of current state. 20260711-103527 family.
- `tatr-same-second-collision` (x1, documented 2026-07-11 -> tatr skill
  gotchas): consecutive `tatr new` calls in one second silently share an
  ID; sleep between calls. Filed from the 2026-07-11 session.

## Domain lessons (nova-protocol specific)

- `two-clocks` (family): FixedUpdate consumers read raw
  `Position`/`Rotation`; render-rate consumers read eased
  `Transform`/`GlobalTransform`, all poses in one computation from one
  frame. Full rule and fix record:
  docs/spikes/20260711-103527-twitching-family-two-clocks.md.
- `degenerate-inertia-frames` (x1): avian's eigen sort gives even an
  axis-aligned symmetric ship a cyclic-permutation local frame; frame
  composition code must be tested with both frames non-identity.
  20260709-125640.
- `global-transform-stale-in-fixedupdate` (family): `GlobalTransform`
  inside FixedUpdate is the previous frame's propagation (eased since
  2026-07-09); avian child-collider poses are one tick stale. See the
  two-clocks spike and tasks/20260711-103527's audit table.

## Pending promotions (3+ occurrences, user decides)

- `one-cargo-test-filter` is at x2; the thrust-balancing retro already
  proposed promoting it to an AGENTS.md testing note on the third
  occurrence.
- `record-the-exact-rig` is at x3 and is a candidate for the work
  skill's close-the-task step ("record the evidence rig in TASK.md for
  any diagnostic or falsification").

## Promoted (kept for history)

- 2026-07-11: `verify-first-plan-steps` -> plan skill;
  `fail-first-regression-ab`, `commit-before-sabotage`,
  `production-faithful-rigs` -> work skill;
  `delivery-guards-on-null-assertions`, independent re-derivation note
  -> review skill; `landing-no-cd`, mid-flow feedback protocol,
  falsification-close legitimacy -> flow skill; `spike-fix-record` ->
  spike skill; `tatr-same-second-collision` -> tatr skill gotchas;
  lessons-ledger step -> compound skill. (All in
  nix.dotfiles/home/modules/agents/skills, same date.)
