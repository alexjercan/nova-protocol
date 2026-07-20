# Base campaign polish + extension: make Shakedown to Broadside longer and more interesting (more beats/acts, pacing, encounters)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.8.0,content,scenario,playtest

## Story

As a player starting New Game, I want the base campaign to carry me through a
fuller arc - more encounters, clearer stakes, story between the fights - so
that finishing it feels like completing a short campaign rather than sampling
two scenarios.

Today the base storyline is Shakedown Run (intro tutorial) -> Broadside (a
two-scenario capital fight: hauler distress + corvette ambush, checkpoint,
then the torpedo gunship). It is short. Make it longer and more varied without
adding new engine features (data/scenario work only, per the v0.8.0
no-new-features rule) - v0.7.0's authoring stack (Outcome frames, StoryMessage
comms, arrival grace, checkpoints via chained scenarios, `scenario_elapsed`
timed beats, invulnerable cover, allegiance) is the toolbox.

## Steps

- [ ] Playtest the current base chain start to finish and note the weak beats
      (pacing lulls, difficulty cliffs, samey encounters, thin narrative);
      write the findings into this task before authoring.
- [ ] Sketch the extended arc (beats per scenario, enemy comp, where the
      checkpoints land) as a beat sheet following the documented convention,
      and get a nod on it before building all the content.
- [ ] Add/extend scenarios or acts so the campaign has a fuller arc: more
      encounter variety (mixed enemy comp, environmental beats like asteroid
      cover or a gravity well), clearer stakes, and comms/objective beats that
      tell a story between fights. Reuse existing actions/events only.
- [ ] Retune balance so win/lose feels earned; keep every fight winnable and
      losable. Use `content lint` balance findings as the floor; ack intended
      drama in balance_acks.ron with reasons.
- [ ] Give new scenarios picker thumbnails (ties to 20260715-220011) and wire
      them into the New Game progression + Scenarios picker.
- [ ] Run `content lint` (references + balance in one pass) over the result; fix findings.
- [ ] Sync the docs surfaces in the same task (per AGENTS.md): player wiki
      scenarios.md flow description, CHANGELOG entry; note anything for the
      v0.8.0 news post.

## Definition of Done

- The base chain is at least one full scenario/act longer than v0.7.0's, with
  no two consecutive encounters sharing the same composition and shape.
- Every scenario in the chain has an Outcome path for both win and lose, a
  checkpoint structure that never replays more than one fight on death, and
  comms beats following the beat-sheet convention (lint clean).
- `content lint` passes, balance findings included (acks only with reasons).
- scenarios.md and CHANGELOG reflect the new chain; playtest questions for the
  owner are listed in this task, not silently decided.

## Notes

- Base scenarios: `assets/base/scenarios/*.content.ron` are GENERATED - edit
  the `nova_assets` builders and run `content gen`, never the .ron directly
  (LESSONS.md: edit-the-builder-not-the-generated-ron).
- Feel/balance is ultimately the user's call; deliver the content + a first
  tuning pass, flag playtest questions.
- Menu ambience scenes are separate and out of scope here.
