# Spike: refactor PDC turret to arbitrary arm count with multiple hinges and rotators

- DATE: 20260717-214834
- STATUS: RECOMMENDED   # RECOMMENDED | INCONCLUSIVE | DROPPED
- TAGS: spike, refactor, weapons, turret

## Question

Today a PDC turret is a hardcoded kinematic chain
`base -> yaw rotator -> pitch rotator -> barrel -> muzzle`, wired with bespoke
marker components and one sync system per joint in
`crates/nova_gameplay/src/sections/turret_section.rs`. The topology cannot
vary: exactly one yaw hinge, one pitch hinge, one barrel, one muzzle.

Can we describe a turret (and mounts in general) as an arbitrary tree of
joints -- N arms, each a chain of hinges/rotators terminating in one or more
muzzles -- without a rewrite of the aim math, the fire path, the editor, and
the lint? What is the data model, how does the aim solver generalize, and what
is the migration path for existing content? A good answer is a concrete data
model plus an aim-solve strategy that a planner can expand into steps without
re-litigating the approach, and an honest read on how far the blast radius
reaches (in particular: does the editor/lint have to change).

## Context

What exists today (all in `crates/nova_gameplay/src/sections/turret_section.rs`
unless noted):

- **Data model.** `TurretSectionConfig` is a flat struct with fixed fields:
  `yaw_speed`, `pitch_speed`, `min_pitch`/`max_pitch`, and per-stage
  `render_mesh_{base,yaw,pitch,barrel}` + `{base,yaw,pitch,barrel,muzzle}_offset`
  (each a `Vec3`), plus firing fields (`fire_rate`, `muzzle_speed`,
  `bullet_damage`, `bullet_kind`, `ammo_capacity`, `reload`, sounds). Authored
  in RON, e.g. `assets/base/sections/base.content.ron` (the
  `better_turret_section`). There is effectively **one** shipped turret def.

- **ECS shape.** An `On<Add, TurretSectionMarker>` observer
  (`insert_turret_section`) spawns a fixed 6-entity chain:
  `TurretRotatorBase -> TurretSectionRotatorYawBase -> ...YawMarker ->
  ...PitchBase -> ...PitchMarker -> ...BarrelMarker -> BarrelMuzzleMarker`.
  Only the two `*Base` entities carry a `SmoothLookRotation` controller
  (`{axis: Y, speed: yaw_speed}` and `{axis: X, speed: pitch_speed, min, max}`).
  Two sync systems (`sync_turret_rotator_yaw_system`,
  `sync_turret_rotator_pitch_system`) copy each controller's
  `SmoothLookRotationOutput` onto the paired visual rotator's transform.

- **Aim solve.** Runs in `PostUpdate` under `TurretSectionAimSystems`, chained:
  `update_turret_aim_point` (solves the intercept lead point via
  `lead_intercept_point`, a quadratic in the shooter frame) then
  `update_turret_target_yaw_system` and `update_turret_target_pitch_system`.
  **Crucial finding:** each of the yaw/pitch systems is already a *per-joint
  analytic decomposition*. It transforms the muzzle position, muzzle forward,
  and target into that one joint's local frame (`world_to_{yaw,pitch}_base`),
  then solves the single-axis angle that swings the muzzle's forward onto the
  target with inverse trig, and writes it to that joint's
  `SmoothLookRotationTarget`. Speed/limit clamping is `SmoothLookRotation`'s
  job. The two systems differ only in which plane they project into (xz vs yz).

- **Firing.** `shoot_spawn_projectile` iterates turrets, reads the single
  `TurretSectionMuzzleEntity` and its one `TurretSectionBarrelFireState` timer,
  spawns bullets with sub-tick lead. Ammo (`SectionAmmo`) and reload live on the
  turret root, not the muzzle. `crates/nova_gameplay/src/input/ai.rs` gates AI
  fire on a single-muzzle forward-vs-aim alignment dot. `audio.rs` plays the
  fire sound off `Add<TurretBulletProjectileMarker>` and is topology-agnostic.

- **HUD.** `crates/nova_gameplay/src/hud/turret_lead.rs` draws one lead pip per
  turret. It reads only the pre-computed `TurretSectionAimPoint` off the turret
  root -- it never touches the joint chain -- so it survives multi-joint turrets
  untouched (multi-*muzzle* would only matter if we want per-muzzle pips).

- **Editor + lint.** `nova_scenario/src/lint.rs`'s mount-base adjacency check
  and `nova_editor/src/placement.rs` operate on the *section's* placement on the
  ship's unit-cube grid (base face = local `-Y` must abut an occupied cell).
  Neither looks inside the joint chain. The joint tree is entirely internal to
  the section and is built by the `Add` observer, so the editor places one
  section entity and gets the whole tree for free.

The key realization from reading the code: the aim math is **already**
per-joint and local-frame, and the editor/lint are **already** blind to the
internal chain. The refactor is therefore mostly contained to the data model,
the spawn observer, and the aim/fire systems -- not a cross-crate rewrite.

## Options considered

### Data model

- **A. Recursive joint tree in RON (recommended).** A `TurretJoint` node:
  `{ offset: Vec3, axis: Option<Vec3> (None = fixed/decorative), min/max:
  Option<f32>, speed: f32, render_mesh: Option<AssetRef>, muzzle:
  Option<MuzzleConfig>, children: Vec<TurretJoint> }`. The turret config carries
  firing/ammo defaults plus a `root: TurretJoint`. Today's turret is exactly one
  tree: base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) -> muzzle-leaf.
  Mirrors the glTF scene-graph mental model authors already use; degrades to the
  current case as one specific tree. Cons: nested RON is more verbose; needs a
  well-formedness lint (non-zero axes, at least one muzzle).

- **B. Flat joint array with parent indices (glTF-skin style).** `joints:
  Vec<Joint>` where each joint names a `parent: usize`. Less nesting, but
  references-by-index are error-prone to hand-author and read worse in RON.

- **C. Keep yaw/pitch special-cased, bolt on optional extra joints.** Minimal
  change but fails the goal (twin-barrel, elevation-two-hinges-down still can't
  be expressed cleanly) and leaves two code paths forever. Rejected.

### Migration of existing content

- **A. Migrate the content, delete the legacy fields (recommended).** There is
  one shipped turret def; rewrite it to the tree form and drop the flat
  `*_offset`/`render_mesh_*`/`min_pitch`/`max_pitch` fields. One code path, no
  compat cruft. A short migration note in the task; a golden test asserts the
  migrated turret produces the same 6-entity chain and aim behavior as today.

- **B. Serde-untagged / dual config (legacy flat OR tree).** Old RON keeps
  loading. Costs a permanent second code path and a second spawn path for a
  single legacy def. Not worth it here.

### Aim solve

- **A. Analytic per-joint CCD sweep (recommended).** Generalize the existing
  yaw/pitch decomposition into a loop. Walk the muzzle's chain of rotational
  DOF from the muzzle outward to the root; for each joint, transform muzzle +
  target into that joint's local frame and solve the single-axis angle that
  best points the muzzle forward at the target (the *exact* primitive the two
  current systems already implement), writing `SmoothLookRotationTarget`. This
  is textbook Cyclic Coordinate Descent with a closed-form per-joint step. For
  the 2-DOF chain it reduces to today's behavior in one pass; longer chains
  converge in a handful of sweeps per frame (and `SmoothLookRotation` rate-limits
  the visible motion anyway, so a partial per-frame solve is fine). Redundant
  DOF: CCD just finds *a* solution. Insufficient DOF: each joint minimizes its
  residual (best-effort aim), which is the sane failure mode. Biggest reuse of
  tested code; lowest risk.

- **B. Full closed-form per topology.** A bespoke analytic solution per joint
  arrangement. Intractable for arbitrary trees; only exists for special cases.
  Rejected.

- **C. Numerical Jacobian IK (damped least squares).** General and handles
  redundancy elegantly, but heavier, another dependency-or-implementation,
  harder to keep stable and to compose with per-joint limit clamping. Overkill
  when the per-joint analytic step already exists. Hold in reserve if CCD
  convergence proves inadequate on pathological chains.

### ECS representation

- **A. Generic joint component + single chain-walk system (recommended).**
  Replace the per-joint marker types and the two sync systems with one
  `TurretJoint { axis, min, max, speed }` component (backed by
  `SmoothLookRotation`, which already exists) on each articulated node, a
  `TurretMuzzle` component on each fire-point leaf, and a
  `TurretSectionMuzzles(Vec<Entity>)` on the root (superseding the single
  `TurretSectionMuzzleEntity`). One `solve_turret_chains` system walks each
  muzzle's ancestor chain via `ChildOf`, runs the CCD sweep, and writes each
  joint's target; one generic sync system (or fold the output straight onto the
  transform) applies the controller output. Mesh spawn walks the tree.

- **B. Keep bespoke markers, add more of them.** Does not scale past a fixed
  count; rejected.

### Multi-muzzle firing

- **A. Iterate muzzles, share the magazine (recommended).** One
  `TurretSectionBarrelFireState` per muzzle; `shoot_spawn_projectile` iterates
  the root's `TurretSectionMuzzles` rather than assuming one. `SectionAmmo` and
  reload stay on the root (shared magazine) so a twin-barrel PDC draws from one
  pool; any muzzle consumes a round. Fire rate is per-muzzle. AI alignment gate
  checks per-muzzle (or the representative shared tip). Audio is already
  per-projectile, no change.

### Editor + lint

- **A. Essentially unchanged, add a content well-formedness lint
  (recommended).** Placement and mount-base adjacency operate on the section on
  the ship grid and are blind to the internal chain, so they keep working as-is.
  Add a cheap lint that the joint tree is well-formed: axes non-zero (or
  explicitly fixed), limits ordered, at least one muzzle reachable. The editor
  reuses the `turret_section` bundle, so it renders the new tree for free.

## Recommendation

Adopt the **recursive joint tree** (`TurretJoint`) as the data model,
**migrate the single shipped turret def to it and delete the legacy flat
fields** (one code path), and generalize aiming to an **analytic per-joint CCD
sweep** that lifts the existing yaw/pitch local-frame decomposition into a loop
over an arbitrary joint chain. In ECS, replace the per-joint marker zoo and the
two sync systems with a **generic `TurretJoint` component plus one chain-walk
solve system**, and carry **N muzzles per mount** with per-muzzle fire timers
over a **shared section magazine**. The editor and the mount-base adjacency
lint need **no structural change** because they already treat the mount as one
grid section and never inspect the chain; add only a small well-formedness lint
for the new tree.

Why it beats the runners-up: the aim math and the editor/lint are already
shaped for this (per-joint + local-frame aim; chain-blind placement), so the
recommended path is mostly *reuse in a loop* rather than new machinery. CCD
reuses the tested single-axis solve; the recursive RON matches how authors
already think about scene graphs; and migrating the one existing def avoids
carrying a second config/spawn path forever. The heavier alternatives
(numerical IK, dual legacy config) buy generality/compat we do not need yet and
can be revisited if a real chain defeats CCD.

### End-to-end example (per repo testing guidance)

Ship a runnable demo with the aim-solver work: a scenario RON (and/or an
example bin under the content CLI) that spawns a **non-trivial mount** -- e.g. a
twin-barrel PDC (shared yaw+pitch, two muzzles at the tip) and a turret whose
elevation lives two hinges down -- tracking a moving target, and asserts the
muzzles converge onto the lead point. Integration tests live alongside the
existing turret tests in `turret_section.rs` (golden: migrated legacy def ==
old 6-entity chain and aim), plus a scenario-level test that the demo mount
tracks. This keeps the follow-up work shipping with integration coverage rather
than isolated unit tests.

## Open questions

- **Shared vs private joints across muzzles.** For a twin-barrel PDC the two
  muzzles share the yaw+pitch joints and split only at the tip -- both chains
  write the same shared-joint target and agree. For fully independent arms
  (each its own yaw/pitch) they diverge cleanly. But a joint shared by muzzles
  that want *different* angles needs a conflict rule (average? first muzzle
  wins? aim shared joints at the target centroid, private joints per muzzle?).
  Resolve during the aim-solver task; the recommended default is "shared joints
  aim at the target, private joints refine per muzzle."
- **CCD convergence budget.** How many sweeps per frame for the longest
  realistic chain before it looks laggy? Measure on the demo mount; likely 1-3
  given `SmoothLookRotation` already rate-limits visible motion.
- **Per-muzzle HUD pips.** Do we want one lead pip per muzzle or keep one per
  turret? One per turret is the cheap default and needs no HUD change; defer
  unless design asks.
- **Does any mount want a non-shared base grid footprint** (arms that each abut
  a different neighbor cell)? If yes, the adjacency lint would need per-arm
  support and this assumption breaks. Confirm none of the intended designs need
  it before relying on the "editor/lint unchanged" conclusion.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps
(dependency order; each links back to this SPIKE.md):

- tatr 20260717-215742: RON/data-model change -- recursive `TurretJoint` tree +
  migrate the shipped turret def, delete legacy flat fields
- tatr 20260717-215804: ECS joint-walk + render -- generic `TurretJoint`
  component, single chain-walk system, tree mesh spawn, retire the marker zoo
- tatr 20260717-215835: generalized aim solver -- analytic per-joint CCD sweep +
  end-to-end multi-hinge demo scenario/example
- tatr 20260717-215857: multi-muzzle firing -- N muzzles per mount, per-muzzle
  fire timers, shared section magazine, AI gate
- tatr 20260717-215920: editor + lint updates -- confirm placement/adjacency
  unchanged, add joint-tree well-formedness lint

## Fix record

- **20260717-215742 + 215804 + 215835 (core, landed 606c3576):** the recursive
  `TurretJoint` data model, one-entity-per-joint generic ECS (base+rotator
  collapsed - `SmoothLookRotation` never reads `Transform`), and a single Jacobi
  per-frame hinge-CCD aim solver all shipped together (inseparable; merged with
  user sign-off). Flat config fields deleted, all authored content migrated
  (parity green), turret-range example retuned per-joint, dev wiki + CHANGELOG
  updated for the breaking schema. Behavioral parity pinned by a muzzle-converges
  -on-target test (< 5 deg) rather than theta-equality. Multi-hinge trees solve
  (< 8 deg on a 3-hinge arm). Default render art is now one generic primitive
  (shipped turrets author GLBs, game unchanged). See tasks/20260717-215742/.
- **20260717-215857 (multi-muzzle, landed e882e2cf):** a turret now fires and
  aims EVERY muzzle in its tree, each with its own timer, all drawing from the
  ONE section magazine (`TurretSectionMuzzles(Vec<Entity>)` beside the primary
  `TurretSectionMuzzleEntity`). Twin-barrel test pins the shared-magazine
  invariant (total == capacity, not capacity x barrels). The shared-vs-private
  joint conflict resolved to the spike default: shared joints aim at target,
  private joints refine per muzzle; the lead point + AI gate stay on the primary
  muzzle (fine for shared-chain mounts). See tasks/20260717-215857/.
- **20260717-215920 (editor/lint, landed 4e5043d2):** editor placement + mount
  adjacency lint confirmed tree-blind (no change). Added `lint_section_config`
  walking the joint tree (degenerate axis / bad speed / min>max / no-muzzle
  errors, limits-without-axis warning), wired into the scenario loop AND the
  bundle catalog lint, plus a runtime axis backstop in `spawn_turret_joint`.
  Per-arm footprint open question resolved: single-base mount per cell for all
  shipped turrets, no per-arm adjacency needed. FEATURE COMPLETE - all five
  seeded tasks landed. See tasks/20260717-215920/.
