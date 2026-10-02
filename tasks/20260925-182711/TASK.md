# Static-well gravity for all mobile bodies and stable orbit placement

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.15.0,gravity,world,design

## User facts

- Research a gravity upgrade for the backlog. Immovable large planet wells pull mobile bodies; do not implement full pairwise N-body simulation or attraction between two mobile bodies.
- Ships, projectiles, and asteroids should feel gravity, both in authored scenarios and free play. Migrate any broken shipped scenarios; do not add backward-compatible behavior just to keep the old setup.
- Asteroid placement must not make every object immediately fall into a well. Explore physically credible, stable orbits, including the consequences of streaming and contact.
- This is research and a decision specification, not permission to implement the physics change.
- Owner clarification, 2026-09-25: exclusions for unpiloted ships and rocks exist for reasons we must remove, not preserve as the end state. All mobile ships, projectiles and asteroids should respond to the static-well field uniformly. True immovable planet/anchor sources are the intentional exception; no mobile-body attraction or full N-body model.
- Owner decision, 2026-10-02: prototype seeded dynamic orbits first. Measure drift, collisions and sector retirement before deciding whether analytic rails are needed. This approves a disposable prototype, not an implementation interface, content migration or physics change.

## Agent findings (read-only master review, 2026-10-02; recheck after PR #104)

- One-way static-well force already exists: `GravityWell`, `GravityAffected`, dominant-well hysteresis, inverse-square acceleration with surface clamp and SOI fade (`crates/nova_gameplay/src/gravity.rs:243-294,378-453`). It checks every affected body against every well, applies only the dominant pull, and does not simulate N-body attraction. A massed asteroid becomes a static source at any radius; a massless one stays dynamic without a well (`crates/nova_scenario/src/objects/asteroid.rs:464-500`). The old radius-based default for *asteroids* is gone. Planets without mass still become default-mass wells when their radius reaches `min_well_radius` (`crates/nova_scenario/src/objects/planet.rs:188-209`).
- Player and AI ships, torpedoes, and swept gun rounds already respond to wells (`gravity.rs:259-273`, `crates/nova_ship/src/input/ai/mod.rs:347-349`, `crates/nova_gameplay/src/rounds.rs:1191-1223`). Gun rounds are not rigid bodies; their sweep picks the strongest well without hysteresis. An AI ship that becomes neutralized keeps its gravity marker, while an unpiloted authored ship or generated derelict does not. Streamed intact ships on master also lack a pilot; PR #104 changes that, so recheck the population after merge.
- Well-less authored and streamed rocks are dynamic but do not feel gravity. Generated rocks at or above 50 m nominal radius get `mass: Some(4000.0)` and become static wells (`crates/nova_world_base/src/clusters.rs:116-121,813-817`); streaming forwards manifest mass (`crates/nova_world/src/streaming.rs:253`). Generated planetoids carry mass (`clusters.rs:902-907,1018-1021`). Do not treat existing rock wells or `Some(0.0)` pins as mobile sinks by merely adding `GravityAffected`.
- Carved chunks start kinematic, become dynamic after a 0.5 s grace and live 30 s (`crates/nova_gameplay/src/integrity/chunk.rs:126-132,230-250`); detached sections similarly start kinematic and inherit drift plus a kick, with 30 s lifetime (`crates/nova_gameplay/src/integrity/explode.rs:85,405-446`). Neither opts into gravity today. Hanabi rock chips are visual particles, not physics bodies. Verify how force interacts with kinematic grace before changing either spawn path.
- Authored and streamed spawns seed positions, not orbital velocities (`crates/nova_scenario/src/actions/spawn.rs:187-231`, `crates/nova_world/src/streaming.rs:183-253`). Adding gravity to a body at rest inside a well makes it fall. Existing `circular_orbit_speed` and orbit-band logic are starting points, not proof of a stable generated orbit. ORBIT uses one well (`crates/nova_ship/src/flight/autopilot.rs:355-385`); its summed pull at `:398-424` is a conservative braking budget, not an N-body force. Overlapping SOIs still need measurement.
- Open-world bodies are children of their owning sector root and regenerate after retirement (`crates/nova_world/src/streaming.rs:840-861`); cluster members can belong to a different sector from the cluster (`clusters.rs:950-976`). Moving bodies can cross a cell face, lose their well when its sector retires, or reset phase on revisit. Chunks and detached sections appear to spawn outside the sector root and may outlive it for up to 30 s; this is unverified. Moving ownership/persistence is also tracked in `tasks/20260824-125938/TASK.md`.
- Content inventory (read-only): the Ledger mod's six hand-authored scenes each have a massed planet and moon; 44 rocks use `mass: Some(0.0)` as pinned props, and Ledger 02/03/05 contain unpiloted ships. Base builders place massed planets in `tutorial/range.rs`, `main_menu/shared.rs`, `season_one/stage.rs`, and unpiloted ships in `season_one/stage.rs`, `duel.rs`, `gauntlet.rs`, `range.rs`. Scenario config has no initial velocity/orbit field. Asteroid mass must be finite and non-negative at lint/load (`crates/nova_scenario/src/lint/scenario.rs:511-521`, `objects/asteroid.rs:310-314`); planet mass must be positive (`lint/scenario.rs:1315-1338`). `GravityWell::from_mass` caps surface gravity (`gravity.rs:96-108`).

## Design options to compare (proposals, not decisions)

1. **Dynamic initial orbit (selected for the first disposable prototype only):** put a chosen mobile body on a seeded tangent velocity inside a well's stable band, then let the current gravity and collisions act. Real impacts and deflections remain; contact, numerical drift, intersecting SOIs, world-seam lifetime, and re-entry must be tested.
2. **Analytic/kinematic rails with explicit handoff:** derive position/phase from well and simulation time until near the player or hit, then promote to a dynamic body. This can keep distant scenery stable, but handoff, collision authority, and 'all asteroids feel gravity' need a clear policy.
3. **Migrate static scenery to explicit immovable sources:** pin *true* authored planet/anchor scenery without claiming a mobile asteroid is exempt. If a pinned asteroid exists only as a visual prop, replace or reclassify it; mobile rocks and unpiloted hulls must still opt into gravity. This may change shipped content and requires a reviewed migration.

Do not infer that a body with no well nearby must orbit. It can coast; the selected rule must say what happens to free-space clusters, stations, debris, and unpiloted hulls.

## Decisions still open

- Exact affected populations: all ship roots including neutral derelicts, every mobile asteroid and carved fragment, swept gun rounds, torpedoes and detached sections; decide whether any future cargo physics body belongs here. Distinguish detached sections from asteroids. Choose whether generated 4000-mass rock wells and authored zero-mass pins remain explicit immovable sources, become planets/anchors or props, or lose their well. Decide whether neutralized AI wrecks and generated derelicts must share the same gravity behavior.
- Initial velocity ownership: authored placement/velocity versus generated orbital elements; selection of the well, stable radius/plane, eccentricity, SOI overlap and distance from surfaces. Decide whether authored bodies inside a well require explicit velocity/orbit intent and fail lint without it. Do not overwrite authored movement silently.
- Runtime motion: first prototype fully dynamic; decide from its measurements whether distant rails are needed. If so, specify promotion triggers and collision authority. Independently decide well unload/death, cross-cell ownership, return and no-persistence phase behavior. Verify kinematic debris grace and whether top-level debris outlives a sector.
- Force model: keep current dominant-well/hysteresis and projectile selection semantics or revise both; preserve the no-N-body rule and document performance bounds.
- Shipped content and format policy: classify authored massed rock wells (including explicit zero-mass pins), dynamic well-less rocks, bystander ships and backdrop scenarios before changing schemas; regenerate Rust-built base RON and migrate hand-authored mods/fixtures if affected.

## Proof and delivery plan

- First build a code/content inventory of all gravity-affected and stationary classes and a scenario list with current positions, well reaches and initial velocities. Include Ledger's zero-mass rock pins, season-one and range pinned planets, explicitly massed rock wells, and authored anchors. Preserve before artifacts and reproduce any scenario failure before a fix.
- Census multiple seeded windows: count rock wells, mobile candidates inside overlapping SOIs, and members whose owning cell differs from the well's; measure actual encounters rather than treating formula estimates as observed frequency. Prove any chosen explicit-mass validation fails loudly for negative or non-finite input.
- First prototype seeded dynamic tangent orbits in a disposable isolated example against real Avian physics; do not edit production gravity or content yet. Remove the reasons for opt-out: prove a neutral hull and an asteroid both receive pull from the same stationary well, can start safely, remain stable within the selected lifetime policy, and are not exempted merely because of spawn path. Check multi-orbit boundedness, fixed-step drift, SOI switches, collisions, planet retirement and cross-sector return using assertions on state, not timing thresholds. Only prototype analytic rails if measured results justify the extra handoff and ownership subsystem.
- Check shipped scenarios, generated sectors, neutral derelicts, projectiles and carved rocks through focused tests and probes; inspect rendered output for appearance. Compare matched repeat sets for physics cost against a named pre-change reference, including contacts.
- Before implementation, review exact affected interfaces, defaults, error rules and migration paths with the owner. Update code, generated content, lint, wiki and changelog only after that approval.

## Done when

- Existing versus missing behavior is documented with current source and content evidence, including all affected populations and shipped scenarios.
- A static-well orbit/streaming design is chosen by the owner or left as explicit options with consequences and named prototype proofs; no N-body simulation is proposed.
- Scenario/content and sector lifetime/persistence impacts are recorded, with owner decisions separated from agent proposals.
- Follow-up implementation tasks are scoped only after the model is reviewed. This backlog research task does not implement gravity.
