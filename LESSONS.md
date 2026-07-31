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

- `collapse-single-item-special-case` (x1): when an aggregate/list path already
  exists, model the single item as a one-item list before preserving a separate
  branch; dual paths make CLI semantics and docs drift. 20260729-003352.
- `outcome-is-last-write-wins-close-the-act` (x1): CurrentOutcome overwrites,
  so EVERY outcome-declaring handler must set a terminal act itself - a
  single unguarded path (player death vs an every-pulse win gate) can
  overwrite a settled Defeat with a Victory. Sweep by class, not by the
  motivating case. 20260721-160957.
- `pickaxe-hit-is-not-a-mechanism` (x1): a `git log -S`/blame hit names a
  commit that TOUCHED the string, not what it did with it - open the commit's
  diff and quote what it DID before writing history into a Record (a misread
  pickaxe put a never-true chain into three surfaces). 20260721-160842.
- `keep-docs-in-sync-with-code` (x9, enforced in AGENTS.md but STILL recurring
  -> needs a tooling guard, not more prose): a code change is not done until
  every doc surface it invalidates (CHANGELOG, news, player + dev wiki, tutorial,
  per-mod READMEs, content-file headers, and the crate table which lives in
  README + AGENTS + MULTIPLE wiki pages - project-tour, architecture) is fixed in
  the SAME task; map: `web/src/wiki/dev/keeping-docs-in-sync.md`. A ticked docs
  step is not proof - `grep -rn <oldname>` the WHOLE doc tree (wiki + news +
  READMEs + CHANGELOGs), fix every live-state hit and LEAVE dated history (root
  CHANGELOG, per-release news, tasks/) with a reason - never pre-narrow the grep
  to one subdir. A NEW content KIND is a doc surface too: sweep the content-kind
  enumerations (modding-ron, guide-make-a-mod, scenario-system). The rule guards
  the sweep's SCOPE; its QUERY is
  [[sweep-docs-for-the-feature-description-not-just-its-symbols]].
  20260718-004723, 20260719-112231, 20260718-231555, 20260720-224236, 20260722-214119, 20260724-193830, 20260729-211200.
- `sweep-docs-for-the-feature-description-not-just-its-symbols` (x2): the
  changed identifiers do not define the search space - docs describe BEHAVIOR
  (what the player sees, or which CATEGORY of thing does what) and often never
  name the module. Write down 3-5 phrases a writer would use for the thing and
  grep THOSE too before ticking the doc step: a symbol-only sweep left a wiki
  sentence selling the deleted objective card (never used the word "reveal"),
  and another left the wiki scoping completion backstops to "the sections,
  gameplay and ui examples" after a screenshots example grew one. Query half of
  [[keep-docs-in-sync-with-code]]. 20260729-211200, 20260729-222131.
- `doc-sweep-covers-source-doc-comments` (x1): when RENAMING/REMOVING a
  command or symbol, `grep -rn '<oldname>' --include='*.rs'` the source
  `//!`/`///` doc comments too - a CLI surface is described in module/crate
  docs as often as in README/wiki, and a markdown-only sweep left three stale
  `audit` mentions for review to catch. 20260718-152240.
- `doc-sweep-includes-the-changed-dirs-own-readme` (x1): when changing a mod's
  (or subdir's) content - names, structure, visibility - grep the CHANGED
  directory ITSELF, not only the central `web/`+`crates` doc tree: a mod ships
  its own `README.md` that is a player-facing doc surface, and a sweep scoped to
  the central docs called it "referenced nowhere" while a stale flat-picker
  instruction sat in the mod's README. 20260724-220842.
- `rustdoc-no-public-to-private-intra-doc-link` (x2): a `pub` item's
  rustdoc cannot `[intra-doc-link]` a PRIVATE symbol (or a cross-module item not
  in scope) without a `cargo doc` warning - plain code span for private refs,
  full paths for cross-module refs; moving documented code across a module/crate
  boundary reliably breaks these, so run `cargo doc -p <crate> --no-deps` as part
  of the move. 20260723-143530, 20260727-015156.
- `extract-type-grep-its-drive-sites-first` (x1): before extracting a
  state-holding type into a new crate, grep every field/method access of it
  across the whole crate FIRST - external systems often read PRIVATE fields
  directly, so the real cost is designing a public accessor API (preserving
  immutable-read-then-conditional-`&mut` change detection) and splitting the
  test module, not moving the definition. Plan that surface, don't discover it
  mid-build. 20260727-015156.
- `ephemeral-news-draft-drifts-behind-content` (x1): the `docs/news-*.md`
  release drafts are ephemeral and easy to skip in a doc sweep, so they drift
  BEHIND the content they describe - when a feature/chapter changes, RE-READ
  the matching news bullet against the current source and REWRITE it, do not
  just append (a stale pre-stealth-rework "ambush" bullet survived two tasks).
  One more surface on `keep-docs-in-sync-with-code`. 20260723-143603.
- `bundle-version-string-pin-bites-on-bump` (x2): a test that asserts a bundle
  `contains "vX.Y.Z"` lives far from the `meta.version` edit even within one
  file, so a version bump silently invalidates it until the test runs - on ANY
  version bump, `grep -rn '"<old-version>"' crates/` in the same change. Bit the
  ch5 rig on 1.10->1.11 and the ch4 rig's sell-chain the cycle before.
  20260723-182855, 20260723-200643.
- `pin-mirrored-list-against-source` (positive, x2): code that hardcodes a copy
  of data owned elsewhere (a lint's reserved keys; the web theme's `:root`) gets
  a test that READS the real source and diffs both directions, so the copy
  self-corrects on drift instead of rotting. Two surfaces mirroring one origin is
  fine; two hand-synced lists is not. 20260718-152240, 20260731-143918.
- `verify-transparent-tool-is-active` (x1): for a transparent tool (compiler
  wrapper / cache), "the build was fast/passed" is not proof it works - a
  silently-inactive `RUSTC_WRAPPER` looks normal. Confirm it is ACTIVE via its
  own counter (`sccache --show-stats` non-zero requests), and have review
  re-derive the measured claim, not just read it. 20260721-000229.
- `completeness-by-running-not-grepping` (x1): for a "make X work across ALL
  crates/items" task, prove completeness by RUNNING the real check per item, not
  by grepping for a marker - the failing case may lack the marker
  (nova_scenario's failing tests were UNGATED, so a `grep cfg(feature)` sweep
  would have missed the very bug being fixed). 20260721-000249.
- `lint-enabled-crate-must-be-zero-of-that-warning` (x1): enabling a
  warn-as-clean lint (`#![warn(missing_docs)]`) per crate is only safe if that
  crate emits ZERO of that warning - verify per-crate (build with the lint and
  count), not just "cargo doc passed"; a lint on a still-dirty crate is a silent
  CI liability under `-D warnings`. 20260525-133032.
- `commit-msg-backticks-are-command-substitution` (x1): backticks (and `$`) in a
  double-quoted `git`/`sprout` `-m "..."` are SHELL COMMAND SUBSTITUTION - bash
  runs the backticked text and injects its output (a bare `pub mod` ate the term
  to empty; a backticked `git`/`sprout`/`tatr` phrase would EXECUTE). Use
  `-F <file>` (heredoc, quoted delimiter) or single quotes for any message with
  backticks/shell metacharacters. 20260721-121316.
- `parallel-builds-race-the-lint-count` (x1): fanning build-verified work across
  parallel agents on ONE shared worktree races their concurrent builds - a
  per-agent "count == 0" self-check is unreliable (one reported done with 40
  items left). The acceptance count comes from ONE settled build after all
  writes quiesce. 20260721-121316.
- `re-run-documented-commands-after-build-config-change` (x1): after changing
  workspace/build config (`default-members`, `[[bin]]`, `default-run`,
  features), RE-RUN THE DOCUMENTED USER COMMANDS (the README quickstart,
  `cargo run`) - not just the intended new behavior; a config change is judged by
  what it PRESERVES too. A `default-members` add (verified only for "bare build
  skips X") shipped a regression that made bare `cargo run` launch the `probe`
  bin instead of the game. 20260721-151934.
- `default-members-retargets-bare-cargo-run` (x1): on a workspace whose ROOT is
  a package, adding `[workspace] default-members` re-targets a bare `cargo run`/
  `build` from the root package to the whole member set (resolving to some other
  bin). A leaf tool that is not a game dependency is ALREADY skipped by bare
  builds, so the key buys nothing and only adds an allowlist footgun - do not add
  it. 20260721-151934.
- `validate-proof-command-shape-at-plan-time` (x5 -> Pending promotions, work
  skill): a `cmd:` proof is unrun until verify, so a malformed OR wrong-target
  one is a silent gate - at verify confirm it runs the INTENDED tests: right
  arity/flags AND a NON-ZERO count of the named tests (read "N passed" PER
  intended module, not just "ok"). `cargo test <a> <b>` rejects the 2nd filter
  (use `-- <a> <b>`); a copied `-p nova_gameplay drawer` filter matched 0 tests
  ("685 filtered out") yet reported ok; `-- f1 f2 ... f8` with many positional
  filters silently ran only SOME modules. An ABSENCE grep fails differently:
  one written from the WORDS of the stale claim (`dim chip|greyed`), or whose
  boundary a token the SAME change ADDS defeats (`--panel\b` also matches the new
  `--panel-radius`), can never reach zero - grep the specific phrases really in
  the tree, and check at plan time, against the tree the change will PRODUCE,
  that the command can return zero. 20260726-115334, 20260727-135208,
  20260728-175731, 20260730-122843, 20260731-143918.
- `inseparable-seeded-tasks-remerge` (x1, PROMOTED 2026-07-19 -> flow skill):
  when seeded tasks prove architecturally inseparable, surface the re-cut and
  merge them instead of building shims. 20260717-215742.
- `review-the-generated-artifact-too` (x1): after changing an authored or
  generated schema, READ the regenerated file with an author's eye - parity
  tests never check readability. 20260717-215742.
- `comment-the-local-wiring-not-the-general-protocol` (x1): when a comment
  explains WHY code follows a protocol, verify the protocol's preconditions
  hold at THIS call site before writing the rationale - check the
  registration/wiring, then write the reason. A new comment claimed reporting
  completion protected a pending capture; `capture_window` spawns a bare
  `Screenshot` and never calls `completion::register`, so the captures are not
  collectors at all and survive on a frame settle. The general story was true
  and locally irrelevant, in the file the next person copies the pattern from.
  Prose-side sibling of [[advertised-but-unwired]]. 20260729-222131.
- `absence-of-logging-is-not-a-measurement` (x1): a last-log-line timestamp is
  not an exit time, a frame count or a duration - a run that logs nothing
  between its beats LOOKS instantaneous. Before reasoning quantitatively about
  something nothing logs, build the instrument (here: one guard panicking with
  the stage index answered in a single run what a frame-time argument had got
  backwards). Cheaper than the theory, and it settles rather than narrows.
  20260729-222131.
- `public-surface-pass-before-review` (x1, -> work skill): when a diff adds
  public API, read the new SURFACE once as API before handing it over - is every
  new `pub` item exported the way the repo requires (nova-protocol: through the
  module's `prelude`), and does clippy have an opinion about its signature
  (`len() > 0` where the type has `is_empty`)? Both of a round's findings were
  that pass never happening: the API grew incrementally while chasing behaviour,
  and nothing forces a look at it as a whole. 20260730-122940.
- `commit-review-retro-before-land` (x2, -> flow/review skills): commit
  REVIEW.md (and any retro/decision file) on the feature branch and confirm the
  worktree `git status` is clean BEFORE `sprout land` - the squash only takes
  committed state and `sprout land` removes the worktree, so an uncommitted
  review file is dropped AND lost with the worktree (an out-of-context
  reviewer that WRITES REVIEW.md is the classic trigger - commit it before
  landing). 20260718-231601, 20260722-092427.
- `classify-at-the-verifier-when-the-edit-site-cant` (x1): when a bulk content
  edit needs a per-item property that is not visible at the edit site, either
  make a robust superset edit whose extra items are harmless or hand the
  classification to a verifier; do not eyeball it per item. 20260722-092320.
- `log-ui-shape-before-plan` (x1): for log-style UI, decide whether the reader
  wants one chronological stream, grouped categories, or separate panes before
  mapping existing data sources into sections. 20260724-102309.
- `live-command-tests-over-snapshot-tests` (x1): command output that promises
  current ECS state needs at least one App-driven submit test that mutates the
  source resource/component and proves the next command reflects it; formatting
  a prebuilt snapshot only proves the renderer. 20260726-115330.
- `display-threshold-switches-on-rounded-value` (x1): a value-display helper
  with a unit/format switch (m -> km) must compare the ROUNDED number the
  player sees, not the raw pre-format value, or rounding prints the very
  string the switch avoids (`metres < 1000` let 999.6 m round to a four-digit
  `1000 m`; `metres.round() < 1000` fixes it). Add a boundary test that the
  switch never emits the other branch's string. 20260728-175731.
- `test-fixture-distances-computed-not-eyeballed` (x1): fixtures for
  distance-based (Levenshtein/did-you-mean) or prefix-based (longest-match/
  completion) logic must pick the collision/typo case by COMPUTING it against the
  whole command set, not eyeballing a plausible string - two nova_os unit tests
  failed first run because `mep` was distance-2 of `help` and `map` prefixed the
  `map view` help row. The boundary IS the test, so the fixture has to sit on the
  right side of it. 20260727-231546.
- `spatial-fixture-off-the-trivial-point` (x1): a coordinate/transform bug hides
  when every test + example fixture spawns the root at the world origin with
  identity rotation - the one pose where world == local == identity. Put the root
  off-origin (and rotated) so a world-vs-local frame mismatch actually surfaces;
  an origin fixture proves almost nothing about placement (a ship-app blip
  projection used world space but the scene was local, invisible until an
  off-origin fixture). 20260726-115339.
- `offorigin-fixture-compose-world-not-fake-it` (x1): when an off-origin fixture
  spawns a root off-origin AND rotated to distinguish local from world, set each
  child's `GlobalTransform` to the genuinely composed `root_world * local`, NOT
  `local + constant_offset` - a hand-faked additive offset ignores the root
  rotation, so the "world" pose is just local shifted and the rotation is dead
  dressing that misleads a reader into thinking the frames are properly composed.
  Kin of [[spatial-fixture-off-the-trivial-point]]. 20260728-125510.
- `pin-each-caller-not-just-shared-core` (x4 -> Pending promotions, work skill): a
  shared helper/renderer covered by ONE caller (or a synthetic spec) does not cover
  the OTHER callers' wiring - target resolution, message plumbing, side effects, or
  a data field set at N registration sites; pin each entry point end-to-end in the
  SAME pass, not after a reviewer points at the missing half. Enumerate the CALL
  sites (grep the spawn/dispatch fn), not the helper functions - a fourth probe
  child built its env inline at the call site and shipped unsandboxed under a
  builders-only sweep, docs already claiming "every". Kin of
  [[advertised-is-not-wired]]. 20260726-115339, 20260728-115430, 20260728-184502, 20260729-015406.
- `computed-expectations-need-a-nonempty-guard` (x1): a test whose expectation set
  is COMPUTED (host env, config, a filtered list) silently asserts NOTHING when the
  computation yields empty - guard with `assert!(!expected.is_empty(), <why>)` so the
  degenerate case fails loudly instead of passing green. 20260729-015406.
- `deleting-a-test-salvage-live-assertions` (x3 -> Pending promotions, work
  skill): a test deleted with the module it
  exercised may carry the only assertion pinning a still-live edge - read each
  assertion and re-home the survivors, a test is a bag of assertions, not one unit
  tied to a symbol. Twice now: a bar/pips test also pinned "unknown health reads
  nominal"; deleting `objective_hint.rs` dropped the only pop/breath pins for
  behaviour the REPLACEMENT widget inherited, and a dead pop shipped through the
  hole. Third time it EARNED its keep rather than being learned the hard way:
  deleting the objective reveal card took four tests, two of which were the only
  pin on still-live stack behaviour. Kin of
  [[pin-each-caller-not-just-shared-core]].
  20260728-125514, 20260729-163816, 20260729-211200.
- `rebuilt-view-writes-go-to-state-not-the-entity` (x2): when a widget REBUILDS
  its nodes every frame from a resource (the comms stack, the objective stack),
  any system reaching in from outside must write the STATE, never the rebuilt
  entity - a `pop()` written onto a chip is overwritten before it eases and the
  animation silently never plays, while docs, tests and CHANGELOG all claim it
  does. Decide this at the moment the rebuild model is chosen and say so at the
  top of the module. Corollary, confirmed by a widget that rebuilds its children
  on a skin flip: the surviving PARENT is the correct home for state the rebuild
  must not lose (a slider track's remembered fraction), and reading it back off a
  sibling component fails for the case that has none. 20260729-163816,
  20260729-211155.
- `identify-the-subject-in-the-event` (x1): an event that triggers work on a
  specific subject must CARRY that subject's id; "find the one that must have
  meant it" (the oldest waiting, the only one pending) is a guess that survives
  exactly until two are in flight or the subject dies early - an anonymous
  objective-card tuck handed the NEXT objective over a second before its own
  card landed, decided by a schedule tie-break. 20260729-163816.
- `test-the-wiring-system-not-just-its-pure-helpers` (x2): a per-frame system that
  maps pure helpers into the live UI tree AND caches state other code reads (e.g.
  `update_ship_panel` writing panel text + caching the button-enabled flags the
  observers read) is a seam that can silently no-op; the helper unit tests pass
  with it reverted. Run the SYSTEM in a live-tree test - "would this pass if the
  system were a no-op?" A Step that NAMES a system/live-tree test is not satisfied
  by a nearby pure-helper test. Kin of [[pin-each-caller-not-just-shared-core]] and
  [[advertised-is-not-wired]]. Run it as a ROUTINE gate, not a suspicion: applying
  it to all 7 tests of one task caught three that proved nothing (two false pins
  and a tautology). 20260728-115430, 20260729-211155.
- `justify-a-deviation-with-a-test-not-a-paragraph` (x1): when the
  implementation departs from the plan, the deviation needs its own fail-first
  pin, not a note in TASK.md explaining why it is right - writing the
  justification IS the signal that it is untested. A dock change swapped a
  forced `Dim` for "the chip's true state", survived full-suite mutation
  untouched, and in the single case where the two differed was the WORSE choice
  (an off-screen chip left marked `Hot` for `grow_hot_chips` to hold grown); the
  reviewer found it by mutation in one pass. Same question as
  [[test-the-wiring-system-not-just-its-pure-helpers]] ("would this pass if I
  reverted it?"), asked of a design choice rather than a system. 20260730-122843.
- `review-current-base-before-ooc` (x1): before spawning out-of-context review,
  compare the branch against CURRENT local default and merge it if the diff
  includes inherited base noise - stale comparisons waste review on unrelated
  files. 20260724-134350.
- `review-agent-needs-tatr-verdict-format` (x2): whoever WRITES REVIEW.md must
  use the exact line format tatr checks (`- VERDICT: APPROVE|REQUEST_CHANGES`,
  as a LIST item) - a bare `VERDICT: APPROVE`, or one qualified with the round
  (`VERDICT (round 2, reviewer): APPROVE`), fails `closed-not-approved` and
  costs a fix-up round at the compound gate. Put the attribution in the
  section heading, never on the verdict line. 20260724-074940, 20260729-222131.
- `the-reviewed-party-does-not-write-the-verdict` (x1): record a review verdict
  only when it comes back FROM the reviewer, attributed to its round - never
  pre-write the expected one. Pre-writing `VERDICT: APPROVE` before the
  re-review ran happened to match, which is what makes it a habit rather than
  an error: in the record it is indistinguishable from a real gate.
  20260729-222131.
- `manual-acceptance-is-not-an-implementation-checkbox` (x1): split human visual
  acceptance out of implementation checklists during planning - keep it as a
  `manual:` DoD item so work close-out does not pretend to self-verify a human
  check. 20260725-163835.
- `rename-id-sweep-in-file` (x1): after renaming a content id, grep the WHOLE
  file for the old id - lint validates spawn/prototype/filter refs but not AI
  orbit/patrol targets. 20260716-215513.
- `git-mv-leaves-empty-parent` (x1): `git mv` out of a dir leaves the emptied
  dir on disk and filesystem-walking tools trip on it; `rm -rf` the old dir
  after a relocation. 20260716-215513.
- `tatr-new-then-sprout-strands-the-task-file` (x4, PROMOTED 2026-07-19 ->
  tatr + flow skills): sprout first, then create OR edit the task file inside
  the worktree; a stub `tatr new`d - or an existing task's Flow State /
  PLANNED markers written at the gate - in the main checkout is orphaned by
  the next sprout (branch cut from committed master). Carry-and-clean the
  main-checkout edit onto the branch. 20260717-101414, 20260718-181305, 20260726-230237, 20260728-175731.
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
- `edit-the-builder-not-the-generated-ron` (x3, PROMOTED 2026-07-19 -> repo
  AGENTS.md, but RECURRED - prose did not hold): base `.content.ron` are
  generated - edit the builder then `cargo run -p nova_assets -- gen` and commit
  the generated RON in the SAME change; a direct RON edit can look correct until
  the next generation wipes it. 20260718-175502, 20260722-142341,
  20260722-092320.
- `local-merge-skips-the-guarding-ci` (x1): an ff-merge to master done LOCALLY
  runs no CI, so any check that lives only in CI - or in a `tests/` integration
  guard that `cargo test --lib` skips (content_ron_parity would have caught the
  stale RON) - never fires. When landing locally without a push, run the FULL
  affected suite (`--test <name>` for integration guards, not just `--lib`) or
  push and let CI gate BEFORE merging. 20260722-142341.
- `build-time-move-weigh-generator-deps` (x1): before scoping a "move X to
  build-time" task, check whether the generator drags in a heavy dep (bevy via
  `Reflect` derives) - a `build.rs` then needs it as a build-dependency and
  DUPLICATE-compiles it in the build graph, usually killing the cost/benefit.
  20260719-092952 (declined on exactly this).
- `removal-sweep-includes-dev-deps-and-test-drivers` (x1): before recording a
  crate/symbol can be removed, grep its NAME across the whole workspace -
  `Cargo.toml` deps AND dev-deps and `tests/` - not just the deploy/production
  path; a dev-dependency test driver (portal_install.rs -> nova_portal_gen) is a
  real consumer that blocks removal. 20260718-152247.
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
- `doc-lint-autofix-misreads-prose` (x1): clippy --fix on doc lints
  silences the marker misparse instead of fixing it - rewrap the prose so
  no line starts with `-`/`+`/`>=`; re-read every --fix doc hunk.
  20260719-001600.
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
- `verify-first-plan-steps` (x10, PROMOTED 2026-07-11 -> plan skill): plan
  steps stating a mechanism/formula/API cite the verifying file - including
  shipped CONTENT data; embedding the exact citation (file:line) in the step
  makes implementation AND review mechanical. 20260717-003613,
  20260719-112231. 20260721-160906.

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
- `inherited-cli-string-drifts` (x2): a CLI invocation copied from a prior
  task's DoD/Steps can be stale against the current flags - run it (or `--help`)
  once before trusting it. Both ch3 tasks this cycle inherited `content lint
  <path>` when the bin now takes `--target <path>`. 20260723-182850, 20260723-182855.
- `relocation-leaves-ignored-siblings` (x1, PROMOTED 2026-07-19 -> sprout
  skill): gitignored files exist only in the main checkout; clean them up
  after landing a stop-shipping-dir move. 20260714-154958.
- `match-ci-feature-set-in-targeted-tests` (x3 -> Pending promotions, work
  skill): match the feature set to the code under test - and a workspace
  `cargo check --all-targets` does NOT enable a crate's self dev-dep `serde`
  feature, so it SILENTLY SKIPS serde-gated targets (a false green); run
  per-crate `cargo test -p <crate> --no-run` on touched crates before trusting
  it. 20260718-004834, 20260718-102022, 20260724-193830.
- `landing-no-cd` (x4, PROMOTED 2026-07-11 -> flow skill): squash-merge from
  the main checkout, own command, no cd, `pwd` first - and never CHAIN the
  land onto a sync command that cd'd into the worktree (the squash merges
  the branch into itself as a silent no-op). 20260709-160753,
  20260719-174541.
- `resume-check-if-already-landed` (x1): when resuming a task with a leftover
  sprout/branch, `git diff master <branch> -- <the-real-file>` FIRST - an
  empty diff means the fix already landed (via PR) and the branch is just
  stale; close and clean up, do not re-do or re-review. 20260718-235837.
- `worktree-cwd-resets-verify-absolute-path` (x1): the Bash cwd resets to the
  MAIN checkout each call, so a bare `grep`/`rg` reads the unmodified tree, not
  the sprout worktree - prefix `cd <worktree> &&` or pass absolute worktree
  paths, or a grep silently "verifies" stale code (caught by a grep/Read
  line-number mismatch). 20260728-160001.
- `epic-parent-list-lags-child-close` (x1): closing a child task ticks its own
  STATUS but nothing walks back to tick its box in the parent epic's Child Tasks
  list - tick the parent list in the same close-out so the container never shows
  landed work as still open. 20260725-104330.
- `record-the-exact-rig` (x3, PROMOTED 2026-07-13 -> work skill): evidence
  notes record the rig (systems, command path, components) or they mislead.
  20260709-125640.
- `probe-surfaces-adjacent-issues` (x2): run de-risk probes for real; they
  pay beyond their stated question (the timeline recorder's first armed run
  exposed an unknown spawn-overlap onenter). 20260710-104421, 20260719-112238.
- `probe-content-not-just-code` (x1): "data-only" content changes carry
  BEHAVIOURAL bugs, so probe them too - a scenario pacing pass skipped probe as
  "data-only, no perf surface" and shipped OnStart objective gates that read an
  undefined `scenario_elapsed`, so the opening objectives never posted (caught
  only when a later task's probe hit the same scenario). Probe is a behaviour
  check, not just a perf one. 20260722-114541, 20260722-092421.
- `review-rig-can-false-green` (x2): a review that BUILDS a bespoke rig to
  clear a flagged risk can false-GREEN when the rig diverges from the real load
  path - task 1's reviewer "verified" the OnStart clock read safe with a
  synthetic scenario that seeded the clock the loader does not; the real loader
  fires OnStart before the first tick. Verify a risk against the PRODUCTION path
  (or a probe of it), not a hand-built stand-in; treat a bespoke-rig green as
  inconclusive; and a rig proving a DIVERGENCE (two endings, a branch) must pin
  the STRUCTURAL fact (one path spawns the boss, the other does not), not just
  banner text a re-convergence would still pass. 20260722-114541, 20260722-214110.
- `upstream-api-gap-fix-beats-workaround` (positive, x1): when the blocking
  gap is a missing accessor in a dependency the USER owns, surface the fork -
  the small upstream fix + tag + pin bump beat both in-repo workarounds
  (per-site wrapper sweep, deferral) on every axis. 20260719-112238.
- `headless-shot-after-load` (x1): BCS_SHOT captures black pre-load; inject
  `Screenshot::primary_window` from the autopilot at a settled moment.
  20260710-104421.
- `registered-system-for-change-detection` (x2): `run_system_once` builds a
  fresh system per call (Changed/Added fire on everything, cursors reset);
  register once and reuse the SystemId. 20260713-082330.
- `run-system-once-always-changed` (x1): same trap on `Res::is_changed`; gate
  behavior needs an App-driven test across real frames. 20260712-093831.
- `resmut-noop-deref-marks-changed` (x1): a system holding `ResMut<T>` marks `T`
  changed on ANY `&mut` deref, even a no-op (`vec.extend(empty)`); a dependent
  `run_if(resource_changed::<T>)` then thrashes - gate the mutation behind an
  actual-change check (`if !fresh.is_empty()`). 20260726-214708.
- `decision-status-enum` (x2 -> template candidate: seed DECISION.md with the
  `- DATE/- STATUS/- TASK/- TAGS` frontmatter): a DECISION.md STATUS must be a
  `- ` bullet with a closed-enum value - `tatr check --ledger` accepts only
  `ACCEPTED` or `SUPERSEDED by <ref>`; `PROPOSED`/`DRAFT` or an un-bulleted prose
  line fail `bad-decision-status`. Copy an existing passing DECISION.md's header
  block rather than authoring the STATUS line freehand. 20260727-112529, 20260728-175726.
- `resource-changed-fires-on-init-frame` (x1): a `run_if(resource_changed::<T>)`
  system that ACTS on the DEFAULT/empty `T` (teardown-on-empty, reset-on-clear)
  fires on the resource's very first frame - `init_resource` marks it changed - so
  it runs before any real change; guard it against the default state, or in tests
  drive `T` to a non-default value first (an empty-init frame silently despawned a
  reveal before its assertions). 20260721-211520.
- `nextstate-input-test-needs-clear-and-two-updates` (x2): a headless test that
  presses an input to drive a `NextState` transition needs TWO updates (the set
  applies next frame) AND a `clear()` of the `just_pressed` edge between them (no
  InputPlugin clears it, so a stale edge re-fires the toggle) - copy the sibling
  press-helper verbatim, do not hand-roll the cadence. Bit `press_tab` then
  `pad_toggles_drawer_state` in the same drawer family. 20260724-102304, 20260724-134312.
- `context-key-handled-in-one-owner` (x1): a context-sensitive key (Escape exits
  an app in app mode but closes the drawer at the prompt) must be interpreted in
  ONE system branched on state, never two readers cooperating over the same
  `ButtonInput`/event edge - a single read cannot race itself, whereas two systems
  reading one Escape can both fire on one press (exit app AND close drawer). Same
  family as the Tab-split. 20260726-115334.
- `route-input-only-when-continuously-active` (x1): a system that owns input while
  a mode is active will process the very keystroke that ENTERED the mode the same
  frame (the Enter that launched an app bleeds into that app). Gate on "was this
  same mode/app live LAST frame too" (a `Local`), dropping the event buffer on
  every transition frame - and make the test fixture sensitive to the routed key
  (an Enter-exit app), or the bleed test proves nothing. 20260726-115334.
- `observer-over-spawn-site` (x1): attach derived components via an
  `On<Add, Marker>` observer, not by hunting spawn sites. 20260712-203345.
- `guard-timing-matches-observer-not-set` (x1): a Bevy `.after(SomeSet)` does
  not order past state written by observers fired from that set's commands; make
  predicates safe under one-frame-late observer output and document that
  guarantee instead of assuming same-frame visibility. 20260722-092320.
- `gate-producer-and-its-consumers` (x1): a flag that skips PRODUCING an
  entity sweeps its CONSUMERS too - each must tolerate the skip (early
  return, not error spam). 20260525-133013.
- `defer-opens-a-consumer-race` (x2): deferring a state change (objective/
  marker) behind a timer while the world it refers to is already interactable
  opens a race - every consumer that can fire in the gap (OnStart-spawned
  pickups, edge-triggered area exits) must be guarded on the deferral latch, or
  the referenced entity spawned at the transition, or a fast actor beats it
  (shakedown's crate pickups + coast-ring exit). REMEDY when deferring an Outcome
  behind a clock gate: keep the terminal/act LATCH synchronous with the trigger
  detection (bump `act` in the same handler, defer only the player-facing
  overlay) so the Defeat/consumer window closes at once - the Ledger ch2 win,
  ch3 breather and ch4 burn all ride this. 20260722-142341, 20260722-214058, 20260722-214110.
- `messagereader-needs-resource-guard-in-tests` (x2): minimal-app rigs omit
  `Messages<T>`; gate on `resource_exists` or init the resource in BOTH
  writing and consuming plugins. 20260714-174126, 20260716-193949.
- `worktree-shares-main-target` (x1, CORRECTED; PROMOTED 2026-07-19 -> sprout
  skill; sccache fast-path 2026-07-21 -> 20260721-000229): never share
  CARGO_TARGET_DIR with the main checkout (artifacts clobber - cargo keys
  fingerprints on name+version, not source path, so a shared dir aliases two
  checkouts). But you no longer eat a full cold build: the devshell now wires
  `sccache` as RUSTC_WRAPPER (content-hash cache, each worktree keeps its OWN
  target/), so a fresh worktree is a warm build - measured ~38s vs ~6m45s cold,
  100% hit rate. sccache is the SAFE way to share compilation; the never-share-
  target-dir rule still stands. 20260709-131502, 20260721-000229.
- `commit-before-sabotage` (x2, PROMOTED 2026-07-11 -> work skill): commit
  the fix before A/B sabotage; anchor splices on unique strings.
  20260710-231930.
- `production-faithful-rigs` (x9, PROMOTED 2026-07-11 -> work skill): rigs
  mirror production - scheduling, hierarchy, shipped configuration,
  required-component DEFAULTS; extract ONE shared registration helper both
  plugin and rigs call; when a rig cannot run a shipped action for a missing
  resource, give the rig the resource PRODUCTION has (an AssetPlugin) rather than
  softening the engine to tolerate its absence. 20260711-103527, 20260717-163042, 20260722-214115.
- `seed-helper-drifts-from-source` (x1): a hand-maintained "seed/mirror the
  whole <source> block" test helper rots SILENTLY when the source grows a field
  - final_tally's seed_live_claim fell behind the OnStart VariableSet block when
  a pacing pass added `*_posted`/`*_gate` vars, so gated handlers read `None`
  and two tests failed with no content bug. Pin the helper's key set against the
  source (or generate it) so the drift fails loudly instead of as a mystery
  test failure. A sharper, actionable form of `production-faithful-rigs`. 20260723-115419.
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
- `generated-links-need-real-targets` (x2): manifest-rendered AND authored doc
  links gate on the target existing or they 404 - check every link target
  resolves on disk (a README banner link went stale when the file moved dirs).
  20260713-225324, 20260718-152205.
- `enumerate-bins-via-cargo-metadata` (x1): to document or audit "every
  binary/target", enumerate with `cargo metadata --no-deps` (or find
  `src/bin/*.rs` + `src/main.rs`), never by grepping `[[bin]]` stanzas -
  default targets carry no stanza (a grep pass reported 2 of 6 bins).
  20260718-152205.
- `ci-skips-client-render` (x2): build-only CI proves the bundle compiles;
  DOM logic needs a runtime check. When the web project has NO browser test
  runner, factor the risky client logic into a pure exported fn and run it
  against REAL API JSON, stub `document`/`fetch` for the apply loop, and
  chromium-screenshot the served build for layout. 20260713-225324,
  20260724-074940.
- `absolute-child-needs-a-positioned-ancestor` (x1): an absolutely-positioned
  child (reticle bracket corners) whose parent is NOT positioned resolves its
  containing block against the viewport, so it flies to a screen corner; a
  whole-scene screenshot can MASK it when a sibling (the DST/CLS chip) fills the
  expected spot, so a composed-eyeball passed and the owner caught it. Give every
  such container an explicit `position` and eyeball the SUB-element, not just the
  scene. Sharpens [[render-output-eyeball]]. 20260728-175726.
- `degrade-paths-need-a-forced-failure` (x1): a plan-claimed fallback ("skips
  gracefully when blocked") is untested until that failure is FORCED once -
  the samply perms case died under set -e and a user found it.
  20260719-112253.
- `roundtrip-hides-shared-bug` (x1): a round-trip test on a self-authored
  forward pass proves symmetry, not correctness; re-derive the reverse
  against the spec. 20260715-004216.
- `one-cargo-test-filter` (x6, PROMOTED 2026-07-13 -> docs/development.md):
  one filter and one `-p` per cargo test invocation. 20260713-082324,
  20260716-162701, 20260726-115320.
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
- `asset-format-change-greps-extension-and-loader` (x1): changing an asset's
  FORMAT (not just its path - e.g. `.ttc` -> `.ttf`) must grep the FORMAT
  EXTENSION and the custom LOADER TYPES, not only the path const. A bespoke
  `.ttc` `FontLoader` + its `nova_meta_gen` sidecar registration + a test were
  invisible to a path-only sweep and would have re-broken the web build
  (`AssetMetaCheck::Always` needs a `.meta`; wrong/absent loader = no sidecar =
  invisible glyphs, the exact bug 20260727-172205 fixed). Switching to a
  built-in-loader format retires the custom loader everywhere it was
  registered. 20260729-000956.
- `verify-plan-named-api-visibility` (x1): before implementing a plan step that
  names a specific external API (`SoundBank::from_handles`), verify that API's
  REAL visibility/availability first - a private method behind a tagged git dep
  changes the whole approach (route around vs force a cross-repo release) and is
  a load-bearing DECISION.md fork, not a detail. 20260729-000956.
- `doc-sweep-grep-plus-reread` (x2): a reference sweep is grep PLUS a full
  re-read of each touched section - grep finds names, not meaning; two
  stale-in-meaning paragraphs survived a clean grep. Verify multi-edit
  anchors (position + uniqueness) with a probe pass before the mutating
  script. Applied deliberately in the examples reorg: the re-read caught
  four meaning-level spots ("four blocks"/"all eighteen" counts, a
  numbered-slot how-to, CHANGELOG Unreleased) a clean grep sailed past.
  20260719-174603, 20260719-193728.
- `fixture-adds-verify-tracked` (x1): `git add -A` says nothing about what
  the ignore rules dropped - a global *.log swallowed a test fixture and the
  squash landed without it; after staging fixtures, `git ls-files` the
  fixture dir and count, and carve ignore exceptions for fixture trees.
  20260719-112304.
- `stage-lock-with-manifest` (x1): a Cargo.toml dep change stages Cargo.lock
  too; explicit-path adds drop it silently. 20260714-113408.
- `pin-the-fix-at-its-boundary` (x4, PROMOTED 2026-07-19 -> review skill):
  guard a fix at its OWN boundary with a unit test that fails under the bug;
  a refactored invariant re-pins on the new mechanism; grep a changed
  predicate's table-test callers first. 20260714-113411, 20260716-214919.
- `shared-id-space-shared-overlay` (x1): containers sharing an id space route
  through ONE overlay helper so overlay semantics cannot diverge.
  20260714-134119.
- `one-writer-per-worktree` (x1): never edit a sprout worktree that has a LIVE
  background agent working in it - one writer per worktree, or your edits and
  its churn clobber each other (a guard edit was overwritten, stashed and lost).
  A vague agent notification ("waiting...", "no action needed", no real report)
  means STILL RUNNING or confused, not done; wait for a complete report before
  touching its tree. 20260722-214115.
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
- `audit-state-gates-on-new-entry-path` (x4, PROMOTED 2026-07-19 -> plan
  skill): a new route into a state greps run_if/in_state + OnEnter/OnExit +
  DespawnOnExit AND the `== <state>`/`is_frozen`-style runtime guards, across the
  WHOLE workspace not just the crate under edit, and writes the what-newly-runs
  list - a "while-frozen" behavior can be wired by SCHEDULE (audio loop-freeze on
  `OnEnter(Paused)` only was invisible to a guard-only grep and let the drawer
  roar). 20260711-180426, 20260716-214919, 20260724-102304.
- `bound-scheduling-both-sides` (x1): a system between producer and reader
  needs both .after and .before. 20260711-180501.
- `set-gates-miss-observers` (x2): gating a SystemSet does not touch
  observers; enumerate systems + observers + hooks - a new PauseStates variant
  had to widen ~14 observer self-guards the `in_state(Unpaused)` set-gate never
  covered. 20260711-185156, 20260724-102304.
- `would-it-fail-without-it` (x8, PROMOTED 2026-07-13 -> work + review
  skills): a verification that cannot fail with the mechanism deleted proves
  nothing; a sabotage that will not go red refutes the assumed mechanism or
  the test's shape. Sharpened 20260729-163816: a REGRESSION test written after
  the fix is the likeliest to pass under the bug it names - one tucked two
  cards in posting order, which the positional bug also got right. Mutate
  before believing it. Sharpened 20260730-123039: a LATER fix can silently
  disarm an earlier test - a bigger hit target absorbed the mis-mapping its own
  corner-click test existed to catch - so sabotage each half of a two-part fix
  SEPARATELY, not the change as a whole. 20260711-180426, 20260717-163033,
  20260729-163816, 20260730-123039.
- `required-component-in-shared-query` (x2): a required fetch narrows an
  existing query's membership; fetch `Option<&T>` or use a separate query.
  20260712-143832.
- `spike-open-question-pays-off` (positive, x1): a spike naming a risky
  unknown lets the implementer resolve it before wiring. 20260712-143832.
- `verify-engine-guarantees-in-source` (x9, PROMOTED 2026-07-19 -> plan
  skill): read the dependency's source or probe before designing around its
  ordering/observer/failure/API behavior - doc comments (upstream AND ours)
  are folklore, and SPIKE docs stating a dependency capability cite the
  verifying grep too (a spike claimed a Bevy per-system diagnostic that does
  not exist); the bcs `Tween` advances on `Res<Time>` (=`Time<Virtual>`), so a
  pause-overlay slide had to use `Time<Real>`; and reading bcs `rebuild_lines`
  (a `Single<..panel>` that skips when absent) at plan time made a compact-panel
  removal a clean deletion instead of a resource-lifetime scramble - all checked
  at design time, not after. 20260717-133332, 20260719-112011, 20260724-102304, 20260724-134312, 20260723-233446.
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
- `reuse-known-good-stack` (x9, PROMOTE 2026-07-31 -> 20260731-102037): a
  POSITIVE lesson - scaffold new
  work - and TEST RIGS especially - by copying the nearest passing in-repo
  reference verbatim, THEN mutate; reconstructing from a signature cost build
  cycles repeatedly (a hand-rolled pad-toggle test re-hit the exact `press_tab`
  clear+two-update gotcha - see nextstate-input-test-needs-clear-and-two-updates;
  manual `ButtonInput` needs `clear()`; the flyable-ship rig omits
  `FlightIntent`; a clock-driven reveal needed the manual-duration rig the sibling
  tests already had). Applies to PRODUCTION and FIXES too: copied
  `screen_indicator`'s node placement (coord rabbit hole avoided) and nova_menu's
  `GlobalZIndex` modal tier + overlay-z test shape (a UI-stacking fix made
  mechanical). The nearest reference may live in the DEPENDENCY, not the repo:
  a hand-derived bevy_ui layout rig cost three anonymous system-param panics
  that bevy_ui's own `setup_ui_test_app` (`layout/mod.rs`) would have avoided.
  20260712-093048, 20260711-180511, 20260724-102304, 20260721-211520, 20260724-121541, 20260730-122909, 20260730-122940.
- `bulk-replace-edits-more-than-you-aimed-at` (x1): a scripted `str.replace`
  (or `sed`) for a SINGLE-site source edit is replace-ALL by default - one meant
  for a new capture beat also rewrote the scenario constant behind a published
  screenshot. Use the Edit tool (which fails loudly on an ambiguous match) or
  pass `count=1`, and read the resulting `git diff`, not just the artifact the
  edit was supposed to produce. 20260730-122909.
- `one-shot-guard-separate-from-its-state` (x1): an `Option` used as BOTH the
  spawned-state handle and the "already ran" guard re-fires forever once the
  teardown `take()`s it - a capture beat respawned its subjects every frame,
  flooding the log and starving the rest of the script. Give the one-shot its
  own `bool`. 20260730-122909.
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
- `document-the-async-failure-path` (x1): concurrent-flow notes trace the
  async failure path and state the real atomicity boundary. 20260715-142931.
- `sibling-change-leaves-stale-fixture` (x3, PROMOTED 2026-07-19 -> work
  skill): grep for fixture tests asserting on data you change; pin durable
  intents, not frozen literals. 20260715-142931, 20260717-151214.
- `benchmark-gates-both-ways` (x1, positive): a measure-first gate justifies
  deferring as legitimately as doing. 20260525-133014.
- `verify-bevy-api-at-callsite` (x1): copy an existing in-repo callsite for
  unfamiliar Bevy API; 0.x churns. 20260712-131348.
- `spike-reuse-over-new-infra` (x1, positive): check whether an existing
  substrate covers the need before building infrastructure. 20260712-131348.
- `trace-vehicle-timeline-first` (x1): pick an evidence rig by its script
  TIMELINE, not scene content. 20260711-183417.
- `derived-not-hardcoded-shared-resources` (x1): displays, ports and temp
  names two concurrent runs could contend on are derived (pid/unique) at
  FIRST writing and unit-tested with the env assembly - a hardcoded Xvfb :97
  reached review before the collision was seen. 20260719-112317.
- `deadline-scales-with-the-work` (x1): a hang-detector timeout must scale with
  the WORK requested, not be a flat constant - a flat deadline either
  false-fails slow-but-progressing work (perf_baseline's 900-frame capture blew
  a flat 120s) or is too loose to catch fast hangs; size it from the work at a
  pessimistic floor, and keep the operator override. 20260720-115935.
- `pkill-pattern-matches-own-shell` (x2, PROMOTED 2026-07-19 -> ~/AGENTS.md):
  `pkill -f` matches your own command line and look-alike processes; kill
  recorded PIDs. 20260716-180352, 20260717-004302.
- `silent-tool-missing-in-pipeline` (x1): a missing launcher dies with 127
  that a pipeline swallows; `which` host tools first. 20260711-183417.
- `no-source-edits-during-inflight-builds` (x1): a tree edited mid-build
  yields an indeterminate evidence binary; quiesce for A/B runs. 20260711-183417.
- `dead-code-hides-under-cfg-test-reader` (x1, -> work skill verify): a field/fn
  read ONLY by `cfg(test)` code looks live under `cargo test`, so the `dead_code`
  lint never fires there - a refactor that moves where a field is read can leave a
  production-dead field that only a plain `cargo check` (non-test cfg) surfaces.
  Run `cargo check` (or `--all-targets` incl. the non-test build), not just
  `cargo test`, before declaring done. 20260728-124443 (from 20260728-115435).
- `drop-the-field-the-change-orphans` (x1): when a display/format change stops
  reading a struct field (here `ShipSectionStatus.name` after switching a
  `ship view` row to the code label + sorting by code), the field AND its upstream
  ECS query fetch (`Option<&Name>`) go dead in the SAME change - remove both in
  that pass rather than leaving a `dead_code` warning or a wasted query column. A
  plan step that says "keep the old field for now" deserves a "does anything still
  read it after this change?" check. Kin of [[dead-code-hides-under-cfg-test-reader]].
  20260728-152856.
- `gpu-example-local-skip` (x2): heavy render examples are ~100x too slow
  under lavapipe AND OOM its software render device on combat scenes (identical
  wgpu OutOfMemory at the same frame across scenarios, with system RAM free);
  one short smoke attempt, then headless tests + CI. 20260717-004302,
  20260722-163718.
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
- `assert-the-new-vocabulary-is-consumed` (x1): for a port/restyle, a test that
  only checks the new tokens are DECLARED with the right values passes on exactly
  the outcome the task rejected the cheap option for (a site that took the new
  colours but kept the flat structure). Assert the main surfaces actually READ
  the new vocabulary, and that every read resolves - that pair also catches a
  rename that stopped halfway. 20260731-143918.
- `parity-test-must-cross-link` (x1): a sync test derives the expected value
  from one side and asserts on the OTHER, never two hardcoded literals.
  20260711-180511.
- `ask-user-facing-control-style` (x1): the interaction style of a user
  control is a genuine preference fork - ask instead of deliberating at
  length. 20260711-180511.
- `mirror-sibling-resolve-site` (x1): a new resource-resolving content field
  mirrors the sibling's resolve SITE, not just its declaration - the site
  decides which systems gain the dependency. 20260717-002228.
- `piped-cargo-masks-exit-code` (x7, PROMOTED 2026-07-19 -> ~/AGENTS.md +
  work skill): never end cargo with tail/grep/echo - the harness reads the
  last exit; write output to a file and grep the FILE. Re-violated in the
  examples reorg (xvfb-run-not-found read as exit 0 through `| tail`); the
  tell that saved it was reading the test COUNTS ("0 passed; 1 filtered
  out"), not the exit. 20260717-002228, 20260718-122932, 20260719-193728.
- `half-ticked-compound-steps` (x4, PROMOTED 2026-07-19 -> work skill): tick
  a step only when every clause is done, or split/amend it in the same edit;
  never bulk-tick with sed - tick each step individually re-reading its
  clauses against the diff. 20260718-122912, 20260719-114931.
- `upstream-dev-via-patch-not-premature-push` (x1): when a change spans
  bcs + nova, develop nova against the LOCAL bcs branch - never push
  unfinished upstream work just to make the pin resolve. What actually
  works: temporary PATH deps in EVERY dependent manifest (nova has FIVE:
  events, gameplay, scenario, assets, debug - one keeps a features
  clause; missing one splits the graph into two bcs instances and traits
  stop matching). A root `[patch]` looks righter but cargo rejects a
  version-BUMPED patch of a git-tag dep ("patch was not used"), and the
  unpatched pin must also stay resolvable. Restore all five lines +
  bump the tag TOGETHER in the landing commit, after the upstream
  push+tag. 20260720-000609.
- `redundant-param-enables-impossible-test` (x1): when a new fn needs a value an
  already-computed local provides, pass that local - don't re-derive or
  re-parameterize it. A redundant param widens the input space to unreachable
  combos a test can then "pass" against (a separate `brandPath` equal to `root`
  let the wiring test assert an armed `root=""`/`current="/home"` that cannot
  occur); collapsing to one source of truth is what makes the test express only
  reachable states. 20260726-210348.
- `web-tests-need-node-from-flake` (x2, reference): the agent shell has no node
  on PATH - use the flake's store bin (version floats, glob it:
  `/nix/store/*-nodejs-*/bin` + the sibling `*-nodejs-*-npm/bin`) and symlink the
  main checkout's `web/node_modules` into the sprout worktree to run `npm`,
  removing it before commit (a bare `node_modules` symlink dodges the
  `node_modules/` gitignore; it is NOT itself gitignored, so never `git add` it).
  `npm test` already compiles first (`tsc --module commonjs ... --outDir
  .test-out && node .test-out/...`); also rm `.test-out`/`dist` before commit.
  20260726-210348, 20260728-185730.
- `compare-crops-at-one-zoom` (x2): a before/after VISUAL comparison must be
  rendered at identical crop and scale or the difference you see is the resize -
  two eyeball crops at 150% and 200% "proved" a glyph-sizing fix that was a
  no-op. Capture the BEFORE while the tree is still unmodified (reverting for it
  later costs a full rebuild), and when the claim is about an ASSET's shape
  MEASURE it (`magick identify`, `-alpha extract -threshold 0 -format '%@'`)
  instead of eyeballing - those numbers are also the independent expectation the
  rig should assert against. 20260728-175742, 20260730-122940.
- `outcome-test-hides-a-dead-redundant-path` (x1): when a value is written by
  BOTH a spawn-time initializer and a per-frame updater, a test asserting the
  final state passes with the initializer completely broken (a dock looked up
  its keycaps by verb name, always missed, and nothing showed it). Assert the
  initializer's own output before the updater runs, or delete the redundancy.
  20260728-175742.
- `sweep-a-rename-where-the-name-is-spoken` (x2): sweep for the CONCEPT, not
  just the symbol - a cross-crate vocabulary survives a crate-local grep
  (`ROW_VERBS` -> `DOCK_VERBS` in nova_scenario's rustdoc), and prose that
  describes the thing without naming it survives a symbol grep entirely (the
  landing page still sold "full chrome to instruments-only" after the HUD
  levels collapsed to On/Cinematic; `web/src/index.html` + `tutorial.html` sell
  features in words). Sibling of [[sweep-content-repo-wide-not-just-assets]].
  20260728-175742, 20260728-175747.
- `ui-node-rebuilt-per-frame-needs-age-seeded-state` (x1): a widget whose nodes
  are DESPAWNED and respawned every frame from a queue (the comms stack) cannot
  carry retained animation state - a freshly spawned tween restarts its ease
  each frame and never leaves rest. Seed the component from the item's AGE (the
  same pure function its alpha already uses), or reconcile instead of rebuild.
  20260728-175747.
- `put-the-playtest-number-in-the-code` (x1): when a deliberate decision yields
  a NUMBER a playtest might complain about (here nested UI emphases composing
  to 1.2544, not the 1.12 the spec named), name it as a constant with a test -
  a future debugger reads the code, not the retro that explains it.
  20260728-175747.
- `stash-ab-before-blaming-your-diff` (x1, positive): on any red met mid-branch,
  `git stash && cargo test <that test>` FIRST - three failures in one task, two
  of them inherited from master (one filed as its own task), zero time spent
  debugging someone else's bug. 20260728-175742.
- `verify-the-cause-not-just-the-symptom-before-writing-a-lesson` (x1): a ledger
  entry is a claim future sessions ACT on without re-deriving, so reproducing the
  symptom is not enough - the mechanism has to be tested, including the
  counter-experiment that would disprove it. A retro here recorded "cargo check
  --all-targets does not compile a lib's `#[cfg(test)]` tests", having watched a
  broken test module stay green; the real cause was package SCOPE (see
  [[bare-all-targets-only-covers-the-root-package]]), and the invented cause
  would have taught future sessions to distrust a gate that works. Caught by
  review, one round before the ledger. 20260729-211155.
- `applied-fix-still-needs-its-own-test` (x1): a fix handed to you by a reviewer,
  another agent or a plan is a hypothesis like any other and gets the same
  red-first test as one you invented - the authority of the source is not
  evidence. Here a well-argued review finding arrived WITH a patch, and the patch
  did not address its own reported failure mode (it read `Option<&SliderValue>`
  to fix a case defined by having no `SliderValue`); only the test written for
  that mode exposed it. 20260729-211155.
- `untestable-is-a-claim-not-a-conclusion` (x1): "this cannot be pinned", once
  written into a doc comment, is trusted and stops being questioned - so before
  writing it, name the specific mechanism that blocks the test and check whether
  that mechanism is CONFIGURABLE. Two failed attempts made "impossible" feel
  earned; the blocker was `auto_insert_apply_deferred`, a `ScheduleBuildSettings`
  field you can switch off, and the test was then a few lines. Kin of
  [[test-the-wiring-system-not-just-its-pure-helpers]]. 20260729-211155.
- `grep-the-old-symbol-after-a-rename` (x1): renaming a function leaves its old
  name in DOC PROSE that still compiles, so nothing fails - four stale
  `sync_slider_meters` references survived a rename, two into review and a fourth
  through three review rounds. Grep the old identifier across `crates/`,
  `examples/`, `tests/` and docs as the LAST step of a rename. The symbol-level
  sibling of [[sweep-a-rename-where-the-name-is-spoken]] (concept-level) and
  [[rename-id-sweep-in-file]] (content ids). 20260729-211155.

- `comment-citing-a-task-is-not-the-wiring` (x1): a doc comment promising
  behavior "once <task-id> lands" is a PROMISE, not evidence - grep for the code
  that would WRITE that state before believing it ships. Three comments claimed
  firing reset the combat-lock decay clock; the cited task closed without the
  wiring, and the gap read as a gameplay bug for weeks until a
  writers-of-this-component grep found the single writer. 20260730-123009.
- `sample-the-regime-the-player-lives-in` (x1): a test for a time-driven visual
  that only samples from t=0 tests the one regime a phase bug hides in - assert
  the SAME output at a realistic session uptime (0 s vs 300 s) and at two frame
  rates. Both cue tests were green while the shipped animation degraded into a
  smooth slide at 60 s and frame-rate aliasing at 300 s. Kin of
  [[sweep-full-scale-before-believing-a-win]]. 20260730-123009.

## Domain lessons (nova-protocol specific)

- `swept-rate-needs-an-integrated-phase` (domain, x1): `phase = t * hz` is only
  valid for a CONSTANT hz; sweep the frequency and the instantaneous rate
  becomes `hz + t * dhz/dt`, so it grows with session uptime. Integrate the
  chirp (`CALM*x + (URGENT-CALM)*x^2/(2*W)`, x measured from the effect's OWN
  clock) and sanity-check `cycles(window)` against the mean-rate product.
  20260730-123009.

- `check-what-points-at-a-thing-before-hiding-it` (domain, x1): when a HUD
  surface gains a rule that REMOVES elements, first find what deliberately
  points AT those elements - the hide and the pointer are mutually exclusive on
  the same element, and that incompatibility is a decision for the owner, not
  something to resolve while coding. Hiding unavailable dock chips collided with
  the scenario spotlight, whose dedicated `EMPHASIS_ALPHA_UNAVAILABLE` band was
  the evidence that "spotlight a verb before it lights up" was a supported case
  and not an accident (shakedown_run emphasizes GOTO while it is unavailable).
  A dead-looking constant is sometimes a feature's only remaining trace.
  20260730-122843.
- `bare-all-targets-only-covers-the-root-package` (x1): this repo's root
  Cargo.toml is a PACKAGE with deliberately no `default-members` (Cargo.toml:274,
  see [[default-members-retargets-bare-cargo-run]]), so a BARE `cargo check
  --all-targets` scopes to the root package and never builds MEMBER crates' test
  targets - it green-lit a nova_menu test module that would not compile.
  `--all-targets` reads `#[cfg(test)]` fine; the variable is scope. Use `cargo
  check --workspace --all-targets` (what CI runs) plus `cargo test -p <crate>
  --lib` for touched crates, and name that form in a DoD. 20260729-211155.
- `guard-every-command-in-the-chain` (x1): silencing ONE call in a Bevy command
  chain does not protect the others - `despawn_related().try_insert(..)` looks
  guarded but only `try_insert` is; `despawn_related` queues through the default
  handler, which the game escalates to a panic under `BCS_AUTOPILOT`. When the
  guard matters, make the whole operation ONE `queue_silenced` entity command
  instead of decorating the last call. 20260729-211155.
- `neutralized-then-destroyed-counters` (x1): any objective counter mirrored
  from `OnDestroyed` to `OnNeutralized` needs an idempotence flag per target;
  a combat-dead ship can later be destroyed and fire both handlers, double
  advancing counters unless the two events share the same down gate.
  20260725-202255.
- `read-all-branches-of-a-load-bearing-engine-rule` (x2): when a leash/cap/guard/
  opt-in is load-bearing, read ALL its branches, not the first that confirms your
  phrasing - the AI `leash` has a `recently_damaged` override (a SHOT ship chases
  past it), and gravity opts in BOTH the player AND the AI ship (so "only piloted
  ships feel gravity" wrongly reads as AI-exempt; AI ships DO feel it). Also: an
  armed AI Engages by CHASING, so "holds station + shoots" needs no-thrusters OR a
  tight leash, never thrusters alone. 20260723-200643, 20260723-223954.
- `lint-is-the-fast-oracle-for-new-scenarios` (x1): for a new/large scenario RON
  the bugs that matter are game-geometry and balance invariants a human cannot
  eyeball (turret mount cells, "spawned-dead" enemies inside their threat
  envelope, flight-rig input collisions), NOT syntax - run `content lint --target
  <mod>` the moment the file parses and iterate to clean BEFORE writing the rig;
  it caught three real ch5 bugs as quick fixes. Also: splice big ship section
  lists from a SHIPPED ship (ids are ship-local, so reuse is safe) rather than
  hand-transcribing cubes. 20260723-182855.
- `avoidance-geometry-is-computed` (x1): a "sneak past / thread it" mechanic is
  only real if the SAFE corridor is pinned OUTSIDE the hazard volume by
  computed geometry (worst-case body radius, detection-bubble radius, leg
  centerline), the same rigor as a threadable-gap pin - a hand-placed bubble
  that merely "looks avoidable" false-greens; and the rig must assert the live
  post-flip COMPONENT (allegiance) after driving the real handler, not the
  action's presence in RON. 20260723-000320.

- `gate-scenario-handlers-to-their-acts` (x1): every handler fires in every
  act unless filtered; gate by default, especially terminal states.
  20260708-203659.
- `crate-solo-tests-miss-unified-features` (x6, FIXED-AT-ROOT 2026-07-21 via
  20260721-000249): `cargo test -p nova_scenario` alone USED to fail because a
  solo `-p` run has no sibling to unify the `serde` feature in, so ungated RON
  round-trip tests hit missing Serialize/Deserialize derives. Root fix: the
  affected crate carries a self dev-dep enabling its own feature
  (`nova_scenario = { path = ".", features = ["serde"] }`), which unifies the
  feature into the test build. Only nova_scenario was genuinely affected -
  nova_gameplay/nova_core compile solo because their feature-gated test code is
  itself behind `#[cfg(feature = ...)]` (skips when off). No more
  `--features serde` incantation. (was PROMOTED 2026-07-19 -> AGENTS.md; dev
  wiki via 20260718-152214.) 20260716-125856, 20260718-122906.
- `deleted-content-tests-carry-engine-coverage` (x1): data tests can be the
  only exercise of an engine mechanism; re-pin at the owning crate before
  deleting them. 20260716-155830.
- `re-homed-coverage-keeps-assertion-fidelity` (x1): re-homing a test onto a
  different tool (Rust lib -> subprocess/script) must carry the ASSERTION, not
  just the case - if the original checked WHY it failed (error string), the port
  checks the same, else coverage silently degrades to "something went wrong".
  Count of cases != fidelity. 20260720-230924.
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
- `input-mapping-overlays-flight-rig` (x1): PlayerControllerConfig
  `input_mapping` sections silently overlay the flight rig bindings
  (consume_input: false), so any section mapped to W/S/Space/RightTrigger
  double-drives flight - map custom actions to LMB/RightTrigger2 and grep
  every reader of a shared binding. A content lint could catch this at author
  time. 20260718-235837.
- `bei-app-finish-in-tests` (x2): bevy_enhanced_input needs `app.finish()` +
  `app.cleanup()` before spawning an action rig. 20260708-165705.
- `bevy-input-is-messages-in-tests` (x1): drive input tests with
  `World::write_message`; MouseWheel needs unit+x+y+window+PHASE.
  20260718-122912.
- `message-reader-run-if-drain` (x1): a `MessageReader` behind a state
  `run_if` does not advance while the system is skipped, so input events can
  replay on mode entry - drain every relevant frame and process only in the
  active mode. 20260726-115324.
- `parse-full-command-line` (x1): command parsers that dispatch only on the
  first token silently accept malformed valid verbs (`help garbage`) - test
  valid verbs with unexpected tails, not only unknown verbs. 20260726-115324.
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
- `lint-covers-types-not-variants` (x2): checks over a config tree enumerate
  every PATH to the checked type, not remembered enum variants; and ADDING an
  enum variant must sweep every non-exhaustive `match` on that enum (a `_ =>`
  arm hides the gap the compiler would otherwise flag) - a new id-bearing
  scenario action skipped the lint's dangling-target check. 20260716-191543, 20260723-000253.
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
- `display-none-grid-child-reflows-tracks` (x1): hiding one child of a CSS grid
  with `display:none` REMOVES it from the grid, so the remaining items reflow up
  into its track - a two-row `auto 1fr` grid with the header hidden dropped the
  body into the collapsed `auto` row (0 height), hiding an absolutely-positioned
  child that keyed off the body's height. Collapse the `grid-template` too
  (`grid-template-rows: 1fr`), don't just hide the child. Caught by a chromium
  eyeball, not the exit code. Pairs with [[render-output-eyeball]]. 20260728-185730.
- `capture-rig-succeeds-on-an-error-page` (x1): `chromium --screenshot` exits 0
  and writes a valid, non-empty PNG of a 404, so a rig guarded only by "the file
  exists and is non-empty" reports a full green run over error pages - worst in a
  rig that IS the `cmd:` proof for an eyeball DoD. Assert the HTTP STATUS before
  capturing, fail the readiness wait instead of falling through, and force the
  failure once. Pairs with [[render-output-eyeball]] and
  [[degrade-paths-need-a-forced-failure]]. 20260731-143918.
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
- `shell-bg-vs-and-chain` (x2): `A && B & C` backgrounds `A && B`; put
  backgrounded processes on their own statement, keep kills out of launching
  commands. Second hit: `cd wt && Xvfb :N &` backgrounded the cd too, so the
  test ran vacuously in the MAIN checkout - anything needing job control
  goes in a script file, where `&` scopes to its own line. 20260718-004723,
  20260719-193728.
- `measure-first-can-falsify-the-premise` (x1): the honest gate can say the
  lever barely helps; report it straight and surface the fork. 20260718-004723.
- `verify-interaction-not-just-rendering` (x1): a screenshot proves the frame
  drew, not that UI is clickable (bevy_ui on an image camera is unclickable);
  verify a CLICK or flag a human re-test. 20260718-132638.
- `verify-runtime-transitions-not-just-fresh-state` (x2): test A->B and B->A
  while running, not just each fresh boot state - both render-scale bugs
  lived only in the switch. 20260718-132638, 20260718-140903.
- `env-filter-governs-spans` (domain, x1): tracing EnvFilter directives
  written to silence LOG chatter also kill SPANS - nova_core's bevy_ecs=warn
  silently emptied the profiler; bevy_log ADDS RUST_LOG directives on top of
  the plugin filter, so a same-target override (bevy_ecs=info) restores them.
  20260719-112253.
- `bevy-camera-ignores-runtime-rendertarget-swap` (domain, x1): bevy 0.19
  re-derives camera target_info only on content change / is_added /
  projection change - swapping RenderTarget in place leaves sizes stale;
  `projection.set_changed()` forces the re-derive. 20260718-140903.
- `asset-meta-always-web-cost` (domain, x1): `AssetMetaCheck::Always` is
  required so DYNAMIC mod paths (not in the fixed `Paths` set) read their
  `.meta` sidecars - without it mod cubemaps crash; the cost is one request
  per missing `.meta`, which is a graceful 404 natively but a 200-OK-HTML SPA
  fallback under `trunk serve`, so `nova_meta_gen` writes default sidecars at
  build time to avoid it. (Distilled from docs/design on the ephemeral-docs
  wipe; the read-the-source half is [[verify-engine-guarantees-in-source]].)
  20260718-175424.
- `custom-asset-loader-needs-meta-gen-registration` (domain, x1): every CUSTOM
  `AssetLoader` the game registers (e.g. `NovaOsTtcFontLoader` for the NOVA OS
  `.ttc` font) must ALSO be registered in `nova_meta_gen`, or its assets ship
  with no `.meta` and fail SILENTLY on web only - under `AssetMetaCheck::Always`
  the 200-OK-HTML SPA fallback for the missing sidecar makes the load die, so
  the NOVA OS text was invisible on web while native (real 404 -> default meta)
  was fine. The generated meta names `L::type_path()`, so meta_gen must register
  the SAME loader type the runtime does. Sibling of [[asset-meta-always-web-cost]];
  symptom shape (non-text UI drew, only glyphs missing) ruled out a render-layer
  cause before any code change. A future guard: assert meta_gen's loader set
  covers every extension the game's registered loaders claim. 20260727-172205.
- `bevy-css-border-triangle-needs-contentbox` (domain, x1): a filled UI
  triangle with no art asset is a zero-CONTENT node + coloured top border +
  transparent sides - Bevy's border shader (`nearest_border_active` in
  bevy_ui_render) paints each side only in its mitered wedge, so a top-only
  colour over a 0x0 box is a down-triangle. But `Node` defaults to
  `BoxSizing::BorderBox`, under which a 0x0 node collapses its border box and
  `extract_uinode_borders` (which gates on non-zero computed border) draws
  nothing - set `box_sizing: ContentBox`. An instance of
  [[verify-engine-guarantees-in-source]]. 20260723-233446.
- `taffy-measures-leaf-nodes-only` (domain, x1): in bevy_ui a node carrying
  `Text` AND `children!` is a CONTAINER, so its text measure is dropped and the
  box collapses to its own padding+border while the glyphs still render at full
  length (the HUD chips' fill became a 20x10 slab under a 58 px label). Text
  goes on a LEAF child of any node that has children; pinned by
  `hud::chip_layout_rig`, the repo's live `UiPlugin` layout rig. 20260730-122909.
- `bevy-anonymous-system-param-panic-read-the-signature` (domain, x1): bevy's
  "Parameter failed validation: Resource does not exist" names neither the
  system nor the resource without bevy_ecs's `debug` feature - run with
  `BEVY_BACKTRACE=full` and grep the backtrace for `run_unsafe<fn(`, whose
  generic argument is the system's full parameter list. Named three missing
  plugins/assets in one pass while building the UI layout rig. 20260730-122909.
- `bevy-ui-property-is-node-field-not-component` (domain, x1): in bevy_ui 0.19
  several UI properties are FIELDS of `Node` (`Node.border_radius`), not
  standalone bundle components - spawning `BorderRadius` as a component fails
  with "not a Bundle". Grep the vendored struct to confirm component-vs-field
  before spawning, and re-check even the ones you "know" when the engine minor
  moved. An instance of [[verify-engine-guarantees-in-source]]. 20260726-193219.
- `bevy-ui-scroll-input-clamps-stored-offset` (domain, x1): when custom wheel
  input writes `ScrollPosition`, clamp the STORED offset with Bevy UI's layout
  max (`content_size - size + scrollbar_size`), not only the rendered position,
  or invisible bottom overscroll accumulates. 20260725-171900.
- `require-default-lands-after-root-add-observer` (domain, x1): a component
  supplied by `#[require]` on a marker (Allegiance/PlayerSpaceshipMarker on the
  controller markers) - or inserted by a deferred command inside another
  `Add<Root>` observer (nova_scenario's `insert_spaceship_sections`) - is NOT
  present when a SIBLING `Add<Root>` observer runs. Read it via change detection
  / a later system, and do a "skip if sibling marker present" action from an
  `Added<Sibling>` SYSTEM (which defers past the deferred-spawn flush), never an
  `Add<Sibling>` observer (which runs before your own deferred `commands.spawn`
  flushes, finding nothing to undo). 20260723-233446.
- `shader-uniform-field-order-must-match-wgsl` (domain, x1): a Rust
  `ShaderType`/encase uniform struct and its WGSL struct must have the SAME field
  order AND compatible alignment - inserting a `vec2`/`vec3` mid-struct silently
  corrupts the whole uniform if it lands on a bad offset. Put a `vec2` right after
  a `vec4` (offset 16, 8-aligned, no padding hole) or pad explicitly; verify the
  two struct definitions line up field-for-field. 20260726-193155.
- `wgsl-not-covered-by-cargo-check` (domain, x1): `.wgsl` shaders load at
  RUNTIME, so `cargo check`/tests never compile them - a syntax/type error only
  surfaces as a wgpu/naga panic when the material first renders. Validate any
  shader edit by RUNNING the app/example that renders it (here: `BCS_AUTOPILOT=1
  cargo run --example screenshot_nova_os --features debug` opens the NOVA OS;
  clean AppExit::Success = the shader compiled). 20260727-135204.
- `bevy-ui-image-camera-is-pickable-via-forwarded-pointer` (domain, x1): bevy
  0.19 `ui_picking` matches pointers to cameras by RENDER TARGET, not
  window-ness, so UI rendered to a `RenderTarget::Image` via `UiTargetCamera` IS
  hover/clickable - spawn a `PointerId::Custom` whose `PointerLocation.target` is
  that image and drive it from the real cursor (map through the display rect +
  inverse any warp). The "bevy_ui on an image camera is unclickable" lesson
  ([[verify-interaction-not-just-rendering]]) is the LEGACY `ui_focus_system`
  only; the modern picking backend does not have that limit. BUT
  `bevy_picking::update_is_hovered` is hard-coded to `PointerId::Mouse`, so the
  `Hovered` COMPONENT needs a manual mirror for the forwarded pointer - and that
  mirror MUST be scoped to the through-image subtree (descendants of the content
  root) or it force-writes `Hovered(false)` on window-space UI every frame,
  fighting the real cursor. An instance of [[verify-engine-guarantees-in-source]].
  20260726-193233.
- `bevy-ui-render-ignores-renderlayers` (domain, x1): `bevy_ui_render` routes UI
  purely by `ComputedUiTargetCamera` (`UiCameraMap`) and never reads
  `RenderLayers`, while 2D sprites DO respect them. So a UI camera placed on a
  dedicated `RenderLayers` layer still draws its `UiTargetCamera`-targeted UI AND
  is isolated from stray world sprites (e.g. the render-scale upscale sprite on
  the default layer) - the way to render a UI subtree to an image without the
  camera also picking up scene 2D. 20260726-193233.
- `rtt-ui-select-via-activate-not-interaction` (domain, x1): a `Button` inside a
  forwarded-pointer RTT (the NOVA OS CRT composite) does NOT get its
  `Interaction` component updated - polling `Changed<Interaction>` for clicks
  silently does nothing. Use the `bevy_ui_widgets::Activate` observer
  (`.observe(handler)`, read `activate.entity`), the same path the terminal's own
  buttons use; it fires through the forwarded pointer. Sibling of
  [[bevy-ui-image-camera-is-pickable-via-forwarded-pointer]]. 20260724-102320.
- `one-pointer-button-cant-both-activate-a-widget-and-drag-the-world` (domain, x1):
  binding the SAME mouse button to a `bevy_ui` `Button`/`Activate` selection AND to
  a camera/world drag on the same viewport makes them fight - a click-with-a-few-px
  of motion is read as a drag, moves the view, slides the widget out from under the
  cursor, and the activation never lands ("it thinks you want to drag so it doesn't
  select"). Reserve the widget's button (LMB/Primary) for the widget; put drag on
  RMB (or gate it behind a modifier). Pin it at the input-system altitude: hold the
  button + send a `MouseMotion`, assert the drag target did NOT move; hold the drag
  button, assert it did. Sibling of [[rtt-ui-select-via-activate-not-interaction]].
  20260728-143430.
- `absolute-child-left-is-from-the-padding-edge` (domain, x1): in bevy_ui (as in
  CSS) an absolutely-positioned child's `left`/`top` is measured from its parent's
  PADDING edge - already inside the parent's border - so a label meant to sit flush
  against a bordered dot needs `left = SIZE - BORDER`, not `left = SIZE`. Both NOVA
  OS blip labels overlooked it and carried 6 px (map) and 4 px (ship) of dead band
  that selected nothing. Measure the gap on the LIVE tree (`ComputedNode` +
  `UiGlobalTransform` rects) rather than reasoning about the offsets; and note that
  bevy's `contains_point` excludes the exact shared edge from BOTH rects, so probe a
  seam by sweeping pixel CENTRES across the band, never the boundary coordinate.
  20260730-123039.
- `forwarded-pointer-must-apply-the-composite-forward-map` (domain, x1): a pointer
  forwarded onto an RTT that is displayed through a distorting shader must apply
  the SHADER'S OWN screen->image map, not an inverse of it - the fragment already
  computes "which texel is drawn at this screen uv", which is exactly the question
  the pointer asks. Keep every constant of that map in the material UNIFORM (Rust
  side) rather than as a WGSL `const`: a WGSL-local overscan was invisible to the
  pointer and put NOVA OS clicks up to 27 px off at the screen corners. Mirror the
  shader's output GATES too (`in_bounds`, raster `collapsed`), so nothing is
  clickable where nothing is drawn. Sibling of
  [[bevy-ui-image-camera-is-pickable-via-forwarded-pointer]]. 20260730-123039.
- `verify-reused-driver-actually-moves` (x1): before building interaction on a
  reused input/animation component, confirm the OUTPUT actually moves in THIS
  context - do not trust that writing its input rotates/animates. The map camera
  wrote `SphereOrbitInput` but the shared `SphereOrbit` plugin's smoothed path
  never turned the RTT camera (only the direct `center` pan moved); owning the
  spherical math in a bespoke `MapOrbit` fixed it at once. A passing arity/
  lifecycle test proves the seam exists, not that the pixels respond. Kin of
  [[verify-interaction-not-just-rendering]] and [[advertised-is-not-wired]].
  20260724-102320.
- `reused-render-pattern-verify-coordinate-frame` (x1): when copying an RTT/
  projection pattern between apps, re-check the COORDINATE FRAME it assumes, not
  just the call shape - the map app projects blips from WORLD positions because
  its scene is world-space; the ship app's scene is ship-LOCAL anchored at the
  origin, so the copied `world_to_viewport(world_pos)` drifted blips off the
  blocks whenever the ship flew off origin. Project in the scene's own frame.
  Kin of [[verify-reused-driver-actually-moves]]; needs an
  [[spatial-fixture-off-the-trivial-point]] to catch. 20260726-115339.
- `new-render-primitive-verify-on-gpu` (x1): introducing a mesh-primitive/material
  combo the repo has not used before (here `LineList` + unlit `StandardMaterial`
  for box outlines) draws NOTHING until proven - a headless entity-tree test pins
  the ECS wiring, not the pixels. Confirm on the real GPU (the `screenshot_*`
  autopilot harness) as part of /work verify, and use a capture fixture holding
  one of EVERY new visual variant (the ship range had no weapon, so the ammo pips
  shipped pixel-unverified). Kin of [[verify-interaction-not-just-rendering]]. 20260728-115435.
- `autoscroll-on-new-content-not-any-change` (x1): pinning a scroll view to the
  bottom on every change of its backing resource defeats manual scroll (PageUp/
  wheel) the moment that resource changes for an unrelated reason (prompt edits,
  a mirrored command list). Gate the auto-scroll on the row COUNT increasing
  (new output), not on `resource_changed`. The unit test forced overflow and
  passed while the real view was un-scrollable. 20260724-102320.
- `single-frame-shot-is-a-coinflip-for-animated-state` (domain, x1): a
  time-driven blink/animation makes ONE screenshot an unreliable proof of a
  static-visibility fix - the NOVA OS block caret blinks at 1.25 Hz (50% duty),
  so the empty-prompt capture caught an OFF phase and showed no caret even though
  the fix was correct (a later frame with typed text proved the caret renders).
  For such proofs either add a capture mode that FREEZES the animation phase, or
  accept the manual DoD and SAY the shot was phase-ambiguous - do not read a
  blink-off as a regression. Pairs with [[widget-tree-eyeball-for-logical-layout]]
  (prefer the deterministic widget-tree assert when the thing under test is
  logical, not pixel). 20260727-162635.
- `imagemagick-recolor-preserve-alpha` (domain, x1): to recolour a PNG's opaque
  pixels to a flat colour while KEEPING its alpha shape, `-fill white -colorize
  100%` is unreliable (it collapsed nova_crt_mark.png to gray+alpha with a broken
  channel split). Instead extract the alpha and composite a solid-colour canvas
  through it: `magick in.png -alpha extract a.png; magick -size WxH xc:white
  a.png -alpha off -compose CopyOpacity -composite PNG32:out.png`. Verify with
  `magick out.png -background black -alpha remove -alpha off -format
  '%[fx:mean.r] %[fx:mean.g] %[fx:mean.b]' info:` - equal R=G=B means white where
  opaque. (Fixed the black NOVA CRT star mark.) 20260727-162635.

## Promoted (resolved 2026-07-21, task 20260720-220051)

Five folded into AGENTS.md's Conventions ("Promoted ledger lessons" block);
out-of-context-review-pass annotated as already /flow round-1 practice. Kept
here (annotated) as the paid record.

- `prose-from-diff-not-intent` (x3, PROMOTED 2026-07-21 -> AGENTS.md Conventions): write CHANGELOG/wiki/NOTES from the final diff (count sites by counting the diff), then re-read asking "does the prose claim anything the diff does not do?". 20260717-112622, 20260717-163058, 20260719-001600.
- `verify-stale-brief-against-tree` (x5, PROMOTED 2026-07-21 -> AGENTS.md Conventions + flow bug playbook): reproduce a filed bug against the CURRENT tree before implementing; a subsystem change can shrink or falsify the fix scope - and so can the WORLD state (broadside/lifeline have no gravity well, so "the Ceres Queen falls in" was impossible and the convoy "crash" was knockback, not gravity; a 5-min grep for `surface_gravity: Some` would have reframed both). Also: a scoping brief can come from an EXPLORATION AGENT's summary ("shakedown DOES NOT USE the pacing module") - read the handlers before writing the plan around it; shakedown used the reverse ordering the summary glossed. 20260714-154958, 20260718-004834, 20260719-233732, 20260722-092427, 20260722-092432, 20260722-142341.
- `render-output-eyeball` (x7, PROMOTED 2026-07-21 -> AGENTS.md Conventions): a dimensionally-valid generated artifact can be empty/wrong while every exit code is green - open it; a layout/readability task is unverified until someone SEES it rendered, and if no capture rig exists for the surface, building it IS step 1 (a widget-tree test cannot stand in - two NOVA OS tasks landed 7 blind rounds without one). 20260718-122923, 20260719-112253, 20260726-180807, 20260726-193219.
- `widget-tree-eyeball-for-logical-layout` (x2): for a text/list "layout", the eyeball is asserting the SPAWNED widget tree (Text/Node content in child/display order) through the real spawn path - it sees the rendered content deterministically and headlessly. Prefer it to a pixel screenshot for logical/text layouts; a pixel shot is flaky+expensive on a software GPU, so read the capture rig's window/settle/GPU limits BEFORE attempting one (a scenarios-picker capture overran the autopilot window on llvmpipe - a limit the rig's own comments documented). Also covers collapse/expand: drive the real toggle observer and assert the header marker + members appearing/vanishing. 20260723-095930, 20260723-095951.
- `discharge-decision-caveat-first` (x1): when a task depends on a decision that carries a "verify X during work" caveat, discharge that check as the FIRST work step (a grep or a tiny probe), not after building - it either unblocks the build or triggers the stop-and-ask while it is still cheap (a "do the hidden members cold-launch?" grep of their OnStart player spawn retired the risk before the UI was written). 20260723-095951.
- `test-must-not-reuse-the-formula-under-test` (x2): a test that recomputes the production formula proves only "the code runs its own arithmetic", not that the arithmetic is RIGHT - it is tautological on exactly the assumption that can be wrong. Assert against an INDEPENDENT value (a stamped `ComputedNode`, a real layout pass, a hand-figured constant). Sibling rule: for font/render-dependent layout, MEASURE (`ComputedNode.size * inverse_scale_factor`) instead of multiplying by an assumed em-fraction - a NOVA OS caret positioned at `chars * 0.6em` (the block WIDTH, not the glyph advance) would have drifted a full cell by ~6 chars, and the first test hid it by reusing the same 0.6. Caught by out-of-context review. Second form (20260730-123039): an independent
  transcription is only independent for the part you actually transcribed -
  copying a shader's sample-UV maths but writing its VISIBILITY rule from memory
  left the reference disagreeing with correct production code, hidden only by a
  grid too coarse to sample the band. 20260727-135200, 20260730-123039.
- `diff-the-mirror-against-its-source` (x1): when OUR code claims to mirror an
  external artifact (a shader, a wire format, a spec), read the artifact and COUNT
  the operations on each side before trusting - or before theorising about - the
  mirror; its own doc comment is folklore. `nova_os_inverse_barrel` was documented
  as "inverse of the shader's forward barrel warp", and the shader in fact mapped
  screen->image directly (so no inverse was wanted at all) in TWO steps, the second
  a 0.93 overscan the Rust side could not even see. "Two operations there, one
  here" IS the finding. Sibling of [[verify-engine-guarantees-in-source]] pointed
  at our own tree. 20260730-123039.
- `new-input-needs-a-new-test-axis` (x1): when a function gains a PARAMETER, extend
  the test's sweep to vary it in the same edit and add a guard proving the new axis
  is non-degenerate - a parameter that never varies across a run is untested. A
  CRT-pointer mapping grew a power-collapse remap and its grid ran only at full
  power, where that remap is exactly the identity, so flipping its divide to a
  multiply passed all 785 tests. Derive the guard from the constants, not from
  intuition ("power < 1 means collapsed" was wrong at 0.65, where the taller
  smoothstep has already reached 1). 20260730-123039.
- `authored-vs-derived-values` (x4, PROMOTED 2026-07-21 -> AGENTS.md Conventions): author content against measured runtime consts, and encode layout invariants as computed rig assertions. 20260716-124722, 20260717-112630.
- `advertised-but-unwired` (x5, PROMOTED 2026-07-21 -> AGENTS.md Conventions): a config surface or UI hint is not a capability until producer/consumer wiring and preconditions are verified in the new context - and a keybind hint is per-SURFACE: `CTRL+C: CLOSE` in the terminal footer was inert at the prompt (Ctrl+C only exits an app), caught in review. 20260712-093044, 20260726-134738, 20260727-135213.
- `out-of-context-review-pass` (positive, x35, PROMOTED 2026-07-21 -> already /flow round-1 practice): a fresh-context review re-derives load-bearing claims and catches MAJORs shared-session eyes miss; verify the verifier's counterexamples too - it caught the drawer's audio-loop freeze gap, overlay z order in the NOVA OS monitor, and unwired NOVA OS footer/doc promises. 20260717-212219, 20260724-102304, 20260724-134312, 20260726-115320, 20260726-134738.

## Pending promotions (3+ occurrences, user decides)

- `deleting-a-test-salvage-live-assertions` (x3, PROMOTE 2026-07-31 -> 20260731-102037) -> work skill (verify step):
  make "read each deleted test's ASSERTIONS and re-home the survivors" a
  standing step of any diff that removes tests, not a lesson to remember.
  20260728-125514, 20260729-163816, 20260729-211200.

- `anchor-edits-in-the-right-scope` (x3, PROMOTE 2026-07-31 -> 20260731-102037) -> work skill: an edit anchored on an
  item's `#[test]`/`const`/attribute line inserts BETWEEN that item and its doc
  block, silently reassigning the doc to the new item - compiles and tests
  clean, only a reader notices. No tool or template can hold this (it is an
  edit-anchor choice, not a checkable artifact state), so the target is prose in
  the work skill's edit guidance: "anchor an insert on the DOC BLOCK start, not
  the attribute, and re-read the produced text around both items".
  20260525-133017, 20260716-193949, 20260728-175742.
- `pin-each-caller-not-just-shared-core` (x4, PROMOTE 2026-07-31 -> 20260731-102037) -> work/review skill: a shared
  helper/renderer covered by ONE caller or a synthetic fixture does not prove the
  OTHER callers' wiring - target resolution, plumbing, side effects, or a data
  field set at N registration sites feeding a pure renderer. Prose target (work
  verify step + review Tests dimension): "when a change adds N symmetric callers or
  registration sites, pin each end-to-end in the SAME pass - enumerate the CALL
  sites, not the helpers". Here `arg_hint` was
  unit-tested on the renderer with a synthetic spec; the reviewer had to point at
  the un-exercised `ship <section>` registration wiring.
  20260726-115339, 20260728-115430, 20260728-184502, 20260729-015406.
- `validate-proof-command-shape-at-plan-time` (x5, PROMOTE 2026-07-31 -> 20260731-102037) -> work skill: at verify,
  confirm a `cmd:`/test proof runs the INTENDED tests - right arity/flags AND a
  NON-ZERO "N passed" PER named module, not a bare "ok". Failure modes seen:
  `cargo test <a> <b>` rejects the 2nd filter; `-p <crate> <name>` matched 0
  ("685 filtered out") yet reported ok; `-- f1 f2 ... f8` with many positional
  filters silently ran only some modules; and an ABSENCE grep built from the
  WORDS of a stale claim that no correct tree can ever satisfy. Prose target
  (verify step): "grep each intended module/test name in the output; re-run any
  absent one alone - and for an absence proof, grep the CLAIMS that were really
  in the tree, checking at plan time - against the tree the change will PRODUCE -
  that zero is reachable". 20260726-115334, 20260727-135208, 20260728-175731,
  20260730-122843, 20260731-143918.
- `match-ci-feature-set-in-targeted-tests` (x3, PROMOTE 2026-07-31 -> 20260731-102037) -> work skill: a workspace
  `cargo check --all-targets` does not enable a crate's self dev-dep `serde`
  feature, so it silently skips serde-gated targets (a false green). Prose
  target (verify step): "run per-crate `cargo test -p <crate> --no-run` on
  touched crates before trusting a workspace check". A tool guard is hard (the
  feature unification is per-crate), so a work-skill line is the realistic home.
  20260718-004834, 20260718-102022, 20260724-193830.
- `reuse-known-good-stack` (x9, PROMOTE 2026-07-31 -> 20260731-102037) -> work skill (a POSITIVE lesson): scaffold a new test rig
  by copying the nearest passing sibling rig verbatim, then mutate - do not
  reconstruct it from the system's parameter signature (a hand-built CRT rig
  omitted the sibling's `init_asset::<Font>/<Image>` and panicked in the
  AssetServer on the app-launch font load). Prose target (the skill already says
  "grep the module for an existing rig of the same kind first"; sharpens it to
  "copy it whole first" - and to "the nearest rig may be the DEPENDENCY'S own,
  e.g. bevy_ui's `setup_ui_test_app`"). 20260712-093048, 20260711-180511, 20260724-102304, 20260727-014148, 20260730-122909.
- `new-required-system-param-sweeps-all-rigs` (x1, DEFER 2026-07-31 at x1: only x1 - it was filed under Pending promotions below the 3+ bar; revisit if it recurs) -> work skill: when a
  widely-run system gains a required `Res`/`ResMut`/param, grep every
  `add_systems`/test rig that runs it and register the resource BEFORE running -
  the compiler cannot catch a missing resource, only a run-time panic can (adding
  `ResMut<NovaOsDegauss>` to two NOVA OS systems broke 5 partial-app rigs at
  once). 20260727-014148.
- `lint-gate-is-the-last-step` (x3, ABSORBED 2026-07-31 by .githooks/pre-commit (fmt gate, armed by scripts/setup-hooks.sh; shipped 20260722-183022)): fmt/clippy/tests run AFTER the final edit;
  mirror remote CI locally before pushing - a post-final-edit prelude tweak
  landed an unformatted line that CI would have bounced (caught at flow Finish).
  Promotion candidate (tool > prose): a pre-commit / pre-land `cargo fmt --check`
  guard would make this impossible instead of relying on the author remembering.
  20260525-133014, 20260715-142931, 20260722-092432.
  SHIPPED 2026-07-22 (20260722-183022): `.githooks/pre-commit` refuses a commit
  whose staged changes touch Rust while the tree is not fmt-clean; armed via
  `scripts/setup-hooks.sh`, and it gates the `sprout land` commit too (sprout
  rolls back on hook failure). The "author remembering" failure mode is now a
  tool guard - the recurrence this entry tracked should stop.
- `sweep-content-repo-wide-not-just-assets` (x3, PROMOTE 2026-07-31 -> 20260731-102037) -> work/review skills:
  relocating/renaming an asset sweeps EVERY content-shaped file repo-wide
  (examples/, include_str!, test data); an "X holds everywhere" audit sweeps
  base + webmods + assets/mods + Rust-coded scenarios, re-derived in review.
  Promotion target: make task planning and review check "base and webmods"
  explicitly for content-wide behavior changes. 20260717-002105,
  20260717-201534, 20260725-202255.
