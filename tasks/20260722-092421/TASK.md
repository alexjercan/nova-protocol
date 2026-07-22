# Sequence objectives after conversations + breathing room between objective-complete and next objective (mainline scenarios)

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.8.0, content, scenario, pacing, playtest

## Story

Playtest verdict (owner, 2026-07-22): message and objective timing collide.
Two concrete complaints:

1. During the initial Capt./player conversation an objective is already
   showing. The objective should appear AFTER the conversation finishes, not
   in parallel with it. Same wherever a scripted conversation precedes an
   objective.
2. "Objective completed" and the next objective pop in the same instant, and
   sometimes a between-beat conversation runs in parallel with the swap. The
   owner wants a timeout / breathing-room beat: complete objective -> (pause,
   optional conversation) -> new objective, sequenced, never simultaneous.

The shakedown scenario already solved this once (task 20260721-211506) with a
clock-gate + breather pattern (stamp_gate / past_gate / breather over the
`scenario_elapsed` engine variable). The other mainline scenarios (broadside,
broadside_gunship, lifeline) still post objectives at OnStart / on transition
in the SAME handler as the story line. This task promotes the pattern to a
shared helper and applies the sequencing across the mainline.

## Steps

- [x] Verify-first: catalogue every place a mainline scenario posts an
      objective in the same handler/frame as a story message, or completes an
      objective and posts the next in the same handler. Files:
      crates/nova_assets/src/scenario/{shakedown,broadside,lifeline}.rs.
      Record the list before editing (the pin for "fixed every hit").
- [x] Promote the gate/breather helpers (stamp_gate, past_gate, breather, and
      the paced_line variant lifeline uses) out of shakedown.rs into a shared
      scenario helper module so broadside and lifeline reuse them instead of
      copy-pasting. Keep shakedown behaviour identical (regression pins).
- [x] Apply the sequencing to the opening conversations: the objective posts
      only AFTER the conversation's last line, gated on the scenario clock
      (the shakedown open_step/opened handoff is the template).
- [x] Apply breathing room to objective swaps: on a beat transition, complete
      the old objective and stamp the gate; post the next objective (and any
      between-beat comms) gated past a delay so "completed" and "new" never
      land the same frame. Pick a delay consistent with BREATHER_DELAY.
- [x] Regen content if any RON is generated from these builders
      (`cargo run -p nova_assets --bin content -- gen`); never hand-edit
      generated RON. Run `content -- lint` clean.
- [x] Harness coverage: extend the scenario walk tests so each mainline
      scenario asserts (a) no objective is live during the opening
      conversation, and (b) an objective-complete is followed by the next
      objective only after the gate delay, not the same tick.
- [x] Docs sweep: scenario-authoring dev wiki / beat-sheet guidance already
      mentions "open with a conversation" - extend it to the objective-swap
      breathing-room rule. CHANGELOG under Scenarios & Objectives.

## Definition of Done

- In every mainline scenario, the first objective appears only after the
  opening conversation completes, and no objective-complete is immediately
  (same frame) followed by the next objective
  (test: scenario walk assertions in nova_assets/nova_scenario;
  manual: owner replays shakedown/broadside/lifeline and the rush is gone).
- Gate/breather helpers live in one shared module, not copy-pasted per file
  (cmd: `grep -rn "fn stamp_gate\|fn breather" crates/nova_assets/src/scenario`
  shows a single definition site).
- CHANGELOG entry (cmd: `grep -ni "pacing\|breathing" CHANGELOG.md`).

## Notes

- Template: shakedown.rs stamp_gate/past_gate/breather + the open_step/opened
  opening-conversation handoff. SCENARIO_ELAPSED_VAR is engine-owned (writing
  it is a lint error); gate off it, do not set it.
- Keep `story` dwell defaults; the sequencing is about WHEN the objective
  posts, not message dwell.

## Fix (2026-07-22)

Verify-first catalogue of same-frame story+objective (or complete+objective)
handlers across the mainline, all now fixed:
- shakedown OnStart posted a "stand by" HOLDING objective during the ~40s
  opening conversation; the scavenger reveal posted OBJ_B12 in the same
  handler as its warning line.
- broadside OnStart posted the distress line + OBJ_CONTACT together; the
  ambush posted the "they're here" line + OBJ_DEFEND together.
- broadside_gunship OnStart posted the Rust Tally taunt + both objectives.
- lifeline OnStart posted the Belt Relay dispatch + OBJ_SCREEN together.
- final_tally OnStart posted the dispatch + OBJ_SURVEY; the survey confirm
  posted OBJ_PICKET with its line; the cast-off posted OBJ_BREAK with the
  "that's the flagship" reveal.

Design: two duplicate gate mechanisms existed (shakedown's
`stamp_gate`/`past_gate`, final_tally's `mark_clock`/`clock_past`) plus
lifeline's `paced_line`. Unified into a shared `scenario/pacing.rs`:
`mark_clock(gate, delay)` stamps a clock deadline, `clock_past(gate)` gates on
it, and `gated_once(done, gate, extra, actions)` is the one-shot OnUpdate that
posts the deferred objective. A `gate > 0` guard prevents firing before the
deadline is stamped (an unread var reads 0, the clock starts at 0). shakedown
and final_tally were refactored onto the shared primitives (behaviour-preserving
- the `+delay` just moved from the filter into the stamp; verified in the RON
diff and the shakedown walk tests).

Owner decision (questionnaire, 2026-07-22): the Shakedown opening panel stays
EMPTY during the conversation (no holding objective), the real objective posts
at the hand-off. The holding OBJ_OPENING was removed entirely.

Coverage:
- Two exhaustive cross-scenario invariants in scenario.rs tests, over all five
  mainline configs: `no_mainline_handler_posts_an_objective_alongside_a_conversation`
  (no handler posts a StoryMessage AND an Objective) and
  `no_mainline_scenario_posts_an_objective_at_onstart` (empty opening panel +
  a deferred objective post exists). Plus
  `opening_objectives_are_deferred_past_frame_one` (the opening objective is
  clock- or conversation-latch-gated).
- Behavioural end-to-end: shakedown's `the_five_beats_walk_end_to_end` and
  `an_early_derelict_kill_skips_to_the_fight` now assert the scavenger
  objective is ABSENT right after the warning and posts only after the clock
  passes the deadline; the opening-conversation walk asserts the panel is empty
  during the briefing and holds exactly one objective at hand-off. All exercise
  the shared gated_once/mark_clock/clock_past primitives.
- Content regenerated (`content -- gen`) and lint clean (0 errors). CHANGELOG
  under Scenarios & Objectives; authoring guide beat-sheet updated to the
  objective-after-conversation rule and the shared pacing toolbox.

Scope note: shakedown's beacon-to-beacon nav swaps (complete + next objective,
NO conversation) stay instant - continuous waypoint flight where delaying the
objective from the beacon spawn would desync target and text; the breather
comms still follows. The invariant enforced is "no objective shares a frame
with a conversation", which those swaps satisfy. Probe not run: data-only
pacing, no new mechanics or perf surface; walk tests + lint cover correctness.
