# Railgun: a spinal kinetic weapon family

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.13.0,content,ship,weapon

Promoted 2026-08-31 from ideation into v0.13.0: the release's new section.

A new weapon family: a spinal kinetic railgun - charge time, a
hitscan-fast slug, brutal recoil that pushes back through the attitude
model, and a distinct vacuum firing effect. A capital-ship reason to
exist.

## Shape

- A spinal section: it fires along the hull axis, so the SHIP aims it.
  That is a different combat verb from the turrets and the bays, and the
  reason the family earns its place.
- Charge before fire, authored charge time, a readable charge cue.
- Recoil is real: firing applies impulse the attitude model has to absorb.
- Model at the thruster's standard from day one (`20260831-083625` sets
  the bar and decides how a section declares an animation - the railgun
  wants a charge/fire track).
- Author it in the content builders, lint it, and give the AI a firing
  rule or explicitly deny it to raiders for now - record the decision.

## Done when

- The railgun is a section a player can place in the editor, charge and
  fire in flight, with model and firing effect (audio may defer to
  `20260824-125955` if that lands later - record it).
- Recoil visibly moves the firing ship.
- At least one scenario or campaign beat features it.
