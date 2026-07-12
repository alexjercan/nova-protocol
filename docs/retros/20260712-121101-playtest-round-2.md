# Retro: Shakedown Run playtest round 2 fixes

- TASK: 20260712-121101
- BRANCH: fix/shakedown-playtest-2 (landed as 8c6a4db)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE)

## What went well

- The knockback fix came from reading the damage observer FIRST: damage
  is computed from masses and velocities, never the solver contact, so
  knockback and damage were separable - sensor bullets keep identical
  damage at zero shove. The user's suggested mass tweak would have
  dragged damage down with the mass; understanding the consumer beat
  following the suggestion literally.
- Round-2 review discipline held: after the previous cycle's no-op-edit
  incident, the reviewer was explicitly told to verify files not claims,
  and did (pair-orientation walk, frame-by-frame test timing,
  re-derived geometry).

## What went wrong

- The sensor-bullet change shipped two collision-event blind spots the
  reviewer caught (one BLOCKER): trigger volumes (Sensor + events) ate
  bullets - the pirate was un-hittable inside a beacon sphere - and
  invulnerable planetoids (no Health -> bcs never enables events -> no
  event on an event-less pair) let rounds tunnel through solid cover.
  Root cause: I changed a collider's CLASS (solid -> sensor) while only
  reasoning about the pair type I was fixing (bullet vs ship). Avian
  tangibility is a 2x2 of (sensor?, events-enabled?) per side, and every
  cell the bullet can meet needed enumerating: solid-with-events (ships,
  crates-as-areas), sensor-with-events (triggers, blasts), solid
  event-less (invulnerable bodies), sensor event-less (other bullets).
- Geometry margins were shipped razor-thin again (3.6u cluster slack,
  zero crate margin) - the same class as the R2.2 factor-band lesson
  one cycle earlier, just at the assert-threshold level.

## What to improve next time

- Changing a physics component class (RigidBody kind, Sensor,
  CollisionEventsEnabled) needs a pair-matrix enumeration: list every
  collider category in the game and check the changed entity against
  each, before tests. The categories exist in this codebase and are
  countable (ships/sections, asteroids, invulnerable bodies, areas,
  beacons, crates, bullets, torpedoes, blast shells).
- Give geometry asserts the same margin discipline as the values they
  guard: an assert passing by 3.6u is one nudge away from a silent
  graze.

## Action items

- [x] LESSONS.md: new `pair-matrix-on-collider-class-change`; margin
      note folded into the authored-vs-derived entry.
