# Ledger diagnostic: campaign-wide pace-map + weak-spot brief (diagnostic-first)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.8.0, content, scenario, playtest

## Story

Diagnostic-first (owner clarification 2026-07-22: I derive the pace-map from
the scripts; owner replays the result at Finish). Before authoring any Ledger
change, produce a beat-by-beat pacing/narrative/difficulty map across all five
scenario files (`webmods/the-ledger/ledger_ch1|ch2|ch2b|ch3|ch4.content.ron`)
so the depth/pacing/ending tasks aim rather than guess. Findings drive the
sibling tasks; owner playtest questions are listed, not silently decided.

Umbrella: 20260722-212808 (see GOAL.md). Reference: the Shakedown pacing pass
(20260721-211506) and the beat sheet (`web/src/wiki/dev/guide-author-scenario.md`,
"The beat sheet").

## Steps

- [x] For each of the five files, tabulate: beats/objectives, encounters
      (spawns, waves, loadouts, engage_delay telegraphs), StoryMessage beats,
      and scenario-clock pacing (scenario_elapsed gates / dwell / beat_gate
      breathers vs pure position-gating). Count beats; mark where objectives
      dump on one frame, spawns lack telegraphs, or >1 StoryMessage shares a
      handler.
- [x] Call out the known weak spots concretely against the script: ch1 OnStart
      dumps 1 message + 3 objectives + all spawns; ch3 is one linear
      position-gated act with a single optional ambush and zero clock pacing;
      ch4's Auditor spawns with no engage_delay and both endings converge.
- [x] Write the target rhythm per chapter (announce -> breathe -> arrive ->
      fight -> confirm -> breathe -> next) and the concrete fixes each sibling
      task will make, as the shared reference.
- [x] List the owner playtest questions (feel/balance calls) for the Finish
      checkpoint, in this task.

## Definition of Done

- A per-chapter pacing table + weak-spot list + target rhythm is written into
  this task (NOTES.md), grounded in real handler/variable names. (manual: the
  sibling tasks cite it.)
- Owner playtest questions are listed. (manual: owner reviews at Finish.)

## Notes

Diagnostic only - no scenario edits in this task. Its output is the brief the
pacing/depth/ending tasks implement against.
