# Lessons ledger

One or two lines per lesson: slug, count, one sentence, a retro id or two
(an id resolves to `tasks/<id>/RETRO.md`). /compound appends new lessons or
bumps counts; two lines is the cap - a new occurrence sharpens the sentence,
never appends a paragraph. At three occurrences a lesson moves to Pending
promotions for the user to fold into AGENTS.md, a skill, or the tool itself;
promoted lessons stay listed one-line with their date and target. When a
lesson is really a skill rule, mark the target (`-> work skill`) at any
count. Seeded 2026-07-11 from 104 retros; condensed 2026-07-13 and
2026-07-19 (the second pass also promoted everything then pending).

## Process lessons

- `keep-docs-in-sync-with-code` (x4, enforced in AGENTS.md): a code change is
  not done until every doc surface it invalidates (CHANGELOG, news, player +
  dev wiki, tutorial) is fixed in the SAME task; map:
  `web/src/wiki/dev/keeping-docs-in-sync.md`. A ticked docs step is not proof
  - check its surface list against the diff; the LAST task of a family sweeps
  the whole feature's player-facing surfaces. 20260716-115938, 20260718-004723.
- `inseparable-seeded-tasks-remerge` (x1, PROMOTED 2026-07-19 -> flow skill):
  when seeded tasks prove architecturally inseparable, surface the re-cut and
  merge them instead of building shims. 20260717-215742.
- `review-the-generated-artifact-too` (x1): after changing an authored or
  generated schema, READ the regenerated file with an author's eye - parity
  tests never check readability. 20260717-215742.
- `rename-id-sweep-in-file` (x1): after renaming a content id, grep the WHOLE
  file for the old id - lint validates spawn/prototype/filter refs but not AI
  orbit/patrol targets. 20260716-215513.
- `git-mv-leaves-empty-parent` (x1): `git mv` out of a dir leaves the emptied
  dir on disk and filesystem-walking tools trip on it; `rm -rf` the old dir
  after a relocation. 20260716-215513.
- `tatr-new-then-sprout-strands-the-task-file` (x2, PROMOTED 2026-07-19 ->
  tatr + flow skills): sprout first and run `tatr new` inside the worktree;
  carry-and-clean a stub unavoidably born in the main checkout.
  20260717-101414, 20260718-181305.
- `flow-land-scope-when-user-says-branch` (x1, PROMOTED 2026-07-19 -> flow
  skill): when the ask mentions a branch AND /flow, confirm land-to-master vs
  stop-at-branch at the START. 20260718-181305.
- `warnings-clean-before-land` (x2): run a warnings-SURFACED build and read
  the warnings before landing - error-only greps ride warnings into the
  squash. 20260716-215423, 20260717-003613.
- `merge-red-check-preexisting` (x2, PROMOTED 2026-07-19 -> flow skill): when
  merging the default branch surfaces a red test, `git show <default>:<file>`
  first to classify inherited vs caused; fix inherited reds as named merge
  integration. 20260716-215423, 20260717-162121.
- `edit-the-builder-not-the-generated-ron` (x1, PROMOTED 2026-07-19 -> repo
  AGENTS.md): base `.content.ron` are generated - edit the builder and
  regenerate, or parity goes red. 20260718-175502.
- `sweep-content-repo-wide-not-just-assets` (x2): relocating/renaming an
  asset sweeps EVERY content-shaped file repo-wide (examples/, include_str!,
  test data); an "X holds everywhere" audit sweeps base + webmods +
  assets/mods + Rust-coded scenarios, re-derived in review. 20260717-002105,
  20260717-201534.
- `audit-framed-task-delivers-the-audit` (x1): for "apply X where it makes
  sense", the deliverable is the bounding audit, not the two-line edit; read
  the existing lint/guard before hand-ruling the boundary. 20260717-201534.
- `truncated-sweep-is-not-a-sweep` (x3, PROMOTED 2026-07-19 -> work skill): a
  sweep feeding a checklist is never head-truncated; dump in full and count
  matches into the plan. 20260716-155816, 20260716-155839.
- `mid-flow-lesson-reaudits-the-queue` (x1, PROMOTED 2026-07-19 -> flow
  skill): a mid-flow lesson applies backward - re-audit the queued tasks and
  re-run the sweeps it invalidates. 20260716-155839.
- `shared-checkout-reads-race` (x1, PROMOTED 2026-07-19 -> repo AGENTS.md):
  parallel sessions own the shared working tree; read repo facts via
  `git show HEAD:<path>`. 20260716-155816.
- `shared-checkout-write-leak` (x2, PROMOTED 2026-07-19 -> repo AGENTS.md +
  flow skill): never leave the index staged-but-uncommitted across tool
  calls; a squash-land is ONE command (merge --squash && commit).
  20260708-165703, 20260718-122906.
- `grep-test-module-before-adding-a-helper` (x1): grep the target test module
  for the helper name first; flight.rs already had `velocity_of`.
  20260718-122906.
- `verbosity-invites-fabrication` (x1): a completeness push makes drafters
  invent; pair it with "verbosity comes from sources - name without
  describing if the source does". 20260716-114245.
- `check-adjacent-sections-for-overlap` (x1): before designing a new
  user-facing section/area, enumerate the adjacent ones and ask "duplicate or
  merge?" - a standalone /changelog/ was merged away one cycle later.
  20260716-111557.
- `exemplar-first-fanout` (x2): for many similar artifacts, hand-write one
  gold exemplar, then fan out drafters against it + a per-item spec, with an
  out-of-context reviewer over the whole set. 20260716-102954, 20260716-111557.
- `cross-boundary-attribution` (x1): per-part drafters attribute a neighbor's
  feature to their part; review asks "does vN claim anything of vN+1?".
  20260716-102954.
- `conserve-on-regroup` (x1): mechanically regrouping a list-shaped doc needs
  a conservation check - grep each source item's token into the new file and
  reconcile counts (93 = 94 - 1). 20260716-102950.
- `authored-durations-clamp-trio` (x2): every authored duration/magnitude/
  vector gets finite-check + runtime-cap + lint-range AT BIRTH; the pattern
  does not transfer across crates by itself. 20260717-163050, 20260717-215920.
- `pin-the-window-not-the-ingredients` (x1): a race pin must reproduce the
  failure WINDOW (no intermediate update inside it); write it against the
  broken code first. 20260717-163033.
- `cited-finding-reread-not-recalled` (x1): when citing a sibling task's
  technical fact, re-open the file and quote it - recall inverted a launch
  axis one cycle later. 20260717-151214.
- `rule-inputs-rederive-from-engine` (x1): a graded rule's meaning re-derives
  from the engine's decision constants, not the metric struct's fields.
  20260717-112656.
- `new-cadence-reaudits-readers` (x1): a value changing cadence (per-event ->
  per-frame) re-prices every reader of its containing STRUCTURE.
  20260717-112647.
- `parallel-gates-pin-all` (x1): tightening N handlers sharing a gate value
  pins all N - count gates in the diff, count pins. 20260717-112639.
- `prose-invariant-becomes-pin` (x1): a design invariant stated in prose
  becomes a computed assertion in the same sitting. 20260717-112630.
- `prose-from-diff-not-intent` (x2): write CHANGELOG/wiki/NOTES from the
  final diff, then re-read asking "does the prose claim anything the diff
  does not do?". 20260717-112622, 20260717-163058.
- `lint-arm-sweeps-own-fixtures` (x2): a new lint arm fires on the test
  module's own fixtures; grep for matching shapes and isolate each fixture to
  its arm before the first run. 20260717-163050, 20260717-163058.
- `chain-gates-must-fail-on-red` (x1): a gate must exit non-zero on red -
  `| grep "test result"` succeeds on FAILED lines too. 20260717-163058.
- `spike-fix-record-appends-on-land` (x1): the fix-record append belongs next
  to the TASK.md close in each landing, not backfilled at flow finish.
  20260717-163058.
- `diagnostic-first` (x11, PROMOTED 2026-07-19 -> flow/work bug playbook):
  trace the exact reported scenario with real numbers before theorizing a
  mechanism. 20260711-140241, 20260718-204640.
- `fail-first-regression-ab` (x12, PROMOTED 2026-07-11 -> work skill): prove
  a fix by failing its test against pre-fix behavior; record the numbers (CI
  history counts when master is already red on the assertion). 20260718-204640.
- `test-across-the-ratio-boundary` (x1): behavior turning on a physical ratio
  is tested on BOTH sides of ratio=1 - a weak-well-only test shipped a
  strong-well crash. 20260718-204640.
- `delivery-guards-on-null-assertions` (x6, PROMOTED 2026-07-11 -> review
  skill): "nothing happens" tests prove the stimulus fired IN the same test.
  20260710-231931.
- `verify-first-plan-steps` (x8, PROMOTED 2026-07-11 -> plan skill): plan
  steps stating a mechanism/formula/API cite the verifying file - including
  shipped CONTENT data (grep the .content.ron when data picks the mechanism).
  20260712-093044, 20260717-003613.
- `scripted-walks-skip-the-bridges` (x1): a hand-fired scenario walk proves
  the script; each consumed event needs one pin driving the production
  bridge. 20260713-150343.
- `collider-needs-a-rigidbody` (x1): an avian Collider without a RigidBody
  registers no contact pair, silently. 20260713-150343.
- `recheck-referenced-task-freshness` (x1): re-check a referenced task's
  STATUS and the code before writing prose around it. 20260715-224030.
- `collisionstart-is-per-collider-pair` (x1): avian CollisionStart fires per
  collider pair, not body pair; dedup on the body or a counter overshoots.
  20260714-090002.
- `landing-chain-and-stub-collision` (x1): land in one &&-chain, and commit
  tatr stubs on master before sprouting so the merge cannot collide.
  20260713-121605.
- `verify-generator-stability-before-commit-diff` (x2): before gating on
  "regenerate + diff --exit-code", prove the generator byte-stable (run
  twice); also assert ORDER directly. 20260715-110417, 20260715-142900.
- `validate-in-every-domain` (x3, PROMOTED 2026-07-19 -> work skill): a gate
  checks a value's meaning in EACH domain it crosses (fs path, URL segment,
  IDB key), with a pin per domain. 20260715-142900, 20260715-163508.
- `toml-keys-before-tables` (x1): top-level TOML keys must precede the first
  `[table]` or they fold into it silently. 20260715-110417.
- `verify-tool-via-subcommand-not-which` (x1): check `cargo <sub> --version`,
  not `which` - ~/.cargo/bin may be off PATH. 20260715-110417.
- `relocation-leaves-ignored-siblings` (x1, PROMOTED 2026-07-19 -> sprout
  skill): gitignored files exist only in the main checkout; clean them up
  after landing a stop-shipping-dir move. 20260714-154958.
- `verify-stale-brief-against-tree` (x2): task briefs go stale - read the
  live tree/scene before trusting the premise, and fix the rationale when
  wrong. 20260714-154958, 20260718-004834.
- `match-ci-feature-set-in-targeted-tests` (x2): run targeted tests with CI's
  feature set or feature-gated test code fails to compile and reads as a
  regression. 20260718-004834, 20260718-102022.
- `landing-no-cd` (x3, PROMOTED 2026-07-11 -> flow skill): squash-merge from
  the main checkout, own command, no cd, `pwd` first. 20260709-160753.
- `record-the-exact-rig` (x3, PROMOTED 2026-07-13 -> work skill): evidence
  notes record the rig (systems, command path, components) or they mislead.
  20260709-125640.
- `probe-surfaces-adjacent-issues` (x1): run de-risk probes for real; they
  pay beyond their stated question. 20260710-104421.
- `headless-shot-after-load` (x1): BCS_SHOT captures black pre-load; inject
  `Screenshot::primary_window` from the autopilot at a settled moment.
  20260710-104421.
- `registered-system-for-change-detection` (x2): `run_system_once` builds a
  fresh system per call (Changed/Added fire on everything, cursors reset);
  register once and reuse the SystemId. 20260713-082330.
- `run-system-once-always-changed` (x1): same trap on `Res::is_changed`; gate
  behavior needs an App-driven test across real frames. 20260712-093831.
- `observer-over-spawn-site` (x1): attach derived components via an
  `On<Add, Marker>` observer, not by hunting spawn sites. 20260712-203345.
- `gate-producer-and-its-consumers` (x1): a flag that skips PRODUCING an
  entity sweeps its CONSUMERS too - each must tolerate the skip (early
  return, not error spam). 20260525-133013.
- `messagereader-needs-resource-guard-in-tests` (x2): minimal-app rigs omit
  `Messages<T>`; gate on `resource_exists` or init the resource in BOTH
  writing and consuming plugins. 20260714-174126, 20260716-193949.
- `worktree-shares-main-target` (x1, CORRECTED; PROMOTED 2026-07-19 -> sprout
  skill): accept the cold build - never share CARGO_TARGET_DIR with the main
  checkout (artifacts clobber). 20260709-131502.
- `commit-before-sabotage` (x2, PROMOTED 2026-07-11 -> work skill): commit
  the fix before A/B sabotage; anchor splices on unique strings.
  20260710-231930.
- `production-faithful-rigs` (x8, PROMOTED 2026-07-11 -> work skill): rigs
  mirror production - scheduling, hierarchy, shipped configuration,
  required-component DEFAULTS; extract ONE shared registration helper both
  plugin and rigs call. 20260711-103527, 20260717-163042.
- `presence-vs-behavior-tests` (x2): component-exists assertions stay green
  while behavior regresses; assert the behavior. 20260709-160753.
- `sweep-then-delete` (x11, PROMOTED 2026-07-13 -> work skill): before
  deleting/moving/renaming anything, grep the whole repo for symbol names,
  describing words, bare filenames + markdown-link forms, and prose twins in
  comments/docs/CHANGELOG - across ALL file types. 20260711-212519,
  20260717-212219.
- `reread-after-insert` (x2): after inserting into a function/test, re-read
  the whole thing for duplicated bindings or broken invariants. 20260710-214316.
- `does-the-old-element-survive` (x2): when a design adds an element
  overlapping an existing one, decide the old one's fate explicitly.
  20260711-000547.
- `destructive-chains-check-completability` (x1): a multi-step destructive
  action must not START unless every step's preconditions hold; state the
  completability invariant in the plan. 20260715-142916.
- `removed-control-orphans-persisted-state` (x1): removing/hiding a control
  sweeps every writer/persister of its state and answers how that state gets
  corrected without it. 20260715-142844.
- `author-facing-schema-needs-syntax-doc` (x1): a hand-written serde field
  documents its literal syntax in the same change (strict RON Option:
  `Some("x.png")`). 20260715-142849.
- `declared-but-not-loaded` (x1): a resource named in config/markup is not
  wired; grep for where it is imported/served. 20260713-222025.
- `generated-links-need-real-targets` (x1): manifest-rendered links gate on
  the target existing or they 404. 20260713-225324.
- `ci-skips-client-render` (x1): build-only CI proves the bundle compiles;
  DOM logic needs a runtime check. 20260713-225324.
- `render-output-eyeball` (x4): a dimensionally-valid generated visual can
  still be wrong - open it; a layout task is unverified until someone SEES it
  rendered (headless cannot see z-order). A scope change re-opens deferrals
  built on the old premise. 20260715-004216, 20260718-122923.
- `roundtrip-hides-shared-bug` (x1): a round-trip test on a self-authored
  forward pass proves symmetry, not correctness; re-derive the reverse
  against the spec. 20260715-004216.
- `one-cargo-test-filter` (x5, PROMOTED 2026-07-13 -> docs/development.md):
  one filter and one `-p` per cargo test invocation. 20260713-082324,
  20260716-162701.
- `check-all-targets-for-struct-field` (x6, PROMOTED 2026-07-19 -> work
  skill): a new non-Default field breaks builders, tests AND examples that
  plain `cargo check` never compiles; grep the repo for the literal and run
  `cargo check --all-targets` before landing. 20260717-165031, 20260718-102022.
- `register-assets-for-new-test-path` (x2): a copied Bevy test omits what the
  NEW path needs (init_asset for loads, schemeless paths, Quat::abs_diff_eq).
  20260718-113307, 20260718-121205.
- `mod-facing-surface-plans-failure-paths` (x1): a mod-data surface plans
  "what breaks when a mod does this badly" as steps up front. 20260716-155849.
- `endpoint-only-color-reasoning` (x1): evaluate the intermediate frames of a
  color/wave transition, not just endpoints. 20260712-152340.
- `data-source-over-schedule-fight` (x2): when a fix needs impossible
  ordering, change where the data comes from. 20260710-231928.
- `if-feasible-must-be-answered` (x1): a plan's "if feasible" hedge gets an
  explicit answer. 20260709-160753.
- `discrete-not-continuous-filters` (x1): compensate a frame-stepped filter
  from its update equation, not its continuous limit. 20260711-121711.
- `dependency-fix-first-reruns-symptom` (x1): after a dependency fix, re-run
  the original symptom before interpreting old traces. 20260709-125640.
- `spike-fix-record` (positive, PROMOTED 2026-07-11 -> spike skill):
  multi-task spikes keep a living fix-record section. 20260711-103527.
- `tatr-same-second-collision` (x7, PROMOTED 2026-07-19 -> FIXED IN TOOL:
  tatr 0.2.0 fails on a same-second ID instead of overwriting; tatr skill
  updated): retry on the error; still one `tatr new` per command.
  20260712-143832, 20260525-133028.
- `serde-scope-grep` (x1): before estimating a derive migration, grep the
  whole type tree for non-derivable leaves; scope hides there. 20260525-133028.
- `generate-data-from-code` (x4, PROMOTED 2026-07-19 -> repo AGENTS.md):
  generated artifacts follow their builder both directions - builder changes
  regenerate in the same commit, hand-edits (even comments) go in the
  builder; parity is the contract. 20260525-133028, 20260718-201532.
- `effect-not-just-helper` (x1): test a spawn/mutation action's EFFECT
  through the ECS harness, not just its pure sub-helper. 20260525-133028.
- `check-examples-skips-tests` (x1): `cargo check --examples` skips
  `#[cfg(test)]`; verify type changes with `cargo test --no-run`.
  20260525-133028.
- `test-the-production-load-path` (x2): exercise asset load/merge the way
  production does (untyped kickoff, resource-read merge), not a convenient
  intermediate. 20260714-163342, 20260714-134127.
- `stemmed-compound-extension` (x2): custom-asset files need a stem so the
  full extension matches the registered loader (`<pack>.bundle.ron`, never a
  bare `bundle.ron`). 20260714-163342.
- `stage-lock-with-manifest` (x1): a Cargo.toml dep change stages Cargo.lock
  too; explicit-path adds drop it silently. 20260714-113408.
- `pin-the-fix-at-its-boundary` (x4, PROMOTED 2026-07-19 -> review skill):
  guard a fix at its OWN boundary with a unit test that fails under the bug;
  a refactored invariant re-pins on the new mechanism; grep a changed
  predicate's table-test callers first. 20260714-113411, 20260716-214919.
- `shared-id-space-shared-overlay` (x1): containers sharing an id space route
  through ONE overlay helper so overlay semantics cannot diverge.
  20260714-134119.
- `verify-the-nit-compiles` (x2): every review fix is a hypothesis -
  compile/typecheck it and verify the contract it assumes. 20260714-134119,
  20260714-210131.
- `agent-interrupted-verify-worktree` (x2): inspect an interrupted subagent's
  worktree (status + compile + deterministic generators) before concluding
  done-or-broken. 20260714-150508.
- `reconcile-plan-to-shipped` (x2): at close-out reconcile the plan's
  aspirational lists with what shipped BEFORE review. 20260525-133028,
  20260714-113411.
- `bg-session-authors-on-branch` (x1): background sessions author task/doc
  content inside the first sprouted worktree; only stubs touch the main
  checkout. 20260713-175415.
- `copied-pattern-weakest-target` (x1): an upstream rendering pattern carries
  device requirements; check downlevel flags against the weakest shipped
  platform. 20260713-175415.
- `additions-join-doc-indexes` (x1): a new artifact of an enumerated kind
  joins the doc list that enumerates its kind. 20260713-175352.
- `maskable-ci-conclusions` (x1): under continue-on-error, cite the job LOG's
  result line, never the step/run conclusion. 20260710-143138.
- `insert-cluster-must-be-removed-as-a-cluster` (x1): removes strip the whole
  insert cluster (requires do not cascade on removal). 20260712-201603.
- `event-driven-autopilot-beats` (x1): harness scripts stage each gesture on
  game state, never wall-clock windows; wall-clock is backstop only.
  20260712-211352.
- `checkpoint-before-building-on-an-audit` (x1): a planned user checkpoint on
  an audit ships the table with ZERO implementation behind it. 20260712-211352.
- `null-result-becomes-a-pin` (positive, x2): a "cannot reproduce" verdict
  converts its evidence rig into a permanent pin of the non-behavior.
  20260713-175352, 20260715-214540.
- `state-diff-aliases-reset` (x1): deriving events by diffing state makes a
  reset look like events; guard teardown/load/clear. 20260712-125342.
- `landing-checkout-not-yours` (x3, PROMOTED 2026-07-13 -> work skill; also
  repo AGENTS.md 2026-07-19): `git branch --show-current` before EVERY commit
  in the shared checkout. 20260525-133004.
- `pair-matrix-on-collider-class-change` (x1): a collider class change checks
  every collider category, not just the pair being fixed. 20260712-121101.
- `verify-scripted-edits-applied` (x4, PROMOTED 2026-07-19 -> ~/AGENTS.md):
  an edit is a hypothesis until the artifact shows it - re-verify every
  member of a retried batch and read the produced TEXT. 20260712-110730,
  20260717-151208.
- `reuse-production-helpers-in-tests` (x3, PROMOTED 2026-07-19 -> work
  skill): compose rigs and expected values via production helpers; grep the
  module for an existing rig first. 20260711-121839, 20260717-112622.
- `constant-offset-is-rig-math` (x1): an error invariant across interpolation
  alpha implicates the rig's math, not the timing under test. 20260711-121839.
- `ab-toggle-via-vcs-not-sed` (x1): toggle a fix off via stash/checkout, not
  sed. 20260711-121839.
- `confounded-knob-experiment` (x2): before concluding a knob A/B, grep every
  reader of the knob/binding (Space fired AND burned). 20260711-140234.
- `recompute-both-sides-of-a-band` (x1): changing one side of a band/guard
  recomputes the other with in-game magnitudes; a value whose MEANING changes
  re-asks every reader. tasks/20260709-193338/NOTES.md.
- `distinct-refusal-reasons` (positive, x1): every refusal path logs its own
  reason string. tasks/20260709-193338/NOTES.md.
- `quat-angle-noise-floor` (x1): f32 quat angle_between floors ~1e-3 rad;
  assert above it or compare components. 20260711-140241.
- `audit-state-gates-on-new-entry-path` (x3, PROMOTED 2026-07-19 -> plan
  skill): a new route into a state greps run_if/in_state + OnEnter/OnExit and
  writes the what-newly-runs list. 20260711-180426, 20260716-214919.
- `bound-scheduling-both-sides` (x1): a system between producer and reader
  needs both .after and .before. 20260711-180501.
- `set-gates-miss-observers` (x1): gating a SystemSet does not touch
  observers; enumerate systems + observers + hooks. 20260711-185156.
- `would-it-fail-without-it` (x6, PROMOTED 2026-07-13 -> work + review
  skills): a verification that cannot fail with the mechanism deleted proves
  nothing; a sabotage that will not go red refutes the assumed mechanism or
  the test's shape. 20260711-180426, 20260717-163033.
- `out-of-context-review-pass` (positive, x30): a fresh-context review
  re-derives load-bearing claims (spawn sites, algorithms, engine semantics,
  course geometry) and catches MAJORs shared-session eyes miss; verify the
  verifier's counterexamples too. 20260712-133343, 20260717-212219.
- `required-component-in-shared-query` (x2): a required fetch narrows an
  existing query's membership; fetch `Option<&T>` or use a separate query.
  20260712-143832.
- `spike-open-question-pays-off` (positive, x1): a spike naming a risky
  unknown lets the implementer resolve it before wiring. 20260712-143832.
- `authored-vs-derived-values` (x4): author content against measured runtime
  consts, and encode layout invariants as computed rig assertions - carried-
  over positions need the same derivation as new ones. 20260716-124722,
  20260717-112630.
- `verify-engine-guarantees-in-source` (x8, PROMOTED 2026-07-19 -> plan
  skill): read the dependency's source or probe before designing around its
  ordering/observer/failure behavior - doc comments (upstream AND ours) are
  folklore; existence greps include dependency checkouts; grep for a
  compensating system before theorizing a missing write. 20260717-013440,
  20260717-133332.
- `advertised-but-unwired` (x3): a config surface is not a capability until
  producer/consumer wiring and preconditions are verified in the new context.
  20260712-093044.
- `cross-cycle-warning-with-numbers` (positive, x2): write hazards and
  findings belonging to a QUEUED task into that task's TASK.md with
  specifics. 20260711-140234, 20260716-155823.
- `verify-at-deploy-base-path` (x2): origin/base-path behavior verifies
  against the real deploy topology, not a local split-port setup.
  20260715-214540.
- `dev-doc-steers-across-boundary` (x1): dev-setup docs crossing a browser/
  security boundary name it and default to the safe path. 20260715-214540.
- `nix-devshell-for-cargo` (x2): no cargo on PATH means prefix with
  `nix develop --command ...` from the repo. 20260715-140049.
- `reuse-known-good-stack` (x2, positive): scaffold new work by copying a
  working in-repo reference verbatim. 20260712-093048, 20260711-180511.
- `measure-before-writing-the-number` (x2): never write a quantity into a doc
  from a mental model; backfill from a run. 20260712-105505, 20260717-143806.
- `manual-time-rig-measures-its-clock` (x2): `Time<Virtual>` clamps manual
  steps to max_delta (0.25s); raise it or count effective ticks - first
  hypothesis when a ManualDuration rig under-advances. 20260716-183220.
- `ab-isolation-bench` (x1, positive): attribute one system's cost with two
  worlds identical except for that system. 20260712-105505.
- `sweep-full-scale-before-believing-a-win` (x1): benchmark across the whole
  scale range and both regimes; an index won at 500 and lost at 5000.
  20260525-133014.
- `lint-gate-is-the-last-step` (x2): fmt/clippy/tests run AFTER the final
  edit; mirror remote CI locally before pushing. 20260525-133014,
  20260715-142931.
- `document-the-async-failure-path` (x1): concurrent-flow notes trace the
  async failure path and state the real atomicity boundary. 20260715-142931.
- `sibling-change-leaves-stale-fixture` (x3, PROMOTED 2026-07-19 -> work
  skill): grep for fixture tests asserting on data you change; pin durable
  intents, not frozen literals. 20260715-142931, 20260717-151214.
- `benchmark-gates-both-ways` (x1, positive): a measure-first gate justifies
  deferring as legitimately as doing. 20260525-133014.
- `verify-bevy-api-at-callsite` (x1): copy an existing in-repo callsite for
  unfamiliar Bevy API; 0.x churns. 20260712-131348.
- `anchor-edits-in-the-right-scope` (x2): text anchors can land in the wrong
  scope or steal the attribute block above a fn; anchor on block starts and
  confirm the module boundary. 20260525-133017, 20260716-193949.
- `spike-reuse-over-new-infra` (x1, positive): check whether an existing
  substrate covers the need before building infrastructure. 20260712-131348.
- `trace-vehicle-timeline-first` (x1): pick an evidence rig by its script
  TIMELINE, not scene content. 20260711-183417.
- `pkill-pattern-matches-own-shell` (x2, PROMOTED 2026-07-19 -> ~/AGENTS.md):
  `pkill -f` matches your own command line and look-alike processes; kill
  recorded PIDs. 20260716-180352, 20260717-004302.
- `silent-tool-missing-in-pipeline` (x1): a missing launcher dies with 127
  that a pipeline swallows; `which` host tools first. 20260711-183417.
- `no-source-edits-during-inflight-builds` (x1): a tree edited mid-build
  yields an indeterminate evidence binary; quiesce for A/B runs. 20260711-183417.
- `gpu-example-local-skip` (x1): heavy render examples are ~100x too slow
  under lavapipe; one short smoke attempt, then headless tests + CI.
  20260717-004302.
- `borrowed-rig-coverage-check` (x1): a borrowed rig inherits its record's
  overclaims; verify coverage against the NEW failure mode. 20260712-115902.
- `refutation-invalidates-earlier-prose` (x1): when a probe overturns the
  theory, re-read every artifact written under the old one. 20260712-115902.
- `run-example-via-cargo-run-for-assets` (x2): run examples via
  `cargo run --example` from the crate root (asset paths) and keep stderr
  (`2>&1`). 20260714-204219, 20260714-214111.
- `despawn-by-owner-not-all-on-cross` (x1): tag shared singletons with their
  owner and despawn only the match; enter/leave can interleave. 20260714-204219.
- `autopilot-is-frame-starved-under-load` (x1): frame-counted waits + a
  wall-clock lifetime stall under load; run timing autopilots before heavy
  builds. 20260714-214111.
- `ui-footprint-vs-3d-picking` (x1): a UI panel over a 3D projection blocks
  its raycast; size against the real window resolution. 20260714-204219.
- `rig-supplies-precondition-hides-regression` (x2): a rig that injects the
  state production establishes is blind to that setup breaking; pin the
  production setup separately. 20260715-224803, 20260715-224812.
- `bg-isolation-guard-allows-sprout-not-main` (x3, PROMOTED 2026-07-19 ->
  repo AGENTS.md): the bg Write/Edit guard blocks the main checkout, not a
  sprout worktree; master-side artifacts via Bash heredoc. 20260715-224803,
  20260718-181305.
- `isolate-off-head-for-unpushed-deps` (x1, PROMOTED 2026-07-19 -> sprout
  skill): work depending on unpushed local commits isolates off local HEAD
  (sprout), not an origin-based worktree. 20260715-140049.
- `count-gate-use-gt-not-eq` (x1): a milestone gate on a multi-fire counter
  uses `>= N`, never `== N`. 20260715-224812.
- `gate-on-what-you-assert` (x1): every condition an assert relies on joins
  the stage gate when it can lag by frames. 20260708-203659.
- `rig-before-fix-on-unreproducible` (positive, x1): for a happened-once
  report, enumerate candidate mechanisms as a boundary rig BEFORE any fix -
  the red subset is the diagnosis. 20260716-162701.
- `probe-the-adversarial-variant` (x1): pick probes by what they can HIDE; a
  Defeat-only probe masked a Victory-only bug. 20260716-125856.
- `pick-the-system-set-seam` (x1): choose the gated SystemSet for a new
  system explicitly; debug convenience is not automatically debug-gated.
  20260716-114125.
- `re-audit-consumers-on-input-model-change` (x1): discrete -> continuous
  control invalidates every policy written for the discrete model (per-drag
  writes need debouncing). 20260711-180511.
- `parity-test-must-cross-link` (x1): a sync test derives the expected value
  from one side and asserts on the OTHER, never two hardcoded literals.
  20260711-180511.
- `ask-user-facing-control-style` (x1): the interaction style of a user
  control is a genuine preference fork - ask instead of deliberating at
  length. 20260711-180511.
- `mirror-sibling-resolve-site` (x1): a new resource-resolving content field
  mirrors the sibling's resolve SITE, not just its declaration - the site
  decides which systems gain the dependency. 20260717-002228.
- `piped-cargo-masks-exit-code` (x6, PROMOTED 2026-07-19 -> ~/AGENTS.md +
  work skill): never end cargo with tail/grep/echo - the harness reads the
  last exit; write output to a file and grep the FILE. 20260717-002228,
  20260718-122932.
- `half-ticked-compound-steps` (x3, PROMOTED 2026-07-19 -> work skill): tick
  a step only when every clause is done, or split/amend it in the same edit.
  20260708-165704, 20260718-122912.

## Domain lessons (nova-protocol specific)

- `gate-scenario-handlers-to-their-acts` (x1): every handler fires in every
  act unless filtered; gate by default, especially terminal states.
  20260708-203659.
- `crate-solo-tests-miss-unified-features` (x6, PROMOTED 2026-07-19 -> repo
  AGENTS.md; dev wiki via 20260718-152214): `cargo test -p nova_scenario`
  alone does not compile - use `--features serde`, a unifying sibling, or
  workspace-wide; grep this ledger for the crate before crate-scoped runs.
  20260716-125856, 20260718-122906.
- `deleted-content-tests-carry-engine-coverage` (x1): data tests can be the
  only exercise of an engine mechanism; re-pin at the owning crate before
  deleting them. 20260716-155830.
- `two-clocks` (family): FixedUpdate reads raw Position/Rotation; render-rate
  reads eased Transform; one computation, one clock, one frame. Full rule:
  tasks/20260711-103527/SPIKE.md.
- `global-transform-stale-in-fixedupdate` (family): GlobalTransform in
  FixedUpdate is last frame's propagation; avian child-collider poses one
  tick stale. See the two-clocks spike.
- `degenerate-inertia-frames` (x1): avian's eigen sort gives symmetric ships
  a cyclic-permutation local frame; test with non-identity frames.
  20260709-125640.
- `assert-each-gesture-step` (x2): modal/chorded input tests assert state
  after every step, not event counts at the end. 20260711-173237,
  20260718-122912.
- `modal-input-observer-dispatch` (x2): model a held modifier as a plain
  action read in observers (component-presence gate), not a binding Chord.
  20260711-173237, 20260718-122912.
- `bei-app-finish-in-tests` (x2): bevy_enhanced_input needs `app.finish()` +
  `app.cleanup()` before spawning an action rig. 20260708-165705.
- `bevy-input-is-messages-in-tests` (x1): drive input tests with
  `World::write_message`; MouseWheel needs unit+x+y+window+PHASE.
  20260718-122912.
- `changed-shared-observer-run-the-module-suites` (x4, PROMOTED 2026-07-19 ->
  work skill): a change to a shared observer/system runs the whole affected
  module suite - existing tests catch the silently broken consumers.
  20260718-122912, 20260718-151102.
- `identity-default-makes-no-regression-structural` (x1): give a new
  parameter a default reproducing the old behavior exactly, so no-regression
  is algebraic (`v - 0 == v`). 20260718-151102.
- `playtest-can-reverse-a-spike-feel-call` (x1, PROMOTED 2026-07-19 -> spike
  skill): a feels-better decision is a hypothesis; keep the deciding
  parameter one tunable. 20260718-185826.
- `new-default-on-capability-changes-tested-behavior` (x1): a default-granted
  capability changes every existing entity when a code path starts honoring
  it; legacy tests opt out or the capability opts in. 20260718-122932.
- `shared-primitive-clear-on-handoff` (x1): a side-effecting component any
  system acts on is CLEARED by each driver when it stops driving; test the
  off-ramp. 20260718-122932.
- `verify-ci-triggers-before-claiming-coverage` (x2): read workflow triggers
  before writing "CI builds this"; run the real build (trunk) when it is the
  sole gate. 20260714-174131, 20260714-233438.
- `lint-covers-types-not-variants` (x1): checks over a config tree enumerate
  every PATH to the checked type, not remembered enum variants. 20260716-191543.
- `content-identifiers-sweep-by-script` (x1): cross-file content ids resolve
  at spawn and pass every gate; sweep by script against the catalogs before
  review. 20260716-123535.
- `spike-list-needs-code-check` (x1): a spike's enumerated mechanism list is
  unverified prose until the implementing cycle greps each item. 20260714-233438.
- `capability-detect-by-acquiring` (x1): gate on ACQUIRING the resource
  (requestAdapter), not the API namespace existing. 20260714-233443.
- `trunk-inline-script-before-deferred-module` (positive, x1): an inlined
  plain script runs before trunk's deferred wasm bootstrap - the place for a
  pre-init gate; confirm in built dist/index.html. 20260714-233443.
- `target-scoped-feature-flips-wasm-backend` (positive, x1): switch only the
  wasm render backend via a target-specific bevy feature; confirm with
  `cargo tree --target wasm32...`. 20260714-233438.
- `portal-mod-ids-dash-only` (x1): the portal id gate accepts lowercase/
  digits/'-' for MOD ids while scenario ids use underscores; name fixtures to
  the validating gate. 20260716-155839.
- `mod-dependency-overrides-are-load-bearing` (x1): a dep can silently
  OVERRIDE a base section by id (balance, not just availability); grep the
  dep's content for the ids you name. 20260716-124722.
- `verify-current-convention-not-task-premise` (x1): a task naming a concrete
  mechanism is a snapshot; grep the live config before following it (rev ->
  tag pin). 20260716-165617.
- `grid-flex-item-needs-min-width-0` (x1): a flex/grid item refuses to shrink
  below its widest child without `min-width: 0`; suspect the item before the
  child's wrapping on sideways scroll. 20260718-114128.
- `isolate-the-lever-before-measuring` (x1): a preset bundles levers; add an
  override to vary ONE knob in isolation before attributing a win.
  20260718-004723.
- `screenshot-disambiguates-a-perf-win` (x1): a frame-time drop is ambiguous
  between fewer pixels and a broken frame; capture the frame. 20260718-004723.
- `quiet-host-before-measuring` (x1): perf numbers on a contended shared box
  are worthless; check load and serialize against parallel jobs.
  20260718-004723.
- `read-harness-contract-before-wiring` (x1): read a harness plugin's
  lifecycle contract (forced states, mutual exclusion) before composing it
  into an example. 20260718-004723.
- `shell-bg-vs-and-chain` (x1): `A && B & C` backgrounds `A && B`; put
  backgrounded processes on their own statement, keep kills out of launching
  commands. 20260718-004723.
- `measure-first-can-falsify-the-premise` (x1): the honest gate can say the
  lever barely helps; report it straight and surface the fork. 20260718-004723.
- `verify-interaction-not-just-rendering` (x1): a screenshot proves the frame
  drew, not that UI is clickable (bevy_ui on an image camera is unclickable);
  verify a CLICK or flag a human re-test. 20260718-132638.
- `verify-runtime-transitions-not-just-fresh-state` (x2): test A->B and B->A
  while running, not just each fresh boot state - both render-scale bugs
  lived only in the switch. 20260718-132638, 20260718-140903.
- `bevy-camera-ignores-runtime-rendertarget-swap` (domain, x1): bevy 0.19
  re-derives camera target_info only on content change / is_added /
  projection change - swapping RenderTarget in place leaves sizes stale;
  `projection.set_changed()` forces the re-derive. 20260718-140903.

## Pending promotions (3+ occurrences, user decides)

(Empty. New 3+ lessons land here as they occur; once folded into AGENTS.md,
a skill, or the tool itself, they get a PROMOTED date + target marker on
their entry above.)
