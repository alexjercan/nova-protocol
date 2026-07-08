# Spike: Which nova systems should be promoted into bevy-common-systems?

- DATE: 20260708-110317
- STATUS: RECOMMENDED
- TAGS: spike, v0.4.0, crates, refactor

## Question

Part of the v0.4.0 goal of pushing tier-2 (`nova_gameplay`) code that is genuinely
game-agnostic out to tier-1, the external `bevy-common-systems` crate
(`~/personal/bevy-common-systems`). Which `nova_gameplay` modules are reusable and
stable enough to promote, which are conditional (generic core wrapped in a nova-specific
seam), and which must stay put?

A good answer is a per-module catalog with, for each candidate: what it is, why it is
game-agnostic, what still couples it to nova, whether its API is stable enough to reuse,
and where it would land in the external crate (new module vs merge). This spike is the
catalog + direction pass only. It moves no code (that is task 20260706-151804) and adds
no in-code markers (that is the remainder of task 20260707-095020, which this spike
feeds).

## Context

Crate-boundary policy lives in `docs/architecture.md`: tier 1 = `bevy-common-systems`
(fully game-agnostic Bevy primitives), tier 2 = `nova_gameplay` (gameplay + generic-leaning
helpers not yet promoted), tier 3 = `nova_core` (wiring only). Promotion is a deliberate,
coordinated cross-repo change, so tier-2 helpers legitimately sit in nova until moved.

Two facts materially shaped this catalog and correct earlier assumptions:

- **`bevy-common-systems` already depends on `avian3d` 0.7** (`Cargo.toml:8`; used by
  `physics/pd_controller`, `physics/doom_controller`, `debug/inspector`). So "touches
  avian" is *not* a promotion blocker, contrary to how the known-candidate list framed
  `hud/velocity` and `integrity/blast`.
- **`bevy-common-systems` already owns the generic `Health` / `HealthApplyDamage` /
  `HealthZeroMarker`** (`src/health/mod.rs`) and a generic `StatusBarPlugin`
  (`src/ui/status.rs`), plus `ExplodeMesh`, `ChaseCamera`, `PointRotation`,
  `DirectionalSphereOrbit`, `TriangleMeshBuilder`, `ColliderDensity` re-exports, and
  `ExplodableEntity`. Several nova candidates are therefore *merges into existing
  modules*, not new modules, and several nova files are just consumers of primitives that
  already live externally.

Existing external module set (for placement decisions): `audio`, `camera`
(chase/wasd/shake/skybox/post/project), `debug`, `feedback` (flash/screen_flash),
`health`, `helpers`, `input`, `mesh` (builder/explode), `meth` (lerp/sphere), `modding`,
`persist`, `physics` (pd/doom controller), `scoring`, `time` (cooldown), `transform`
(sphere/directional/point/random orbits, smooth_look), `tween`, `ui`
(animate/menu/popup/status/touchpad).

No `PROMOTE(` markers currently exist anywhere in `nova_gameplay`.

## Options considered

The strategic choice is **granularity**: promote leaf-level helpers piecemeal, or promote
cohesive subsystems as a unit.

- **Piecemeal, leaf-first** - move the small self-contained pieces (a formula, a bundle, a
  data model, a text-HUD) one file at a time. Pros: each move is low-risk, independently
  verifiable, and unblocks task 20260706-151804 incrementally. Cons: leaves the
  higher-value destruction system split awkwardly if we stop halfway.
- **Whole-subsystem** - promote the entire integrity/destruction pipeline
  (components + blast + plugin + explode) as one `destructible` module. Pros: the pipeline
  is where the real reusable value is. Cons: it is entangled with `integrity/glue.rs`
  (nova section grid) and `nova_events::OnDestroyedEvent`; promoting it well requires
  designing a seam (a graph-builder trait + a generic destroyed trigger) first, which is a
  design task, not a file move.
- **Do nothing / defer to v0.5** - always an option. Cost: the primitives keep being
  re-derived per game, and the audit note in `architecture.md` keeps growing stale.

Recommendation: **do both, in tiers** - promote the leaves now (they are unambiguous
wins), and treat the destruction pipeline as a deliberate seam-design promotion tracked
separately. The catalog below encodes that as tiers A-D.

## Recommendation

Promotion catalog. Tier A = promote now; Tier B = promote as a bundled subsystem behind a
seam; Tier C = conditional, real decoupling work; Tier D = keep in nova (the game-specific
seam/orchestration that consumes the promoted primitives).

### Tier A - promote now (game-agnostic, API-stable, minimal seam)

- **`game_object.rs` -> `rigid_body_point_velocity`** *(new candidate, not on the prior
  list)*. Pure rigid-body formula `v_point = v_linear + omega x (p - com)`, zero Bevy
  state, unit-tested. Fully generic. Land in `bevy-common-systems` `physics/` (or `meth/`).
  Stable.
- **`game_object.rs` -> `destructible_body(health, density)`** bundle
  (`Health` + `ColliderDensity` + `ExplodableEntity` + `Visibility`). Every referenced
  type is already external; the only nova coupling is the import *path*
  (`crate::prelude::ExplodableEntity`, re-exported from bcs). Land alongside the formula in
  `physics/` or a small `common/`. Stable.
- **`integrity/components.rs`** - the destructible-graph data model (`IntegrityRoot`,
  `ConnectedTo`, `IntegrityLeafMarker`, `IntegrityDisabledMarker`,
  `IntegrityDestroyMarker`). Pure component defs, no behavior, no nova types. Reusable by
  any graph-based destructible structure, not just ships. This is the foundation the Tier-B
  pipeline needs, so promote it together with (or just ahead of) that work. Stable.
- **`hud/health.rs`** - text HUD reading the external `Health` (target entity configurable
  via self-contained marker components; no nova imports). Merge into `ui` as a companion to
  `status` (e.g. `ui/health_display`). Stable.
- **`hud/objectives.rs`** - generic id+message objectives list driven by a plain resource;
  Bevy-only, no nova imports. New `ui/objective_display` (or generalize to a dynamic text
  list). Stable.
- **`integrity/blast.rs` + `calculate_blast_damage` / `on_impact_collision_deal_damage`**
  (from `integrity/mod.rs`, on the prior candidate list) - radial linear-falloff blast
  volume and impulse/energy collision damage over avian + the external `Health`. No nova
  coupling; avian is already a bcs dependency. Ships as the damage-delivery half of the
  Tier-B destruction module (`physics/blast` or into `health`). API is stable; the only
  care needed is that the collision-damage system is currently wired inside the integrity
  observers, so it extracts rather than plain-moves.

### Tier B - promote as a bundled subsystem, behind a seam

- **`integrity/plugin.rs`** - the destruction pipeline: impact/blast damage ->
  health depletion -> disabled -> destroy (leaves only) -> chain reaction. The algorithm is
  genuinely game-agnostic and heavily unit-tested, but its value only materializes bundled
  with `components.rs` + `blast.rs` as one `destructible`/`integrity` module. Promotion is
  a *seam design*, not a file move: the graph is currently built by the nova-specific
  `integrity/glue.rs` (section grid), so the promoted module must expose a graph-builder
  trait / API that nova's glue implements, plus a generic "destroyed" trigger to replace
  the direct `OnDestroyedEvent` coupling that `explode.rs` relies on. Treat as a deliberate
  design+move task. Core API stable; the *seam* is the unstable part to design.

### Tier C - conditional (generic core, real decoupling work)

- **`hud/velocity.rs`** (on the prior list, incl. `shaders/directional_*.wgsl`) -
  velocity magnitude/direction visualization. avian `LinearVelocity` coupling is fine (bcs
  has avian). The real friction is vendoring the two `.wgsl` shaders and the
  `DirectionMagnitudeMaterial` / `DirectionSphereMaterial` materials as embedded shader
  assets in bcs (it already has `transform/directional_sphere_orbit`, so the orbit half is
  home). Moderate effort; do after the Tier-A UI merges.
- **`integrity/explode.rs`** - extends the external `ExplodeMesh` with debris spawn +
  auto-despawn of meshless bodies. Core slicing/debris physics is generic, but it fires
  `nova_events::OnDestroyedEvent` and hardcodes section-vs-asteroid render assumptions.
  Needs the Tier-B generic destroyed-trigger plus extraction of the section-specific debris
  path. Promote with / after Tier B.

### Tier D - keep in nova (game-specific seams and orchestration)

These consume the promoted primitives; they are the reason the primitives are clean.

- **`integrity/glue.rs`** - section-grid adjacency, section-disable, ship health rollup.
  This is the nova implementation of the Tier-B seam. Stays.
- **`hud/mod.rs`** - HUD lifecycle keyed on `PlayerSpaceshipMarker` add/remove +
  `NovaHudAssets`. Spaceship-specific orchestration. Stays.
- **`hud/torpedo_target.rs`** - reticle over `SpaceshipPlayerTorpedoTargetEntity` +
  `SpaceshipCameraController`. Weapon-specific. Stays.
- **`camera_controller.rs`** - spaceship camera modes (Normal/FreeLook/Turret) over the
  external `ChaseCamera` + `PointRotation`. Game-specific orchestration of external
  primitives. Stays.

### Marker convention

Adopt `// PROMOTE(bevy-common-systems): <one-line rationale>` on the eligible item (or at
module top when the whole file is eligible), so `grep -rn 'PROMOTE(bevy-common-systems)'
crates/` surfaces the catalog from code. Tier A/B/C items get a marker; Tier D items do
not. Include the tier and target module in the rationale, e.g.
`// PROMOTE(bevy-common-systems): tier A, pure rigid-body formula -> physics/`.

## Open questions

- **Tier-B seam shape**: trait-based graph builder vs a data-driven `ConnectedTo` the
  caller fills in vs an events-in/events-out plugin. Wants its own small design spike before
  the move.
- **Generic destroyed trigger**: does bcs already have (or want) a generic
  "entity destroyed" event that `explode.rs` and integrity can target instead of
  `nova_events::OnDestroyedEvent`? Check the bcs `modding`/event surface before inventing
  one.
- **Shader vendoring**: how does bcs embed wgsl for its existing materials
  (`embedded_asset!` vs runtime load)? Match that for `hud/velocity`.
- **Naming/placement**: `physics/` vs a new `destructible/` vs `common/` for the
  `game_object.rs` helpers and the integrity bundle - a bcs-side decision.

## Next steps

Direction-level tasks for `/plan` to expand. Task 20260707-095020 (this catalog's parent)
is completed by the first task below; the cross-repo moves stay under 20260706-151804.

- tatr (existing) 20260707-095020: apply the `PROMOTE(bevy-common-systems)` markers to the
  Tier A/B/C items above and write the short `docs/promotion-candidates.md` catalog that
  cross-links this spike, task 20260706-151804 (moves) and 20260706-160503 (mesh slicer,
  already external). No code moves.
- tatr (existing) 20260706-151804: perform the Tier-A cross-repo moves first
  (`game_object` helpers, `integrity/components`, `hud/health`, `hud/objectives`,
  `integrity/blast` + collision-damage), then re-point nova at the external symbols and
  delete the local copies.
- tatr (new, see report): design the Tier-B destructible-graph seam
  (graph-builder API + generic destroyed trigger) before moving `integrity/plugin` +
  `explode`; blocks the Tier-B/C portion of 20260706-151804.
