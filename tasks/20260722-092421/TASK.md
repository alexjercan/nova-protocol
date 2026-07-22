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

- [ ] Verify-first: catalogue every place a mainline scenario posts an
      objective in the same handler/frame as a story message, or completes an
      objective and posts the next in the same handler. Files:
      crates/nova_assets/src/scenario/{shakedown,broadside,lifeline}.rs.
      Record the list before editing (the pin for "fixed every hit").
- [ ] Promote the gate/breather helpers (stamp_gate, past_gate, breather, and
      the paced_line variant lifeline uses) out of shakedown.rs into a shared
      scenario helper module so broadside and lifeline reuse them instead of
      copy-pasting. Keep shakedown behaviour identical (regression pins).
- [ ] Apply the sequencing to the opening conversations: the objective posts
      only AFTER the conversation's last line, gated on the scenario clock
      (the shakedown open_step/opened handoff is the template).
- [ ] Apply breathing room to objective swaps: on a beat transition, complete
      the old objective and stamp the gate; post the next objective (and any
      between-beat comms) gated past a delay so "completed" and "new" never
      land the same frame. Pick a delay consistent with BREATHER_DELAY.
- [ ] Regen content if any RON is generated from these builders
      (`cargo run -p nova_assets --bin content -- gen`); never hand-edit
      generated RON. Run `content -- lint` clean.
- [ ] Harness coverage: extend the scenario walk tests so each mainline
      scenario asserts (a) no objective is live during the opening
      conversation, and (b) an objective-complete is followed by the next
      objective only after the gate delay, not the same tick.
- [ ] Docs sweep: scenario-authoring dev wiki / beat-sheet guidance already
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
