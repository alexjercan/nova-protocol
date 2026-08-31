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

## Owner notes (2026-08-31)

- Damage: "really really powerful pierce damage that basically goes
  through the entire ship" - one slug should cross the target's whole
  section stack, not stop at the first hull.
- Aiming is the ship: "you have to align the ship to be able to hit" - a
  custom shoot mechanic, not a turret cone. The gun fires where the hull
  points, and the skill is getting the hull pointed.
- Cadence: "takes some time to load, has only one shot every X seconds
  (so it's really slow reload time)" - a charge-up before the shot AND a
  long reload after it. One shell in the air per gun, ever.
- Model: 1x1x3 is fair. The WFC segment chain (`0c64e2cf`) already
  places multi-cell parts, so the shape costs the generator nothing.

## Design round (2026-08-31)

Four mockups fielded on the `railgun-mockups` sprout: twin (1x1x3 open
rails), quad (1x1x2 vented muzzle brake), coil (1x1x3 ring stack),
blade (1x1x2 vertical blade rails). Owner verdict: blend - the twin's
open-rail read with the quad's vented muzzle brake, SINGLE bore, 1x1x3.
The blend (`railgun_lance`) is SETTLED as the main design idea. A
thickness round fielded brake diameters 0.50 / 0.60 / 0.75 of the cell
(0.90 skipped - its clamp yokes would leave the cell box); owner picked
0.60.

Landed as squash `020e3306`: the lance recipe
(`scripts/section-part-recipes/railgun_lance.json`), the staged mockup
(`art/part-candidates/sections/railgun_lance.glb`), and a railgun row
in the default `screenshot_section_gallery` grid - no env switch. The
dropped mockups and thickness variants are off disk; this section is
their record. Design side quest CLOSED - next work here is gameplay.

## Done when

- The railgun is a section a player can place in the editor, charge and
  fire in flight, with model and firing effect (audio may defer to
  `20260824-125955` if that lands later - record it).
- Recoil visibly moves the firing ship.
- At least one scenario or campaign beat features it.
