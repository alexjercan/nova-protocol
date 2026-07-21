# Spike: the Tab ship-computer drawer - objectives, comms log, 3D minimap, what else?

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,spike,ui,hud

## Goal

Owner direction (playtest, 2026-07-21): a "Tab" ship-computer DRAWER - an
expandable right-side surface on a keybind that opens with more detail,
potentially pausing the game and enabling the cursor. Candidate contents:
expanded objectives, the full comms/conversation log, a 3D minimap, and
"other cooler things" - this spike exists to find them and to design the
interaction model (keybind, open/close animation, pause semantics, cursor,
how the diegetic objective animation lands INTO the drawer tab, how comms
history renders).

v0.9.0 candidate (this sprint is no-new-features); parked in backlog until
v0.9.0 planning pulls it. Related follow-up tasks that ride this spike's design:
20260721-211520 (diegetic objectives), 20260721-211526 (comms stack).

Owner questionnaire answers (2026-07-21):

- CONTENTS - all four are core: objectives detail, full comms log, 3D
  minimap, ship status/damage. The spike still explores extras beyond
  these.
- BEHAVIOR: opening the drawer PAUSES the game and frees the cursor
  (menu-like browsing); builds on the cursor state machinery from
  20260721-211500.
- KEYBIND: Tab ONLY (no O shortcut - O stays free; note ORBIT already
  uses O in flight, so Tab avoids the collision entirely).

Spike output per the spike skill: SPIKE.md here + seeded v0.9.0 tasks.
