# Review: Broadside act-split + cover hardening

- TASK: 20260717-112639
- BRANCH: work/broadside-rework

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) crates/nova_assets/src/scenario/broadside.rs:73 - the VAR_ACT
  doc comment still describes the pre-split single-scenario numbering: "Story
  act (0 contact, 1 corvettes, 2 gunship, 3 won)". After the split there is no
  act 3 anywhere, part one's act 2 means "checkpoint won", and part two runs
  1 (gunship) -> 2 (won). The flag comment at :77-79 has the same drift ("the
  act-2 escalation gates on BOTH" - that handler is now the checkpoint, and it
  still sets act 2 but no longer escalates). This is the single-source builder
  for generated content, so the stale map is what the next author will read.
  Suggested change: reword to the per-part numbering, e.g. "Story act. Part
  one: 0 contact, 1 corvettes, 2 checkpoint won. Part two: 1 gunship, 2 won."
  - Response: fixed - the VAR_ACT doc now describes both parts' numbering.

- [ ] R1.2 (MINOR) crates/nova_assets/tests/broadside_assault.rs:307-309 - the
  comment above the loadout asserts still says "and the turret never runs dry
  (no resupply mechanic exists yet)", which contradicts the assert three lines
  below it (`!player_controller.infinite_ammo`, "finite auto-reloading ammo
  ... since task 20260717-085640"). NOTES.md claims the stale infinite-ammo
  doc comment was "fixed in passing", but only the builder's copy was; this
  test copy was missed. Suggested change: reword the comment to match the
  assert (torpedoes stay the enemy's weapon; ammo is finite and auto-reloads).
  - Response: fixed - the test comment now matches the finite auto-reload assert.

- [ ] R1.3 (MINOR) crates/nova_assets/tests/broadside_assault.rs:239
  (player_death_after_the_win_declares_nothing) - the post-win test only pins
  the PLAYER death gate; the HAULER soft-fail gate (act < 2 in both parts) is
  unpinned. Loosening it back to act < 3 fails no test, and that regression is
  exactly the objective-pushed-under-the-Victory-overlay class the builder
  comment cites (review R1.5 at broadside.rs:419-421). The task step promised
  "post-win deaths declare nothing", which is currently only half-tested.
  Suggested change: in the same act-2 loop, also destroy "hauler" and assert
  GameObjectives gained no "hauler_lost" entry (and no outcome change).
  - Response: fixed - the five unrelated examples restored from master; only
    19_broadside keeps changes. fixed - the post-win test now also destroys the hauler and asserts
    no hauler_lost objective lands under the Victory, with a new live-act
    delivery-guard test (hauler_death_on_a_live_act_pushes_the_soft_fail_beat).

- [ ] R1.4 (NIT) crates/nova_assets/tests/broadside_assault.rs:234 - stale
  phrasing from the old numbering: "(act 3 - the gunship's death blast, a rock
  under the gold banner)" - the test now seeds act 2.0, and the death-blast
  case lives in part two. Same family: the expect message at :369 says "the
  escalation spawns the gunship" but part two's OnStart spawns it now.
  Suggested change: update both strings to the split-world wording.
  - Response: fixed - both stale comments updated to the split's reality.

- [ ] R1.5 (NIT) examples/04_turret_section.rs:194, 05_torpedo_section.rs:196,
  08_scenario.rs:104, 10_playable.rs:155, 18_screenshot_orbit.rs:115 -
  indentation-only churn in five examples untouched by this task (a cargo fmt
  sweep of pre-existing misindentation). Harmless and fmt was a mandated step,
  but it is unrelated noise in this diff; ideally it lands as its own chore
  commit so the scenario diff stays reviewable.
  - Response:

### Verification record

Reviewer-recomputed geometry (own python over the GENERATED RON, both files;
factor bounds confirmed at nova_scenario/src/objects/asteroid.rs:328-329 as
3.5/6.0):

- Five invulnerable boulders, byte-identical layout in both parts; scatter
  seed identical in both parts (433757350076153856), scatter box identical
  (x [-200,200], y [-45,45], z [-430,-80]):
  - cover_boulder_1 (90, 20, -520) r4.0, 6x body 24u
  - cover_boulder_2 (-110, 0, -530) r4.0, 6x body 24u
  - cover_boulder_3 (20, -15, -575) r5.0, 6x body 30u
  - cover_boulder_4 (130, 40, -700) r3.5, 6x body 21u
  - cover_boulder_5 (-70, 30, -750) r3.5, 6x body 21u
- Stations: hauler (0,10,-450), player spawn (0,0,40), corvette_a
  (140,30,-560), corvette_b (-150,-20,-540), gunship (80,60,-1170); all
  identical to master (spawn-distance-kept claim verified against
  master:broadside.rs).
- Corridor hauler -> corvette midpoint (-5,5,-550), half-width 120u, t in
  (0.05,0.95) after clamping: boulder_1 IN (perp 94.3u, t 0.647), boulder_2
  IN (perp 106.0u, t 0.856), boulder_3 out (perp 40.6u but t clamps to 1.0),
  boulders 4/5 out. Count = 2 (needs >= 2). PASS.
- Corridor hauler -> gunship spawn: all five IN - b1 81.8u/t0.110,
  b2 119.1u/t0.092 (marginal vs the 120u half-width), b3 34.2u/t0.171,
  b4 102.3u/t0.364, b5 102.7u/t0.401. Count = 5 (needs >= 2). PASS.
- Worst-case 6x body clearances (distance minus 6x nominal), all positive;
  tightest per station: hauler 90.5u (b1), player spawn 543.5u (b1),
  corvette_a 40.8u (b1), corvette_b 21.8u (b2, tightest overall - just above
  the test's 20u floor), gunship 426.0u (b5). PASS.
- Boulder-vs-boulder 6x gaps all positive; tightest b1-b3 = 41.7u. PASS.
- Scatter box: all five boulder centers at z <= -520 < -430 (outside the
  box); worst boulder-body-to-box-edge gap 66u vs max 24u chaff poke-out,
  so chaff and anchors can never merge. PASS.
- Gunship -> hauler distance 726.2u ("~720u burn" claim checks out; the old
  "~1177u approach" figure was gunship -> player, now 1214.1u at part-two
  spawn since the player restarts at z=40).
- Docs claims audited: 5 boulders (CHANGELOG "five" correct), nominal
  3.5-5 = 12-30u bodies correct (12.25 at 3.5x floor), "same seed" verified
  byte-equal, part one id "broadside" unchanged, shakedown chains into
  "broadside" (shakedown_run.content.ron:1250).

Act-machine walk (every handler in both generated files):

- Part one: OnStart (act 0 seed, no gunship anywhere in the file or the
  drained world - also pinned by test); OnEnter act==0 -> act 1 ambush;
  two ungated corvette OnDestroyed flag-setters (idempotent flags, act gate
  on the consumer makes double-fire safe); OnUpdate act==1 + both flags ->
  act 2 + Victory + NextScenario("broadside_gunship", linger: true) - the
  checkpoint, one-shot via the act gate; hauler OnDestroyed act<2 soft-fail;
  player OnDestroyed act<2 -> Defeat + retry "broadside" (itself). No
  handler can fire post-win.
- Part two: OnStart seeds act 1 and stages player + hauler + scatter +
  boulders + gunship; gunship OnDestroyed act==1 -> act 2 + Victory, NO
  queued next; hauler OnDestroyed act<2 soft-fail; player OnDestroyed
  act<2 -> Defeat + retry "broadside_gunship" (itself, THE checkpoint).
- Stale-numbering grep: only hits are broadside_assault.rs:234 (R1.4) and
  historical task records; no live code or data references act 3.
- examples/19_broadside.rs: stage 8 gates on act==2 AND Victory outcome AND
  overlay entity together (act 2 in part one is only set by the checkpoint
  handler), hard-asserts the queued id is "broadside_gunship", then rides
  Continue (Outcome Primary Button). Stage 9 cannot fire early: before
  teardown outcome==Victory blocks it, during load act is absent, and the
  kill target "gunship" exists only in part two; after OnStart act==1
  matches the seeded value. No vacuous stage: every stage needs positive
  world state, and guard_script_completion still panics on AppExit with
  done=false (stage 11 is the only setter). Compiles under
  `cargo check --example 19_broadside --features debug`.
- Constraint sweep: diff touches no crates/nova_gameplay files and no
  sections catalog - AI, weapon stats, and damage-to-player untouched.

Regression -> catching test:

- Gunship spawn re-added to part one:
  breaking_both_corvettes_declares_the_chapter_checkpoint (data-level "no
  gunship spawn action" assert + drained-world entity scan).
- Checkpoint not queueing part two / linger dropped: same test (queued id +
  linger asserts); the example's stage-8 assert catches it live.
- Player-death gate loosened back to act<3:
  player_death_after_the_win_declares_nothing (seeds act 2, expects no
  outcome and nothing queued - would see Defeat). Hauler gate: UNPINNED
  (R1.3).
- Retry cross-wired to the other part:
  player_death_retries_the_current_part_only (asserts each part requeues
  ITSELF, lingering).
- Part-two win queueing a next:
  killing_the_gunship_declares_victory_with_no_queued_next.
- Boulders made destructible / moved off-lane / overlapping:
  hard_cover_anchors_both_threat_lanes (filters on invulnerable, corridor
  count >= 2 per lane, 20u station floor, 6x pairwise gap > 0, z < -430).
- Part two unhidden or OnStart understaged:
  the_gunship_part_is_hidden_and_stages_itself.
- Dropped from the bundle: base_bundle_ships_broadside +
  base_bundle_ships_exactly_the_generated_files.
- Hand-edited RON: committed_content_matches_builders (and reviewer-run
  gen_content produced zero diff: `git status --porcelain` empty before and
  after the run).
- Shakedown chain-in broken: story_chain_declares_outcomes_at_both_ends.

Verbatim test/lint results (run by the reviewer in this worktree):

- content_ron_parity:
  `test base_bundle_ships_exactly_the_generated_files ... ok`
  `test committed_content_matches_builders ... ok`
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- broadside_assault:
  `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- ledger_ch2_encounter:
  `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s`
- content_lint:
  `WARN  [the-ledger] scenario 'ledger_ch4_the_buyer': object id 'auditor' is spawned by more than one handler - fine only if the handlers are mutually exclusive`
  `content_lint: clean (1 warning(s))` (pre-existing ch4 warning only)
