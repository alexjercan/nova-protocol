# Bug: avian3d 'no mass or inertia' WARN spam when firing bullets

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.7.0,bug,physics,weapons


## Symptom

Firing bullets spams the log with an avian3d warning, once per spawned
projectile:

```
2026-07-16T16:54:09.334744Z  WARN avian3d::dynamics::rigid_body::mass_properties:
Dynamic rigid body 1866v0 has no mass or inertia. This can cause NaN values.
Consider adding a `MassPropertiesBundle` or a `Collider` with mass.
```

The dynamic rigid body spawned for a bullet/projectile carries no mass or
inertia. Beyond the log noise, avian warns this can produce NaN values, so it
is a correctness risk, not only an annoyance.

## Direction (do not implement yet)

- Find the projectile/bullet spawn site (likely a weapons/turret/torpedo
  system spawning a `RigidBody::Dynamic`).
- Give the projectile a defined mass: attach a `Collider` (which derives mass)
  or an explicit `MassPropertiesBundle` / `Mass` so avian stops warning and
  cannot hit the NaN path.
- Follow `collider-needs-a-rigidbody` and `production-faithful-rigs`: assert
  the fix with a rig that spawns a real projectile and checks the computed
  mass is finite/non-zero, not just that the component exists.
- Verify no other dynamic bodies (debris, spawned sections) trip the same
  warning.

## Notes

- Reported by user 2026-07-16 while playtesting (shooting bullets).
