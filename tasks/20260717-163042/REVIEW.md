# Review: Arrival grace

- TASK: 20260717-163042
- BRANCH: feature/arrival-telegraphs

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/input/ai.rs:755 - the grace
  early return's most load-bearing path is unpinned: it demotes a ship
  whose CURRENT state is Engage, and that path is reachable on every real
  spawn. The scenario spawn inserts AISpaceshipMarker with no explicit
  state (crates/nova_scenario/src/objects/spaceship.rs:322), so the
  required-component default applies - and AIBehaviorState's default is
  Engage (ai.rs:587). A graced scenario ship therefore starts its first
  behavior tick as Engage+live-grace, and the unconditional early return
  is exactly what flips it onto its passive routine. But no test covers
  grace with current=Engage: both new table rows start from Patrol/Idle
  (ai.rs:1842) and the system test seeds AIBehaviorState::Patrol
  explicitly (ai.rs:5046), not the spawn default. Verified by mutation:
  gating the return with `&& current.is_passive()` (a plausible future
  cleanup - the component doc itself says "holds its PASSIVE routine")
  passes all 92 tests while breaking the shipped feature in its flagship
  use case (spawn 600u out with the player inside the 800u engage range:
  default-Engage holds via the `state => state` arm and the ship fights
  through its "grace"). Suggested change: add a table row
  `next_behavior_state(Engage, near, false, true, false, true, calm()) ==
  Patrol` (grace demotes the spawn-default Engage), seed grace_app's ship
  with `AIBehaviorState::default()` so the system test mirrors the real
  spawn shape, and note on the early return that it deliberately also
  demotes the spawn-default Engage.
  - Response: fixed three ways - the Engage+grace->Patrol table row, the
    system rig now seeds AIBehaviorState::default() (the production spawn
    shape), and the early return carries the load-bearing comment. Your
    mutation D now fails the new table row.

- [x] R1.2 (MINOR) tasks/20260717-163042/TASK.md:49 - the ticked docs
  step claims "guide-author-scenario.md AI controller fields + the
  WARNING-BEAT authoring pattern", but the branch never touches
  web/src/wiki/dev/guide-author-scenario.md; the field and the pattern
  landed in web/src/wiki/dev/scenario-system.md:245-253 (plus a field
  list mention in sections.md:37). The authoring guide is the doc the
  warning-beat convention was promised in, and it currently documents
  neither engage_delay nor the pattern. Suggested change: either add the
  engage_delay field + warning-beat pattern to guide-author-scenario.md,
  or amend the step and NOTES.md to say where the docs actually landed
  and that the guide convention is owned by the beat-sheet task
  20260717-163058 (whose spike step explicitly targets
  guide-author-scenario). Recent repo lesson applies verbatim: "ticked
  step is not proof" (commit 678bfa79).
  - Response: fixed - guide-author-scenario.md gained a Telegraphed
    arrivals section (RON example + grace semantics); the ticked step is
    true now.

- [x] R1.3 (NIT) tasks/20260717-163042/NOTES.md:22 - "28 call sites
  gained the grace argument" undercounts: master had 29 call sites (plus
  the definition), and all 29 gained the argument (28 test callers + the
  update_behavior_state system call at ai.rs:892). The sweep itself is
  intact - all 34 current call sites (29 updated + 5 new) pass exactly 7
  args, every pre-existing test caller passes `false`, and the multiline
  ThreatSignals callers (e.g. ai.rs:1927) are uncorrupted. Suggested
  change: fix the count in NOTES.md.
  - Response: fixed - NOTES says 29 pre-existing sites (system + 28 tests).

### Notes (no action required)

- Corner case grace+leash+damage: leash check first (ai.rs:748), grace
  second (ai.rs:755), both overridden by recently_damaged, so a
  telegraphed leashed ship that gets shot engages - and in the system the
  damage pins the grace the same frame, so grace_held and
  recently_damaged never reach next_behavior_state true together. The
  pure-function damage override on the grace branch is defensively
  redundant but correctly pinned as contract. When the 3s damage memory
  fades, the leash reasserts; the grace never does (pinned). Sane.
- aimed_at during the grace deliberately does NOT end it, and that is
  the right call, not just spike fidelity: the aim signal fires on a
  hostile hull holding its nose inside 500u, and a player will naturally
  look at an announced arrival - an aim override would cancel every
  telegraphed entrance on sight. Damage-only override stands.
- The PD test (ai.rs:5130) passes identically without the AIEngageGrace
  insert (verified by mutation) - today it pins the bypass (a passive
  Patrol ship with a PD target fires), since on_projectile_input and
  update_point_defense_target never read the grace. The insert still
  earns its keep as a forward guard: if PD ever starts gating on the
  grace, this test breaks. Right pin, correctly scoped.
- The tick semantics check out: damage_memory ticks before the grace
  reads recently_damaged() and before ThreatSignals derives it, so pin
  decision and signals agree within a frame (worst case one frame of
  latency if the damage observer fires after this system - same as the
  leash override). tick(remaining) on the pin lands elapsed exactly on
  duration (Duration math is exact) and sets the finished flag;
  subsequent per-frame ticks on a finished Once timer are no-ops. The
  NOTES' set_elapsed/finished-flag trap is real Bevy Timer behavior and
  the tick(remaining) comment matches a first-test-run catch.
- No re-arm path exists: AIEngageGrace is inserted only by the spawn
  observer (spaceship.rs:348) and only ever ticked - "permanently if
  shot" in the wiki is accurate. Non-positive guard (`delay > 0.0`) also
  rejects NaN. serde attrs mirror the leash field; strict-RON syntax and
  omitted-default both parse (pinned by engage_delay_ron_parses_and_defaults).
- No shipped content uses engage_delay yet - expected, the beat-sheet
  task 20260717-163058 is the consumer.

### Verification record

- `git diff master...feature/arrival-telegraphs` reviewed in full
  (ai.rs, spaceship.rs, CHANGELOG, scenario-system.md, sections.md,
  TASK/NOTES).
- `cargo test -p nova_gameplay input::ai::` ->
  `test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 461 filtered out; finished in 1.01s`
- `cargo test -p nova_scenario --features serde` (flag required) ->
  `test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.31s`
  including `engage_delay_ron_parses_and_defaults ... ok` and
  `engage_delay_inserts_the_grace_only_when_positive ... ok`.
- Caller sweep audited by script: 34 call sites, all 7 args, top-level
  comma parse clean; master had 29 call sites + definition; grace_held
  values: 29x false, 4x true (new rows), 1x variable (system call).
- Mutation A (grace branch disabled): 3 of the 4 new tests fail
  (table row, hold-then-engage, damage-pin), PD test unaffected -> the
  grace tests pin the branch.
- Mutation B (pin removed): exactly the "grace is pinned finished"
  assert fails -> the permanence claim is carried by a real assert.
- Mutation C (PD test without the grace insert): passes -> bypass pin +
  forward guard, see notes.
- Mutation D (grace gated on is_passive): all 92 tests pass while the
  real spawn shape breaks -> R1.1.
- All mutations reverted; `git status` clean before writing this review.

## Round 2

- VERDICT: APPROVE

All three Round 1 findings verified fixed at commit d7fb1a47; checkboxes
ticked above. No new problems introduced by the fixes.

- R1.1 confirmed: the Engage+grace->Patrol table row landed in
  the_grace_holds_passive_and_damage_overrides_it (ai.rs:1875-1881),
  grace_app now seeds `AIBehaviorState::default()` - the production
  Engage default - with a comment naming why (ai.rs:5057-5059), and the
  early return carries the load-bearing UNCONDITIONAL-on-purpose comment
  (ai.rs:754-759). Mutation D re-run: it is now caught THREE ways -
  the new table row fails at ai.rs:1878 (`left: Engage, right: Patrol`)
  and both system tests fail too because the rig now spawns the default
  Engage state (a_graced_arrival_holds_its_routine_then_engages,
  damage_ends_the_grace_immediately_and_permanently). Tree restored
  clean after the probe.
- R1.2 confirmed: guide-author-scenario.md gained a "Telegraphed
  arrivals" section (web/src/wiki/dev/guide-author-scenario.md:352-369)
  with the RON example (`engage_delay: Some(8.0)` - the tested strict
  syntax), the grace semantics including the permanent damage override
  and the PD carve-out, and the non-positive rule. The ticked TASK step
  is now true as written.
- R1.3 confirmed: NOTES.md now says 29 pre-existing call sites (the
  system call plus 28 test callers), matching the audited count.

### Verification record (Round 2)

- `git diff 925f4bce..HEAD` reviewed in full (ai.rs, NOTES.md,
  REVIEW.md responses, guide-author-scenario.md; single commit
  d7fb1a47).
- Mutation D re-run (`&& current.is_passive()` on the grace return):
  `test input::ai::behavior_state_tests::the_grace_holds_passive_and_damage_overrides_it ... FAILED`
  at ai.rs:1878 (`left: Engage, right: Patrol`), plus both engage_grace
  system tests FAILED. Reverted; `git status` clean.
- `cargo test -p nova_gameplay input::ai::` ->
  `test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.95s`
- `cargo fmt --check` -> clean (no output).
