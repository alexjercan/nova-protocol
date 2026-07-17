# Review: Comms pacing queue

- TASK: 20260717-163033
- BRANCH: feature/comms-pacing-queue

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/comms_panel.rs:179 (with
  crates/nova_scenario/src/world.rs:76 and
  crates/nova_scenario/src/loader.rs:595) - the teardown reset pin does not
  survive a same-window clear+repush, and the shipped campaign hits it on
  every early retry. The chain: `on_load_scenario` tears the old scenario
  down (`world.clear()`, loader.rs:603) and fires `OnStartEvent` in the SAME
  observer (loader.rs:797); next frame the event chain runs
  `queue_system -> state_to_world_system` (bcs events, chained), so an
  OnStart `StoryMessage` lands in the log BEFORE the sync ever observes the
  empty state. The sync's diff is length-only (`feed.0.len() !=
  story.len()`, world.rs:76), and the panel's reset is `feed.0.len() <
  queue.seen` (comms_panel.rs:179). Consequences by case, where K = old feed
  len, M = lines pushed in the first post-clear window:
  - M == K: the sync never writes (len equal), `enqueue_new_lines` never
    runs, `seen == len`, and the new scenario's opening line NEVER displays.
    Deterministic repro: pause-Retry or defeat-Retry (nova_menu lib.rs:598
    re-triggers `LoadScenario`) during a scenario's opening beat - K == M ==
    1. The example arena and all five ledger chapters open with an OnStart
    `StoryMessage`, and ledger ch1 ships a player-death Defeat + retry loop,
    so this is the campaign's most common path. Note the panel cannot fix
    this by content-compare alone: on retry the content is IDENTICAL.
  - M > K (a chained scenario pushing more lines in the first window than
    the whole previous log): the sync replaces the feed, no reset fires
    (len >= seen), and `skip(seen)` silently drops the new scenario's first
    K lines; a still-Showing old line also survives into the new scenario -
    exactly the leaked-line class the pin exists for.
  - M < K works (len < seen resets, then enqueues from 0).
  Suggested change: clear `StoryFeed` (guarded on non-empty, like the
  `GameObjectives` reset for this exact aliasing class, task 20260716-214338)
  in `teardown_scenario_entities` + its two observers - nova_scenario already
  depends on nova_gameplay and `StoryFeed` is in its prelude. That makes the
  length-only sync diff sound again (append-only + explicit reset) and makes
  the panel's `len < seen` reset fire on every teardown. Belt-and-braces:
  content-compare in world.rs instead of len-compare. Add a retry-shaped
  regression test (trigger `LoadScenario` twice around an OnStart story
  line, assert the feed re-flags / the line re-enqueues).
  - Response: fixed teardown-side as suggested - teardown_scenario_entities
    clears StoryFeed (mirroring the GameObjectives precedent), both callers
    wired. The regression pin (scenario_switch_replaces_an_equal_length_
    story_feed) uses the true SAME-WINDOW shape - trigger + equal-count
    repush before any sync frame - and was proven red under a sabotage that
    reverted the clear (a first, looser version of the test survived the
    sabotage and was tightened; recorded in NOTES).

- [x] R1.2 (MAJOR) tasks/20260717-163033/TASK.md:56 /
  crates/nova_scenario/src/world.rs:284 /
  crates/nova_scenario/src/actions.rs:998 - the close-out record claims a
  test that does not exist. TASK step 6 ticks "sync carries dwell
  (nova_scenario --features serde)" and NOTES says "98 + 1 green (sync
  carries dwell; lint warn test)", but every `dwell` in nova_scenario's
  tests is `None`: `story_messages_sync_clear_and_tolerate_a_missing_feed`
  pushes `dwell: None` and asserts only speaker/text, and
  `story_message_ron_round_trips` (actions.rs:998) round-trips the legacy
  shape with no dwell. Net effect: no test carries a `Some` dwell through
  the world sync, and the exact author-facing syntax the wiki documents
  (`dwell: Some(12.0)`, strict RON) is never parsed anywhere in the suite.
  The lint-warn test does exist (lint.rs `story_dwell_out_of_range_warns`).
  Suggested change: extend `story_message_ron_round_trips` with an authored
  `dwell: Some(12.0)` string (parse + assert `config.dwell == Some(12.0)` +
  round-trip), and give the world.rs sync test a `Some` dwell asserted on
  the resulting `StoryLine`. Then the ticked step is true.
  - Response: fixed - story_sync_carries_the_authored_dwell (world.rs) and
    the documented Some(12.0) strict-RON syntax + omitted-default now parse
    in story_message_ron_round_trips (actions.rs).

- [x] R1.3 (MINOR) web/src/wiki/dev/guide-author-scenario.md:355 - the
  authoring guide now contradicts the code and the ticked docs step. TASK
  step 7 claims a "guide-author-scenario mention", but the branch never
  touches the file, and its StoryMessage section still describes the
  REMOVED behavior: "a new line replaces the previous one and rewinds the
  clock" (latest-wins). It also omits the new `dwell` field. Suggested
  change: rewrite that paragraph to the queue semantics (arrival order, 8s
  dwell / 4s yield floor, 4-line drop-oldest pending cap) and show the
  `dwell: Some(...)` field, or at minimum link the new "Story pacing"
  section in scenario-system.md.
  - Response: fixed - the guide's StoryMessage section now documents the
    queue semantics and dwell syntax; the latest-wins sentence is gone.

- [x] R1.4 (MINOR) crates/nova_gameplay/src/hud/comms_panel.rs:224 and 267 -
  two cosmetic fade artifacts from the tween-absence lifecycle:
  (a) One-frame full-alpha flash on the first show: the Idle pop flips
  `Visibility` immediately (line 224), but the fade-in tween is
  command-inserted (flushed at end of frame) and first advanced next frame,
  and `apply_comms_fade` writes nothing without a tween - so the first
  rendered frame shows the panel at its spawn colors (full theme alpha),
  then snaps to ~0 and fades in. After a teardown cancels a fade mid-flight
  the colors freeze at that stale alpha and the next first-show flashes at
  it.
  (b) The final tween frame is never applied: `advance_tween` completes the
  tween (value() does clamp to end exactly - bcs `advance` clamps elapsed at
  duration) but removes it via `Commands`, and the ordering edge
  `apply_comms_fade.after(TweenSystems::Advance)` makes Bevy auto-insert a
  sync point on that edge (predecessor has deferred params), so the removal
  is flushed BEFORE `apply_comms_fade` runs on the completion frame. The
  last APPLIED alpha is the previous frame's: ~0.996 at 60fps, ~0.98 at
  30fps for the QuadraticOut fade-in. Imperceptible against the translucent
  panel, so cosmetic - but the "lands exactly at 1.0" assumption is false.
  Suggested change (fixes both): fade-in uses `TweenOnComplete::Keep` so the
  kept tween holds `value() == 1.0` and `apply_comms_fade` pins full alpha
  for the whole dwell (Showing's fade-out insert overwrites the kept
  component; FadingOut's absence-detection only concerns the fade-out, which
  keeps Remove; the teardown branch already removes `Tween<f32>`); and write
  the alpha-0 colors directly at pop time to kill the first-frame flash.
  - Response: fixed - pop writes alpha-0 colors before the fade-in (no
    first-frame flash) and the fade-in keeps its tween (Keep) so the end
    value 1.0 stays applied; the fade-out's Remove remains FadingOut's
    transition edge and its sub-1.0 residue vanishes behind Hidden.

- [x] R1.5 (NIT) crates/nova_scenario/src/lint.rs:258 - the clamp range is
  hardcoded (`3.0..=30.0`) in the lint, duplicating comms_panel's private
  `COMMS_DWELL_MIN_SECS`/`COMMS_DWELL_MAX_SECS` (and the wiki's "[3, 30]").
  nova_scenario already depends on nova_gameplay; exporting the two consts
  from the panel (or a shared `pub const` pair next to `StoryLine`) and
  referencing them in the lint keeps the three copies from drifting.
  - Response: fixed - the clamp constants are pub in comms_panel's prelude
    and content_lint warns against the SAME values it clamps to.

### Verification record

Run from /home/alex/.cache/sprouts/nova-protocol/feature/comms-pacing-queue
on feature/comms-pacing-queue (2026-07-17):

- `cargo test -p nova_gameplay hud::` - ok. 138 passed; 0 failed
  (411 filtered out); includes the 5 new/rewritten comms tests and the 3
  color-aware objective-feedback tests. The burst test is a genuine
  fail-first vs the old latest-wins panel.
- `cargo test -p nova_scenario --features serde` - ok. 98 passed +
  1 passed (integration) + 0 doc-tests; 0 failed.
- `cargo run -p nova_assets --bin content_lint` - "content_lint: clean
  (1 warning(s))"; the single WARN is pre-existing (ledger_ch4 'auditor'
  multi-handler spawn), unrelated to this branch. No dwell warnings.
- `cargo check --workspace --all-targets` - Finished (only the pre-existing
  proc-macro-error2 future-incompat note).
- `cargo fmt --check` - clean.
- Content grep: no shipped `StoryMessage` uses `dwell` yet (assets/,
  webmods/); the only "dwell" hit is the unrelated lock_dwell_ring.wgsl.
- `TweenPlugin`/`Tween` grep: bcs tween appears nowhere in Nova outside
  this branch's comms_panel.rs + hud/mod.rs - NOTES' "first Nova adoption
  of the tween stack" claim is accurate. Placeholder audio (CommsLine ->
  ui_toggle.wav) is disclosed in both audio.rs and NOTES; the
  placeholder-art task 20260716-205214 covers visuals only, so recording it
  in-task was the right branch of the TASK step.
- Pause semantics (reviewed, no finding): the dwell timer and the tween
  advance both run on `Res<Time>` in Update (virtual time), and
  nova_menu's `pause_clocks` pauses `Time<Virtual>` - a showing line
  freezes mid-dwell/mid-fade under the pause and outcome overlays and
  resumes after, which is the sensible behavior (the player never loses
  read time to a pause).
- State machine walk (reviewed, no further finding beyond R1.1/R1.4):
  yield-during-fade-in is unreachable (min dwell 3s vs 0.25s fade-in, both
  on the same clamped virtual delta) and Showing's fade-out insert
  overwrites any `Tween<f32>` anyway; teardown mid-FadingOut is covered by
  the enqueue->advance chain (enqueue sets Idle immediately, removes the
  tween, and the Idle branch no-ops on an empty queue); a line arriving
  during FadingOut is enqueued before the absence check, so the panel hands
  over without hiding.

## Round 2

- VERDICT: APPROVE

All five Round 1 findings verified against 49dfb22f..a3f156a6 (review diff
5c2403e9..HEAD); no new problems introduced by the fixes. Per finding:

- R1.1 CONFIRMED FIXED. `teardown_scenario_entities` clears `StoryFeed`
  (guarded on non-empty, mirroring the `GameObjectives` precedent), and
  BOTH callers (`unload_scenario`, `on_load_scenario`) pass the new
  `Option<&mut StoryFeed>` through. The regression pin
  `scenario_switch_replaces_an_equal_length_story_feed` uses the true
  same-window shape (synchronous unload trigger, equal-count repush before
  any sync frame, then assert "beta" lands) - and I re-ran the sabotage
  myself: with the feed-clear reverted the test goes RED
  (`test result: FAILED. 0 passed; 1 failed`), restored clean after.
  Structural note on why the fix closes every CURRENT trigger path: the
  event chain (queue + sync) runs in PostUpdate while the panel's enqueue
  runs in Update, so a clear landing in a PreUpdate observer cascade
  (mouse/keyboard Retry via picking / input-focus dispatch) is observed by
  the same frame's Update enqueue before PostUpdate can repush, and a
  NextScenario clear lands inside PostUpdate AFTER that frame's sync, so
  the next Update enqueue sees the empty feed before the following
  PostUpdate writes the new lines. Residual (non-blocking, recorded for
  future-proofing): a hypothetical future trigger whose observer cascade
  flushes BETWEEN Update's enqueue and PostUpdate's queue_system could
  still hide the empty state inside one enqueue interval - content
  comparison cannot catch it (a retry repushes identical text), so if such
  a path ever appears, a teardown-bumped generation counter on `StoryFeed`
  is the robust close. Not reachable today.
- R1.2 CONFIRMED FIXED. `story_sync_carries_the_authored_dwell` (world.rs)
  asserts `Some(12.0)` rides the sync into the `StoryLine`;
  `story_message_ron_round_trips` (actions.rs) now parses the documented
  strict-RON `dwell: Some(12.0)` and asserts the omitted field defaults to
  `None`. The close-out claim is true now (nova_scenario lib count 98 ->
  100).
- R1.3 CONFIRMED FIXED. The guide's StoryMessage section documents the
  queue (arrival order, ~8s hold, 4s yield, 4-line drop-oldest cap), the
  strict-RON dwell syntax with the [3, 30] clamp and lint warn, and the
  latest-wins sentence is gone; the teardown wording ("clears the log AND
  the on-screen mirror... or a retry of the same scenario") matches the
  R1.1 fix.
- R1.4 CONFIRMED FIXED, state machine re-walked with Keep in play: the pop
  writes alpha-0 text/background/border BEFORE flipping visibility (the
  pop frame renders invisible - flash gone, including the stale-alpha
  variant after a cancelled fade); the kept fade-in holds `value() == 1.0`
  so `apply_comms_fade` pins exactly full alpha for the whole dwell
  (undershoot gone). No new stuck states: FadingOut can only ever observe
  the Remove fade-out (Showing's insert OVERWRITES the kept fade-in, so
  tween-absence remains an unambiguous edge), and the teardown branch's
  `remove::<Tween<f32>>` covers a kept tween the same as a running one.
  The fade-out's sub-0.0-residue frame hides behind `Visibility::Hidden`
  or is overwritten by the next pop's alpha-0 write.
- R1.5 CONFIRMED FIXED. `COMMS_DWELL_MIN_SECS`/`COMMS_DWELL_MAX_SECS` are
  pub in the comms_panel prelude and the lint's range check uses them.
  Leftover nit, not blocking: the warn MESSAGE text still hardcodes
  "[3, 30]s" - if the consts ever change, the check will be right but the
  message will lie; interpolating the consts into the format string
  finishes the job.

### Verification record (Round 2)

Run from /home/alex/.cache/sprouts/nova-protocol/feature/comms-pacing-queue
on feature/comms-pacing-queue at a3f156a6 (2026-07-17):

- `cargo test -p nova_gameplay hud::` - ok. 138 passed; 0 failed
  (411 filtered out).
- `cargo test -p nova_scenario --features serde` - ok. 100 passed +
  1 passed (integration) + 0 doc-tests; 0 failed.
- `cargo run -p nova_assets --bin content_lint` - "content_lint: clean
  (1 warning(s))"; the single WARN is the pre-existing ledger_ch4
  'auditor' multi-handler note, unrelated.
- `cargo fmt --check` - clean.
- `cargo check --workspace --all-targets` - Finished, exit 0 (only the
  pre-existing proc-macro-error2 future-incompat note).
- Sabotage re-run (R1.1): reverting the teardown `story_feed` clear makes
  `scenario_switch_replaces_an_equal_length_story_feed` FAIL; the working
  tree was restored to HEAD afterwards (`git status` clean).
