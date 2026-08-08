# Configurable section collider shape and size (replace hardcoded Collider::cuboid(1,1,1))

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.7.0, feature, physics, content

## Problem

The per-section physics collider is hardcoded to `Collider::cuboid(1.0, 1.0, 1.0)`
in several places. This forces every section to be a unit cube regardless of the
section's actual visual shape or size. The recently added `craft_cargob` spaceship
has non-cube exterior shapes whose colliders no longer match their geometry.

Known hardcoded sites (from grep at task creation):
- crates/nova_scenario/src/actions.rs:1260
- crates/nova_scenario/src/objects/salvage.rs:562
- crates/nova_gameplay/src/integrity/glue.rs:312, 659
- crates/nova_gameplay/src/flight.rs (many, mostly tests)
- crates/nova_scenario/src/lint.rs:661 (doc comment referencing the assumption)

## Goal

Add a config setting on the section data model that lets content authors specify
the collider shape and size, defaulting to the current unit cube so existing
content is unaffected. Wire it through the spawn paths that currently hardcode
`Collider::cuboid(1.0, 1.0, 1.0)`. Support at least cuboid with authorable
half-extents; consider sphere/capsule if cheap. Apply it to craft_cargob's
exterior sections as the exercising example.

## Notes

Worktree: /home/alex/.cache/sprouts/nova-protocol/section-collider-config
Branch: section-collider-config

The lint at lint.rs:661 encodes the unit-cube assumption for section overlap
detection - revisit it so custom collider sizes do not break the lint.

## Design

`base_section(config)` at base_section.rs:104 and `preview_section` at :132 both
hardcode `Collider::cuboid(1.0, 1.0, 1.0)`. `BaseSectionConfig` (the injection
point, in scope at both) is RON-authored. `destructible_body(health, DENSITY)` -
the 2nd arg is density; avian derives real mass from density * collider volume,
so a custom collider size scales the section's physical mass (bigger = heavier).
That is acceptable and arguably more correct; documented as a known effect.

Add a `SectionCollider` enum { Cuboid{size:Vec3}, Sphere{radius}, Capsule{radius,
length}, Cylinder{radius,height} } with Default = Cuboid{size:ONE}, a
`to_collider()` -> avian Collider, and `aabb_half_extents()` for the lint.
avian 0.7 verified: cuboid(x,y,z) full lengths, sphere(r), cylinder(r,h),
capsule(r,length), all Y-aligned. Field is `collider: Option<SectionCollider>`
with `#[serde(default, skip_serializing_if = "Option::is_none")]` (mirrors
impact_sound) so content omitting it stays byte-identical and RON parity holds;
None -> unit cube.

Lint: generalize the overlap check to AABB half-extents (a.half + b.half per
axis). Inline sources resolve their authored collider; Prototype sources fall
back to the unit cube (catalog not in scope), preserving current behavior.

## Steps

- [x] Add `SectionCollider` enum + Default + `to_collider()` + `aabb_half_extents()` in base_section.rs
- [x] Add `collider: Option<SectionCollider>` to `BaseSectionConfig` (serde default + skip_serializing_if)
- [x] Wire `base_section()` and `preview_section()` to `config.collider` (unit-cube fallback)
- [x] Generalize `check_section_overlaps` lint to actual AABB half-extents + update doc comment
- [x] Exercise it: give a craft_cargob exterior section a non-cube collider
- [x] Tests: to_collider/aabb unit tests, lint overlap with custom sizes, RON parity stays green
- [x] check + fmt + clippy on changed crates; write docs/ note (what/why, density-mass effect, difficulties)

## Close-out

Delivered. `SectionCollider` enum (Cuboid/Sphere/Capsule/Cylinder) added to
`BaseSectionConfig` as `Option`, resolving to the unit cube when unset;
`base_section`/`preview_section` build it; the section-overlap lint now sums
real AABB half-extents; craft_cargob's two beveled top-front corner cubes carry
a tightened 0.8 collider. Full design, the density-vs-mass decision, and the
difficulties are in docs/design/section-collider-config.md. Reviewed R1 APPROVE
(one MINOR + one NIT, both accepted/documented) - tasks/.../REVIEW.md.

Verification: `cargo check --workspace --all-targets --features debug` clean;
new base_section tests (4), lint test, and `content_lint_gate` (deserializes the
new cargob RON + lints the whole tree clean) all pass. Branch merged up to date
with master before landing.

