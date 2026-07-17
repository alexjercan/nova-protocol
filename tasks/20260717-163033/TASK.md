# Comms pacing queue: ordered story lines, min display, per-line dwell, fades, comms blip, objective flash

- STATUS: CLOSED
- PRIORITY: 39
- TAGS: spike,v0.7.0,hud,scenario,gameplay

Goal: kill the latest-wins story-line bug and make comms readable: queue
StoryMessage lines and display them in arrival order with a minimum
on-screen time (8s dwell stays the default; add an optional per-line
dwell seconds on the action, clamped ~[3,30], syntax documented); fade
in/out and the new-objective gold flash via the UNUSED bevy-common-systems
Tween/UiAnimate helpers (reuse-known-good-stack); a comms blip in the
UiSfx bank per displayed line (the anti-masking pattern from
objective_feedback applies). Queue depth ~4 drop-oldest (decide in-task);
the full log stays in StoryFeed. Spike: tasks/20260717-155740/SPIKE.md.

Verified at plan time: comms_panel.rs renders feed.last() with a fixed 8s
dwell (latest-wins, the spike's core readability bug); StoryLine/StoryFeed
live HUD-side; StoryMessageActionConfig { speaker, text } syncs via
world.rs state_to_world (write-on-diff, teardown empties the feed - the
reset pin must survive); the UiSfx bank maps 4 keys to assets/sounds/
root files; objective_feedback.rs already diffs added ids (green ghosts
exist for completions only); bcs Tween (~/personal/bevy-common-systems/
src/tween) is unused in Nova.

## Steps

- [x] Queue state machine in comms_panel.rs: a display queue DECOUPLED
  from StoryFeed (track a seen-index; new feed entries enqueue, capped at
  4 drop-oldest with the full history still in StoryFeed; an EMPTIED feed
  clears queue + hides instantly - keep the teardown reset pin). Current
  line holds COMMS_DWELL_SECS (8.0) when alone but yields to a pending
  line after COMMS_MIN_SECS (4.0). Lines display in ARRIVAL order.
- [x] Fades via the bcs Tween helper (first Nova adoption;
  reuse-known-good-stack): ~0.25s alpha-in, ~0.4s alpha-out mapped onto
  the panel's text/border/background colors; expiry and yield both fade
  out before the next line shows.
- [x] Per-line dwell: StoryMessageActionConfig gains
  `dwell: Option<f32>` (serde default None; strict-RON syntax
  `dwell: Some(12.0)` documented author-facing); StoryLine carries it
  through the world.rs sync; the panel clamps to [3, 30] at use;
  content_lint WARNs on an authored dwell outside the clamp.
- [x] Comms blip: UiSfx::CommsLine played when a line SHOWS (not when it
  enqueues), reusing ui_toggle.wav as the placeholder file (distinct
  key so real art can swap in; note the placeholder in NOTES + a line in
  the existing placeholder-art task 20260716-205214 if it covers sounds,
  else record here).
- [x] Objective gold flash: mirror the green completion ghosts with
  gold ADDITION ghosts in objective_feedback.rs (the panel rebuilds rows
  on change, so row-level animation is out; the ghost column is the
  established pattern).
- [x] Tests: burst-of-three shows the FIRST line first and plays all
  three in order (the fail-first vs latest-wins: this test fails on the
  old code); solo line holds the full dwell; pending line yields at MIN;
  cap drops oldest; emptied feed hides instantly (existing pin kept);
  dwell override respected + clamped; sync carries dwell
  (nova_scenario --features serde); lint warn on out-of-range dwell;
  gold ghost on posted objective (mirror the green test).
- [x] Docs: dev wiki scenario-system.md StoryMessage section (dwell
  syntax + queue semantics), guide-author-scenario mention; CHANGELOG
  (Interface & HUD + Modding for the schema field); NOTES.md.
- [x] Verify: cargo test -p nova_gameplay hud::comms + hud::objective;
  cargo test -p nova_scenario --features serde; content_lint;
  cargo check --workspace --all-targets; fmt last. Full suite on CI.

## Close-out record

All eight steps landed; design, the cap semantics decision and the
teardown-ghost discovery are in NOTES.md. Verification: 138 HUD tests
green (5 new comms + 3 color-aware objective tests), nova_scenario
--features serde 98+1 green, content_lint clean, workspace --all-targets
green, fmt last. Full suite on CI per standing instruction.

Reflection: the three "broken" objective tests were the new contract
announcing itself - and one of them pointed at a real pre-existing leak
(live ghosts fading over the menu after teardown), which the reset now
closes. Tests that break on an intended contract change deserve a look
for what ELSE they were quietly asserting before being updated.
