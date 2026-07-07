# Review: Asteroid RigidBody husk lingers after collider child explodes

- TASK: 20260706-212910
- BRANCH: fix/asteroid-husk

## Round 1

- VERDICT: APPROVE

Diff is a single file (`crates/nova_scenario/src/objects/asteroid.rs`, +91): one marker
component, one `Add IntegrityDestroyMarker` observer, one `Update` despawn system, both
registered in `AsteroidPlugin::build`, plus two co-located tests.

Verified:

- Correctness. The observer reads `ChildOf` off the entity that just got
  `IntegrityDestroyMarker`, so the parent link is captured before the node despawns later in
  the pipeline - no dependency on the child outliving the mark. The `AsteroidMarker` guard
  scopes the despawn to asteroid roots only; ship sections (whose parent is a ship root) are
  untouched, as the negative test asserts. `try_despawn` is despawn-safe and the
  `AsteroidHuskDespawn` tag makes the despawn idempotent (entity leaves the query once gone).
- Race avoidance. Marking in the observer + despawning in a later `Update` system is the right
  call: it lets the other destruction observers (explosion fragments, node despawn) run first.
  Fragments are separate `TempEntity`s, not children of the root, so recursive `try_despawn`
  of the root does not eat them.
- Tests assert behavior, not execution: positive case checks the husk is gone, negative case
  checks a non-asteroid parent survives. Both pass.
- Full suite green: `cargo test --workspace` (42 nova_gameplay, 2 new nova_scenario,
  examples_smoke under Xvfb 44.9s), `cargo clippy --workspace --all-targets` clean (the lone
  `struct update` warning is pre-existing in nova_gameplay lib test, outside this diff).

No BLOCKER/MAJOR/MINOR findings. The change is minimal, well-scoped, and matches the repo's
observer + system conventions.
