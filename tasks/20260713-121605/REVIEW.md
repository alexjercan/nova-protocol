# Review - 20260713-121605 lock-wins turret routing

## Round 1 (2026-07-13)

Traced the four stance x lock states against the new feed: raised+lock ->
lock tiers (component strongest), raised+no-lock -> ray (manual),
lowered+lock -> lock tiers, lowered+no-lock -> ray (rest). All match the
user's directive; the deleted special-case had exactly one observable
behavior (raised+lock -> ray) and that is the one the playtest rejected.

- Grepped for other manual-wins consumers: none - the hint rows that
  would teach the stance are still deferred to 090653 (nothing stale to
  edit), the AI aims through its own path, HOLD_FIRE_DURING_RADAR and the
  press-deny gates are stance-independent.
- The turret-view lead pip now rides the LOCK while raised (it renders
  TurretSectionAimPoint) - that IS the requested behavior made visible;
  noted for the 090653 playtest pass, not a defect.
- The inverted pin fails against the old feed by construction (it
  asserted the ray exactly where the new test asserts the section) and
  covers the tap-clear handover with the lead-velocity delta as the
  delivery guard.
- Docs: the 082207 routing paragraph carries the verdict banner, the knob
  line is marked ANSWERED, the CHANGELOG supersession note updated. D5's
  torpedo/turret asymmetry closes as a side effect.
- 471 lib tests, fmt, 12_hud_range live run re-verified.

VERDICT: APPROVE (round 1).
