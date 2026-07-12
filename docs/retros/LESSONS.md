# Lessons ledger

One line per recurring lesson; /compound appends new lessons or bumps
counts, linking the retros that earned them. A lesson at three or more
occurrences moves to Pending promotions for the user to fold into
AGENTS.md or a skill; promoted lessons stay listed with their promotion
date so counts keep history. Seeded 2026-07-11 from the corpus at 104
retros.

## Process lessons

- `diagnostic-first` (x8): trace the exact reported scenario before
  theorizing a mechanism; the trace has beaten the hypothesis list every
  time it has been tried. 20260709-125640, 20260711-103527,
  20260710-231930, 20260711-121701, 20260711-125225, 20260711-140241
  (frame trace overturned the "dither" theory in one run). Balance-request
  variant: for "PDC damage too high, one-shots asteroids", tracing the actual
  numbers (20/hit x 100 rps vs 100-HP rock = ~5 rounds in 50ms) showed the
  "one bullet" was PERCEPTUAL (a stream), not literal - reframing the fix from
  damage-vs-HP to DPS/rounds-to-kill. 20260712-172035.
- `fail-first-regression-ab` (x10, PROMOTED 2026-07-11 -> work skill): a
  bug-fix regression is proven by failing it against the pre-fix
  behavior and recording the numbers. 20260711-103527 (7.1 rad/s -> 0),
  20260710-231931 (4.26 -> 0), 20260710-231930, 20260710-231928
  (54 px -> sub-pixel), 20260710-231929, 20260711-121711 (20 u drift),
  20260711-121839 (2.09 u -> 0), 20260711-140234 (bit-for-bit unchanged
  number falsified two placebo fixes), 20260710-214316 (ribbon at
  [0,0,-300] vs park [0,0,-250], the full 50u standoff), 20260711-180426
  (2.88 pre-fix drift vs 51.40 post-fix; the harness's naive >0.1 pass
  threshold would have passed pre-fix - only the A/B exposed the placebo).
- `delivery-guards-on-null-assertions` (x5, PROMOTED 2026-07-11 ->
  review skill): "nothing happens" tests need proof the stimulus fired.
  20260710-231931 (R1.1 MAJOR), 20260710-231930, 20260711-121701,
  20260711-121711, 20260710-214316 (expect-guard on the
  inside-envelope sample).
- `verify-first-plan-steps` (x5, PROMOTED 2026-07-11 -> plan skill):
  plan steps encoding mechanisms/formulas/orderings must cite the
  verifying file or be phrased verify-first. 20260710-231931 (torque
  -blind burn), 20260710-231930 (wrong overshoot algebra),
  20260711-121701 (balancer chatter on a single-engine ship); related
  ordering cases in 20260710-231928; 20260708-165705 (plan assigned
  DPadDown already bound to ORBIT - concrete key/button assignments must
  quote the current binding table). Consumer-gate variant: "system Y
  will accept X" claims must follow the data into Y's filter/gate logic,
  not stop at the query/function signature - a signature-level read
  missed the in-lambda gate that rejected Static bodies and would have
  broken the GOTO beat. 20260712-093044.
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
- `production-faithful-rigs` (x5, PROMOTED 2026-07-11 -> work skill):
  clock/schedule test rigs must mirror production scheduling components;
  a clean trace on a non-faithful rig is not evidence. 20260711-103527,
  20260710-231930, 20260711-140234 (arrival dynamics proved
  wiring-dependent; the regression had to be re-wired to production
  mid-cycle). 20260525-133025: a headless ammo test ran only the torpedo
  bay's fire system but NOT the separate `update_spawner_fire_state` that
  ticks its timer in production, so the bay fired once and never re-armed;
  and `Time<Virtual>`'s 0.25s max-delta clamp silently under-advanced a 2s
  manual dt, starving the 1s fire interval. Two rig-vs-production mismatches
  in one test - list every production system that ticks the state, and
  bound the manual dt to the virtual clamp. 20260712-133343
  (entity-hierarchy flavor): a typed-damage integration test built its target
  as a single flat entity (destructible_body + Collider, no RigidBody), so
  bcs's impact/blast observers read body1/body2 = None and the test read a
  false damage-of-zero; production sections are a RigidBody ROOT with child
  colliders holding the Health. When a headless target must be SEEN by an
  engine observer (collision/damage/integrity), mirror the real
  body-vs-collider hierarchy (grep a spawn site), not a flattened stand-in.
- `presence-vs-behavior-tests` (x2): component-exists assertions stay
  green while the behavior regresses; assert the behavior.
  20260709-160753 (R1.2), applied in 20260710-231931.
- `sweep-then-delete` (x5): grep for consumers BEFORE deleting or
  slimming a symbol/readout. 20260711-000547, 20260711-125226. Prose
  variant: comments/docs citing a retired invariant are consumers too -
  grep the workspace for the invariant's fingerprint, not just files the
  diff touches; two crates' comments described the opposite of reality
  after the gate moved (R1.1, R1.2). 20260711-212519. Behavior-words
  variant: sweep the deleted mechanism's DESCRIBING words ("ballistic",
  "seeds"), not just its symbol names - a fn doc in the edited file and
  a CHANGELOG Unreleased entry survived a clean symbol sweep.
  20260711-212504. Observer-consumer variant: when SWAPPING a marker
  component for a new one (bcs `BlastDamageMarker` -> nova `NovaBlast`),
  grep every consumer that OBSERVES or queries the old marker - `On<Add, X>`
  observers, `With<X>` queries, example loggers, test assertions - and
  retarget them in the same change. The torpedo blast VFX/SFX observers keyed
  on `Add<BlastDamageMarker>` would have silently stopped firing (no test
  covers particle spawning); the proactive grep, not a test, caught it, and
  two "no blast" test asserts were rescued from querying a now-never-spawned
  marker (a vacuous pass). 20260712-133343.
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
- `one-cargo-test-filter` (x3): `cargo test a b c` errors after the slow
  compile; one substring filter or a module prefix per run. The multi-package
  form is the same trap: `cargo test -p A f1 -p B f2` errors with "unexpected
  argument 'f2'" - one `-p` + one filter per invocation, separate runs for
  separate packages. 20260709-155922, 20260709-155920, 20260712-143832.
- `check-all-targets-for-struct-field` (x1): adding a non-`Default` field to
  a widely-constructed struct (config structs especially) breaks every
  initializer, but `cargo check --workspace` compiles only libs+bins - it
  gave a false all-clear while six `examples/*.rs` stayed broken (surfaced
  only via editor diagnostics). Run `cargo check --workspace --all-targets`
  so examples/tests/benches are caught in one pass. 20260712-140250.
- `endpoint-only-color-reasoning` (x1): a cross-hue RGB lerp passes
  through desaturated blends that read as washed white on a dark HUD -
  when designing or changing a color transition, evaluate the
  INTERMEDIATE frames, not just the endpoints; the emphasis pulse's
  cyan->gold mix shipped unreadable mid-blends that playtest caught.
  Sibling note: wave-based "did it move" test guards must avoid the
  wave's stationary points (a period/4 advance lands on the crest where
  the factor is 1.0). 20260712-152340.
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
- `tatr-same-second-collision` (x4, documented 2026-07-11 -> tatr skill
  gotchas): consecutive `tatr new` calls in one second silently share an
  ID; sleep between calls. Filed from the 2026-07-11 session; hit again
  in 20260711-180426 (five tatr new in one command -> one task) and AGAIN
  in 20260711-212519's spike phase (three in one && chain) despite the
  skill gotcha - the note does not fire when composing the command. FOURTH
  time in 20260525-133004's spike phase (three tatr new in one script ->
  one task, the other two overwritten) - the recovery is to rm the collided
  task and re-create the three in SEPARATE tool calls, seconds apart. The
  recurrence-despite-note is the argument for the mechanical fix in Pending.
  FIFTH in 20260712-143832's spike phase (three tatr new in one && chain ->
  one task); recovered with rm + separate calls each after a clock-tick
  busy-wait. Five sessions, same trap - the note still does not fire when
  composing a multi-`tatr new` command.
- `state-diff-aliases-reset` (x1, MAJOR): deriving domain events by
  diffing a state snapshot makes a state RESET look like a batch of
  events - scenario teardown emptied GameObjectives and the feedback
  diff played the success chime over green ghosts of the FAILED
  objectives. Before shipping cue/feedback logic on a diff, enumerate
  the transitions that are not domain events (teardown, load, clear)
  and guard them. 20260712-125342 (R1.1).
- `landing-checkout-not-yours` (x3): parallel sessions share the in-place
  main checkout, so one switching branches moves EVERY session's HEAD.
  20260712-125342 (near-miss): the flow landing assumed the checkout sat on
  the default branch, but a parallel session had switched it - the squash
  nearly staged onto the wrong branch before `git branch --show-current` was
  read. 20260712-133832: branch-guarded squash+commit then landed cleanly
  while master moved twice under a parallel session. 20260525-133004 (actual
  leak): a parallel /flow checked out master in this shared checkout
  mid-session, so a later `git commit` (a spike doc) meant for the feature
  branch landed on master. Recovered non-destructively - cherry-pick the
  stray commit onto the right branch via a temporary `git worktree`, then
  remove it from master with a compare-and-swap-guarded reset
  (`[ "$(git rev-parse HEAD)" = <sha> ]` before `git reset --hard`) so a
  concurrent advance aborts. RULE: verify `git branch --show-current` before
  EVERY commit, not just at session start; prefer a real sprout/worktree
  when asked for a branch. At x3 across three sessions - promotion candidate.
- `pair-matrix-on-collider-class-change` (x1, BLOCKER + MAJOR in one
  review): changing a collider's class (solid -> Sensor, adding/removing
  CollisionEventsEnabled) must be checked against EVERY collider
  category in the game, not just the pair being fixed - sensor bullets
  were eaten by trigger volumes (pirate un-hittable inside a beacon
  sphere) and tunneled through event-less invulnerable bodies; avian
  tangibility is a per-side 2x2 of (sensor?, events?). Enumerate the
  categories (ships, asteroids, invulnerable bodies, areas, beacons,
  crates, bullets, torpedoes, blast shells) before the tests.
  20260712-121101 (R1.1, R1.2).
- `verify-scripted-edits-applied` (x1, two MAJORs in one round): a
  python/sed replacement that matches nothing is indistinguishable from
  success - two test "fixes" no-opped against fmt-reflowed bodies and
  were reported as done in three documents without reading the files
  back; the reviewer caught both. Assert the replace count in the
  script or grep the expected new text before claiming any scripted
  edit. 20260712-110730 (R2.1, R2.2).
- `reuse-production-helpers-in-tests` (x2): a test composing an expected
  value along a hierarchy should call the production composition helper
  (private is fine from the test module) instead of re-deriving inline;
  the inline version shipped a 0.148 u phantom offset. 20260711-121839.
  Spawn-path variant: a UI test that hand-assembles a simpler bundle
  than production stays green while the real spawn panics - the
  objectives-panel duplicate-Node bundle panic shipped to a live
  playtest because the test spawned the bare bcs panel; production and
  test must call the same spawn helper. 20260712-110730 (R1.5).
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
- `audit-state-gates-on-new-entry-path` (x2): a change that adds a new
  route into an existing state (or a new mode-enum variant) must grep
  configure_sets/run_if/in_state for that state across the workspace and
  audit each hit against the new route, and each enum variant needs its
  own delivery-proved verification - the menu's NewGame path shipped with
  all spaceship input sets disabled because only the Sandbox route was
  exercised (R1.1 BLOCKER). 20260711-180426. Enumeration variant: a
  gating change that ENABLES systems in a new context needs a written
  what-newly-runs list per context, checked item by item - the menu
  orbiter's PD attitude hold running in MainMenu went unnoticed until the
  out-of-context review named it (R1.5). 20260711-212519.
- `bound-scheduling-both-sides` (x1): a system inserted between a
  producer and a downstream same-schedule reader (visibility propagation,
  UI layout) needs BOTH .after(producer) and .before(downstream) - .after
  alone leaves the downstream side to an arbitrary topo tie-break that
  any unrelated system can flip; and the contract is only real once a
  test registers a stand-in producer in the actual SystemSet.
  20260711-180501 (R1.1 MAJOR).
- `set-gates-miss-observers` (x1): gating a SystemSet does not touch
  observers - the pause gate froze the sets while all 14 input observers
  (autopilot verbs, weapon intents, cycles, camera look, scenario next)
  kept mutating the world; enumerate systems + observers + hooks before
  claiming a global gate covers "input". 20260711-185156 (R1.1 MAJOR).
- `would-it-fail-without-it` (x4): a verification that cannot fail with
  the mechanism deleted proves nothing - 20260711-180426 (harness pass
  threshold also passed pre-fix), 20260711-180455 (pixel-identical
  screenshots false-positived a broken scene), 20260711-185156 (frozen-
  ship e2e could not prove the input gate; a claimed unit test did not
  exist). Ask the question before ticking any verification step.
  Vacuous-by-copying variant: a new test copied from a neighboring test
  inherited its vacuousness (no-churn assert where a re-engage is
  bit-identical without the mutating system in the loop; sentinel
  mutation is the fix) - apply the question to copied test shapes too.
  20260711-212521 (R1.1 MAJOR).
- `out-of-context-review-pass` (positive pattern, x10, fourth catch: the
  ungated-observers MAJOR above): for a substantial
  branch reviewed in the implementer's own session, a fresh-context agent
  review caught a MAJOR (BCS_SHOT force-advance vs the Loaded hook) that
  shared-session eyes missed; the review skill's re-derive rule caught the
  BLOCKER the same round. 20260711-180426. Second cycle: caught the
  missing regression tests on the two bug-carrying systems and the
  OnExit-teardown generalization. 20260711-180455. Fifth: found stale
  invariant prose in crates the diff never touched and the one real
  behavioral delta (PD hold in MainMenu). 20260711-212519. Sixth (x6):
  caught a vacuous test MAJOR and also RULED OUT a suspected stuck-state
  by tracing completion semantics - the pass verifies non-issues, not
  just finds issues. 20260711-212521. Seventh (x7): caught two MAJORs
  (an every-frame HUD rebuild the new pulse induced, and a folklore
  numeric bound whose failure mode was a silent softlock - empirically
  confirmed by the demanded sweep), then mutation-tested the fix's
  regression in the re-review round. 20260711-180506. Eighth (x8):
  caught the sensor-sized highlight bracket (an inverted-behavior MAJOR
  invisible to component-state tests) and independently re-derived two
  load-bearing claims (command FIFO ordering, colors-do-not-wake-layout)
  the implementer had only asserted. 20260712-093831. Ninth (x9): caught a
  fail-closed coupling MINOR - a new REQUIRED component fetch folded into the
  existing `flyable` query would brick any controller missing the component -
  that the same-session author had convinced himself was fine.
  20260712-143832. Tenth (x10): on the typed-damage core it did not just read
  the diff - it independently re-derived the neutralization residual (~2e-4 at
  1e-6 mass) and the authored per-hit numbers (20.25 / 3.825) against bcs's own
  constants, and confirmed the `BlastDamageMarker` -> `NovaBlast` sweep was total
  (zero stragglers), turning an APPROVE into a verified one rather than a trusting
  one. 20260712-133343.
- `required-component-in-shared-query` (x2): adding a REQUIRED component fetch
  to an EXISTING system query narrows that query's membership set, so every
  gate computed from the query silently inherits the new precondition - folding
  `ControllerVerbs` into `q_computer` coupled `flyable` ("can this ship fly")
  to "does the controller carry the flags component", failing closed. When the
  new data is orthogonal to what the query already gates, fetch it `Optional`
  or in a separate query. Sibling of reread-after-insert /
  does-the-old-element-survive, applied to query membership. 20260712-143832.
  x2 (applied PROACTIVELY): adding `&LoadedBullet` to `shoot_spawn_projectile`'s
  turret query would have stopped every headless fire rig (they spawn turrets
  without the slot) from firing; recalled the lesson before running and used
  `Option<&LoadedBullet>` + a config fallback. The lesson caught the trap at
  design time instead of via a broken-test loop. 20260712-133349. Sibling
  (run_system_once variant): adding a `Res<T>` param to a system that tests drive
  via `run_system_once` makes every such rig panic on the missing resource (a
  RUNTIME error, not a compile error) - grep the callsites and `init_resource` it
  in each rig in the same change. Adding `Res<ButtonInput<KeyCode>>` to
  `update_turret_target_input` needed both turret-feed rigs to init it.
  20260712-164031.
- `spike-open-question-pays-off` (positive pattern, x1): a spike that names a
  risky unknown lets the implementer resolve it BEFORE wiring, not after a
  flake - the verb-flags spike flagged the OnStart spawn-vs-action ordering
  window, so the shakedown's initial GOTO-off was authored in config (off the
  instant the section is built) instead of an action that could run before the
  controller existed. 20260712-143832 (spike 20260712-143551).
- `authored-vs-derived-values` (x2): content authored against a system
  that derives its own runtime geometry/parameters (collider-derived body
  radius, well mu/SOI) must be placed using the runtime values, not the
  nominal constants - three scene bugs in one cycle (orbit inside the
  collider, ring inside the mesh, camera inside the rock) from reasoning
  with nominal sizes. Read the consumer's derivation before authoring.
  20260711-180455. Folklore-bound variant: using the derived FORMULAS but
  feeding them a range quoted from an observation comment is the same
  bug one level up - a 256-seed sweep measured [3.70, 5.64] against the
  comment's 4.0-4.55, a live beat-4 softlock; ranges consumed by content
  must be measured by a test and exported as consts the content test
  cites. 20260711-180506 (R1.2).
- `verify-engine-guarantees-in-source` (x1): before DESIGNING around an
  engine ordering/scheduling guarantee, read the engine's own docs/source -
  do not assume. The intuitive "variable damage by section type" design was
  a nova observer that scales `HealthApplyDamage.amount` before bcs's
  subtractor runs; grepping bevy_ecs found the flat statement that observer
  execution order between observers of the same event is ARBITRARY ("make no
  assumptions"), so that design would have raced and lost half the time. The
  source check cost minutes and turned the wall into the recommendation (own
  the trigger, pre-scale) instead of a shipped flaky system. 20260525-133004
  (spike docs/spikes/20260712-133135). Sibling of advertised-but-unwired:
  that one verifies YOUR wiring, this one verifies the ENGINE's contract.
- `advertised-but-unwired` (x3): a config surface (enum variant, doc
  claim, query candidate) is not a capability until its producer/consumer
  side is verified wired - grep for who fires/admits it before building
  on it. Targeting gate rejected Static bodies the query surface
  suggested were candidates (20260712-093044); EventConfig::OnUpdate
  existed and was documented "every frame" but nothing ever fired the
  event (20260711-180506). Same probe caught both, one day apart.
  Generic-mode-vs-this-anchor variant: a mode that IS wired and working
  must still have its DATA SOURCE read against the new consumer's entity
  shape - ApparentSize measures collider AABBs, which meant "visible
  hull" on every prior anchor (ships) and "8u pickup sensor" on the first
  sensor-only prop, ballooning the highlight bracket to the trigger
  volume (R1.1 MAJOR, caught in review). 20260712-093831. Deferred-fix
  corollary: R1.1 deferred the generic sensor exclusion because no
  CURRENT consumer anchored a sensor-only entity - but beacons are
  lockable, so the reticle-on-beacon path was reachable and cost a
  playtest round + a second cycle; when deferring, enumerate the
  entities the code path can MEET, not today's call sites.
  20260712-154318. Reuse-precondition variant (positive): before reusing a
  subsystem in a NEW context, verify its runtime PRECONDITIONS hold there, not
  just that its API fits - the gameplay `screen_indicator` compiles fine in the
  editor but its `ScreenIndicatorCamera` is attached only to the spaceship chase
  camera (absent in the editor's WASD-camera scene), so reusing it would have
  silently shown nothing; checking that before planning led to a small
  editor-local `world_to_viewport` projection instead. 20260712-163912.
- `run-system-once-always-changed` (x1): run_system_once registers a
  fresh system per call, so Res::is_changed/Added filters are ALWAYS true
  inside it - a change-detection-gated branch tested that way looks
  covered while being untestable; gate behavior needs an App-driven test
  across real frames (set -> acts, clear -> restores, quiet frames ->
  holds). 20260712-093831 (R1.3).
- `cross-cycle-warning-with-numbers` (positive pattern): when a landed
  cycle discovers a hazard for a QUEUED task, write the warning into
  that task's TASK.md with the measured numbers and an explicit
  fallback exit - it turned a would-be shipped regression into a
  planned diagnosis. 20260711-140234 -> 20260711-140241.
- `verify-at-deploy-base-path` (x1): behavior that depends on the deploy
  base path (subpath links, client-side `location.pathname` logic) must
  be verified base-path-faithfully, not just at local root where the
  assumptions can accidentally hold; a root-only screenshot passed while
  the subpath active-nav highlighted Home on every page (R1.1). A cheap
  deterministic simulation over the real page URLs is enough.
  20260712-093048.
- `reuse-known-good-stack` (x1, positive pattern): scaffolding a new
  sub-project by copying a working reference project's toolchain verbatim
  (webpack/eslint/prettier/partials) built first-try with zero config
  findings in review. Reuse beat authoring from scratch. 20260712-093048.
- `measure-before-writing-the-number` (x1): never write a specific quantity
  (deflection, ms/tick, a threshold) into a comment/doc/task from a mental
  model - leave a placeholder and backfill from an actual run. Guessed "~6.6u"
  (actual 3.25u) and "~1.5 ms/tick" (actual ~0.1) both had to be corrected;
  the failure mode when uncorrected is a shipped folklore number (cf
  `authored-vs-derived-values`). 20260712-105505.
- `ab-isolation-bench` (x1, positive pattern): to attribute one system's
  marginal cost, run two worlds identical except for that system (here: N
  gravity-affected bodies + a well vs N plain bodies, so the physics
  integrator cost cancels) and read the delta. It isolated the gravity system
  to per-entity allocation, not the O(wells) scan I assumed - and stopped a
  misdirected optimization. 20260712-105505.

- `verify-bevy-api-at-callsite` (x1): before writing a new Bevy
  bundle/struct-literal, grep the repo for an existing use of each unfamiliar
  component/field and copy its exact shape - the 0.x API churns. Wrote
  `TextFont { font_size: 9.0 }` (now a `FontSize` enum;
  `TextFont::from_font_size(..)`) and `BorderRadius::MAX` as a component (it is
  a `Node` field); both had in-repo callsites to copy and cost two check
  round-trips instead. 20260712-131348.
- `spike-reuse-over-new-infra` (x1, positive pattern): when a request's framing
  implies new infrastructure ("diegetic, on the weapon" -> a world-space 3D
  widget), the spike's job is to check whether an existing substrate already
  covers the real need. Reusing `screen_indicator` (rides/scales/hides for
  free) beat a net-new billboard+material+3D-text path, and occlusion - the
  new path's only edge - was actively wrong for a status readout.
  20260712-131348.

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
  20260711-174840. Same-frame variant: a Transform write queued alongside
  a controller-component removal gets overwritten by the controller
  running later that frame; write only on frames where the producer is
  already gone. 20260711-180455. Downstream variant: ordering after the
  producer is not enough when a same-schedule consumer reads the output
  later - see bound-scheduling-both-sides. 20260711-180501.
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

- `sweep-then-delete` is at x5 and recurred even WITH the prose variant
  freshly on record (same day) - promote to the work skill with the
  concrete grep rule: "before closing a task that deletes, moves, OR SWAPS a
  mechanism/marker, grep the workspace (1) for its symbol names, (2) for its
  describing words, and (3) for who OBSERVES/queries it (`On<Add, X>`,
  `With<X>`), covering comments, module docs, examples, tests and CHANGELOG".
  The x5 occurrence (20260712-133343) added the marker-swap/observer-consumer
  case, where the escaped consumer was a silent VFX observer no test covered.
- `tatr-same-second-collision` is at x4 despite the tatr skill gotcha -
  the note demonstrably does not fire when composing a multi-`tatr new`
  command, so promote a MECHANICAL fix over another note: teach `tatr new`
  to disambiguate same-second IDs (suffix or sub-second component), or an
  AGENTS.md rule "never chain/script tatr new - one call per tool
  invocation".
- `landing-checkout-not-yours` is at x3 across three sessions in two days,
  all from parallel sessions sharing the in-place main checkout - promote to
  the flow/work skills: verify `git branch --show-current` before EVERY
  commit (not just at session start), and prefer a real sprout/worktree when
  the user asks for a branch so the shared HEAD cannot be moved out from
  under the session. The deeper fix is that concurrent in-place sessions on
  one checkout is the root hazard.

- `would-it-fail-without-it` is at x3 across three consecutive cycles -
  candidate for the work skill's verify step and the review skill's test
  checklist ("every verification must be able to fail").

- `one-cargo-test-filter` is now at x3 (the third occurrence added the
  multi-package `-p A f1 -p B f2` variant) - promote to an AGENTS.md testing
  note: "cargo test takes one filter and one -p per invocation; separate runs
  for multiple filters or packages."
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
