# Notes

## Result

- A directly depleted ship section is destroyed at any graph degree.
- Surviving section graphs are partitioned after the frame's destruction batch.
- A live controller selects the component that keeps spaceship identity. Total
  surviving maximum health and stable entity order resolve remaining ties.
- Detached components become persistent inert dynamic wreck roots. Their
  sections remain damageable and keep health-derived materials.
- Wrecks can sever again. Empty wreck roots despawn.
- Avian recomputes each new compound body's mass properties before motion is
  restored.
- Linear velocity follows rigid-point velocity at each new COM. Angular velocity
  is preserved. The 1 u/s outward kicks are mass balanced.
- Structural collapse defaults to 5 percent and retains its leaf-first behavior.

## Important fix

The first implementation let both generic leaf handling and ship direct-
depletion handling queue destruction for a depleted leaf. In a rendered damage
run, one command despawned the section and the second command targeted its stale
entity generation. Ship handling now owns depleted non-leaves only. Generic
integrity remains the sole leaf-destruction owner.

## Visual scope

Generic cube destruction debris stays unchanged. Representation-independent mesh
destruction remains in `tasks/20260817-154330/`.
