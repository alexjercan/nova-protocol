# Add base game object abstraction

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Adds physics + health automatically; reduces boilerplate per entity. Legacy #107.

## Steps

- [x] Identify the boilerplate shared across destructible entities.
- [x] Add a base abstraction that supplies physics + health.
- [x] Adopt it where the composition is provably identical.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

Ship sections (base_section) and scenario objects like asteroids repeated the same
destructible-body makeup: `Health`, `ColliderDensity`, `ExplodableEntity`,
`Visibility::Inherited` (only the Collider shape differed).

Added a `game_object` module in nova_gameplay exposing:

    pub fn destructible_body(health: f32, density: f32) -> impl Bundle

which returns exactly those four components (health pool, physics density, explode-on-
destroy, inherited visibility), exported through the gameplay prelude. It is meant to be
paired with a Collider on an entity parented to a RigidBody.

Adopted it in both spawn sites:
- base_section: `(Name, SectionMarker, Collider::cuboid(..), destructible_body(health, mass))`
- asteroid: `(Transform, AsteroidRenderMesh, collider, destructible_body(health, 1.0))`

Both are provably identical to before - same components, same values - so behavior is
unchanged; the win is that each site now spells out only what is genuinely different
(markers + collider shape). New destructible objects get physics + health for free.

Design note: kept the abstraction a bundle *function* rather than a `#[require(..)]`
component, because Health and Collider need per-object values (a required-components
marker would force a default health/shape that no caller actually wants).

Verified: build --all-targets, clippy, fmt all green. Runtime not exercised (no display),
but composition is byte-for-byte equivalent to the previous inline bundles.

Self-reflection: resisted over-designing a rich GameObject marker; the honest, low-risk
version is a small bundle that captures the genuinely-shared components and is adopted
only where equivalence is provable. That reduces boilerplate now without betting on a
speculative component hierarchy.
