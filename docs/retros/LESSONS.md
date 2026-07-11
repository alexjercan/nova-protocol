# Lessons ledger

One line per recurring lesson; /compound appends new lessons or bumps
counts, linking the retros that earned them. A lesson at three or more
occurrences moves to Pending promotions for the user to fold into
AGENTS.md or a skill; promoted lessons stay listed with their promotion
date so counts keep history. Seeded 2026-07-11 from the corpus at 104
retros.

## Process lessons

- `diagnostic-first` (x6): trace the exact reported scenario before
  theorizing a mechanism; the trace has beaten the hypothesis list every
  time it has been tried. 20260709-125640, 20260711-103527,
  20260710-231930, 20260711-121701, 20260711-125225, 20260711-140241
  (frame trace overturned the "dither" theory in one run).
- `fail-first-regression-ab` (x9, PROMOTED 2026-07-11 -> work skill): a
  bug-fix regression is proven by failing it against the pre-fix
  behavior and recording the numbers. 20260711-103527 (7.1 rad/s -> 0),
  20260710-231931 (4.26 -> 0), 20260710-231930, 20260710-231928
  (54 px -> sub-pixel), 20260710-231929, 20260711-121711 (20 u drift),
  20260711-121839 (2.09 u -> 0), 20260711-140234 (bit-for-bit unchanged
  number falsified two placebo fixes), 20260710-214316 (ribbon at
  [0,0,-300] vs park [0,0,-250], the full 50u standoff).
- `delivery-guards-on-null-assertions` (x5, PROMOTED 2026-07-11 ->
  review skill): "nothing happens" tests need proof the stimulus fired.
  20260710-231931 (R1.1 MAJOR), 20260710-231930, 20260711-121701,
  20260711-121711, 20260710-214316 (expect-guard on the
  inside-envelope sample).
- `verify-first-plan-steps` (x4, PROMOTED 2026-07-11 -> plan skill):
  plan steps encoding mechanisms/formulas/orderings must cite the
  verifying file or be phrased verify-first. 20260710-231931 (torque
  -blind burn), 20260710-231930 (wrong overshoot algebra),
  20260711-121701 (balancer chatter on a single-engine ship); related
  ordering cases in 20260710-231928; 20260708-165705 (plan assigned
  DPadDown already bound to ORBIT - concrete key/button assignments must
  quote the current binding table).
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
- `production-faithful-rigs` (x3, PROMOTED 2026-07-11 -> work skill):
  clock/schedule test rigs must mirror production scheduling components;
  a clean trace on a non-faithful rig is not evidence. 20260711-103527,
  20260710-231930, 20260711-140234 (arrival dynamics proved
  wiring-dependent; the regression had to be re-wired to production
  mid-cycle).
- `presence-vs-behavior-tests` (x2): component-exists assertions stay
  green while the behavior regresses; assert the behavior.
  20260709-160753 (R1.2), applied in 20260710-231931.
- `sweep-then-delete` (x2): grep for consumers BEFORE deleting or
  slimming a symbol/readout. 20260711-000547, 20260711-125226.
- `reread-after-insert` (x2): after inserting into an existing function
  or test, re-read the whole function for bindings/assertions the
  insertion duplicated or obsoleted; a mid-test insertion left the
  pre-existing identical binding as a redundant shadow (R1.1).
  20260710-214316. Variant: when extending a resource, re-read consumer
  modules' documented invariants - new hint fields broke keybind_hints'
  "no rig, no keys, no hints" rule (R1.1). 20260708-165705.
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
- `reuse-production-helpers-in-tests` (x1): a test composing an expected
  value along a hierarchy should call the production composition helper
  (private is fine from the test module) instead of re-deriving inline;
  the inline version shipped a 0.148 u phantom offset. 20260711-121839.
- `constant-offset-is-rig-math` (x1): an error invariant across the
  interpolation alpha implicates the test rig's math, not the timing
  under test - timing artifacts scale with alpha. 20260711-121839.
- `ab-toggle-via-vcs-not-sed` (x1): toggle a fix off for A/B against the
  committed pre-fix state (stash/checkout), not by sed-editing source; a
  substitution restored two fields with one value. 20260711-121839.
- `confounded-knob-experiment` (x1): before concluding a knob-turning
  A/B, grep every reader of the knob - a global setting with two readers
  (crumb band + urgency denominator) attributed the whole effect to one
  and cost two placebo fix variants; cross wiring x knob variants
  instead of holding one factor fixed. 20260711-140234.
- `quat-angle-noise-floor` (x1): f32 Quat::angle_between of
  near-identical rotations floors around 1e-3 rad (acos near dot=1);
  angle assertions sit an order above it and say so, or compare
  components. 20260711-140241.
- `cross-cycle-warning-with-numbers` (positive pattern): when a landed
  cycle discovers a hazard for a QUEUED task, write the warning into
  that task's TASK.md with the measured numbers and an explicit
  fallback exit - it turned a would-be shipped regression into a
  planned diagnosis. 20260711-140234 -> 20260711-140241.

## Domain lessons (nova-protocol specific)

- `two-clocks` (family): FixedUpdate consumers read raw
  `Position`/`Rotation`; render-rate consumers read eased
  `Transform`/`GlobalTransform`, all poses in one computation from one
  frame. Full rule and fix record:
  docs/spikes/20260711-103527-twitching-family-two-clocks.md. The
  raw-clock spawn pattern lives in the shared `local_pose_in_root`
  (sections/mod.rs) since 20260711-114640; freshly spawned interpolated
  bodies also seed their easing `start` (20260711-121839) so the first
  rendered frame sits on the render clock. Generalizes beyond transforms:
  a consumer of state written in PostUpdate (e.g. the screen-indicator
  widget's arrow visibility) must slot after its producer, not in the
  default Update HUD set - a label mirrored the arrow one frame late.
  20260711-174840.
- `degenerate-inertia-frames` (x1): avian's eigen sort gives even an
  axis-aligned symmetric ship a cyclic-permutation local frame; frame
  composition code must be tested with both frames non-identity.
  20260709-125640.
- `assert-each-gesture-step` (x1): tests guarding modal/chorded input
  must assert the affected state after EVERY step of the gesture (press
  modifier, gesture, release), not count events at the end - an
  event-count e2e test passed coincidentally while bare CTRL misfired
  the cycle. 20260711-173237 (bug shipped by 20260708-165705).
- `modal-input-observer-dispatch` (x1): when bevy_enhanced_input's
  condition DSL fights a modal gesture (Chord ignores the binding value;
  the combiner + Start-on-Ongoing leak the unmodified gesture), dispatch
  in an observer reading the modifier action's TriggerState instead of
  stacking conditions. 20260711-173237.
- `half-ticked-compound-steps` (x1): a plan step bundling two actions
  ("verify + update docs") got ticked when only the first half was done;
  tick only when every clause is done, or split the step first.
  20260708-165704.
- `bei-app-finish-in-tests` (x1): bevy_enhanced_input finalizes its
  context registry in `App::finish`; an input test must call
  `app.finish()`/`app.cleanup()` before spawning an action rig or the
  ContextInstances resource does not exist. 20260708-165705.
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
