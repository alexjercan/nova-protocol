# Diegetic objective presentation: big on the cockpit HUD, then tucks into the right tab

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,feature,hud,ui

## Goal

Owner direction (playtest, 2026-07-21): objectives should appear
DIEGETICALLY - imagine a HUD cockpit: the new objective appears on it "a
bit rotated and big", holds, then animates away INTO the right tab (the
future Tab drawer's handle), where it lives in the compact list. The
right-tab list gains more detail and expands via the drawer (own spike).

v0.9.0 candidate; the Tab spike (see Notes) owns the family's interaction
design - this task implements the objective-presentation piece once the
spike lands. /plan breaks it into steps at pickup.

## Notes

- Depends on: 20260721-211512 (the Tab drawer spike - design); bcs Tween/UiAnimate plugins
  exist and are the likely animation vehicle (spike 20260717-155740 noted
  them unadopted).
- Owner decision (questionnaire, 2026-07-21): the BIG COCKPIT MOMENT -
  the new objective appears large and slightly rotated on the cockpit HUD
  (~2-3s), then animates into the right tab. Pair with authored pacing
  gaps (20260721-211506 sets the pattern) so it never overlaps a fight.
