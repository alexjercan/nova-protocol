# Base campaign polish + extension: make Shakedown to Broadside longer and more interesting (more beats/acts, pacing, encounters)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.8.0,content,scenario,playtest

## Goal

Polish and lengthen the base campaign so it is a more interesting play, building
on what v0.7.0 shipped. Today the base storyline is Shakedown Run (intro) ->
Broadside (three-act capital fight: neutral hauler, corvette ambush, torpedo
gunship). It is short. Make it longer and more varied without adding new engine
features (data/scenario work only, per the v0.8.0 no-new-features rule).

## Steps

- Playtest the current base chain start to finish and note the weak beats
  (pacing lulls, difficulty cliffs, samey encounters, thin narrative).
- Add/extend scenarios or acts so the campaign has a fuller arc: more encounter
  variety (mixed enemy comp, environmental beats like asteroid cover or a
  gravity well), clearer stakes, and comms/objective beats that tell a story
  between fights. Reuse existing actions/events (Outcome, area OnEnter, comms).
- Retune balance against the graphics/perf baseline and the outcome frames so
  win/lose feels earned; keep it winnable and losable.
- Give new scenarios picker thumbnails (ties to 20260715-220011) and wire them
  into the New Game progression + Scenarios picker.
- Run the content lint/audit (20260718-152240) over the result; fix findings.

## Notes

- Base scenarios: `assets/base/scenarios/*.content.ron` authored via the
  `nova_assets` builders -> `content gen`. Menu ambience scenes are separate.
- Feel/balance is ultimately the user's call; deliver the content + a first
  tuning pass, flag playtest questions.

