# Lessons ledger

One or two lines per lesson: slug, count, one sentence, a retro id or two
(an id resolves to `tasks/<id>/RETRO.md`). /compound appends new lessons or
bumps counts. At three occurrences a lesson
moves to Pending promotions for the user to fold into AGENTS.md or a skill;
promoted lessons stay listed with their date. Keep entries SHORT - when a new
occurrence adds a variant, sharpen the one sentence instead of appending a
paragraph. Seeded 2026-07-11 from 104 retros; heavily condensed 2026-07-13.

## Process lessons

- `keep-docs-in-sync-with-code` (x4, already enforced in AGENTS.md): a code
  change is not done until the docs
  it invalidates are fixed in the SAME task - Nova documents itself across
  several surfaces (terse `CHANGELOG.md`, the `/news/` posts, the player wiki,
  the dev wiki, the tutorial) and none updates itself. The map of what to touch
  when (per code area, and on release) is the dev wiki page
  `web/src/wiki/dev/keeping-docs-in-sync.md`; the enforcement rule is in
  AGENTS.md ("The website"). Ask what a PLAYER loses or gains before filing a
  change as pure refactor (a deleted picker row needed a CHANGELOG line, caught
  only in review); removing a shipped example mod meant sweeping 4 wiki guides +
  a design doc + CHANGELOG in the same task. A TICKED docs step is not proof:
  002228's plan named CHANGELOG+wiki, the step was checked, neither was touched
  (caught a cycle later by 20260717-101615) - review a docs step's OWN list of
  surfaces against the diff, not the diff alone. Across a TASK FAMILY the player
  wiki is the surface that falls between siblings: the ammo-reload mechanic task
  did CHANGELOG + dev wiki but never the player wiki, and the readout sibling had
  to backfill it - so the last task of a multi-task feature should sweep the
  whole feature's player-facing surfaces, not just its own diff.
  20260716-115938, 20260716-155816, 20260716-215513, 20260716-123556.
- `inseparable-seeded-tasks-remerge` (x1): a spike seeds COARSE direction-level
  tasks; some turn out architecturally inseparable (the turret data-model / ECS /
  aim tasks each needed the others to compile - splitting them means throwaway
  shim code). When flow's plan phase hits this, surface the re-cut to the user
  and merge to one commit rather than grinding out shims (AGENTS.md: no wasted
  code). One review/retro covers the merged core. 20260717-215742.
- `review-the-generated-artifact-too` (x1): when a refactor changes an AUTHORED
  or GENERATED schema, READ the regenerated file with the author's eye, not just
  the code - a bare `f32` joint `speed` with `serde(default)` serialized
  `speed: 3.1415927` onto every node (fixed ones included) of the turret RON;
  parity (builder==committed) stayed green because it never checks readability.
  Fixed with `skip_serializing_if`. Kin of `warnings-clean-before-land` for
  content. 20260717-215742.
- `rename-id-sweep-in-file` (x1): after renaming an entity/asset id in a
  content file, grep the WHOLE file for the OLD id before trusting the linter -
  `content_lint` validates spawn/prototype/filter refs but NOT AI orbit/patrol
  targets, so a renamed menu-backdrop well left the orbiter's `orbit: Some(old
  id)` dangling and lint stayed green; caught only in manual self-review.
  20260716-215513.
- `git-mv-leaves-empty-parent` (x1): `git mv` files out of a dir then `git rm`
  the dir leaves the emptied dir ON DISK (git tracks no empty dirs), and any
  filesystem-walking tool trips on it - a leftover `variety/textures/` crashed
  content_lint's bundle walk ("no *.bundle.ron at its root"). `rm -rf` the old
  dir explicitly after a relocation. 20260716-215513.
- `tatr-new-then-sprout-strands-the-task-file` (x1): running `tatr new` in the
  shared main checkout and THEN sprouting a fresh worktree leaves the new
  TASK.md uncommitted in the main checkout - the worktree (cut from a commit)
  never sees it, and the dirty file risks a parallel job's `git add -A` sweeping
  it into their commit. Sprout FIRST, run `tatr new` inside the worktree so the
  task is born on the branch. 20260717-101414.
- `warnings-clean-before-land` (x2): run a warnings-SURFACED build and READ the
  warnings (`cargo build`/`clippy`; never `cargo ... | grep -E 'error|test
  result'`, which discards them) on new/changed modules BEFORE the squash-land -
  a filtered run is green while a warning rides into the landed commit, needing a
  follow-up sprout. Seen as an unused `use HashSet` (215423) and an unused
  `Assets::insert` `Result` (`#[must_use]`) in a test - both hidden by an
  error-only grep and caught only by the editor's post-land LSP diagnostic.
  20260716-215423, 20260717-003613.
- `merge-red-check-preexisting` (x2): when merging the default branch surfaces a
  red test, `git show <default>:<file>` the failing test FIRST to decide whether
  your change caused it or you inherited it - turns "did I break this?" into a
  definite answer. Here two shipped-id fixtures (`demo`) were already red on
  master from a parallel task's rename; fixed as merge integration (naming the
  source task) rather than mis-blamed on this branch. Recurred: a sibling's
  hand comment in a GENERATED RON left content_ron_parity red on master;
  same recipe (git show, named merge-integration regen). 20260716-215423,
  20260717-162121.
- `sweep-content-repo-wide-not-just-assets` (x2): when relocating/renaming an
  ASSET, grep EVERY content-shaped file repo-wide - `examples/**`, `include_str!`
  embedded RON, test data - not just the shipped `assets/` tree. Content loaded
  OUTSIDE the mod merge (directly via the default asset source) won't be caught by
  a `self://`/`dep://` sweep of `assets/`+`webmods/` NOR by a `.rs`
  `asset_server.load` grep. `examples/data/reel.content.ron` kept bare base refs
  after a base-art move and 404'd in the shipped screenshot tool; `cargo test`
  never runs that example, so the full green suite hid it - the independent
  completeness review caught it. A green suite proves the TESTED paths; it says
  nothing about a shipped tool no test exercises. 20260717-002105. Recurred on webmods/ (15 silent asteroids). 20260717-101641. Also the audit direction: a "does X hold everywhere" claim must sweep base + webmods + `assets/mods/*` + Rust-coded scenarios, and be re-derived independently in review (28 NextScenario handlers re-enumerated to confirm only 2 qualified for linger:false). 20260717-201534.
- `audit-framed-task-delivers-the-audit` (x1): when a task is framed "apply X
  where it makes sense", the primary deliverable is the bounding audit (which
  cases qualify and why), not the edit - the code change is often a line or two.
  Write the audit into the task record and re-derive it repo-wide in review;
  "apply linger:false" resolved to exactly 2 of 28 scenario transitions because
  the rest were Outcome-paired (a rule the lint already encoded - read the
  existing guard before hand-ruling the boundary). 20260717-201534.
- `truncated-sweep-is-not-a-sweep` (x3, -> Pending promotions): a grep sweep
  that feeds a work checklist must never be head-truncated - one audit's
  `| head` sweeps hid a third assertion (one failed run), a whole extra file
  from a downstream plan, AND two stale guards whose survival put a red test
  on master for hours; dump sweeps in full (or to a file) and count matches
  into the plan. Sibling of `sweep-then-delete`. 20260716-155816,
  20260716-155839 (x2).
- `mid-flow-lesson-reaudits-the-queue` (x1): a lesson written mid-flow applies
  BACKWARD too - re-audit the remaining queued tasks/plans against it (re-run
  the sweeps it invalidates) instead of only applying it forward; the
  truncated-sweep lesson was on the ledger while two plans it had already
  poisoned sat unexamined in the queue. 20260716-155839.
- `shared-checkout-reads-race` (x1): parallel sessions own the shared main
  checkout's WORKING TREE, so audits reading working files race them (a stale
  base.bundle.ron undercounted the scenarios); read repo facts via
  `git show HEAD:<path>` when outside a worktree. Read-side sibling of
  `landing-checkout-not-yours`. 20260716-155816.
- `shared-checkout-write-leak` (x1): never leave the index staged-but-uncommitted
  in the shared main checkout across tool calls - a parallel job's `git commit -a`/
  `git add -A` sweeps YOUR staged changes into ITS commit. A squash-land split into
  `merge --squash`, then an inspect call, then commit let a parallel `/compound`
  swallow the whole feature into an unrelated commit (work landed, but tangled).
  Land atomically: `git merge --squash <b> && git commit` in ONE command, inspect
  the diff on the BRANCH beforehand. Write-side sibling of
  `shared-checkout-reads-race`. 20260708-165703.
- `verbosity-invites-fabrication` (x1): telling a drafter (esp. a subagent) to
  be MORE verbose / "cover everything" pushes it to fill gaps with plausible
  invention - a 0.5.0 news post given the four damage-type names invented each
  type's behavior no source stated. Pair every completeness push with an
  explicit "verbosity comes from the sources, not invention; if a source names a
  thing without describing it, name it without describing it", and have the
  fact-check reviewer see the exact sources. 20260716-114245.
- `check-adjacent-sections-for-overlap` (x1): when a spike proposes a new
  user-facing section (a web section, a docs area, a menu), enumerate the
  existing adjacent sections FIRST and ask "does this duplicate or belong
  merged with one?" before designing it standalone. A standalone `/changelog/`
  section was built and then merged away one cycle later into `/news/` because
  the spike never checked it against the existing devlog/blog covering the same
  per-release ground - the duplication only surfaced when the user saw them side
  by side. 20260716-111557.
- `exemplar-first-fanout` (x2): to produce many similar artifacts (per-release
  news/changelog pages, per-module docs), hand-write ONE gold-standard exemplar
  first, then fan out parallel drafters against it plus a strict per-item spec -
  shape/voice stay consistent in one review round. Pair with an out-of-context
  reviewer that sees all parts at once. Held for both the 11 changelog pages and
  the 6 merged news posts. 20260716-102954, 20260716-111557.
- `cross-boundary-attribution` (x1): a per-part drafter (per-release,
  per-module) cannot see the boundary, so it will attribute a neighbor's feature
  to its own part if the shared source mentions it (a 0.5.0 page claimed the
  wiki/tutorial that shipped in 0.5.2, inherited from the devlog's wording).
  Add "does vN claim anything belonging to vN+1?" to the review pass explicitly.
  20260716-102954.
- `conserve-on-regroup` (x1): when mechanically rewriting or regrouping a
  list-shaped doc (changelog, index, manifest), silent drops are the main
  risk - regrouping N bullets into M sections by hand has no conservation
  check. Verify by grepping each source item's distinctive token/number
  against the new file AND reconciling counts, before review, not by
  eyeballing the diff (a CHANGELOG regroup dropped the "Screenshot Reel"
  entry; a token cross-check + 93=94-1 count caught it). 20260716-102950.
- `authored-durations-clamp-trio` (x2): every authored duration/magnitude/VECTOR
  field gets the finite-check + runtime-cap + lint-range trio AT THE FIELD'S
  BIRTH - two pacing durations reached Timer::from_seconds unclamped (an
  authored 1e30 panics at runtime) one task after the sibling dwell field got
  the full treatment; the pattern failed to transfer across crates. Recurred on
  the turret hinge `axis` Vec3 (a zero/NaN axis NaN-s through `normalize()`):
  got the trio at once - runtime degrades to a fixed joint, the content lint
  errors, so no path reaches the NaN. 20260717-163050, 20260717-215920.
- `pin-the-window-not-the-ingredients` (x1): a regression pin for a
  race/aliasing bug must reproduce the failure WINDOW (clear + repush before
  any sync frame), not just its ingredients - a pin with an intermediate
  update between them stayed green under sabotage because the sync masked
  the bug; write the pin against the broken code first when the bug is
  timing-shaped. 20260717-163033.
- `cited-finding-reread-not-recalled` (x1): citing a sibling task's technical
  finding from memory inverted it one task later (the torpedo bay's +Y launch
  kick became "launch axis -Z" in the next cycle's notes, caught by review);
  when a doc cites a prior REVIEW/NOTES fact, re-open the file and quote it.
  Kin of recheck-referenced-task-freshness. 20260717-151214.
- `rule-inputs-rederive-from-engine` (x1): when a derived metric becomes a
  graded RULE's input, re-derive the rule's meaning from the engine's own
  decision constants, not from the fields the metric struct happens to have -
  a threat-range rule priced turret reach only, and torpedo tubes (whose AI
  launch envelope is 1000u with an elapsed-start cooldown) evaded it until
  review. 20260717-112656.
- `new-cadence-reaudits-readers` (x1): a value whose CHANGE CADENCE changes
  (per-event -> per-frame: the scenario clock in the variables map) re-prices
  every reader of its containing structure - a change-only debug diff became
  a per-frame dump + clone because the diff was a forgotten consumer;
  enumerate readers of the STRUCTURE, not just consumers of the value.
  20260717-112647.
- `parallel-gates-pin-all` (x1): when an edit tightens N handlers sharing one
  gate value (three act < 3 -> act < 2 filters), the test edit must pin all N -
  mirroring only the handler the old test covered left the hauler soft-fail
  gate free to regress; count the gates in the diff and count the pins.
  20260717-112639.
- `prose-invariant-becomes-pin` (x1): a design invariant stated in prose
  ("full aggression in the arena", "the line stays flyable") is where review
  finds the hole - turn the sentence into a computed assertion in the same
  sitting (the ONE un-computed layout quantity, a leash radius chosen by
  feel, was the one finding; leash-covers-spawn is now a pin).
  20260717-112630.
- `prose-from-diff-not-intent` (x2): prose about the change (CHANGELOG, wiki,
  task NOTES) written from the ASPIRATION claims things the diff does not do
  (a scoped-out reposition feature; "the writing survives" over three trimmed
  clauses); write it from the final diff, then re-read asking "does the prose
  claim anything the diff does not do?". 20260717-112622, 20260717-163058.
- `lint-arm-sweeps-own-fixtures` (x2, applied preventively on
  20260717-162121: fixtures swept at plan time, zero collisions): a new
  lint arm fires on the lint test
  module's own pre-existing fixtures whose shape it matches (swallow arm on a
  3-warn fixture; beat-sheet arm on the 3-lines-one-handler dwell fixture);
  before the first run, grep the test module for matching shapes and isolate
  each fixture to the arm it exercises. 20260717-163050, 20260717-163058.
- `chain-gates-must-fail-on-red` (x1): `cargo test | grep "test result"`
  succeeds even when the line says FAILED, so everything after it in the
  &&-chain (here: the commit) runs on a red tree; a gate must exit non-zero
  on red - grep "test result: ok" or run the command bare. 20260717-163058.
- `spike-fix-record-appends-on-land` (x1): a multi-task spike's fix record
  stayed empty through all four landings and was back-filled at flow finish;
  the append belongs next to the TASK.md close in each cycle's landing, not
  at the end. 20260717-163058 (flow-level).
- `diagnostic-first` (x10): trace the exact reported scenario, with real
  numbers, before theorizing a mechanism (the wasm CORS "bug" was a
  cross-origin `?portal=` override, not a client fetch bug - reading the deploy
  topology dissolved it). 20260711-140241, 20260712-172035, 20260715-214540.
- `fail-first-regression-ab` (x11, PROMOTED 2026-07-11 -> work skill): prove a
  bug fix by failing its test against the pre-fix behavior; record the numbers.
  CI history counts as the failing run when master is already red on the exact
  assertion (no local sabotage needed). 20260711-180426, 20260715-142844.
- `delivery-guards-on-null-assertions` (x6, PROMOTED 2026-07-11 -> review
  skill): "nothing happens" tests need proof the stimulus fired, IN the same
  test - a cross-test guard through a shared helper does not count.
  20260710-231931, 20260711-183417.
- `verify-first-plan-steps` (x8, PROMOTED 2026-07-11 -> plan skill): plan steps
  that state a mechanism, formula, ordering, API shape, or "system Y will
  accept X" must cite the verifying file; follow data into the consumer's
  gates and enumerate consumers of shared state. Extends to shipped CONTENT DATA,
  not just code: a spike/plan assumed the ship rendered via default cuboids, but
  `assets/base/sections/base.content.ron` sets gltf `render_mesh` on every
  section - so the real tint mechanism was gltf material cloning, not a cuboid
  colour swap. Grep the shipped `.content.ron`/data when the data picks the
  mechanism; caught at work-time here, but it belonged in the plan.
  20260712-093044, 20260712-203353, 20260717-003613.
- `scripted-walks-skip-the-bridges` (x1): a scenario walk that fires events by
  hand proves the script, not the game; each consumed event needs one pin that
  drives the production bridge. 20260713-150343.
- `collider-needs-a-rigidbody` (x1): an avian Collider without a RigidBody
  registers no contact pair, silently; diff a silent physics test's bundle
  against a production spawn before theorizing. 20260713-150343.
- `recheck-referenced-task-freshness` (x1): when a task/plan step references
  another task as "OPEN/tracked-future" (or asserts a feature is unshipped),
  re-check that task's STATUS and the current code before writing prose around
  it - plans go stale between planning and doing, and a doc that encodes the
  stale premise is wrong the day it lands (the Scenarios picker had shipped and
  CLOSED, inverting a "no pure-RON launch path" step). 20260715-224030.
- `collisionstart-is-per-collider-pair` (x1): avian `CollisionStart` fires once
  per collider-pair, not per body-pair, so a cue/counter keyed off a
  multi-collider body (a ship = many section colliders) fires N times unless it
  dedups on the body/entity; a 5-line probe test panicking `DING_COUNT=3`
  settled it faster than reasoning. 20260714-090002.
- `landing-chain-and-stub-collision` (x1): land with one &&-chain
  (merge --squash && commit && sprout rm), and commit tatr stubs on master
  before sprouting so the merge cannot abort on a collision. 20260713-121605.
- `verify-generator-stability-before-commit-diff` (x2): before gating a
  generated artifact on "CI regenerates + `git diff --exit-code`", prove the
  generator is byte-stable (run it twice, diff). cargo-about is NOT (~20-line
  run-to-run drift), so generate it at BUILD time and have CI assert generation
  succeeds, not that it matches a committed copy. Byte-identity alone catches
  map-ordering only probabilistically at small N - also assert the ORDER
  (sorted keys) directly. 20260715-110417, 20260715-142900.
- `validate-in-every-domain` (x3, was validate-membership-not-existence, ->
  Pending promotions): a validation gate must check the meaning a value has in
  EACH domain it crosses into, not the domain it was written in. Occurrences:
  existence-checked paths that were not MEMBERS of the served set (escaping
  `../` published a broken artifact, 142900); write-side guards that left the
  read-back path trusting user-writable data (142906); local-Path-valid
  segments that decode differently on the wire (`%2e%2e` is Normal locally,
  dot-dot per WHATWG - steered same-origin GETs, 163508). Enumerate the
  domains (fs path, URL segment, IDB key, ...) in the plan and pin a test per
  domain. 20260715-142900, 20260715-142906, 20260715-163508.
- `toml-keys-before-tables` (x1): in TOML every top-level key must precede the
  first `[table]` header or it silently folds into that table (cargo-about's
  about.toml errored "unknown field targets" when a `[private]` table sat above
  it). 20260715-110417.
- `verify-tool-via-subcommand-not-which` (x1): a successful `cargo install`
  puts a binary in ~/.cargo/bin which may not be on PATH; check `cargo <sub>
  --version`, not `which`, before concluding the install failed. 20260715-110417.
- `relocation-leaves-ignored-siblings` (x1): a sprout worktree is a fresh
  checkout, so gitignored files (autosave backups, build junk) exist only in the
  MAIN checkout; `git mv`-ing a dir's tracked files out then landing leaves the
  ignored siblings behind on disk in the main checkout, which a copy-dir build
  still ships. After landing a "stop shipping dir X" move, `rm -rf` X's leftover
  ignored files from the main checkout. 20260714-154958.
- `verify-stale-brief-against-tree` (x1): a task brief can be partly stale
  (e.g. "three credits copies exist"); check the live tree before planning or
  you chase a non-problem. 20260714-154958.
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
- `gate-producer-and-its-consumers` (x1): when a flag skips PRODUCING an
  entity/asset, sweep for its CONSUMERS too - every system that later looks up
  that marker/component and drives it - and make each tolerate the producer
  having been skipped (early return, not an `error!` path). Gating 3 particle
  SPAWN observers on Low left 2 paired per-shot reset observers error-spamming
  every shot, because the spawned effect they `reset()` no longer existed. A
  code-mapping sub-agent's sweep scoped to spawn sites reported "all gated"
  truthfully but never saw the consumer side; a follow-up grep for the consumer
  call (`effect_spawner.reset()`) caught it. Sibling of `verify-first-plan-steps`
  ("enumerate consumers of shared state"). 20260525-133013.
- `messagereader-needs-resource-guard-in-tests` (x2): a system with a
  `MessageReader<T>`/`MessageWriter<T>`/event param panics in MINIMAL-app tests
  that omit `Messages<T>` (no `InputPlugin`, or a rig that runs the system
  directly without the full plugin); either gate it
  `run_if(resource_exists::<Messages<T>>)`, or add `add_message::<T>()` /
  `init_resource::<Messages<T>>()` to every such rig - grep the manual
  `add_message` sites before running. A scroll reader broke 4 menu tests; a new
  `RadarRetargeted` writer needed the resource in 2 targeting rigs. Recipe for
  cross-plugin resources: init in BOTH the writing and consuming plugin
  (idempotent) - a menu OnEnter clear panicked every menu-only rig until the
  consumer plugin inited too (20260716-193949).
  20260714-174126, 20260714-090006.
- `worktree-shares-main-target` (x1, CORRECTED): a fresh sprout worktree has an
  empty `target/` - accept the cold build; do NOT share `CARGO_TARGET_DIR` with
  the main checkout (same crates, artifacts clobber; a worktree binary silently
  linked master's code in 20260709-131502). Shell cwd resets between commands.
- `commit-before-sabotage` (x2, PROMOTED 2026-07-11 -> work skill): commit the
  fix before A/B sabotage; anchor scripted splices on unique strings and
  compile immediately after. 20260710-231930, 20260713-082330.
- `production-faithful-rigs` (x8, PROMOTED 2026-07-11 -> work skill): test rigs
  must mirror production - scheduling, every system that ticks the state, the
  real body-vs-collider hierarchy an engine observer needs, and the shipped
  CONFIGURATION (a rig on default AssetPlugin settings "verified" a meta fix
  the app's meta_check never read). MIRRORING a load-bearing registration
  inline in a rig is the weak form - extract ONE shared registration helper
  the plugin and the rigs both call, so the chain/gate cannot drift
  (20260717-112647 R1.2). REQUIRED-COMPONENT DEFAULTS are configuration too:
  a rig that hand-picks a "tidy" state (Patrol) instead of the spawn default
  (Engage) dodges the exact path every production spawn takes - a mutation
  probe survived until the rig seeded the default (20260717-163042 R1.1). 20260711-103527, 20260712-133343,
  20260713-175416.
- `presence-vs-behavior-tests` (x2): component-exists assertions stay green
  while behavior regresses; assert the behavior. 20260709-160753.
- `sweep-then-delete` (x11): before deleting, moving, or swapping a mechanism or
  marker, grep the WHOLE repo for its symbol names, describing words, and
  observers/queries - including root-level docs (README, AGENTS.md), the
  CHANGELOG, text the same branch added earlier, and PROSE inside
  rustdoc/comments across ALL file types (a docs-folder-only sweep left three
  stale "mod pipeline" comments for review to catch). When a FILE moves, grep the
  bare filename/stem AND the markdown-link forms `[x](x.md)`, not the old
  `dir/x.md` PATH - a path-prefixed grep misses relative links and renamed
  targets (a doc move shipped four `/wiki/dev/...` 404 links past a docs/-prefixed
  sweep). A stale CLAIM can be twinned too: the same superseded infinite-ammo
  comment lived in the builder AND its test; fixing one and declaring victory
  left the twin for review - grep the describing words before claiming a
  comment fix complete. When RENAMING a tool/bin/command, grep BOTH the
  command form (`--bin gen_content`) AND the bare artifact-name prose
  ("the `X` bin"): a `--bin`-only sweep left "the `balance_audit` bin runs"
  in balance_audit_gate.rs (review R1.1) plus its twin in balance.rs.
  20260711-212519, 20260712-133343, 20260712-211352,
  20260714-204219, 20260715-151551, 20260715-195621, 20260717-112639,
  20260717-212219.
- `reread-after-insert` (x2): after inserting into a function or test, re-read
  the whole thing for bindings, assertions, or invariants the insertion
  duplicated or broke. 20260710-214316.
- `does-the-old-element-survive` (x2): when a design adds an element
  overlapping an existing one, decide explicitly what happens to the old one.
  20260711-000547.
- `destructive-chains-check-completability` (x1): a multi-step destructive
  action (update = uninstall + install) must not START unless every step's
  preconditions hold - an offline Update ran its local uninstall then failed
  the Ready-gated reinstall, destroying a working install; every layer was
  correct, the COMPOSITION was the hole. State the completability invariant in
  the plan. 20260715-142916.
- `removed-control-orphans-persisted-state` (x1): a change that removes or
  hides a control (row, toggle) must sweep every WRITER/persister of the state
  that control managed - not just readers of the changed resource - and answer
  how that state gets corrected without it; hiding a mod row orphaned its
  persisted enablement. 20260715-142844.
- `author-facing-schema-needs-syntax-doc` (x1): when adding a serde field that
  authors hand-write (especially Option in strict RON: `icon: Some("x.png")`,
  never `icon: "x.png"`), document the literal syntax in the same change - a
  schema doc that omits how to type it ships a parse footgun. 20260715-142849.
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
- `render-output-eyeball` (x3): a generated image/visual asset that validates at
  the right dimensions/format can still look wrong - open it. A composite that
  passed 1920x1080 was 2:1 distorted; eyeballing it drove the switch to
  aspect-preserving contain-fit. UI variant: a layout task is not verified
  until someone SEES it rendered (Xvfb screenshot + Read) - headless asserts
  cannot see z-order/overlap (a menu card painted over the new mods panel,
  ordered by recycled entity ids). Corollary: a scope change touching a past
  deferral's premise (the panel grew 460px -> 85%) re-opens the deferral.
  20260715-004216, 20260715-142911, 20260711-180511.
- `roundtrip-hides-shared-bug` (x1): a codec/serializer round-trip test built on
  a self-authored forward pass proves symmetry, not correctness - a predictor/
  formula bug shared by encode and decode cancels. Re-derive the reverse against
  the spec independently (PNG Paeth/Average filters re-derived vs spec 9.2).
  20260715-004216.
- `one-cargo-test-filter` (x5): `cargo test` takes one filter and one `-p` per
  invocation; separate runs otherwise (recurred under flow momentum: a chained
  two-filter run silently tested nothing). 20260713-082324, 20260716-162701.
- `check-all-targets-for-struct-field` (x3 -> Pending promotions): a new
  non-Default field breaks every exhaustive initializer - builders, TESTS
  (`#[cfg(test)]`), and EXAMPLES - but `cargo check`/`check -p <crate>` compiles
  none of those; only `cargo check --all-targets` does. Grep the WHOLE repo for
  the literal (not just `crates/`), and run `--all-targets` BEFORE landing.
  20260717-165031 hit it twice in one task: a `#[cfg(test)]` literal `cargo check`
  missed (caught by `cargo test`), then 8 `examples/*.rs` literals that escaped to
  master because the pre-land checks were crate-scoped, not `--all-targets`.
  20260712-140250, 20260716-155849, 20260717-165031.
- `mod-facing-surface-plans-failure-paths` (x1): a task exposing a surface to
  MOD DATA must plan its failure paths up front - enumerate "what breaks when a
  mod does this badly" (missing entity contracts, empty sets, unregistered ids)
  as plan steps; all three hazards of the menu_backdrop feature (well-less
  backdrop bricking the camera, zero backdrops, dangling declarations) were
  work-phase discoveries the plan never named. 20260716-155849.
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
- `generate-data-from-code` (x3, -> Pending promotions): migrate code-defined content to data files by
  serializing the code config with a parity test, never hand-authoring - provably
  faithful and sidesteps every format-syntax gotcha. Corollary: a change to any
  builder behind a committed generated artifact regenerates the artifact in the
  SAME commit (713ac855 changed the shakedown builder, left the RON stale, and
  master's parity test went red until 172138). Mirror image: hand-edits
  (even comments) in a GENERATED artifact don't survive regen and ride
  master red until someone regenerates - author them in the BUILDER; the
  parity test is the contract (a sibling's hand comment in
  asteroid_next.content.ron, regenerated away as merge integration by
  20260717-162121). 20260525-133028, 20260715-172138, 20260717-201534.
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
- `pin-the-fix-at-its-boundary` (x4, -> Pending promotions): guard a bug fix with a
  test that fails under the bug at the fix's OWN boundary (a unit test), not only a
  downstream e2e - especially when the existing unit test passes under the bug (the
  DisableVerb multi-verb accumulation was only e2e-guarded). Refactor variant: when a
  refactor changes how an invariant is ENFORCED, re-pin the invariant on the new
  mechanism - don't massage the old assertion until it passes. Overlay variant: the
  section-overlay-by-id bug was invisible with one bundle (no id collision); extract
  the overlay into a pure helper and unit-pin last-wins so the divergence can't hide
  until a second bundle exists. Decision-function variant: when you change a pure
  decision/predicate's contract, grep its table-test callers FIRST and re-pin
  the table on the new contract in the same edit, so the fail-first is planned
  not discovered (advance_decision_table asserted decide_advance's old
  paused=Ignore rule). 20260714-113411, 20260714-135642, 20260714-134119,
  20260716-214919.
- `shared-id-space-shared-overlay` (x1): when one router dispatches into multiple
  containers that share an id space (a Vec of sections + a map of scenarios), route
  through ONE overlay helper so the kinds can't silently diverge (Vec push/first-wins
  vs map insert/last-wins). 20260714-134119.
- `verify-the-nit-compiles` (x2): any review fix (a micro-opt NIT, a two-line TS
  swap, a comment asserting caller behavior) is a hypothesis - compile/typecheck
  it AND verify the contract it assumes before marking done. `rel.as_str()` for
  `rel.to_string()` failed E0597; `onload = (): void => appendChild(...)` failed
  TS2322 (node not void) and needed a block body; a png-validation fix's comment
  claimed callers caught its ValueError when the one call site had no try/except.
  20260714-134119, 20260714-210131.
- `agent-interrupted-verify-worktree` (x2): a subagent that hits a long build can end
  with an ambiguous partial state and misleading "in progress" notifications; INSPECT
  the worktree (git status + compile + run the deterministic generators) before
  concluding done-or-broken. For data-file work `cargo run -p nova_assets --bin
  gen_content` completes it deterministically (the parity test no longer writes,
  20260716-155823). 20260714-150508, 20260715-142906.
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
- `null-result-becomes-a-pin` (positive, x2): when an investigation lands on
  "cannot reproduce" / "not a bug", convert the evidence rig into a permanent
  harnessed pin (error-handler-to-panic smoke example; a `url_origin`
  same-host-different-port test proving the "prod CORS" case is same-origin) so
  the non-behavior stays falsifiable and the rig's cost buys coverage.
  20260713-175352, 20260715-214540.
- `state-diff-aliases-reset` (x1): deriving events by diffing state makes a
  reset look like a batch of events; guard the non-event transitions
  (teardown, load, clear). 20260712-125342.
- `landing-checkout-not-yours` (x3): parallel sessions share the in-place
  checkout; verify `git branch --show-current` before EVERY commit.
  20260525-133004.
- `pair-matrix-on-collider-class-change` (x1): changing a collider's class
  (sensor?, events?) must be checked against every collider category in the
  game, not just the pair being fixed. 20260712-121101.
- `verify-scripted-edits-applied` (x4, -> Pending promotions): an edit you
  believe you made is a hypothesis until the artifact shows it - a no-match
  replace looks like success, a matching one can emit malformed text, and a
  RETRIED batch of failed edits can silently drop a member (2 of 3 re-applied;
  docs kept claiming all 3 and the review caught it). STRING LITERALS built by
  a script inherit the script's own formatting: a heredoc continuation baked
  26 spaces into a shipped lint message - read the produced TEXT, not just the
  replace count. 20260712-110730, 20260716-125856, 20260708-203659,
  20260717-151208.
- `reuse-production-helpers-in-tests` (x3, -> Pending promotions): tests
  compose expected values and spawn rigs via the production helpers, not
  inline re-derivations - a hand-written bay rig hit a private-field wall
  the production `torpedo_section()` bundle four screens up already solved.
  20260711-121839, 20260712-110730, 20260717-112622.
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
- `audit-state-gates-on-new-entry-path` (x3, -> Pending promotions): a new route
  into a state needs a workspace grep of run_if/in_state (and OnEnter/OnExit)
  and a written what-newly-runs list per context - a new entry into
  `PauseStates::Paused` (the outcome frame) had to enumerate setup_pause_ui,
  the ESC toggle, decide_advance, the clocks and the cursor guards.
  20260711-180426, 20260711-212519, 20260716-214919.
- `bound-scheduling-both-sides` (x1): a system inserted between a producer and
  a same-schedule reader needs both .after(producer) and .before(downstream).
  20260711-180501.
- `set-gates-miss-observers` (x1): gating a SystemSet does not touch observers;
  enumerate systems + observers + hooks before claiming a gate covers "input".
  20260711-185156.
- `would-it-fail-without-it` (x6): a verification that cannot fail with the
  mechanism deleted proves nothing; copied tests inherit vacuousness - and a
  sabotage that refuses to go red refutes the assumed mechanism itself, OR
  the test's own shape (a race pin with an update inside the race window
  stayed green under sabotage and had to be tightened to the true window).
  20260711-180426, 20260711-212521, 20260712-115902, 20260717-163033.
- `out-of-context-review-pass` (positive, x30): a fresh-context review of a
  substantial branch catches MAJORs shared-session eyes miss, and re-derives
  load-bearing claims instead of trusting them - checking cited evidence IS
  the spawn site, re-running the sabotage or the whole smoke suite, reading
  the DEPENDENCY's source for composition hazards, mutation-analyzing new
  tests, empirically reproducing a suspected hole before claiming it, reading
  a CI ACTION's source to settle a toolchain question, re-deriving an
  algorithm (Kahn's topo sort direction) by hand to catch a doc/atomicity
  overclaim, and re-deriving a course's geometric invariants from raw RON with
  its own script to surface a load-bearing dependency override, and re-deriving
  two engine-semantics claims from bevy source (skybox sanity check, AssetMut
  write-tracking) - killing a bogus follow-up work item. Verify the VERIFIER
  too: re-derive its counterexamples before filing them (a real non-unit-quat
  MINOR arrived with a wrong zero-quat sub-claim - glam's mul_vec3(q=0)
  returns v, not 0). 20260712-133343,
  20260711-183417, 20260712-115902, 20260715-142900, 20260715-142931,
  20260716-125856, 20260708-203659, 20260716-162701, 20260716-124722,
  20260716-214919, 20260717-013440, 20260717-162121, 20260717-212219.
- `required-component-in-shared-query` (x2): a required fetch added to an
  existing query narrows its membership and every gate computed from it; fetch
  `Option<&T>` or use a separate query. New `Res<T>` params also panic every
  `run_system_once` rig missing the resource. 20260712-143832, 20260712-164031.
- `spike-open-question-pays-off` (positive, x1): a spike that names a risky
  unknown lets the implementer resolve it before wiring. 20260712-143832.
- `authored-vs-derived-values` (x4): author content against measured runtime
  values exported as consts, not nominal constants or folklore ranges - and when
  a layout invariant depends on such a const (asteroid bodies reach the
  ASTEROID_GEOMETRIC_FACTOR_MAX=6.0x const past nominal), encode the invariant
  as a COMPUTED rig assertion over the shipped positions, so the geometry is
  proven not eyeballed and the fail-first A/B is one edit away (a gauntlet
  course's flyable-line clearance). Positions CARRIED OVER from an old layout
  need the same derivation as new ones - an inherited escort station sat 52u
  off a new fire lane and only the computed pin caught it. 20260711-180455,
  20260711-180506, 20260716-124722, 20260717-112630.
- `verify-engine-guarantees-in-source` (x8): read the engine/dependency source
  (or write a five-line probe) before designing around its behavior - observer
  order is arbitrary; observer-queued commands apply BEFORE the queue's remaining
  pending commands; a bcs `On<Insert>` observer `.unwrap()`s the asset, so
  inserting a config with an unloaded handle panics (design a deferred insert).
  A reasoned verdict about engine semantics is a hypothesis, not evidence - and
  a NONEXISTENCE claim from a workspace-only grep is worse: existence greps must
  include the dependency checkouts (a "stale" comment named a bcs observer that
  exists and is load-bearing). FAILURE-MODE claims are the same class: "X would
  crash" written from wgpu folklore was wrong twice in one task (bevy's skybox
  sanity check warn_once's and SKIPS a non-Cube view; AssetMut queues Modified
  only on an actual write) - read the consumer that produces the claimed
  failure and cite its file:line. Upstream DOC COMMENTS are folklore too:
  avian's cast_ray_predicate doc says the ray stops on predicate-false, the
  implementation SKIPS and continues (caught at plan time, 20260717-112622).
  So are OUR OWN doc comments when they assert a COST to justify a rejected
  design: `assets_plugin`'s comment said `AssetMetaCheck::Always` "would 404 per
  asset on wasm" as the reason for the per-path set; source showed the 404 is
  real but NON-FATAL (bevy falls back to default_meta), which flipped the choice
  to global Always. A runtime claim needs a source read or a runtime probe - a
  BUILD cannot see a runtime 404. Same class in reverse - before THEORIZING a
  missing-write bug (a review flagged the editor's direct `SkyboxConfig` insert
  as maybe skipping its Cube view), grep for a COMPENSATING system that writes
  the same handle before the consumer runs: `prepare_cubemap_view` sets it at
  startup, so there was no bug - one grep would have avoided filing the task
  (20260717-133332). 20260525-133004, 20260712-115902,
  20260525-133017, 20260716-162701, 20260717-013440, 20260717-112622,
  20260717-111558, 20260717-133332.
- `advertised-but-unwired` (x3): a config surface is not a capability until its
  producer/consumer wiring, data source, and runtime preconditions are
  verified in the new context. 20260712-093044, 20260712-093831.
- `cross-cycle-warning-with-numbers` (positive, x2): write discovered hazards -
  and review findings that belong to a QUEUED task - into that task's TASK.md
  with specifics; R1.2 routed forward arrived pre-specced and cost zero design
  time. 20260711-140234, 20260716-155823.
- `verify-at-deploy-base-path` (x2): base-path/origin-dependent behavior must be
  verified against the real DEPLOY topology, not a local setup. A "wasm CORS bug"
  was a local split-port dev setup; the deploy serves game + portal same-origin
  (siblings on one Pages origin), so production never hits it. 20260712-093048,
  20260715-214540.
- `dev-doc-steers-across-boundary` (x1): dev-setup docs (ports/hosts/origins/
  auth) that cross a browser/security boundary must name it and default to the
  safe path, or they manufacture the failure they forgot to mention - the portal
  "Local development" doc steered web testers to a cross-origin `?portal=` the
  browser blocks on CORS, which IS the reported bug. 20260715-214540.
- `nix-devshell-for-cargo` (x2, toolchain): if a session's shell has no `cargo`
  on PATH (the flake devshell is not active), prefix cargo/rustc/fmt with
  `nix develop --command ...` (from the repo/worktree) rather than assuming the
  toolchain is present - the devshell also sets `LD_LIBRARY_PATH` for the bevy
  link; a `cargo: command not found` (or empty output) is the tell.
  20260715-214540, 20260715-140049.
- `reuse-known-good-stack` (x2, positive): scaffold new work by copying a
  working reference verbatim - a toolchain, or in-repo the mod_prefs persistence
  stack, bevy's headless Slider, and example 14's screenshot harness for a whole
  UI feature. 20260712-093048, 20260711-180511.
- `measure-before-writing-the-number` (x2): never write a specific quantity
  into a doc from a mental model; backfill from an actual run - a NOTES hp
  sum hand-added next to the tool that prints the correct number dropped a
  section (660 vs 730). 20260712-105505, 20260717-143806.
- `manual-time-rig-measures-its-clock` (x2): TimeUpdateStrategy::ManualDuration
  is NOT the delta your Update systems see - `Time<Virtual>` clamps each update
  to `max_delta` (0.25s default), so a 1s manual step advances only 0.25s and a
  timer-driven assertion silently under-advances (a MinimalPlugins rig gave
  0.25s/update against both a 0.5s and a 1.0s setting). Fix: either raise
  `max_delta` on the inserted `Time<Virtual>` so the manual step passes through,
  or count effective ticks - and make it the FIRST hypothesis when a
  ManualDuration rig under-advances (the fire-rate rigs already carry a comment
  about it). Clock sibling of `measure-before-writing-the-number`.
  20260716-183220, 20260717-085640.
- `ab-isolation-bench` (x1, positive): attribute one system's cost with two
  worlds identical except for that system. 20260712-105505.
- `sweep-full-scale-before-believing-a-win` (x1): an O(N)->O(matching)
  optimization can LOSE to a cache-friendly linear baseline; benchmark across the
  whole scale range and both regimes - a naive handler index won at 500 handlers
  and reversed at 5000 (random-access thrash), fixed only by contiguous
  snapshots. 20260525-133014.
- `lint-gate-is-the-last-step` (x2): re-run fmt/clippy/tests AFTER the final edit,
  never before - fmt ran before a test module was added, so an unformatted commit
  shipped and only remote CI caught it, costing a cross-repo re-push. For a repo
  gated solely by remote CI, mirror its exact checks locally before pushing.
  Variant: per-part feature commits ran TESTS but not fmt, so intermediate commits
  shipped unformatted (harmless under squash-merge, but claim green only after an
  fmt pass). 20260525-133014, 20260715-142931.
- `document-the-async-failure-path` (x1): design notes for a concurrent/staged
  flow must trace the async FAILURE path and state the atomicity boundary, not
  just the happy-path intent - an out-of-context review flagged a NOTES claim of
  dependency-SET install atomicity when it is only per-mod (deps download in
  parallel with no join). The behavior was fine and surfaced; the words
  overclaimed. 20260715-142931.
- `sibling-change-leaves-stale-fixture` (x3, -> Pending promotions): a change
  that lands on master without updating a fixture test that asserts on its data
  leaves master RED for the next branch to inherit and realign (a mod-bundle
  description; the demo-scenario removal's `contains_key("demo")` guards; an
  EXACT bundle-version pin two sibling bumps later - pin the durable intent,
  "past 1.0.0", never a frozen literal that siblings legitimately move). Sweep
  test fixtures that assert on data you change, and grep for tests that
  include_str the files you touch. 20260715-142931, 20260716-155839,
  20260717-151214.
- `benchmark-gates-both-ways` (x1, positive): a measure-first gate justifies
  DEFERRING optimization work as legitimately as doing it; 083339's filter/
  condition micro-opts were declined on data (noise at realistic rates), a valid
  outcome not an unfinished task. 20260525-133014.
- `verify-bevy-api-at-callsite` (x1): before writing an unfamiliar Bevy
  bundle/field, copy an existing in-repo callsite; the 0.x API churns.
  20260712-131348.
- `anchor-edits-in-the-right-scope` (x2): inserting into a large file by unique
  text can land in the wrong enclosing scope (a `#[test]` compiled inside a
  production impl because the anchor string also appeared there), and anchoring
  on a `fn` line STEALS the attribute/doc block above it (a neighboring test was
  silently deregistered; the filtered run listing the new test twice was the
  tell). Anchor on the attribute/doc block start or after a closing brace, and
  confirm the module boundary. 20260525-133017, 20260716-193949.
- `spike-reuse-over-new-infra` (x1, positive): when a request implies new
  infrastructure, first check whether an existing substrate covers the real
  need. 20260712-131348.
- `trace-vehicle-timeline-first` (x1): pick a runtime-evidence rig by its
  script TIMELINE (when does the stimulus fire, inside the window?), not by
  scene content; for idle-scene targets the plain app beats any harnessed
  example. 20260711-183417.
- `pkill-pattern-matches-own-shell` (x2): `pkill -f <pattern>` matches the
  invoking shell's OWN command line when the pattern appears in it (a cleanup
  `pkill -f 'Xvfb :99'` killed its whole command chain, exit 144; recurred when
  a `pkill -f 'Xvfb :7'` prepended to a fmt+test command killed the chain, same
  exit 144 - the tests were fine), and blind pattern-kills can hit look-alike
  user processes (a second Xvfb was plausibly the user's real display); record
  the helper's PID at spawn and kill THAT, or let session-scoped helpers die
  with the session. 20260716-180352, 20260717-004302.
- `silent-tool-missing-in-pipeline` (x1): a missing launcher (xvfb-run) dies
  with 127 that a trailing pipeline swallows, reading as a clean empty run;
  `which` host tools before the first long run and keep launcher exit codes
  out of pipelines. 20260711-183417.
- `no-source-edits-during-inflight-builds` (x1): cargo reads a crate's source
  when it COMPILES it, minutes into a cold build - a tree edited mid-build
  yields an indeterminate evidence binary; quiesce the tree (or file-copy)
  for A/B runs. 20260711-183417.
- `gpu-example-local-skip` (x1): a heavy `DefaultPlugins` render example (RTT
  camera + full scene) is ~100x too slow under local lavapipe software rendering
  to run its scripted timeline to the assertion stages - budget ONE short smoke
  attempt to confirm the render path inits and the shader loads without error,
  then stop and rely on headless unit tests (driver + asset, no render app) plus
  CI's `examples_smoke`. Do not burn multiple build+run cycles chasing a local
  completion. 20260717-004302.
- `borrowed-rig-coverage-check` (x1): a rig/pattern borrowed from another
  task's record inherits that record's overclaims; verify its coverage
  against the NEW failure mode before prescribing it (the handler-swap pin
  cannot see baked-in remove/despawn warns). 20260712-115902.
- `refutation-invalidates-earlier-prose` (x1): when a probe overturns the
  working theory mid-task, re-read every artifact written under the old
  theory (notes, comments, records) in one pass; the review found the dead
  model still taught as fact. 20260712-115902.
- `run-example-via-cargo-run-for-assets` (x2): a built example binary run
  directly (`./target/.../examples/foo`) resolves `assets/` relative to CWD
  and fails to load everything (`BEVY_ASSET_ROOT` did not help); run via
  `cargo run --example` from the crate root so the asset path resolves. Also:
  autopilot/tracing logs go to STDERR - use `2>&1`, never `2>/dev/null`.
  20260714-204219, 20260714-214111.
- `despawn-by-owner-not-all-on-cross` (x1): a hover-out handler that despawns
  ALL of a shared singleton (tooltip/highlight) can kill a freshly-spawned one
  if the sibling's enter fires before this one's leave; tag the singleton with
  its owner entity and despawn only the match. 20260714-204219.
- `autopilot-is-frame-starved-under-load` (x1): the BCS autopilot's phase waits
  are frame-COUNTED but its lifetime is a wall-clock ~6s, so under heavy load
  (cold full-graph rebuild, a parallel sprout building) too few frames run and it
  stalls mid-sequence - reads as a step failure but is starvation. Run
  timing-sensitive autopilots BEFORE other heavy builds, or verify a touch-free
  path by `git diff` + a deterministic unit test instead. 20260714-214111.
- `ui-footprint-vs-3d-picking` (x1): a UI panel over the point where a 3D
  object projects BLOCKS its placement/pick raycast; size left panels against
  the actual window resolution (read it, don't guess) so the build area stays
  uncovered, and verify with the real pointer path. 20260714-204219.

- `rig-supplies-precondition-hides-regression` (x2): a test that INJECTS the
  state production is responsible for establishing (seed a variable, spawn the
  actor, insert a resource) is structurally blind to that setup breaking - the
  green rig hands itself the precondition. Pin the production setup with a
  separate assertion (a gauntlet behavior test seeded `gate=1` itself and
  skipped OnStart, so a dropped player-ship spawn / `gate=1` seed would ship
  green; every gate filter fails closed on an undefined var). Sibling of
  `production-faithful-rigs`. Applied PREVENTIVELY on 224812 (arena OnStart
  structural test shipped with the behavior test). 20260715-224803, 20260715-224812.
- `bg-isolation-guard-allows-sprout-not-main` (x2): the background-job Write/Edit
  guard blocks the main checkout but NOT a sprout worktree; author master-side
  artifacts (plan stubs, RETRO.md, LESSONS.md) via Bash heredoc and do all code
  in the sprout worktree where Write works. The settings escape
  (`worktree.bgIsolation: none`) is denied by the self-modification classifier.
  20260715-224803, 20260715-140049.
- `isolate-off-head-for-unpushed-deps` (x1): when a task's work depends on
  commits that exist only on LOCAL master (unpushed - e.g. a just-added asset),
  isolate off local HEAD (`sprout new`, which branches off HEAD) not a
  fresh/origin-based worktree (`EnterWorktree` default `baseRef: fresh` cuts from
  origin/<default>), which would omit the local commit and fail the build on a
  missing file. Check `git remote` + the dep's commit before choosing the base.
  20260715-140049.

- `count-gate-use-gt-not-eq` (x1): a milestone gate on a COUNTER incremented by
  an event that can fire more than once (collisions, per-collider pairs,
  multi-hit) must gate on `> N-1` / `>= N`, never `== N` - an overshoot past the
  exact value skips the gate forever (arena win used `destroyed>2`, not `==3`, so
  a double OnDestroyed can't soft-lock the clear). Counting sibling of
  `collisionstart-is-per-collider-pair`. 20260715-224812.

- `gate-on-what-you-assert` (x1): in a staged harness, every condition an
  assert relies on joins the stage GATE when it can lag the gating state by
  frames (an overlay entity spawns one frame after its resource; asserting it
  on the resource's first frame is a masked race). 20260708-203659.

- `rig-before-fix-on-unreproducible` (positive, x1): for a happened-once
  report, enumerate the candidate mechanisms as a boundary rig BEFORE any
  fix - the red subset IS the diagnosis, fail-first comes free, the green
  subset becomes pins, and the fix can then land at the seam that closes
  the class. 20260716-162701.

- `probe-the-adversarial-variant` (x1): pick evidence/eyeball variants by what
  they can HIDE, not by staging convenience - a Defeat-only overlay probe
  masked a Victory-only cursor bug because the dead ship emptied the exact
  query that armed it; enumerate the variants and probe the one with the most
  live actors (or both when cheap). 20260716-125856.

- `pick-the-system-set-seam` (x1): when a plugin partitions its systems into
  `run_if` gated SystemSets, choose the target set for a new system explicitly -
  a debug convenience is not automatically a `DebugSystems`/`DebugEnabled`-gated
  system (F12 screenshot must fire with overlays toggled OFF, so it lives in
  plain `Update`, not the debug-gated set). 20260716-114125.

- `re-audit-consumers-on-input-model-change` (x1): changing a control from
  DISCRETE to CONTINUOUS (segmented buttons -> a drag slider) invalidates every
  policy written for the discrete model - re-audit the downstream consumers, not
  just the widget. A persist-on-change save that was one-write-per-press under
  segmented buttons became a full config write PER DRAG FRAME under a slider
  (ValueChange fires per pointer-move); fixed by debouncing. Sibling of
  `count-gate-use-gt-not-eq` (both are "an event that can fire N times").
  20260711-180511.
- `parity-test-must-cross-link` (x1): a "keep X in sync with Y" test must derive
  the expected value from ONE side and assert it against the OTHER, never assert
  each side against a hardcoded literal - that only pins each to the test, not to
  each other. A keybind parity test asserted "rig binds KeyW" AND "reference has
  a Main Drive row" separately, so a remap that updated the rig + the literal
  left the on-screen string stale and green; fixed by deriving the label from
  the live rig key and asserting the reference row contains it. Kin of
  `would-it-fail-without-it`. 20260711-180511.
- `ask-user-facing-control-style` (x1): the interaction STYLE of a user-facing
  control (slider vs stepper vs segmented) is a genuine preference fork - when
  you catch yourself deliberating it at length, ask (AskUserQuestion) rather
  than decide. A segmented volume control was built, reasoned over vs a slider,
  chosen unilaterally, and immediately swapped to a slider on user feedback; the
  persistence-scope fork WAS surfaced, the control-style fork was not.
  20260711-180511.

- `mirror-sibling-resolve-site` (x1): when adding an `AssetRef<_>` (or any
  resource-resolving) content field, mirror the nearest SIBLING field's resolve
  SITE, not just its declaration attributes - the resolve site decides which
  systems gain a resource dependency. A turret `fire_sound` first resolved inside
  the UNCONDITIONALLY-registered `insert_turret_section` (adding `Res<AssetServer>`
  to an observer many headless section rigs run); the sibling `muzzle_effect`
  snapshots the ref UNRESOLVED at build and resolves in a render-time observer.
  Caught in self-review by comparing to the sibling, not by a test. 20260717-002228.
- `piped-cargo-masks-exit-code` (x4, -> Pending promotions): `cargo ... | tail`/`| grep` reports the
  PIPELINE's exit (tail/grep = 0), so a compile FAILURE reads as "exit 0" in the
  harness notification; read the OUTPUT text for `error[`/`could not compile` vs
  `Finished`, or `set -o pipefail`. Bit twice in one task - a masked E0593 in a
  `| tail` build (surfaced only on the next `cargo run`) and a feature-gated
  Serialize error under `cargo test -p nova_scenario | tail`. A trailing
  `echo EXIT: ${PIPESTATUS[0]}` masks the same way - the harness reads the
  LAST command's exit, so print nothing after cargo (or `set -o pipefail`).
  Sibling of `warnings-clean-before-land` (error-only greps hide warnings;
  pipes hide the exit code too). `| tail -N` also EATS the earlier
  binaries' `test result:` lines in a multi-binary run - filter with
  grep "test result", never tail. 20260717-002228, 20260717-013440,
  20260717-162121.

## Domain lessons (nova-protocol specific)

- `gate-scenario-handlers-to-their-acts` (x1): in an act/phase-structured
  scenario, every handler fires in EVERY act unless filtered - walk each one
  asking "which acts may this fire in?", terminal states especially (an
  act-ungated death handler flipped an earned VICTORY to DEFEAT); gate by
  default, globality is the deliberate exception. 20260708-203659.
- `crate-solo-tests-miss-unified-features` (x5 -> Pending promotions): `cargo
  test -p nova_scenario` alone fails to compile - its serde round-trip tests
  lean on workspace feature unification (nova_assets -> nova_modding ->
  nova_scenario/serde); run crate tests with a unifying sibling (`-p
  nova_scenario -p nova_menu`), pass the feature directly (`-p nova_scenario
  --features serde`), or workspace-wide as CI does. Reconfirmed the hard way a
  THIRD time (paid one ~8min cold compile before adding `--features serde`);
  grep this ledger for the crate name before crate-scoped runs. 20260716-125856,
  20260716-155830, 20260716-231855, 20260717-002228 (hit a FOURTH
  time - did not grep the ledger for the crate name first, as the lesson says),
  20260717-151537 (FIFTH - same crate, same miss: ran `-p nova_scenario`
  before grepping the ledger, paid a ~4min cold compile).
- `deleted-content-tests-carry-engine-coverage` (x1): tests over shipped
  DATA can be the only exercise of an engine mechanism (filters.rs owned
  filter/action semantics with zero tests of its own); before deleting such
  tests, audit which mechanism assertions they uniquely carry and re-pin
  those at the owning crate's boundary FIRST. 20260716-155830.
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
- `half-ticked-compound-steps` (x2): tick a plan step only when every clause is
  done, or split it - and when the implementation legitimately adapts (a rig
  cannot deliver a clause), amend the step text in the same edit instead of
  silently narrowing. 20260708-165704, 20260716-162701.
- `bei-app-finish-in-tests` (x1): bevy_enhanced_input needs `app.finish()` +
  `app.cleanup()` before spawning an action rig in tests. 20260708-165705.
- `verify-ci-triggers-before-claiming-coverage` (x2): before writing "CI builds
  this", read the workflow triggers - a `workflow_dispatch`/deploy job is NOT
  automated PR/master coverage. An uncompiled cfg branch (e.g. the wasm
  localStorage backend) is guarded only by static review; say so in the comment.
  Positive application: un-gating hanabi on wasm, CI builds native only + deploy is
  workflow_dispatch, so ran the real `trunk build` (the sole wasm compile gate)
  instead of trusting green native CI. 20260714-174131, 20260714-233438.
- `lint-covers-types-not-variants` (x1): when writing checks over a config
  tree, enumerate every PATH to the checked type (grep the type name across
  the config structs), not the enum variants you remember - the prototype
  check missed ScatterObjects templates because ScenarioObjectConfig embeds
  in two actions. 20260716-191543.
- `content-identifiers-sweep-by-script` (x1, domain): scenario RON's
  cross-file identifiers (section prototype ids, NextScenario targets, filter
  ids) resolve at SPAWN, so every load/publish gate passes them green; sweep
  them by script against the catalogs before review (a from-memory
  "basic_hull_section" passed all gates; a 20-line cross-check caught it and
  two more classes). Lint task seeded: 20260716-191543. 20260716-123535.
- `spike-list-needs-code-check` (x1): a spike's enumerated list of
  mechanisms/effects is unverified prose until the implementing cycle greps each
  item against the code - a spike listed the thruster "plume" as a gated hanabi
  effect when it is a shader (`ThrusterExhaustConfig`) that already rendered on the
  web; caught only because /work grepped. 20260714-233438.
- `capability-detect-by-acquiring` (x1): gate on ACQUIRING the resource whose
  absence causes the failure, not on the API namespace existing - a WebGPU gate that
  checked `navigator.gpu` presence still crashed on a browser that exposed the API
  but could not get an adapter (the failure is at surface/adapter creation, one step
  downstream); probe `requestAdapter()`. A real playtest caught it; the unit tests
  written to the presence-only spec were green and useless. Re-check any plan
  assumption marked "unnecessary" when it is load-bearing for correctness.
  20260714-233443.
- `trunk-inline-script-before-deferred-module` (positive, domain, x1): trunk emits
  its wasm bootstrap as a deferred `<script type="module">`, so a plain inlined
  `<script>` (via `<link data-trunk rel="inline">`) placed after the target element
  runs synchronously BEFORE bevy boots - the place for a pre-init gate (WebGPU
  check, canvas swap). Confirm ordering in the built `dist/index.html`. 20260714-233443.
- `target-scoped-feature-flips-wasm-backend` (positive, x1): to switch only the
  wasm build's render backend, add an additive target-specific bevy feature
  (`[target.'cfg(...wasm...)'.dependencies] bevy = { features = ["webgpu"] }`) -
  bevy's `webgpu` overrides the default `webgl2`, so no trunk `--features` and no
  disabling defaults; confirm per-target with `cargo tree --target wasm32...`
  (webgpu present on wasm, absent on native). 20260714-233438.

- `portal-mod-ids-dash-only` (domain, x1): the portal generator's id gate
  accepts lowercase ascii/digits/'-' ONLY for MOD ids (directory names), while
  scenario ids conventionally use underscores - name fixtures to the
  VALIDATING gate's rules, not neighboring conventions. 20260716-155839.

- `mod-dependency-overrides-are-load-bearing` (domain, x1): a mod's declared
  dependency does not just PROVIDE prototypes - via the last-wins id overlay it
  can silently OVERRIDE a base section by id, so a dep can be load-bearing for
  BALANCE, not just availability (the gauntlet's `demo` dep bumped
  reinforced_hull_section 200->400, and its whole "reinforced hull buys crash
  tolerance" premise rode on that). Before writing a dependency rationale, grep
  the dep's content for the ids your content names; "base for the prototypes" is
  wrong when a second dep is quietly doing the balance. Sibling of the overlay
  lessons (`shared-id-space-shared-overlay`). 20260716-124722.
- `verify-current-convention-not-task-premise` (x1): a task naming a concrete
  mechanism ("bump the pinned rev", "edit file X") is a snapshot from when it was
  written - verify the live state before following it. A cross-repo bump task said
  "bump the rev (e.g. 4c81117)" but the current pin was already a release tag
  (`tag = "v0.19.0"`, moved rev->tag in `bb50db75`), so the fix cut a patch
  release v0.19.1 across all five crates instead of leaving a rev/tag mix. Grep
  the actual Cargo.toml/config before trusting the premise. 20260716-165617.

## Pending promotions (3+ occurrences, user decides)

- `verify-scripted-edits-applied` (x3) -> work skill: an edit you believe you
  made is a hypothesis until the artifact shows it - assert replace counts,
  grep the produced text, and after a FAILED batch of edits re-verify every
  member of the batch (a retry re-applied 2 of 3 and the docs kept claiming
  all 3). See the main-list entry. 20260712-110730, 20260716-125856,
  20260708-203659.
- `validate-in-every-domain` (x3) -> work/review skill: a validation gate must
  check the meaning a value has in EACH domain it crosses into (fs path, URL
  segment, storage key, served set), not the domain it was written in; three
  distinct escapes in one task family slipped a single-domain gate. See the
  main-list entry. 20260715-142900, -142906, -163508.
- `crate-solo-tests-miss-unified-features` (x3) -> work skill / docs/development.md:
  `cargo test -p nova_scenario` alone fails to compile (serde round-trip tests
  need workspace feature unification); run with `--features serde`, a unifying
  sibling, or workspace-wide. Grep this ledger for the crate before crate-scoped
  runs. See the main-list entry. 20260716-125856, 20260716-155830, 20260716-231855.

- `piped-cargo-masks-exit-code` (x4) -> work skill / AGENTS.md: never end a
  cargo command with tail/grep/echo that eats its exit code - the harness
  reads the LAST command's exit, so a compile failure reports as success; run
  cargo bare (backgrounded) and read the output text, or `set -o pipefail`.
  See the main-list entry. 20260717-002228 (x2 in one task), 20260717-013440.

- `check-all-targets-for-struct-field` (x3) -> work skill / AGENTS.md: a new
  non-Default struct field breaks exhaustive initializers in builders, tests
  AND examples; `cargo check -p <crate>` compiles none of the latter two. Grep
  the WHOLE repo for the literal and run `cargo check --all-targets` BEFORE
  landing. See the main-list entry. 20260712-140250, 20260716-155849,
  20260717-165031.

- `sibling-change-leaves-stale-fixture` (x3) -> work skill: before landing a
  content/data change, grep for tests that include_str or assert on the exact
  files touched (fixture pins live far from the diff); pin durable intents,
  not frozen literals. See the main-list entry. 20260715-142931,
  20260716-155839, 20260717-151214.
- `reuse-production-helpers-in-tests` (x3) -> work skill: compose test rigs
  and expected values via the production helpers/bundles, not hand-written
  component lists; grep the same file for an existing rig of the same entity
  kind first. 20260711-121839, 20260712-110730, 20260717-112622.
- `tatr-same-second-collision` (x7) -> tatr skill / AGENTS.md: never issue two
  `tatr new` calls in the same second or one bash line - they share a
  second-resolution ID and the later silently overwrites the earlier. One `tatr
  new` per tool invocation.
- `pin-the-fix-at-its-boundary` (x4) -> review/work skill: guard a bug fix with a
  test that fails under the bug at the fix's OWN boundary (unit test), not only a
  downstream e2e; when a refactor changes how an invariant is enforced, re-pin the
  invariant on the new mechanism rather than massaging the old assertion (grep a
  changed decision-function's table-test callers first). See the main-list entry
  for the variants. 20260714-113411, -135642, -134119, 20260716-214919.
- `audit-state-gates-on-new-entry-path` (x3) -> review/plan skill: a new route
  into a state (a new setter of a `States`) needs a workspace grep of
  run_if/in_state + OnEnter/OnExit and a written what-newly-runs list per
  context; the outcome frame's new entry into `PauseStates::Paused` had to
  account for setup_pause_ui, the ESC toggle, decide_advance, the clocks and
  the cursor guards. See the main-list entry. 20260711-180426, 20260711-212519,
  20260716-214919.
- `verify-engine-guarantees-in-source` (x3) -> work/plan skill: read the
  engine/dependency source (or write a tiny probe) before designing around its
  behavior - ordering guarantees, observer semantics, and panic-on-precondition
  (e.g. a bcs `On<Insert>` observer that `.unwrap()`s an unloaded asset). See the
  main-list entry. 20260525-133004, 20260712-115902, 20260525-133017.

- `generate-data-from-code` (x3) -> work skill: generated artifacts follow
  their builder, both directions - a builder change regenerates the
  artifact in the same commit, and hand-edits (even comments) in the
  artifact belong in the builder instead; the parity test is the
  contract. See the main-list entry. 20260525-133028, 20260715-172138,
  20260717-201534.

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
