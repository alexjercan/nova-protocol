# HUD health percent rounds a living sliver to 0% - ceil sub-1% so alive never reads dead (bcs health_display)

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.7.0,bug,hud,ui


## Goal

Sibling finding of the ghost-ship investigation (20260716-162701): bcs
`health_display.rs:96` computes `(current / max * 100.0).round()`, so a ship
with a barely-alive section (e.g. a 0.4 hp sliver on a 230 max aggregate)
displays 0% while alive and targetable - which may be exactly what the
playtest report saw ("survived with 0hp"). Fix upstream in
bevy-common-systems (~/personal/bevy-common-systems): sub-1% health ceils to
1% (alive never reads dead; zero still reads 0), add the unit test there,
bump nova's pinned rev.

## Notes

- Cross-repo: bcs change + rev bump in nova's Cargo.toml, the established
  upstreaming pattern (e.g. rev 4c81117).
- The structural half of the report (unmarked 0-HP root) is fixed by
  20260716-162701's backstop; this task closes the display half.
- Priority 50: slotted with the v0.7.0 release tasks (below the AI/content
  strand heads, above the polish tail) per the AGENTS.md tagging rule.

## Widened (review R1.3 of 20260716-162701)

Also guard the NaN half: the ship-root aggregate writes Health{0,0} on a
section-less root, and health_display divides current/max - "NaN%" during
the death window. Fix both in the same bcs pass: max <= 0.0 renders as 0%
(dead), and 0 < percent < 1 ceils to 1% (alive never reads dead). Nova's
own hud/torpedo_target.rs:289 already has the max<=0 guard to mirror.
