# Lessons ledger

One or two lines per lesson: slug, count, one sentence, a retro id or two
(an id resolves to `tasks/<id>/RETRO.md`). /compound appends new lessons or
bumps counts. At three occurrences a lesson
moves to Pending promotions for the user to fold into AGENTS.md or a skill;
promoted lessons stay listed with their date. Keep entries SHORT - when a new
occurrence adds a variant, sharpen the one sentence instead of appending a
paragraph. Seeded 2026-07-11 from 104 retros; heavily condensed 2026-07-13.

## Process lessons

- `diagnostic-first` (x9): trace the exact reported scenario, with real
  numbers, before theorizing a mechanism. 20260711-140241, 20260712-172035,
  20260711-183417.
- `fail-first-regression-ab` (x10, PROMOTED 2026-07-11 -> work skill): prove a
  bug fix by failing its test against the pre-fix behavior; record the numbers.
  20260711-180426.
- `delivery-guards-on-null-assertions` (x6, PROMOTED 2026-07-11 -> review
  skill): "nothing happens" tests need proof the stimulus fired, IN the same
  test - a cross-test guard through a shared helper does not count.
  20260710-231931, 20260711-183417.
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
- `messagereader-needs-resource-guard-in-tests` (x1): a system with a
  `MessageReader<T>`/event param added to a plugin panics in that plugin's
  MINIMAL-app tests (which omit `InputPlugin` etc, so `Messages<T>` is absent);
  gate it `run_if(resource_exists::<Messages<T>>)`. A new scroll system broke 4
  menu tests that only entered the state. 20260714-174126.
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
- `sweep-then-delete` (x6): before deleting, moving, or swapping a mechanism or
  marker, grep the WHOLE repo for its symbol names, describing words, and
  observers/queries - including root-level docs (README, AGENTS.md), the
  CHANGELOG, and text the same branch added earlier. 20260711-212519,
  20260712-133343, 20260712-211352.
- `reread-after-insert` (x2): after inserting into a function or test, re-read
  the whole thing for bindings, assertions, or invariants the insertion
  duplicated or broke. 20260710-214316.
- `does-the-old-element-survive` (x2): when a design adds an element
  overlapping an existing one, decide explicitly what happens to the old one.
  20260711-000547.
- `declared-but-not-loaded` (x1): a resource named in config/markup (font stack,
  asset URL, class hook) is not proof it is wired; grep for where it is actually
  imported/served before assuming it renders. 20260713-222025.
- `generated-links-need-real-targets` (x1): links rendered from a data manifest
  must be gated on the target existing (or marked unavailable), or they 404;
  a coming-soon flag rendered planned-but-unbuilt pages as non-links.
  20260713-225324.
- `ci-skips-client-render` (x1): a build-only CI proves the bundle compiles, not
  that client-rendered UI works; DOM logic needs a runtime check (headless DOM
  or an eyeball), which a green build does not give. 20260713-225324.
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
- `tatr-same-second-collision` (x7): consecutive `tatr new` calls in one second
  silently share an ID; one call per tool invocation, never chained.
  20260712-143832, 20260713-175415, 20260525-133028.
- `serde-scope-grep` (x1): before estimating a serde/derive (or any trait)
  migration, grep the whole target type tree for non-derivable members - raw
  `Handle`s, foreign-crate types, Reflect-only types; scope hides in the leaves
  ("2 handles" was really 13 + 3 foreign types, a whole second tier). 20260525-133028.
- `generate-data-from-code` (x1): migrate code-defined content to data files by
  serializing the code config with a parity test, never hand-authoring - provably
  faithful and sidesteps every format-syntax gotcha. 20260525-133028.
- `effect-not-just-helper` (x1): test a spawn/mutation action's EFFECT through the
  ECS harness (fire -> drain -> assert on the world), not just its pure sub-helper
  plus a non-asserting example; the helper passing hid an untested spawn loop.
  20260525-133028.
- `check-examples-skips-tests` (x1): `cargo check --workspace --examples` does NOT
  compile `#[cfg(test)]` code; after a type change verify with `cargo test
  --workspace --no-run` (CI's build) or a test build broke silently past the branch
  gate onto master (a stray `Handle` in a test helper). 20260525-133028.
- `test-the-production-load-path` (x2): exercise an asset load / merge the SAME way
  the production consumer does. A typed `asset_server.load::<T>` test masks failures
  the game hits via bevy_asset_loader's UNTYPED `load_untyped` kickoff (extension-only
  loader resolution, no by-type fallback); a test that calls the pure merge core
  directly bypasses the real system that reads it from a resource. Add the test that
  drives the production path, not just the convenient intermediate.
  20260714-163342, 20260714-134127.
- `stemmed-compound-extension` (x2): name a custom-asset file with a STEM so its bevy
  full extension (everything after the FIRST dot) equals the registered compound
  extension; a bare `bundle.ron`/`mods.ron` resolves to `ron` (no loader) under an
  untyped load. `<pack>.bundle.ron` / `<name>.content.ron` / `<name>.mods.ron`.
  20260714-163342, 20260714-134127.
- `stage-lock-with-manifest` (x1): a commit that changes a `Cargo.toml` dep list
  must stage `Cargo.lock` too; explicit-path `git add` (the no-worktree habit)
  silently drops the lock, leaving a stale `--locked`/CI build. Glance at
  `git status` for related generated files before committing. 20260714-113408.
  (In an isolated sprout worktree, `git add -A` is the safe fix - it caught
  everything in 20260714-113411, no recurrence.)
- `pin-the-fix-at-its-boundary` (x3, -> Pending promotions): guard a bug fix with a
  test that fails under the bug at the fix's OWN boundary (a unit test), not only a
  downstream e2e - especially when the existing unit test passes under the bug (the
  DisableVerb multi-verb accumulation was only e2e-guarded). Refactor variant: when a
  refactor changes how an invariant is ENFORCED, re-pin the invariant on the new
  mechanism - don't massage the old assertion until it passes. Overlay variant: the
  section-overlay-by-id bug was invisible with one bundle (no id collision); extract
  the overlay into a pure helper and unit-pin last-wins so the divergence can't hide
  until a second bundle exists. 20260714-113411, 20260714-135642, 20260714-134119.
- `shared-id-space-shared-overlay` (x1): when one router dispatches into multiple
  containers that share an id space (a Vec of sections + a map of scenarios), route
  through ONE overlay helper so the kinds can't silently diverge (Vec push/first-wins
  vs map insert/last-wins). 20260714-134119.
- `verify-the-nit-compiles` (x1): a reviewer's micro-optimization NIT (remove this
  alloc, borrow instead of own) is a hypothesis - compile it before treating it as
  done; `rel.as_str()` for `rel.to_string()` failed E0597 (borrow outlived by the
  resolved path), so the owned string was load-bearing. 20260714-134119.
- `agent-interrupted-verify-worktree` (x1): a subagent that hits a long build can end
  with an ambiguous partial state and misleading "in progress" notifications; INSPECT
  the worktree (git status + compile + run the deterministic generators) before
  concluding done-or-broken. For data-file work the parity/generator write-on-missing
  usually completes it deterministically. 20260714-150508.
- `reconcile-plan-to-shipped` (x2): at close-out reconcile the plan's aspirational
  lists (which variants/scope actually shipped, deferrals, overstated guarantees)
  with reality BEFORE review - it keeps flagging stale plan text as findings.
  20260525-133028, 20260714-113411.
- `bg-session-authors-on-branch` (x1): background sessions cannot Write in the
  shared checkout, and parallel sessions sweep loose files there; author task
  and doc content inside the first sprouted worktree, only `tatr new` stubs
  touch the main checkout. 20260713-175415.
- `copied-pattern-weakest-target` (x1): a rendering pattern copied from an
  upstream example carries implicit device requirements; check its downlevel
  flags/limits against the weakest shipped platform (WebGL2) before adopting.
  20260713-175415.
- `additions-join-doc-indexes` (x1): adding an artifact of an enumerated kind
  (example, crate) must update the doc list that enumerates its kind; grep
  docs/ for a sibling's name before committing. 20260713-175352.
- `maskable-ci-conclusions` (x1): a continue-on-error step reports success
  even when its command fails - cite the job LOG's own result line as
  evidence, never the step/run conclusion, whenever the workflow modifies
  failure semantics. 20260710-143138.
- `insert-cluster-must-be-removed-as-a-cluster` (x1): a component insert can
  bring requires and hook-inserted companions; the matching remove must strip
  the whole cluster (requires do not cascade on removal) - and code copied
  from a context that never exercised a branch is unproven on that branch.
  20260712-201603.
- `event-driven-autopilot-beats` (x1): headless harness scripts stage each
  gesture on the game state it produces (locks, variables, components),
  never wall-clock windows - llvmpipe stutter collapses time windows into
  single frames and the fixed-timestep catch-up clamp lets sim time lag
  wall time; wall-clock belongs only in backstops. 20260712-211352.
- `checkpoint-before-building-on-an-audit` (x1): when a task plans a user
  checkpoint on an audit/table, ship the table with ZERO implementation
  behind it; building first turns the checkpoint into sunk cost.
  20260712-211352.
- `null-result-becomes-a-pin` (positive, x1): when an investigation lands on
  "cannot reproduce", convert the evidence rig into a permanent harnessed pin
  (error-handler-to-panic smoke example) so the non-behavior stays falsifiable
  and the rig's cost buys coverage. 20260713-175352.
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
- `confounded-knob-experiment` (x2): before concluding a knob A/B - or
  attributing the effects of a held input key - grep every reader of the
  knob/binding (Space drives fire AND the global burn; the "torpedo drift
  physics bug" was the ship's own engine). 20260711-140234, 20260713-203709.
- `recompute-both-sides-of-a-band` (x1): changing one side of a guard, band, or
  inequality (clearance floor, hysteresis pair, arrival margin) requires
  recomputing the OTHER side with realistic in-game magnitudes; when a value's
  MEANING changes (nominal -> geometric), re-ask every reader which meaning it
  wants. Shipped a playtest-visible "no stable band" regression once (see
  tasks/20260709-193338/NOTES.md).
- `distinct-refusal-reasons` (positive, x1): every refusal/disengage path logs
  its own distinct reason string; one pasted log line then names the failing
  gate (see tasks/20260709-193338/NOTES.md).
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
- `would-it-fail-without-it` (x5): a verification that cannot fail with the
  mechanism deleted proves nothing; copied tests inherit vacuousness - and a
  sabotage that refuses to go red refutes the assumed mechanism itself.
  20260711-180426, 20260711-212521, 20260712-115902.
- `out-of-context-review-pass` (positive, x14): a fresh-context review of a
  substantial branch catches MAJORs shared-session eyes miss, and re-derives
  load-bearing claims instead of trusting them - checking cited evidence IS
  the spawn site, re-running the sabotage or the whole smoke suite, reading
  the DEPENDENCY's source for composition hazards. 20260712-133343,
  20260711-183417, 20260712-115902, 20260712-211352, 20260712-201603.
- `required-component-in-shared-query` (x2): a required fetch added to an
  existing query narrows its membership and every gate computed from it; fetch
  `Option<&T>` or use a separate query. New `Res<T>` params also panic every
  `run_system_once` rig missing the resource. 20260712-143832, 20260712-164031.
- `spike-open-question-pays-off` (positive, x1): a spike that names a risky
  unknown lets the implementer resolve it before wiring. 20260712-143832.
- `authored-vs-derived-values` (x2): author content against measured runtime
  values exported as consts, not nominal constants or folklore ranges.
  20260711-180455, 20260711-180506.
- `verify-engine-guarantees-in-source` (x2): read the engine's source (or
  write a five-line probe) before designing around an ordering guarantee -
  observer order is arbitrary; observer-queued commands apply BEFORE the
  queue's remaining pending commands, not after. A subagent's reasoned
  verdict about engine semantics is a hypothesis, not evidence.
  20260525-133004, 20260712-115902.
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
- `trace-vehicle-timeline-first` (x1): pick a runtime-evidence rig by its
  script TIMELINE (when does the stimulus fire, inside the window?), not by
  scene content; for idle-scene targets the plain app beats any harnessed
  example. 20260711-183417.
- `silent-tool-missing-in-pipeline` (x1): a missing launcher (xvfb-run) dies
  with 127 that a trailing pipeline swallows, reading as a clean empty run;
  `which` host tools before the first long run and keep launcher exit codes
  out of pipelines. 20260711-183417.
- `no-source-edits-during-inflight-builds` (x1): cargo reads a crate's source
  when it COMPILES it, minutes into a cold build - a tree edited mid-build
  yields an indeterminate evidence binary; quiesce the tree (or file-copy)
  for A/B runs. 20260711-183417.
- `borrowed-rig-coverage-check` (x1): a rig/pattern borrowed from another
  task's record inherits that record's overclaims; verify its coverage
  against the NEW failure mode before prescribing it (the handler-swap pin
  cannot see baked-in remove/despawn warns). 20260712-115902.
- `refutation-invalidates-earlier-prose` (x1): when a probe overturns the
  working theory mid-task, re-read every artifact written under the old
  theory (notes, comments, records) in one pass; the review found the dead
  model still taught as fact. 20260712-115902.

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

- `tatr-same-second-collision` (x7) -> tatr skill / AGENTS.md: never issue two
  `tatr new` calls in the same second or one bash line - they share a
  second-resolution ID and the later silently overwrites the earlier. One `tatr
  new` per tool invocation.
- `pin-the-fix-at-its-boundary` (x3) -> review/work skill: guard a bug fix with a
  test that fails under the bug at the fix's OWN boundary (unit test), not only a
  downstream e2e; when a refactor changes how an invariant is enforced, re-pin the
  invariant on the new mechanism rather than massaging the old assertion. See the
  main-list entry for the three variants. 20260714-113411, -135642, -134119.

## Promoted (kept for history)

- 2026-07-13: `sweep-then-delete` + `would-it-fail-without-it` +
  `record-the-exact-rig` + `landing-checkout-not-yours` -> work skill (the
  last also in flow's landing and compound's commit steps);
  `would-it-fail-without-it` also -> review skill;
  `tatr-same-second-collision` -> sharpened "never chain tatr new" in the
  tatr/plan/spike skills; `one-cargo-test-filter` -> docs/development.md.
  Same date: skills now write task-folder records (SPIKE.md, RETRO.md,
  NOTES.md next to TASK.md) and the ledger moved to docs/LESSONS.md.
- 2026-07-11: `verify-first-plan-steps` -> plan skill; `fail-first-regression-ab`,
  `commit-before-sabotage`, `production-faithful-rigs` -> work skill;
  `delivery-guards-on-null-assertions` -> review skill; `landing-no-cd` -> flow
  skill; `spike-fix-record` -> spike skill; `tatr-same-second-collision` ->
  tatr skill gotchas; lessons-ledger step -> compound skill. (All in
  nix.dotfiles/home/modules/agents/skills.)
