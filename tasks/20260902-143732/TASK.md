# Explore an ionized wake for the railgun slug

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.13.0, weapon, vfx, example, exploration

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

## Phase 1 result

The bench is `examples/playable/railgun_wake_bench.rs`.

```text
cargo run --example railgun_wake_bench --features debug
DISPLAY=:99 NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir> \
  cargo run --example railgun_wake_bench --features debug
```

Three lanes (250, 750, 1500 u/s) fire the production slug on a 240 u range
with six dark hull plates and a deck strip under each lane. Keys: `1`-`3`
policy, arrows tune, `[` `]` slow motion, `P` pause, `H` `J` `L` layers and
light, `T` spread, `G` quality, `C` camera (wide, chase, close pass, free).
The readout prints every value in meters.

Captures in this folder, from the autopilot walk at a tenth speed:

- `railgun-wake-fixed-lifetime.png`, `railgun-wake-fixed-distance.png`,
  `railgun-wake-clamped.png`: wide comparison of the three policies.
- `railgun-wake-chase.png`: the pilot's view of a volley leaving.
- `railgun-wake-close.png`, `railgun-wake-close-haze.png`,
  `railgun-wake-close-filaments.png`: the close pass, then each layer alone.
- `railgun-wake-close-medium.png`, `railgun-wake-close-low.png`: the same
  frame on Medium and Low.

Answers:

- Haze reads as ionized material, not smoke, once it is dense enough: below
  about 6 particles per unit it reads as a string of beads. It expands in
  place, drifts under 3 u/s in no preferred direction, and fades cold blue.
- Filaments read as discharge from the side. End-on (the chase pose) they
  vanish: a velocity-oriented streak has no thickness along the view axis.
  0.2 u thick, 0.7 of the haze lifetime, a quarter of its count, strobing.
- The wake stays continuous at 1500 u/s because each frame's particles are
  spread along the ground the slug covered since the last spawn and born that
  much older. `T` shows the row of puffs a point spawner draws.
- The slug stays a projectile: the dart leads, the wake is behind and thins
  over its lifetime. Nothing is drawn ahead of the slug.
- Fixed lifetime reads best. The length growing with speed is the point: a
  0.5 s wake is 1.25 km behind the slow lane and 7.5 km behind the lance,
  and the lance's is the one that reads as a trail from combat range.
- The light is what sells a close pass: a highlight slides down the deck
  and across each plate. The particles alone do not light anything.
- Slot policy below, under Phase 2.
- Low draws the dart and the tracer only. That is the whole slug there and it
  is still legible.

## Decision

Owner, 2026-09-02: fixed lifetime, 0.5 s. Ship it in the weapon as constants,
no authored fields.

## Phase 2: production integration

Done in the same change:

- `crates/nova_ship/src/sections/railgun_section/wake.rs`: `RailgunWakeTuning`
  (default is the shipped look: 0.5 s, 6 per unit, 1.5 u wide, both layers at
  1x, spread on), `RailgunWakeEmitter`, the shared lazy `RailgunWakeArt`, the
  `RailgunWakeSpawner` system param, `follow_railgun_wakes` (Update) and
  `count_railgun_wake_spawns` (PostUpdate, after hanabi's tick).
- The slug's render observer spawns the wake when `GraphicsBudget::particles`
  is on and asks for the light through `light_railgun_slug`.
- Emitters are their own entities riding the slug's transform, so they
  outlive it: when the slug goes the spawner stops and the emitter lingers
  1.3 lifetimes before it despawns.
- The light is a child of the slug carrying `CappedLight`, a new
  `nova_gameplay::transient_light` marker. The flash cap and the slug both
  count `TransientLight` or `CappedLight` against `transient_lights`, so a
  slug's light takes a slot on the same terms as a flash, is refused when the
  cap is full, and gives the slot back when the slug despawns. The count runs
  in a queued command so a volley sees its own earlier lights.
- Buffers: 8192 haze and 2048 filament particles per emitter. The shipped
  tuning holds 4500 haze behind the lance; the bench's top sliders overrun it
  on purpose, and the overrun shows as a tail that thins.
- The bench drives the same code: it writes its sliders into every live
  emitter's `tuning` and every slug light, and its defaults are the constants.

Proof, 2026-09-02, under Xvfb on llvmpipe:

- `railgun_wake_bench` autopilot walk: nine shots, cycle complete, no panic.
  The captures above are from that run, after the integration, so they show
  the production wake and light on a bench-fired production slug.
- `system_railgun_lance` autopilot: one real shot from a lance, every lance
  invariant held, no panic.
- Unit: six wake tests in `wake.rs` (the spread arithmetic, the layer ratios,
  the buffers holding the shipped wake, both graphs building) and the new
  `a_light_holding_a_slot_counts_against_the_next_flash` in transient_light.

## The range

`loop_vfx_range` fires a lance too: a platform parked four units above the
shooter, bore down -Z, so the slug passes over the target and flies its whole
lifetime into empty sky with the whole wake behind it. A lance on the
shooter's axis would put its slug into the target 36 units on, a tick and a
half of flight, gone before the wake's first frame at a software renderer's
frame rate. The platform carries a hard magazine of three shells (one per
pass, no reload) and is locked in place against the recoil.

`NOVA_VFX_RANGE_BARE_SLUG=1` strips the wake and the light off every slug as
it spawns, so the before and the after are the same cycle on the same
revision.

`vfx-range-lance.png` is one frame of the range's wide loop a tenth of a
second after the first pass's shot: the wake across the frame over the
target, the burst still landing under it.

## Frame time

Two repeat sets of `loop_vfx_range` through the probe (`probe run
loop_vfx_range --release --repeat 5`) on 2026-09-02, on this 24-core box
under the probe's own Xvfb on llvmpipe, five passes each. The revisions
differ only by task files, so both sets ran the same code. The bare set's
passes ran before a `wfc_arena` session started on the box; the wake set's
passes ran after it ended, with a load sampler alongside reading 1.9 to 2.6
and the example as the top process. One wake set whose passes overlapped
that session came back WARN at a 32 ms p99 and is discarded, not averaged in.

| set | revision | admitted | mean ms | median ms | p99 ms | worst ms | refresh cap |
|---|---|---|---|---|---|---|---|
| bare slug | b6aa4289 (stamped e65b5e07) | 5/5 | 15.27 | 15.48 | 24.76 +- 0.13 | 29.87 +- 0.49 | not suspected |
| wake + light | 3384e919 | 5/5 | 15.40 | 15.56 | 23.60 +- 0.11 | 29.70 +- 0.58 | not suspected |

Deltas, wake minus bare: mean +0.12 ms, median +0.08 ms, p99 -1.16 ms, worst
-0.17 ms. Every per-pass p99 of both sets lies between 21.8 and 25.2 ms, and
the wake set sits inside the bare set's spread on every figure, so the wake
and the light cost nothing this method can see on this box. This is a
repeat-set reading on one software renderer, not a frame-timing claim; a GPU
host balances the particle fill against the rest of the frame differently.

Two probe behaviours met on the way, neither a defect in the range: a commit
landing while a set runs makes the aggregate index report ERROR ("holds an
earlier run's checks.json") and exit 1 while the example's own verdict is
OK, and `fps_within_baseline` turns from N/A into a soft WARN once an
earlier run of the same example exists to compare against.
