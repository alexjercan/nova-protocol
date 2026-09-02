# Explore an ionized wake for the railgun slug

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.13.0,weapon,vfx,example,exploration

The railgun slug already has a pale-blue emissive dart and a frame-rate-aware
stretched tracer. Its charge glow and muzzle flash use the same blue hardware
palette. What is missing is a wake that lets the shot leave energy in the
frame: ionized rail or sabot material, not atmospheric smoke.

This starts as an exploration. Do not add trail fields to
`RailgunSectionConfig` or the authored content format before the look is proven.
Keep the controls local to the example. If the exploration finds a convincing,
affordable effect, expand this same task with the selected production design,
evidence, and acceptance criteria before integrating it into the weapon.

## Direction

The candidate look is:

- a bright blue-white slug core;
- a short moving blue point light that throws a highlight on nearby hulls;
- a soft cyan haze left in world space, expanding and fading quickly;
- sparse violet-blue filaments moving through the haze, giving an electrical
  rather than smoky reading;
- an uneven outline around the existing tracer, never a solid beam or laser.

The likely implementation is two small shared Hanabi effects because the two
halves need different orientation: camera-facing soft-dot billboards for haze,
and velocity-oriented streaks for the electrical filaments. The existing solid
tracer remains the path backbone. Particle effects stay behind
`GraphicsBudget::particles`.

A real travelling point light is intentional. The shipped lance has a long
reload and normally puts only one slug in flight. Still respect the shared
light budget and the Low preset: several ships or modded cadence can invalidate
the one-light assumption. Note that the existing torpedo light is only a brief
ignition flash, not a light that follows the torpedo for its full flight.

## Phase 1: visual bench

Add a dedicated human-operated example under `examples/playable/`, built with
`AppBuilder`. It is a tuning bench, not a `systems/` correctness range.

The bench should:

- repeat projectiles without waiting through the production charge and reload;
- show parallel lanes at representative speeds, initially 250, 750, and the
  shipped 1500 u/s;
- place dark hull plates near the lanes so the moving point light is judged by
  what it illuminates, not only by its visible core;
- provide example-local controls for particle lifetime, emission density, haze
  width, filament intensity, point-light intensity, and point-light range;
- provide pause and slow motion if needed to inspect the wake between frames;
- use a fixed comparison camera suitable for screenshots or a short capture;
- label each lane and display the active tuning values.

Compare at least these wake policies:

1. Fixed lifetime: physical wake length grows with projectile speed.
2. Fixed distance: the same length at every speed.
3. Time-based lifetime with a maximum world-length clamp.

The current recommendation is the third policy. For reference, a 0.10 second
wake is 25 units at 250 u/s and 150 units at 1500 u/s. The bench must make that
trade visible rather than selecting constants from arithmetic alone.

## Exploration questions

Record answers and visual evidence here:

- Does the haze read as ionized material in vacuum rather than smoke?
- Do sparse filaments read as electrical discharge without becoming noise?
- Does the wake remain continuous at 1500 u/s instead of becoming clusters at
  render-frame positions?
- Does the effect preserve the slug as a kinetic projectile rather than making
  it look like a beam weapon?
- Which policy and values read at both close-pass and normal combat framing?
- Does the point light improve nearby interaction enough to justify its cost?
- How should a moving light acquire and release a slot from the shared cap?
- What remains legible on Low when particles and dynamic lights are absent?

## Evidence

For the exploration:

- run the example at all three speeds and inspect it at normal speed and slow
  motion;
- capture one fixed-camera comparison of the leading variants;
- inspect High, Medium, and Low graphics behavior;
- record the chosen values or the reason no candidate should ship;
- if a candidate advances, use `loop_vfx_range` for repeatable before/after
  rendering and frame-time evidence before production integration.

Do not claim a performance result from one live fight. Record repeat-set probe
evidence against the same revision and hardware reference. Do not assert frame
timing.

## Initial done condition

The example makes the speed and wake-policy differences directly comparable,
the observations and captures are recorded on this task, and the task ends in
one explicit decision:

- expand it with a selected production effect and integration criteria; or
- close the exploration with the reason the wake should not ship.
