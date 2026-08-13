# Retrospective

## What worked

- Semantic `PartSpec` assemblies kept mesh, collider, health, behavior, and mate intent together.
- Centered primitive colliders plus render offsets preserved exact source-mesh assembly.
- Strict link-point graph and overlap lint found detached or invalid fixture placements early.
- Migrating all three cube-based ships avoided leaving coordinate vocabulary in production.
- The player-path harness and full probe catalog caught runtime assumptions beyond content lint.

## What changed during delivery

- CargoA joined Racer and CargoB because it used the same obsolete cube vocabulary.
- Directly mated sections may overlap by AABB; unmated overlap remains invalid.
- Turrets remain modules rather than body parts. CargoB pods own torpedo behavior.
- Two probe fixtures were corrected: one detached turret and one generic-despawn kill-cam setup.
- Balance drift was recorded and deferred as planned.

## Next time

- Design overlap policy with the collider representation before generating final parts.
- Include all ships that share an obsolete vocabulary in the initial task scope.
- Test destruction-confirmed UI fixtures through integrity markers, not generic despawn.
