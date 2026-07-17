# Spike: refactor PDC turret to arbitrary arm count with multiple hinges and rotators

- STATUS: OPEN
- PRIORITY: 4
- TAGS: v0.7.0,spike,refactor,weapons,turret

This is a SPIKE, not shippable work. The output is a SPIKE.md documenting the
chosen direction plus the concrete follow-up tatr tasks it seeds (data model,
render, aim solve, tests). Run it with `/spike`. Do not write production code
until the direction is agreed.

## Problem

Today the PDC turret is a hardcoded kinematic chain:
`base -> yaw rotator -> pitch rotator -> barrel`, with a fixed set of marker
components and per-joint sync systems in
`crates/nova_gameplay/src/sections/turret_section.rs` (see the
`TurretSectionRotator{Yaw,Pitch,Barrel}Marker` types, the
`sync_turret_rotator_{yaw,pitch}_system`s, and the offset/mesh fields on the
turret settings). Every joint is bespoke, so the mount topology cannot vary:
you get exactly one yaw hinge and one pitch hinge, one barrel, one arm.

We want turrets (and mounts in general) that can be described as an arbitrary
tree of joints: N arms, each arm a chain of hinges and rotators terminating in
one or more muzzles. Think a twin-barrel PDC, a multi-arm point-defense mount,
or a turret whose elevation lives two hinges down the chain.

## Questions the spike must answer

- Data model: how is an arbitrary joint tree expressed in the section/scenario
  RON? (recursive joint node: axis, limits, rotator speed, offset, child mesh,
  children[]). How does it degrade to today's yaw/pitch/barrel case so existing
  content still loads (or what migration is needed)?
- Aim solve: today's intercept solve outputs a yaw+pitch pair. With an
  arbitrary chain the solver has to distribute a desired muzzle-aim direction
  across the available DOF. What is the generalization -- closed-form for the
  common 2-DOF case, iterative (CCD/IK) for the general case, or a constrained
  per-joint decomposition? What happens when DOF is insufficient or redundant?
- ECS representation: replace the fixed marker components + one-sync-system-per-
  joint with a generic joint component carrying its axis/limits/target, and a
  single system that walks the chain. How do muzzles (fire points) get located
  and how does `turret_lead` / the HUD find them?
- Multi-muzzle firing: fire-rate, ammo, and the projectile spawn hooks
  currently assume one barrel. What changes for N muzzles per mount?
- Editor + lint: `nova_editor` placement and the mount-base adjacency lint
  (recent commit eeef46dd) assume the current shape -- what has to generalize?

## Deliverables of the spike

- SPIKE.md in this task folder capturing the chosen data model, the aim-solve
  strategy, and the migration path for existing turret content.
- Seeded follow-up tasks (plannable, correctly tagged) for: the RON/data-model
  change, the ECS joint-walk + render, the generalized aim solver, multi-muzzle
  firing, and editor/lint updates.
- Per the repo testing guidance: the direction should include an end-to-end
  example (e.g. a scenario/example bin that spawns a non-trivial multi-hinge
  mount tracking a target) and where the tests live, so the follow-up work
  ships with integration coverage rather than isolated unit tests.

## Pointers

- `crates/nova_gameplay/src/sections/turret_section.rs` -- current turret
  section, settings fields, joint markers, sync systems, aim chain (`TurretAim`
  system set, PostUpdate).
- `crates/nova_gameplay/src/hud/turret_lead.rs` -- lead/aim HUD consumer.
- `crates/nova_scenario/src/lint.rs` -- mount-base adjacency lint.
- `crates/nova_editor/src/placement.rs` -- mount placement in the editor.
