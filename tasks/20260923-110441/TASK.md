# Investigate asteroid and planet vulnerability ownership

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, scenario, destruction, world, research

## User facts

- The `invulnerable: bool` field on `AsteroidConfig` and `PlanetConfig` is obsolete because the body type should own vulnerability.
- Asteroids should always be carvable and destructible.
- Planets should remain invulnerable until planet destruction is separately designed and proven.

## Decisions

- Remove `invulnerable` from both `AsteroidConfig` and `PlanetConfig`. Do not replace it with another durability, carving, static-body, or invulnerability option.
- Always install the existing asteroid carve, damage, collision-event, and destruction behavior, including for streamed asteroids.
- Never install asteroid damage behavior for planets. Planet invulnerability is an invariant of `PlanetConfig`, not authored data.
- Migrate immutable asteroid-shaped planetoids and gravity anchors to `PlanetConfig` where they are semantically planets. Let ordinary rocks and test cover become destructible rather than preserving a special immutable asteroid path.
- Keep the legacy asteroid in `planet_types` as a visual comparison, but make it follow normal destructible asteroid behavior.
- Remove redundant runtime invulnerability marker components and the `invulnerable` field from probe body snapshots. Body kind is authoritative; do not add a compatibility alias for the unshipped schema.
- Streamed asteroid destruction remains session-local until world mutation persistence is designed. A retired and revisited sector regenerates its pristine deterministic contents, as it already does for destroyed derelicts.
- If making all active streamed asteroids destructible is too expensive, optimize damage or collision activation without restoring author-controlled vulnerability.
- Keep the approved clamped planet sizes for migrated planetoids and gravity anchors (owner decision, 2026-09-24).
- Planet destruction remains out of scope and unsupported. It requires a separate design for damage, render and collider failure, gravity-well teardown, orbit effects, IDs, objectives, and persistence.

## Agent findings

- `AsteroidConfig::invulnerable` changes construction: destructible rocks receive `DamageMarks` and `CollisionEventsEnabled`, while invulnerable rocks omit the carve pipeline (`crates/nova_scenario/src/objects/asteroid.rs`).
- `PlanetConfig::invulnerable` is not a choice. `PlanetConfig::validate`, scenario lint, and planet spawning reject `false` because planets have no damage or destruction implementation (`crates/nova_scenario/src/objects/planet_type.rs`, `objects/planet.rs`, and `lint/scenario.rs`).
- Full asteroid destruction already fires `OnDestroyedEvent` and despawns the asteroid root. A gravity well attached to that root is removed with it, and orbit tracking retains the well ID needed to report an end edge.
- `nova_world` currently hardcodes streamed asteroid configs as invulnerable. Sector retirement discards all local mutations and deterministic regeneration restores pristine bodies on return; this limitation already applies to destroyed streamed derelicts.
- The extra streamed-world cost is unmeasured. Every active asteroid can gain empty damage state and collision-event reporting, while carve remeshing is paid only after damage. The maximum active-window case needs a matched performance proof.
- Existing invulnerable asteroid uses split into old planetoid or gravity-anchor representations, visual comparisons, and test or screenshot cover. The latter cases do not establish a gameplay need for immutable asteroids.
- `AsteroidInvulnerable` and `PlanetInvulnerable` are read by probe snapshots but do not own the damage behavior. With type-owned policy, the body kind fully determines the answer.

## Delivery

1. Preserve the current planet `false` lint and load refusal as before evidence.
2. Change the owning scenario configs first by removing both fields and their validation branches.
3. Make asteroid spawning always install the existing carve and destruction components; keep planet spawning permanently outside that pipeline.
4. Remove invulnerability marker components, reflection registration, snapshot fields, docs, tests, and stale failure text.
5. Use compiler and content errors to migrate every Rust builder, generated or shipped RON source, editor template, example, probe fixture, and world materializer.
6. Convert semantic planetoids and immutable gravity anchors to explicit `PlanetConfig` objects. Do not preserve asteroid aliases or adapters.
7. Regenerate owned content and run focused lint, lifecycle, world-streaming, and performance proofs.

What dies: both config fields, both runtime marker components, planet false-value checks, immutable asteroid construction, and probe snapshot vulnerability reporting.

What may break: authored RON schema, Rust builders, editor templates, examples that use asteroid-shaped planetoids, captures that rely on immutable cover, probe golden data, and streamed-world performance.

What must fail loudly: stale content that still authors either field, invalid migrated planet configurations, malformed generated bodies, and any lifecycle proof that leaves a gravity or orbit reference after asteroid destruction.

## Verification

- Preserve a before artifact showing that `PlanetConfig { invulnerable: false }` is currently rejected by lint and loading.
- Prove an authored asteroid always receives carve and collision-event state and can be fully destroyed.
- Prove destroying a massive asteroid removes its body, gravity well, children, authored ID resolution, and active orbit relationship while emitting the expected destruction and orbit-end behavior.
- Prove planets receive no asteroid damage state and remain non-destructible without an authored flag.
- Prove a streamed asteroid can be carved and destroyed, then document and observe pristine regeneration after sector retirement and return.
- Compare matched maximum-window streamed-world runs before and after the migration. Inspect frame and physics evidence against the named pre-change run; do not use an absolute timing assertion.
- Run focused content lint and confirm no source or generated content retains either field.

## Proof evidence

- Planets: `a_planet_takes_no_damage_state` (`crates/nova_scenario/src/objects/planet.rs:398`) spawns a planet and asserts a `Collider` on its node and no `DamageMarks` or `CollisionEventsEnabled` on its root or node.
- Authored asteroids: `asteroid_geometry_is_the_only_durability` (`crates/nova_scenario/src/objects/asteroid.rs:1161`) asserts a massive rock node has `DamageMarks` and `CollisionEventsEnabled`.
- Destruction: `an_exhausted_authored_rock_takes_its_well_and_id_with_it` (`crates/nova_scenario/src/objects/asteroid_carve.rs:1128`) spawns a 45 000-mass rock through the `ScenarioObjectConfig` action, puts one mark wider than the rock on its node, and lets seed, carve and remesh run. It asserts the root and every child despawn, no `GravityWell` remains, `scoped_entities` no longer resolves the id, and `OnDestroyed` fires once.
- ORBIT end, as a chain across owners:
  1. The destruction test above proves exhaustion despawns the root that carries the `GravityWell` (`asteroid_carve.rs:798-803`).
  2. `orbit_disengages_when_the_well_dies` (`crates/nova_ship/src/flight/tests/orbit.rs:254`) proves ORBIT removes `Autopilot` when its well entity is gone (`crates/nova_ship/src/flight/autopilot.rs:332-335` and `:689-692`).
  3. `orbit_lifecycle_events_are_edge_triggered` (`crates/nova_scenario/src/loader/trackers.rs:802`) proves a despawned well plus a removed `Autopilot` emits exactly one `OnOrbitEnd` from the well id that `OrbitEcho` kept (`trackers.rs:105-107`).
- Streamed regeneration: `system_world_sectors` claim 13, `outcome: a destroyed streamed asteroid returns pristine after its sector retires` (roster row `examples/systems/system_world_sectors.rs:40`; beats wired at `:465` and `:486`; `exhaust_retiring_rock` at `:579`; `report_regenerated_rock` at `:1324` emits the marker at `:1354`). Before the +X crossing, the range puts one crater wider than the rock on the first streamed rock in the retired face, and waits for the carve chain to despawn it. After the return, it asserts the cell has a new root, the rock id resolves again to an `AsteroidMarker`, and its node has an empty `DamageMarks`. No persistence is asserted. Roster slug added and `SYSTEMS_INVARIANTS` is 445.
- `probe run system_world_sectors --correctness-only` (2026-09-24): OK, 16 of 16 markers, 0 violations over 363 frames, log clean. The log shows one remesh delivery, `integrity: destroyed 1 node`, `retiring (0, -1, 2)`, then `rock 'sector_0_n1_2_body_0' in (0, -1, 2) came back pristine`.
- Immutable-cover capture: `lesson_combat_field` lost its `invulnerable` cover rock, and a hardware capture of `combat_cover` showed the 16 m rock bored through about cell 11 with the covered hostile hit in later cells. The `open fire` step (`examples/screenshots/lesson_combat_field.rs:534`) now records every `Health` node under the covered hostile before the trigger goes down. The end step `the covered hostile is untouched` (`:574`) fails if any recorded node is gone, has come off the hull (shed cladding), or is below max health. At 16 m it failed with the root at 4710/6240, sections gone and plates shed; 18 m and 19 m also failed. `ROCK_RADIUS` is now 20 m (`:129`). Three 20 m hardware captures (RTX 3060 Ti, Vulkan, Xvfb) with this check passed. The inspected 20-cell sheet shows every round and chip ends at the rock's near face, with no sparks or debris at the hostile, and the log has only the known X11 and mod-bundle warnings. The allegiance still keeps all five hulls clear. `probe run lesson_combat_field --correctness-only` is OK with a clean log. That mode does not hold the burst over a sheet, so only the capture run observes cover wear.
- Before artifact: [proof/before-planet-false-refusal.md](proof/before-planet-false-refusal.md) records the `8a85df12a` lint and load refusal of `invulnerable: false`, with both test commands and results.
- Focused unit tests (2026-09-24), each `nix develop --command cargo test -p <crate> --lib <path> -- --exact`, 1 passed and 0 failed: the planet, asteroid, and destruction tests above, `loader::trackers::tests::orbit_lifecycle_events_are_edge_triggered`, `flight::tests::orbit::orbit_disengages_when_the_well_dies`, `objects::planet_type::tests::a_planet_refuses_the_removed_invulnerable_key`, and `objects::asteroid::tests::an_asteroid_refuses_the_removed_invulnerable_key`.
- Content lint (2026-09-24): `nix develop --command cargo run content lint` reports 0 errors, 0 warnings, 0 findings over 15 balance-audited scenarios and 20 creative maps. No `.ron` source or generated content retains `invulnerable`; the remaining Rust hits are the two stale-key refusal tests.
- Matched performance (2026-09-24), `world_sectors` only, with one disposable identical edit that enabled frametime capture on both trees. Baseline `8a85df12a` (`/tmp/nova-perf/out-base/8a85df12a/world_sectors/`) against this change applied as the uncommitted working tree on `4606841cc` (`/tmp/nova-perf/out-after/4606841cc/world_sectors/`). Release build, software Vulkan llvmpipe, 1280x720, 5 repeats of 120 frames each. All repeats admitted, with no refresh-cap suspicion and no refused captures.
  - Baseline: means 185.7-191.8 ms, median p99 379.7 ms, p99 spread 8.7%.
  - After: means 183.0-188.6 ms, median p99 350.8 ms, p99 spread 4.2%. Worst reported delta +0.3%.
  - Fixed-step clamp amplification runs 4-16 steps per frame on llvmpipe, so these numbers ground no performance delta in either direction.
  - Census is identical: 2142 entities, 709 archetypes, 356 mesh instances. The expected difference is +2 components on the 352 asteroid collider nodes (archetype of 28 components before, 30 after); `AsteroidInvulnerable` on 356 entities is gone and `CollisionEventsEnabled` on 356 entities appears. The census lists only its 40 most common components, so it does not name the second added node component.
  - This measures the matched full maximum-window spawn and stream lifecycle, crossing and return, not a stationary bubble. It finds no grounded regression. It does not exercise asteroid contacts, so the runtime cost of collision events under contact remains unmeasured. Optimize damage or collision activation only if future evidence shows a problem.

## Done when

- Both config fields, marker components, validation branches, snapshot fields, aliases, and obsolete tests are deleted.
- Every content source, Rust caller, editor path, example, world generator, probe consumer, and invalidated document uses the type-owned policy.
- Semantic planetoids and immutable gravity anchors use `PlanetConfig`; ordinary asteroids are destructible.
- Focused destruction, gravity, orbit, streaming, lint, and matched performance proofs pass with inspected evidence.
- Remaining mutation persistence limits are stated without claiming persistent asteroid destruction.

## Limitation

Streamed asteroid and derelict destruction is session-local. A retired and revisited sector regenerates its pristine deterministic contents until world mutation persistence is designed. Planets remain non-destructible.
