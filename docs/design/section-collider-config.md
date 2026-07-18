# Authorable section collider shape and size (task 20260718-102022)

## What changed

Ship sections no longer hardcode `Collider::cuboid(1.0, 1.0, 1.0)`. A new
optional field on `BaseSectionConfig` lets content authors pick the collider
shape and size per section:

```rust
pub enum SectionCollider {
    Cuboid { size: Vec3 },            // full side lengths, like Collider::cuboid
    Sphere { radius: f32 },
    Capsule { radius: f32, length: f32 },   // along local Y
    Cylinder { radius: f32, height: f32 },  // along local Y
}

// on BaseSectionConfig:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub collider: Option<SectionCollider>,
```

`base_section()` and `preview_section()` (crates/nova_gameplay/src/sections/base_section.rs)
now build `config.collider.unwrap_or_default().to_collider()`. `Default` for
`SectionCollider` is `Cuboid { size: Vec3::ONE }` - exactly the old unit cube -
so a `None` field resolves to the pre-existing behavior. The field mirrors the
`impact_sound` / `destroy_sound` pattern: `serde(default, skip_serializing_if =
"Option::is_none")`, so any content that omits it serializes byte-for-byte
identically and the `content_ron_parity` set is unaffected.

RON authoring (see craft_cargob for the shipped example):

```ron
base: (
    id: "...", name: "...", description: "...", mass: 1.0, health: 100.0,
    collider: Some(Cuboid(size: (0.8, 0.8, 0.8))),
),
```

craft_cargob's two beveled top-front corner cubes (`cube_i1_j2_km2`,
`cube_im1_j2_km2`) now carry a tightened `0.8` cuboid collider that matches the
cut hull instead of overshooting the whole cell - the exercising example the
task called for.

## The overlap lint

`check_section_overlaps` (crates/nova_scenario/src/lint.rs) assumed unit cubes:
two sections error iff their centers are closer than `1.0` on every axis. It now
sums the two sections' actual AABB half-extents per axis (`SectionCollider::
aabb_half_extents()`), so a tightened collider is allowed closer and an oversized
one is flagged sooner. For default unit cubes the threshold is still `1.0`, so
all shipped content lints exactly as before. Only `Inline` sources resolve their
authored collider; a `Prototype` source falls back to the unit cube because the
prototype catalog is not in scope in the linter - matching pre-config behavior
and keeping the existing prototype-based overlap test green.

## Decision: mass follows collider volume (density, not mass)

`base_section` feeds the section's authored `mass` field to avian as *density*
(`destructible_body(health, density)`), and avian derives real mass from
`density * collider_volume`. The unit cube has volume `1.0`, so historically
`mass == density` and the distinction never mattered. With authorable colliders
a smaller/larger collider now scales the section's *physical* mass.

We kept this behavior rather than compensating density to hold mass constant:
a physically smaller hull piece being lighter is the more correct default, and
authors who want a specific mass can tune the `mass`/density field. This is
documented on `SectionCollider` and here so a future handling-tuning session
knows why a shrunk collider also lightened a section.

## Why an AABB for the lint

The overlap lint is deliberately rotation-agnostic (all shipped content uses
quarter-turn rotations, under which a unit cube is symmetric). Reusing an
axis-aligned bounding box keeps that property: for a non-cube box the AABB is a
conservative over-approximation, and for sphere/capsule/cylinder it is the tight
bound on the two axis-aligned faces that matter. It never produces a false
"clear" verdict, only occasionally a conservative "overlap" for exotic angles -
the same tradeoff the original check documented.

## Difficulties

- **Density vs mass.** The `mass` field is actually density; that was not
  obvious until reading `destructible_body` in bevy_common_systems. Called out
  above so it is not rediscovered the hard way.
- **Required-field ripple.** Adding a non-`Default`-filled field to
  `BaseSectionConfig` broke the seven fully-explicit struct literals in
  `nova_assets/src/sections.rs` (the base-section generator). Every other site
  in the workspace already used `..Default::default()`, so a workspace
  `cargo check --all-targets` was the check that proved nothing else regressed.
- **Feature gating when testing single crates.** `nova_scenario` /
  `nova_assets` serde derives are behind the `serde` feature, which the
  workspace build unifies via the app crate but `cargo test -p <crate>` does
  not. Run those crate-scoped tests with `--features serde` (or the whole
  workspace) or the serde-dependent code fails to compile.

## Tests

- `base_section` unit tests: default is the unit cube, `aabb_half_extents` per
  shape, `to_collider` builds every variant, and a serde round-trip proving the
  field round-trips and is omitted (not serialized) when unset.
- `lint` test `overlap_uses_authored_collider_half_extents`: 0.8-spaced
  sections clip as unit cubes but sit flush once tightened to 0.8 cubes, and
  oversized 2.0 cubes clip where unit cubes would pass.
- `content_lint_gate::repo_content_tree_has_no_lint_errors` exercises the whole
  path end to end: it deserializes craft_cargob's new `collider` fields and runs
  the updated overlap lint over the entire shipped content tree, clean.
