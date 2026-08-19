# Section density means nothing, and the field is named mass

- STATUS: OPEN
- PRIORITY: 77
- TAGS: v0.11.0,content,physics,ship

Epic: `20260818-220812`. **Blocks `20260819-140314`** (attitude control by
physics) and should probably run in the same lane - the two are one problem seen
from two ends, and the measurement step is shared.

## The root cause is a lying field name

`SectionConfig.mass` (`crates/nova_ship/src/sections/base_section.rs:229`) is
passed to `destructible_body(health, config.mass)` at `:375`, whose signature is
`destructible_body(health: f32, density: f32)`
(`crates/nova_gameplay/src/integrity/health.rs:163`), and lands as
`avian3d::prelude::ColliderDensity`.

**The field is called `mass` and is used as a density.** An author writing
`mass: 350.0` for a cargo pod is being entirely reasonable - they believe they
are setting a mass. It is silently multiplied by the collider volume:

| cargoa part | authored `mass` | volume | actual mass |
|---|---:|---:|---:|
| fuselage | 350 | 6.87 | **2404** |
| pod, x2 | 350 | 1.62 | 567 each |
| nose | 180 | 2.50 | 449 |
| tail | 150 | 1.81 | 271 |
| engine, x2 | 70 | 0.60 | 42 each |

Total ~4342. Nobody chose that number. Standard sections default to `mass: 1.0`
(`base_section.rs:490,518,556`), so a unit cube is mass 1 - and cargoa carries
roughly **660x** the rotational inertia of a same-size hull built from them.

Numbers above are computed from the AUTHORED boxes in `cargo_a.rs:16-96`. Those
parts carry GLB meshes, so if the collider is built from the mesh the volumes
differ. The DENSITY ratio - 70-350 against a default of 1.0 - does not depend on
that and is enough on its own.

## The trap: mass is NOT unused today

Do not simply normalise the densities. Linear acceleration is `thrust / mass`,
so cargoa's engines are already balanced against a mass of 4342. Normalising to
density 1.0 drops it to 15.6 and the ship accelerates **278x harder** on the
same engines.

The state of the tree: **linear flight respects mass, angular flight ignores
it**, because `max_angular_acceleration` is flat. The two halves have been on
different footings since `2b03a2f8`, which is exactly why a 350x density error
survived unnoticed. Changing density is a FLIGHT MODEL change, not a turning
change, and the thrust pass is the real cost of this task.

## What normalising is worth

| | mass | `I` | flip at 8 G |
|---|---:|---:|---:|
| cargoa, authored | 4342 | 7408 | 7.88 s, torque-bound |
| cargoa, uniform density 1.0 | 15.6 | 34.4 | 1.83 s, structure-bound |
| 1-1-1 | 3 | 2.5 | 1.55 s |

At uniform density the corvette lands beside the 1-1-1 - slightly slower, five
times the volume, structure-bound like everything its size. That is the shape
the attitude model wants, and it closes the 660x gap entirely.

## Steps

1. **Rename the field to `density`** and document what `1.0` MEANS - name the
   reference material, so an author can tell what they are typing. This is the
   defect; nothing else is safe first. Grep every construction site, including
   examples and content builders.
2. **Measure, do not compute.** Dump avian's `ComputedAngularInertia`, mass and
   `r` for every shipped ship and several WFC hulls. `20260819-140314` needs the
   same numbers to pick `max_torque`, so do it once. Establish whether the
   collider is the authored box or the GLB mesh.
3. **Re-author in a narrow band around 1.0** - roughly 0.3 to 3 - preserving the
   relative intent that clearly WAS meant: engines dense (machinery), cargo pods
   light (mostly void), fuselage middling. Keep the design, lose the 350x.
   Covers `cargo_a`, `cargo_b`, the racer and anything else off the default.
4. **Re-pass thrust**, because step 3 moves linear acceleration by two orders of
   magnitude. Unavoidable, and the expensive half.
5. Hand off to `20260819-140314`, which can then pick one global `max_torque`.

## Done when

- The field is named `density`, and its doc says what 1.0 is.
- Measured mass, `I` and `r` for the shipped fleet are recorded here, from
  avian, not from arithmetic on authored boxes.
- Every authored density sits in a stated band, with the reason for each
  departure from 1.0.
- Ships fly correctly in a STRAIGHT LINE after the change - owner-flown, since
  thrust-to-mass is feel, not a metric.
- `20260819-140314` is unblocked: one `max_torque` produces sane turn rates
  across both populations.
