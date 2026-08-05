# Review: Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- TASK: 20260805-105154
- BRANCH: master

## Round 1

- REVIEWER: owner
- VERDICT: APPROVE

Reviewed as it was built: each of the six scenes was run plainly (no
`NOVA_REEL`), shown, and approved on its own before the next one started, and
no PNG was captured until all six had passed. The owner then accepted the
packaged set.

- Scene 1 `screenshot_scene` - APPROVED 2026-08-05 (`bb57a9d2` kit, `3b5a715f`
  set).
- Scene 2 `screenshot_combat` - APPROVED 2026-08-05, as a two-act flight
  (travel-lock a beacon, fly the real GOTO leg, its trigger springs the
  ambush).
- Scene 3 `screenshot_flight` - APPROVED 2026-08-05; the autopilot figures moved
  here from the combat leg.
- Scene 4 `screenshot_sections` - APPROVED 2026-08-05; turntable, not a camera
  fly-around, so all five closeups share one key and rim.
- Scene 5 `screenshot_ui` - APPROVED 2026-08-05; whole walk on real pointer
  gestures, each shot asserting the state it claims.
- Scene 6 `screenshot_nova_os` - APPROVED 2026-08-05; terminal plus ship app,
  fidelity beats kept and their orphan captures dropped.
- Packaged set - ACCEPTED 2026-08-05. 29 figures, zero `capturable` gaps, no
  framing rejected.

Carried out of the review, not fixed here: the screen-indicator labels collide
in four shots (`wiki-radar` and `tutorial-radar-lock` stack WAYPOINT over its
own distance readout, `feature-autopilot`'s FLIP label sits under the blip,
`wiki-flight`'s SURVEY chip overlaps the debug fps/version chip). In-game HUD
layout, identical in play, so no reframe fixes it - it wants its own task.
