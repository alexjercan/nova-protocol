# Turret crosshair (orange square) twitches while tracking

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,turret,bug


## Goal

Playtest bug (user, 2026-07-10): the turret pointer twitches - the
"orange square" (assumed crosshair) visibly jitters while tracking.
User's hypothesis: the target calculation takes time, so the pointer
lags/recomputes visibly.

## Notes

- Check whether the aim solution is computed in FixedUpdate and rendered
  per-frame (same aliasing family as 20260710-231928/230/231), or whether
  the lead calculation itself oscillates (iterative solver flip-flop).
- Relevant: turret aim/lead code, the crosshair screen indicator.
