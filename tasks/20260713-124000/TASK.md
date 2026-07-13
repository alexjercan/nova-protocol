# Slot-colored lock language: combat reticle always red, relation tint + reticle pips retired

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,hud,ux,playtest

## Outcome (CLOSED 2026-07-13)

User request (2026-07-13, done directly on master): the on-object lock
bracket should be RED for combat locks and WHITE for travel locks, instead
of the relation tint + the four corner pips. Shipped:

- torpedo_target.rs: the reticle color is baked combat-red into the bundle;
  `update_reticle_relation_tint` + `reticle_color` + the relation test are
  DELETED (this is the user veto the tint was kept awaiting, R1.2 of
  082330), and the four armed corner pips + their two systems are deleted
  too - a visible combat reticle already IMPLIES weapons-hot (lock => hot,
  the safety truth table), so the pips said nothing the red bracket does
  not. The raised-manual hot cue stays on the lead pips; the rich state
  stays on the inset frame.
- edge_indicators.rs: the lock arrow follows suit - constant combat-red
  (it used to mirror the relation tint); `arrow_color` loses its relation
  param.
- New pin `the_reticle_is_always_combat_red`; the relation-tint test
  deleted with its system. Stale doc lines swept (lock_crosshairs module
  doc, EdgeIndicatorKind::Lock).
- Trade recorded: the reticle no longer distinguishes hostile / own /
  neutral targets - the user's slot-color language wins; relation is
  still visible in the readout name and the world itself.

Verified: nova_gameplay lib suite green, fmt, 12_hud_range autopilot
exit 0 (its reticle assert is size/drift-based; the inset armed-tick
assert is untouched - the inset frame keeps its ticks).

## Notes

- Supersedes the reticle half of Q5a (spike 20260713-110039): the
  shape+color hot redundancy now lives on the inset frame only; on-object
  elements are slot-colored.
- Requested "directly on master" - no worktree cycle; single commit.
