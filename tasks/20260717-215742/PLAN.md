# Implementation plan: turret joint-tree core (merged tasks 215742 + 215804 + 215835)

Spike: `tasks/20260717-214834/SPIKE.md`. This plan is the agreed Steps for the
merged core (data model + generic ECS + CCD aim solver). Multi-muzzle firing
(215857) and editor/lint (215920) are separate follow-up commits.

Everything is in `crates/nova_gameplay/src/sections/turret_section.rs` unless a
path is given. Keep `cargo check -p nova_gameplay` green, `cargo fmt`, and read
`cargo build -p nova_gameplay` warnings before done (lesson
`warnings-clean-before-land`).

## 1. Data model (recursive tree)

Add, replacing the flat joint fields of `TurretSectionConfig`:

```rust
/// A fire point on a turret: where bullets leave. A joint carries at most one.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MuzzleConfig {
    /// Rounds per second for THIS muzzle.
    pub fire_rate: f32,
    /// Muzzle effect (flash) asset; None = no flash.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub muzzle_effect: Option<AssetRef<EffectAsset>>,
}
```

```rust
/// One node of a turret's kinematic joint tree. Recursive. Today's turret is
/// the tree base(fixed) -> yaw(axis Y) -> pitch(axis X) -> barrel(fixed) ->
/// muzzle(fixed, has `muzzle`). Arbitrary arm count / multi-hinge = wider/deeper
/// trees.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurretJoint {
    /// Local translation from the parent joint (section origin for the root).
    pub offset: Vec3,
    /// Local hinge axis. None = fixed node (offsets + may carry mesh/muzzle,
    /// never rotates). Some(axis) = articulated, driven by the aim solver.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub axis: Option<Vec3>,
    /// Rotation speed rad/s (only when `axis` is Some).
    #[cfg_attr(feature = "serde", serde(default = "default_joint_speed"))]
    pub speed: f32,
    /// Lower/upper rotation limits in radians (only when `axis` is Some).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub min: Option<f32>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub max: Option<f32>,
    /// This joint's render mesh; None = a generic default primitive.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Present iff this joint is a fire point.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub muzzle: Option<MuzzleConfig>,
    /// Child joints, composed in this joint's ROTATED frame.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub children: Vec<TurretJoint>,
}
```

`TurretSectionConfig` keeps the firing/ammo/sound/damage/projectile fields
(`muzzle_speed`, `projectile_lifetime`, `bullet_damage`, `bullet_kind`,
`projectile_render_mesh`, `fire_sound`, `dry_fire_sound`, `ammo_capacity`,
`reload`) and REPLACES the removed fields (`yaw_speed`, `pitch_speed`,
`min_pitch`, `max_pitch`, `render_mesh_base/yaw/pitch/barrel`,
`base/yaw/pitch/barrel/muzzle_offset`, `muzzle_effect`, `fire_rate`) with a
single `pub root: TurretJoint`. `fire_rate` and `muzzle_effect` move to
`MuzzleConfig` (per-muzzle). `muzzle_speed`/`bullet_*` stay section-wide (all
muzzles fire the same round for now).

`Default for TurretSectionConfig` must build the SAME tree as today:
base(offset (0,-0.5,0), fixed) -> yaw(offset (0,0.1,0), axis Y, speed PI) ->
pitch(offset (0,0.2,0), axis X, speed PI, min -PI/6, max PI/2) -> barrel(offset
(0.1,0.2,0), fixed) -> muzzle(offset (0,0,-0.5), fixed, muzzle: MuzzleConfig {
fire_rate: 100, muzzle_effect: None }). Firing defaults unchanged.

Add `fn default_joint_speed() -> f32 { std::f32::consts::PI }`.

Note: today the muzzle is the barrel's child (muzzle_offset from barrel). Keep
that: barrel is a fixed joint, muzzle is its fixed child with the muzzle set.

## 2. Generic ECS components

Replace the marker zoo (`TurretRotatorBaseMarker`,
`TurretSectionRotatorYaw{,Base}Marker`, `TurretSectionRotatorPitch{,Base}Marker`,
`TurretSectionRotatorBarrelMarker`, the per-type `*RenderMesh` components) with:

```rust
/// A turret joint entity: the runtime of one TurretJoint node. Articulated
/// joints (axis Some) also carry a SmoothLookRotation. `part_of` is the turret
/// section root.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretJointMarker { axis: Option<Vec3> }

/// This joint's render mesh (generic; was the per-type *RenderMesh zoo).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretJointRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);
```

Keep `TurretSectionBarrelMuzzleMarker` as the muzzle marker (rename optional;
keeping it avoids churn in ai.rs/audio.rs/turret_lead.rs). Keep
`TurretSectionMuzzleEntity(Entity)` on the section root for now (single muzzle);
multi-muzzle (task 215857) generalizes to a Vec. Keep `TurretSectionPartOf`.

## 3. Recursive spawn observer

Rewrite `insert_turret_section` to walk `config.root` recursively and spawn one
entity per joint. Collapse the old base+rotator pair into ONE entity per joint:
`SmoothLookRotation` does NOT read Transform (verified), so a single entity can
carry both `Transform{translation: offset, rotation: from_axis_angle(axis,0)}`
AND the controller. Math is identical: `parent * T(off) * R(theta) * ...`.

For each joint node:
- spawn `(Name, TurretJointMarker{axis}, TurretSectionPartOf(root),
  TurretJointRenderMesh(mesh), Transform::from_translation(offset),
  Visibility::Inherited)`;
- if `axis` is Some: also insert `SmoothLookRotation { axis, initial: 0.0,
  speed, min, max }`;
- if `muzzle` is Some: also insert the muzzle bundle
  (`TurretSectionBarrelMuzzleMarker`, `TurretSectionBarrelFireState(timer from
  muzzle.fire_rate)`, `TurretSectionBarrelMuzzleEffect(muzzle.muzzle_effect)`)
  and record the entity as THE muzzle (single-muzzle core: assert/permit exactly
  one; if the tree has >1, use the first and log - multi-muzzle is 215857);
- recurse into `children`, add each as a child entity.

Set `TurretSectionMuzzleEntity(muzzle)` + `TurretSectionFireSound` +
`TurretSectionDryFireSound` on the root, and `SectionAmmo`/`SectionReload` as
today. The root joint entity is added as a child of the turret section entity.

## 4. Generic aim + sync (CCD)

Replace `update_turret_target_yaw_system` + `update_turret_target_pitch_system`
with ONE `update_turret_target_joints_system`, and the two `sync_*` systems with
ONE `sync_turret_joint_rotation`.

`sync_turret_joint_rotation`: for every articulated `TurretJointMarker{axis:
Some(a)}` with a `SmoothLookRotationOutput`, set `transform.rotation =
Quat::from_axis_angle(a.normalize(), output)`. (Fixed joints keep identity
rotation.)

`update_turret_target_joints_system` (Jacobi hinge-CCD, one pass/frame):
- For the muzzle chain: from `TurretSectionMuzzleEntity` walk `ChildOf` up to the
  turret root, collecting articulated joints.
- Muzzle pose via `TransformHelper::compute_global_transform(muzzle)`: `m =
  translation`, `d = forward()` (= -Z).
- Turret `aim_point` (skip if None or `SectionInactiveMarker`).
- For each articulated joint J with axis `a_local`:
  - parent global `P = compute_global_transform(childof(J))`;
  - pre-rotation joint frame `F = P * Transform::from_translation(J.transform.translation)`
    (orientation = parent's, position = joint origin);
  - `w2j = F.to_matrix().inverse()`; transform `m,d,target` into J-local:
    `ml = w2j.transform_point3(m)`, `dl = w2j.transform_vector3(d)`,
    `tl = w2j.transform_point3(target)`;
  - axis `a = a_local.normalize()`;
  - CCD step: project `dl` and `(tl - ml)` onto the plane perpendicular to `a`,
    take the signed angle about `a`:
    ```
    let des = (tl - ml);
    let d_perp = dl - a * dl.dot(a);
    let t_perp = des - a * des.dot(a);
    if d_perp.length() > 1e-6 && t_perp.length() > 1e-6 {
        let delta = signed_angle_about(d_perp, t_perp, a); // atan2 of cross.dot(a), dot
        let out = q_output.get(J).map(|o| **o).unwrap_or(0.0);
        target.0 = out + delta;   // SmoothLookRotationTarget; clamp handled by controller
    }
    ```
    with `fn signed_angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
        let from = from.normalize(); let to = to.normalize();
        let c = from.cross(to).dot(axis); let d = from.dot(to); c.atan2(d) }`.
- This is basis-independent and reduces to today's behavior for the Y/X chain
  (verify behaviorally, not by theta equality). Ordering vs bcs
  `SmoothLookRotationSystems::Sync` is a stable servo either way; keep the aim
  systems in `TurretSectionAimSystems` in PostUpdate as today.

Update the plugin `build`: swap the two sync systems for `sync_turret_joint_rotation`
and the two target systems for `update_turret_target_joints_system`.

## 5. apply_turret_config_to_children

Rewrite generically: on `Changed<TurretSectionConfigHelper>`, re-walk the config
tree in lockstep with the joint entities of that turret and push
`speed/min/max` onto each articulated joint's `SmoothLookRotation`, and each
muzzle's fire interval onto its `TurretSectionBarrelFireState`. Simplest robust
approach: match entities to config nodes by tree position (DFS order). If that is
fragile, store the node's config on each joint entity at spawn (a
`TurretJointTuning` component) and refresh from it. Pick the cleaner one.

## 6. Generic render

Replace the four per-type render observers with ONE
`insert_turret_joint_render` on `Add, TurretJointMarker` (gated by
`self.render`). If `TurretJointRenderMesh` is Some -> spawn
`WorldAssetRoot(resolve)` child as today; if None -> spawn a generic default
primitive (a small `Cylinder::new(0.2, 0.2)` dark-grey child). The bespoke
per-type placeholder art (ridged yaw cylinder, barrel shape) is dropped in favor
of one generic default; shipped turrets author GLB meshes so the visible game is
unchanged. NOTE this in the review/commit + CHANGELOG. Keep
`insert_turret_barrel_muzzle_effect`, `insert_projectile_render` unchanged.

## 7. Content migration (delete legacy fields, one code path)

Rewrite each authored turret to the tree form. Values come from today's fields:
- `assets/base/sections/base.content.ron` lines 62 + 134 (two turret defs).
- `crates/nova_assets/src/sections.rs` lines ~187 and ~261 (two Rust configs).
- `crates/nova_assets/src/mod_refs.rs` line ~538 (test config).
- `examples/data/reel.content.ron` if it authors a turret with flat fields.
Map: yaw_speed->yaw joint speed; pitch_speed/min_pitch/max_pitch->pitch joint;
each *_offset->that joint's offset; render_mesh_*->that joint's render_mesh;
fire_rate/muzzle_effect->muzzle's MuzzleConfig. Preserve every numeric value.

## 8. Example + consumer updates

- `examples/04_turret_section.rs`: the range demo tunes yaw_speed/pitch_speed/
  min_pitch/max_pitch live. Rework `Knob::read/write` to reach the yaw joint
  (first axis ~= Y) and pitch joint (first axis ~= X) inside `config.root` and
  mutate their `speed/min/max`. `range_turret_config` builds via the tree.
- `update_turret_aim_point`, `shoot_spawn_projectile`: still single-muzzle via
  `TurretSectionMuzzleEntity`; only compiles-changes if a field moved (muzzle
  effect now per-muzzle - but the muzzle entity already carries
  `TurretSectionBarrelMuzzleEffect`, so unaffected). `config.muzzle_speed` stays.
- `crates/nova_gameplay/src/input/ai.rs`, `hud/turret_lead.rs`, `audio.rs`:
  should be untouched (they use the muzzle marker + aim point, both preserved).
  Fix only if the compiler complains.
- Screenshot examples 15/16/17, 10_playable, 11_hud_range: fix only compile
  breaks (they likely spawn base sections, unaffected).

## 9. Tests (integration-first)

In `turret_section.rs` tests (follow existing patterns there):
- GOLDEN CHAIN: spawn `turret_section(TurretSectionConfig::default())`, run the
  Add observer, assert the entity chain shape matches today (base->yaw->pitch->
  barrel->muzzle: 5 joint entities, muzzle marker on the leaf, SmoothLookRotation
  only on yaw+pitch with the right axes/speeds/limits, offsets preserved).
- AIM CONVERGENCE (behavioral parity): spawn a default turret, set
  `TurretSectionTargetInput(Some(target))`, step the aim + bcs sync + transform
  propagation enough frames, assert the muzzle forward (-Z) points within a small
  angle of `target - muzzle_pos`. Do it for a couple of target directions in the
  reachable envelope. This is the parity guarantee for the CCD swap.
- MULTI-HINGE: a hand-built 3-hinge tree (e.g. Y, then X, then Y two down)
  converges onto a target the 2-DOF turret could not reach purely (sanity that
  arbitrary chains solve). Keep it modest.
- RON ROUND-TRIP: if the crate has a serde test harness for sections, assert a
  tree config serializes+deserializes (ron) to an equal config.
- Update/retire any existing tests that referenced the removed markers/fields
  (there are tests spawning the old chain - migrate them to the new components).

## 10. Verify

`cargo fmt`; `cargo check -p nova_gameplay`; `cargo check -p nova_assets`;
`cargo build -p nova_gameplay 2>&1` and READ warnings (do not grep them away);
run the NEW tests: `cargo test -p nova_gameplay turret 2>&1 | tail`. Report what
passed and any pre-existing unrelated failures (do not fix unrelated).
Do NOT git commit - leave the tree dirty for review.
