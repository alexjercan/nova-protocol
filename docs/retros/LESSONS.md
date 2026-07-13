# Lessons ledger

One or two lines per lesson: slug, count, one sentence, a retro id or two
(an id resolves to `tasks/<id>/RETRO.md`). /compound appends new lessons or
bumps counts. At three occurrences a lesson
moves to Pending promotions for the user to fold into AGENTS.md or a skill;
promoted lessons stay listed with their date. Keep entries SHORT - when a new
occurrence adds a variant, sharpen the one sentence instead of appending a
paragraph. Seeded 2026-07-11 from 104 retros; heavily condensed 2026-07-13.

## Process lessons

- `diagnostic-first` (x8): trace the exact reported scenario, with real
  numbers, before theorizing a mechanism. 20260711-140241, 20260712-172035.
- `fail-first-regression-ab` (x10, PROMOTED 2026-07-11 -> work skill): prove a
  bug fix by failing its test against the pre-fix behavior; record the numbers.
  20260711-180426.
- `delivery-guards-on-null-assertions` (x5, PROMOTED 2026-07-11 -> review
  skill): "nothing happens" tests need proof the stimulus fired. 20260710-231931.
- `verify-first-plan-steps` (x7, PROMOTED 2026-07-11 -> plan skill): plan steps
  that state a mechanism, formula, ordering, API shape, or "system Y will
  accept X" must cite the verifying file; follow data into the consumer's
  gates and enumerate consumers of shared state. 20260712-093044, 20260712-203353.
- `scripted-walks-skip-the-bridges` (x1): a scenario walk that fires events by
  hand proves the script, not the game; each consumed event needs one pin that
  drives the production bridge. 20260713-150343.
- `collider-needs-a-rigidbody` (x1): an avian Collider without a RigidBody
  registers no contact pair, silently; diff a silent physics test's bundle
  against a production spawn before theorizing. 20260713-150343.
- `landing-chain-and-stub-collision` (x1): land with one &&-chain
  (merge --squash && commit && sprout rm), and commit tatr stubs on master
  before sprouting so the merge cannot abort on a collision. 20260713-121605.
- `landing-no-cd` (x3, PROMOTED 2026-07-11 -> flow skill): squash-merge from
  the main checkout, its own command, no cd, `pwd` first. 20260709-160753.
- `record-the-exact-rig` (x3): evidence notes record the rig (systems run,
  command path, components) or they mislead the next session. 20260709-125640.
- `probe-surfaces-adjacent-issues` (x1): run de-risk probes for real; they pay
  beyond their stated question. 20260710-104421.
- `headless-shot-after-load` (x1): `BCS_SHOT` captures black (shoots before
  assets load); inject `Screenshot::primary_window` from the autopilot script
  at a settled moment instead. 20260710-104421.
- `registered-system-for-change-detection` (x2): `run_system_once` builds a
  fresh system per call, so Changed/Added filters fire on everything and
  MessageReader cursors reset; register the system once and reuse the
  SystemId. 20260713-082330, 20260713-110311.
- `run-system-once-always-changed` (x1): same trap on Res::is_changed - gate
  behavior needs an App-driven test across real frames. 20260712-093831.
- `observer-over-spawn-site` (x1): attach a derived component to every entity
  of a kind with an `On<Add, Marker>` observer, not by hunting spawn sites.
  20260712-203345.
- `worktree-shares-main-target` (x1, CORRECTED): a fresh sprout worktree has an
  empty `target/` - accept the cold build; do NOT share `CARGO_TARGET_DIR` with
  the main checkout (same crates, artifacts clobber; a worktree binary silently
  linked master's code in 20260709-131502). Shell cwd resets between commands.
- `commit-before-sabotage` (x2, PROMOTED 2026-07-11 -> work skill): commit the
  fix before A/B sabotage; anchor scripted splices on unique strings and
  compile immediately after. 20260710-231930, 20260713-082330.
- `production-faithful-rigs` (x6, PROMOTED 2026-07-11 -> work skill): test rigs
  must mirror production - scheduling, every system that ticks the state, the
  real body-vs-collider hierarchy an engine observer needs, and the shipped
  CONFIGURATION (a rig on default AssetPlugin settings "verified" a meta fix
  the app's meta_check never read). 20260711-103527, 20260712-133343,
  20260713-175416.
- `presence-vs-behavior-tests` (x2): component-exists assertions stay green
  while behavior regresses; assert the behavior. 20260709-160753.
- `sweep-then-delete` (x5): before deleting, moving, or swapping a mechanism or
  marker, grep the workspace for its symbol names, its describing words, and
  everything that observes or queries it (`On<Add, X>`, `With<X>`) - including
  comments, docs, examples, tests, and the CHANGELOG. 20260711-212519, 20260712-133343.
- `reread-after-insert` (x2): after inserting into a function or test, re-read
  the whole thing for bindings, assertions, or invariants the insertion
  duplicated or broke. 20260710-214316.
- `does-the-old-element-survive` (x2): when a design adds an element
  overlapping an existing one, decide explicitly what happens to the old one.
  20260711-000547.
- `one-cargo-test-filter` (x4): `cargo test` takes one filter and one `-p` per
  invocation; separate runs otherwise. 20260713-082324.
- `check-all-targets-for-struct-field` (x1): a new non-Default field breaks
  every initializer, but plain `cargo check --workspace` skips examples/tests;
  use `--all-targets`. 20260712-140250.
- `endpoint-only-color-reasoning` (x1): evaluate the intermediate frames of a
  color or wave transition, not just its endpoints. 20260712-152340.
- `data-source-over-schedule-fight` (x2): when a fix needs impossible ordering,
  change where the data comes from instead of fighting the schedule.
  20260710-231928.
- `if-feasible-must-be-answered` (x1): a plan's "if feasible" hedge must be
  answered explicitly. 20260709-160753.
- `discrete-not-continuous-filters` (x1): compensate a frame-stepped filter
  from its actual update equation, not its continuous limit. 20260711-121711.
- `dependency-fix-first-reruns-symptom` (x1): after a dependency fix, re-run
  the original symptom before interpreting old traces. 20260709-125640.
- `spike-fix-record` (positive, PROMOTED 2026-07-11 -> spike skill): multi-task
  spikes keep a living fix-record section. 20260711-103527.
- `tatr-same-second-collision` (x6): consecutive `tatr new` calls in one second
  silently share an ID; one call per tool invocation, never chained.
  20260712-143832, 20260713-175415.
- `bg-session-authors-on-branch` (x1): background sessions cannot Write in the
  shared checkout, and parallel sessions sweep loose files there; author task
  and doc content inside the first sprouted worktree, only `tatr new` stubs
  touch the main checkout. 20260713-175415.
- `copied-pattern-weakest-target` (x1): a rendering pattern copied from an
  upstream example carries implicit device requirements; check its downlevel
  flags/limits against the weakest shipped platform (WebGL2) before adopting.
  20260713-175415.
- `state-diff-aliases-reset` (x1): deriving events by diffing state makes a
  reset look like a batch of events; guard the non-event transitions
  (teardown, load, clear). 20260712-125342.
- `landing-checkout-not-yours` (x3): parallel sessions share the in-place
  checkout; verify `git branch --show-current` before EVERY commit.
  20260525-133004.
- `pair-matrix-on-collider-class-change` (x1): changing a collider's class
  (sensor?, events?) must be checked against every collider category in the
  game, not just the pair being fixed. 20260712-121101.
- `verify-scripted-edits-applied` (x1): a sed/python replace that matches
  nothing looks like success; assert the replace count or grep the new text.
  20260712-110730.
- `reuse-production-helpers-in-tests` (x2): tests compose expected values and
  spawn rigs via the production helpers, not inline re-derivations.
  20260711-121839, 20260712-110730.
- `constant-offset-is-rig-math` (x1): an error invariant across interpolation
  alpha implicates the rig's math, not the timing under test. 20260711-121839.
- `ab-toggle-via-vcs-not-sed` (x1): toggle a fix off via stash/checkout, not by
  sed-editing source. 20260711-121839.
- `confounded-knob-experiment` (x1): before concluding a knob A/B, grep every
  reader of the knob. 20260711-140234.
- `quat-angle-noise-floor` (x1): f32 quat angle_between floors around 1e-3 rad;
  assert an order above it or compare components. 20260711-140241.
- `audit-state-gates-on-new-entry-path` (x2): a new route into a state needs a
  workspace grep of run_if/in_state and a written what-newly-runs list per
  context. 20260711-180426, 20260711-212519.
- `bound-scheduling-both-sides` (x1): a system inserted between a producer and
  a same-schedule reader needs both .after(producer) and .before(downstream).
  20260711-180501.
- `set-gates-miss-observers` (x1): gating a SystemSet does not touch observers;
  enumerate systems + observers + hooks before claiming a gate covers "input".
  20260711-185156.
- `would-it-fail-without-it` (x4): a verification that cannot fail with the
  mechanism deleted proves nothing; copied tests inherit vacuousness.
  20260711-180426, 20260711-212521.
- `out-of-context-review-pass` (positive, x10): a fresh-context review of a
  substantial branch catches MAJORs shared-session eyes miss, and re-derives
  load-bearing claims instead of trusting them. 20260712-133343.
- `required-component-in-shared-query` (x2): a required fetch added to an
  existing query narrows its membership and every gate computed from it; fetch
  `Option<&T>` or use a separate query. New `Res<T>` params also panic every
  `run_system_once` rig missing the resource. 20260712-143832, 20260712-164031.
- `spike-open-question-pays-off` (positive, x1): a spike that names a risky
  unknown lets the implementer resolve it before wiring. 20260712-143832.
- `authored-vs-derived-values` (x2): author content against measured runtime
  values exported as consts, not nominal constants or folklore ranges.
  20260711-180455, 20260711-180506.
- `verify-engine-guarantees-in-source` (x1): read the engine's docs/source
  before designing around an ordering guarantee (observer order between
  observers of one event is arbitrary). 20260525-133004.
- `advertised-but-unwired` (x3): a config surface is not a capability until its
  producer/consumer wiring, data source, and runtime preconditions are
  verified in the new context. 20260712-093044, 20260712-093831.
- `cross-cycle-warning-with-numbers` (positive): write discovered hazards into
  the queued task's TASK.md with measured numbers and a fallback exit.
  20260711-140234.
- `verify-at-deploy-base-path` (x1): base-path-dependent behavior must be
  verified at the real subpath, not local root. 20260712-093048.
- `reuse-known-good-stack` (x1, positive): scaffold new sub-projects by copying
  a working reference toolchain verbatim. 20260712-093048.
- `measure-before-writing-the-number` (x1): never write a specific quantity
  into a doc from a mental model; backfill from an actual run. 20260712-105505.
- `ab-isolation-bench` (x1, positive): attribute one system's cost with two
  worlds identical except for that system. 20260712-105505.
- `verify-bevy-api-at-callsite` (x1): before writing an unfamiliar Bevy
  bundle/field, copy an existing in-repo callsite; the 0.x API churns.
  20260712-131348.
- `spike-reuse-over-new-infra` (x1, positive): when a request implies new
  infrastructure, first check whether an existing substrate covers the real
  need. 20260712-131348.

## Domain lessons (nova-protocol specific)

- `two-clocks` (family): FixedUpdate consumers read raw Position/Rotation;
  render-rate consumers read eased Transform/GlobalTransform; one computation
  uses one clock from one frame, and consumers of PostUpdate-written state
  must slot after their producer. Full rule and fix record:
  tasks/20260711-103527/SPIKE.md.
- `global-transform-stale-in-fixedupdate` (family): GlobalTransform in
  FixedUpdate is last frame's propagation; avian child-collider poses are one
  tick stale. See the two-clocks spike.
- `degenerate-inertia-frames` (x1): avian's eigen sort gives even a symmetric
  ship a cyclic-permutation local frame; test frame composition with
  non-identity frames. 20260709-125640.
- `assert-each-gesture-step` (x1): modal/chorded input tests assert state after
  every step of the gesture, not event counts at the end. 20260711-173237.
- `modal-input-observer-dispatch` (x1): when bevy_enhanced_input's condition
  DSL fights a modal gesture, dispatch in an observer reading the modifier's
  TriggerState. 20260711-173237.
- `half-ticked-compound-steps` (x1): tick a plan step only when every clause is
  done, or split it. 20260708-165704.
- `bei-app-finish-in-tests` (x1): bevy_enhanced_input needs `app.finish()` +
  `app.cleanup()` before spawning an action rig in tests. 20260708-165705.

## Pending promotions (3+ occurrences, user decides)

- `sweep-then-delete` (x5) -> work skill: the three-way grep rule (symbols,
  describing words, observers/queries) before closing any delete/move/swap.
- `tatr-same-second-collision` (x6) -> mechanical fix: teach `tatr new` to
  disambiguate same-second IDs, or an AGENTS.md rule "never chain tatr new".
- `landing-checkout-not-yours` (x3) -> flow/work skills: branch check before
  every commit; prefer a real worktree when asked for a branch.
- `would-it-fail-without-it` (x4) -> work/review skills: every verification
  must be able to fail.
- `one-cargo-test-filter` (x4) -> now noted in docs/development.md.
- `record-the-exact-rig` (x3) -> work skill's close-the-task step.

## Promoted (kept for history)

- 2026-07-11: `verify-first-plan-steps` -> plan skill; `fail-first-regression-ab`,
  `commit-before-sabotage`, `production-faithful-rigs` -> work skill;
  `delivery-guards-on-null-assertions` -> review skill; `landing-no-cd` -> flow
  skill; `spike-fix-record` -> spike skill; `tatr-same-second-collision` ->
  tatr skill gotchas; lessons-ledger step -> compound skill. (All in
  nix.dotfiles/home/modules/agents/skills.)
