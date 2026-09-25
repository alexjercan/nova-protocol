# Static-well gravity for all mobile bodies and stable orbit placement

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,gravity,world,design

## User facts

- Research a gravity upgrade for the backlog. Immovable large planet wells pull mobile bodies; do not implement full pairwise N-body simulation or attraction between two mobile bodies.
- Ships, projectiles, and asteroids should feel gravity, both in authored scenarios and free play. Migrate any broken shipped scenarios; do not add backward-compatible behavior just to keep the old setup.
- Asteroid placement must not make every object immediately fall into a well. Explore physically credible, stable orbits, including the consequences of streaming and contact.
- This is research and a decision specification, not permission to implement the physics change.
- Owner clarification, 2026-09-25: exclusions for unpiloted ships and rocks exist for reasons we must remove, not preserve as the end state. All mobile ships, projectiles and asteroids should respond to the static-well field uniformly. True immovable planet/anchor sources are the intentional exception; no mobile-body attraction or full N-body model.

## Agent findings (source-checked against master, 2026-09-25)

- The core one-way static-well force already exists: `GravityWell`, `GravityAffected`, dominant-well hysteresis, inverse-square acceleration with surface clamp and SOI fade in `crates/nova_gameplay/src/gravity.rs:64-217,306-459`. It does not simulate N-body attraction. Asteroids of at least 50 m with no explicit mass, or asteroids with any explicit mass (including zero), become static well sources in `crates/nova_scenario/src/objects/asteroid.rs:468-491`; planets also act as static wells (`objects/planet.rs:191-212`).
- Player and AI ships, torpedoes, and gun rounds already respond to wells (`gravity.rs:264-278`, `crates/nova_ship/src/input/ai/mod.rs:347`, `crates/nova_gameplay/src/rounds.rs:1194-1223`). Unpiloted ships intentionally float (`gravity.rs:252-263,978`); dynamic small rocks do not opt in. Streamed ships are unpiloted and streamed rocks have `mass: None` (`crates/nova_world/src/streaming.rs:183-246`).
- Asteroid `mass: None` has radius-dependent behavior: at or above `GravitySettings.min_well_radius`, it becomes a static well with the default mass; smaller ones remain dynamic but do not feel gravity (`asteroid.rs:468-491`). Thus 'all asteroids fall' conflicts with an existing asteroid-as-well policy. Do not silently choose which role survives.
- Authored and streamed body spawns seed positions, not orbital velocities (`crates/nova_scenario/src/actions/spawn.rs:75-114`, `crates/nova_world/src/streaming.rs:183-246`). Simply adding `GravityAffected` to bodies spawned at rest inside a well will make them fall. The existing pure `circular_orbit_speed` and orbit-band logic are starting points (`gravity.rs:337`, `crates/nova_ship/src/flight/guidance.rs:294`), not proof of generated-orbit stability.
- Open-world bodies are sector-owned, regenerated from seed after retirement; moving or orbiting bodies can cross a cell face, outlive their well's sector, or reset position/phase on return (`crates/nova_world/src/streaming.rs`, `crates/nova_world_base/src/clusters.rs:638-988`). Persistence and moving ownership remain open in `tasks/20260824-125938/TASK.md`. This task coordinates with that spike, not settles it.
- The static-body rule is currently coupled to well creation. Existing authored planets and anchors rely on it, and the shipped Ledger mod pins many decorative rocks with `mass: Some(0.0)` (`crates/nova_scenario/src/objects/asteroid.rs:481-490`, `objects/planet.rs:208-211`, `webmods/the-ledger/ledger_01_drift_run.content.ron`). Do not remove the mass field or convert these rocks blindly. Gravity force skips bodies with `GravityWell` (`crates/nova_gameplay/src/gravity.rs:389`); rocks cannot both remain static wells and become mobile gravity-affected asteroids under that rule.
- ORBIT targets one well and uses its circular speed, while ship physics follows the dominant well; gravity planning also sums overlapping pulls in one path (`crates/nova_ship/src/flight/autopilot.rs:358-364,710`, `crates/nova_gameplay/src/gravity.rs:448-457`). Overlapping rock/planet SOIs therefore need a focused live proof, not an assertion of stable circular orbits. Numeric overlap frequencies are unmeasured estimates.

## Design options to compare (proposals, not decisions)

1. **Dynamic initial orbit:** put a chosen mobile body on a seeded tangent velocity inside a well's stable band, then let the current gravity and collisions act. Real impacts and deflections remain; contact, numerical drift, intersecting SOIs, world-seam lifetime, and re-entry must be tested.
2. **Analytic/kinematic rails with explicit handoff:** derive position/phase from well and simulation time until near the player or hit, then promote to a dynamic body. This can keep distant scenery stable, but handoff, collision authority, and 'all asteroids feel gravity' need a clear policy.
3. **Migrate static scenery to explicit immovable sources:** pin *true* authored planet/anchor scenery without claiming a mobile asteroid is exempt. If a pinned asteroid exists only as a visual prop, replace or reclassify it; mobile rocks and unpiloted hulls must still opt into gravity. This may change shipped content and requires a reviewed migration.

Do not infer that a body with no well nearby must orbit. It can coast; the selected rule must say what happens to free-space clusters, stations, debris, and unpiloted hulls.

## Decisions still open

- Exact affected populations: all ship roots including neutral derelicts, every mobile asteroid and carved fragment, all existing projectile paths, section debris, and any future cargo bodies. Distinguish a detached section (not an asteroid) explicitly rather than silently exempting it. Decide how old asteroid wells and zero-mass pinned rocks migrate to immovable planets/anchors or another *source* representation, not to gravity-exempt mobile asteroids.
- Initial velocity ownership: authored placement/velocity versus generated orbital elements; selection of the well, stable radius/plane, eccentricity, SOI overlap and distance from surfaces. Do not overwrite authored movement silently.
- Runtime motion: dynamic bodies versus distant rails, promotion triggers, collision policy, well unload and death, cross-cell ownership and return, and no-persistence phase behavior.
- Force model: keep current dominant-well/hysteresis and projectile selection semantics or revise both; preserve the no-N-body rule and document performance bounds.
- Shipped content and format policy: classify existing static rocks (including explicit zero-mass examples), bystander ships and backdrop scenarios before changing schemas; regenerate Rust-built base RON and migrate hand-authored mods/fixtures if affected.

## Proof and delivery plan

- First build a code/content inventory of all gravity-affected and stationary classes and a scenario list with current positions, well reaches and initial velocities. Include Ledger's zero-mass rock pins, season-one and range pinned planets, default 50 m rock wells, and authored anchors. Preserve before artifacts and reproduce any scenario failure before a fix.
- Census multiple seeded windows: count rock wells, mobile candidates inside overlapping SOIs, and members whose owning cell differs from the well's; measure actual encounters rather than treating formula estimates as observed frequency. Prove any chosen explicit-mass validation fails loudly for negative or non-finite input.
- Prototype dynamic tangent orbits and (if warranted) analytic rails in an isolated example against real Avian physics. Remove the reasons for opt-out: prove a neutral hull and an asteroid both receive pull from the same stationary well, can start safely, remain stable within the selected lifetime policy, and are not exempted merely because of spawn path. Check multi-orbit boundedness, fixed-step drift, SOI switches, collisions, planet retirement and cross-sector return using assertions on state, not timing thresholds.
- Check shipped scenarios, generated sectors, neutral derelicts, projectiles and carved rocks through focused tests and probes; inspect rendered output for appearance. Compare matched repeat sets for physics cost against a named pre-change reference, including contacts.
- Before implementation, review exact affected interfaces, defaults, error rules and migration paths with the owner. Update code, generated content, lint, wiki and changelog only after that approval.

## Done when

- Existing versus missing behavior is documented with current source and content evidence, including all affected populations and shipped scenarios.
- A static-well orbit/streaming design is chosen by the owner or left as explicit options with consequences and named prototype proofs; no N-body simulation is proposed.
- Scenario/content and sector lifetime/persistence impacts are recorded, with owner decisions separated from agent proposals.
- Follow-up implementation tasks are scoped only after the model is reviewed. This backlog research task does not implement gravity.
