# content lint: flag an unpiloted (controller:None) ship parked inside a gravity well's SOI

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Story

Follow-up from the gravity-behavior fix (task 20260722-092427, review MINOR):
gravity wells now pull only PILOTED ships, so an unpiloted (`controller: None`)
bystander floats and never falls into a well. That guarantee is currently only
enforced by the runtime rule, not by content validation. The one place a None
ship sits near a well today (the asteroid_field sandbox) clears the SOI by just
~80u, so a future content nudge could silently place a None ship INSIDE a
well's SOI - where an author might EXPECT it to fall (old behaviour) but it now
floats, or vice versa, with no warning.

Make the guarantee load-bearing: a `content lint` check (or scenario invariant)
that flags a `controller: None` spaceship whose spawn position lies within any
authored gravity well's sphere of influence, so the "bystanders float, never
fall" rule cannot be silently violated by a content change.

Deferred to the backlog - a hardening guard, no observed bug.

## Steps

- [ ] In the content lint (crates/nova_assets, the reference/geometry/balance
      pass), for each scenario compute each authored well's SOI (8x body radius
      per GravitySettings) and flag any `SpaceshipController::None` ship whose
      spawn position is inside it.
- [ ] Decide severity: WARN (with a balance_acks-style ack) vs ERROR. Likely
      WARN - a None ship in an SOI is now well-defined (it floats), just
      possibly surprising to the author.
- [ ] Pin with a lint test: a synthetic scenario with a None ship inside an SOI
      trips the check; the shipped scenarios stay clean.

## Definition of Done

- `content lint` flags an unpiloted ship parked inside a gravity well's SOI;
  shipped scenarios lint clean.
- Deferred: pull from backlog into a real vX.Y.Z tag before scheduling.

## Notes

- Origin: review of 20260722-092427. Key numbers: SOI = soi_factor (8) x body
  radius; asteroid_field rock r=20 -> SOI 160u, its None ship at 240u (80u
  clearance). Gravity opt-in now keyed on PlayerSpaceshipMarker /
  AISpaceshipMarker (gravity.rs).
