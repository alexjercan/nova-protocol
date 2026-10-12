# Stations: dynamic thrusterless body that self-levels; orbit proof

- STATUS: CLOSED
- PRIORITY: 87
- TAGS: v0.16.0, stations

## User facts
- Epic `20261012-095958`. Decision 3 and 3a (2026-10-12): a station is a dynamic body affected by gravity and hits, never `RigidBody::Static`; its controller sections damp spin so it self-levels; a cluster with a planet gets its station in orbit (circular orbital velocity at materialize), else it stands beside a rock with clearance; no station-keeping thrust; the frozen pose is whatever it was when the player left. Needs a test.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Gravity wells live in `crates/nova_gameplay/src/gravity`; controller sections and the AI stance in `crates/nova_ship/src/input/ai/`. Planetoids are 500-1600 m (`crates/nova_world_base/src/clusters.rs:116-126`); 500 m clearance (`crates/nova_world_base/src/layered.rs:31`).

- Handed from `20261012-095959` (2026-10-12): section invulnerability for stations (design page, sections invulnerable in v1) is owned HERE; nothing in the code can do it yet. The station root already self-levels through its Controller section with no thruster (`controller_section.rs:481-533`); `system_station_body` nudges 0.5 rad + 0.3 rad/s and sees 0.0038 rad off spawn after settling.

- Landed (2026-10-12): `StationMarker`/`StationDesignId` moved to `nova_gameplay::markers` (re-exported from the station prelude) so the health store can gate. One choke point: `on_damage` in `crates/nova_gameplay/src/integrity/health.rs` walks `ChildOf` to the root and spends nothing under a `StationMarker`, so no `HealthZeroMarker` is ever inserted and every disable/neutralize/destroy path stays shut, living or derelict alike. Unit test `a_hit_under_a_station_root_leaves_every_pool_whole`.
- No controller change: `insert_controller_section_target` parks the spawn rotation, `sync_controller_section_forces` applies `PDControllerOutput` as torque only (`crates/nova_ship/src/sections/controller_section.rs:463-533`); thrust comes only from Thruster sections, which the station lint refuses. `STATION_LEVEL_TOLERANCE_RAD = 0.03` in `crates/nova_scenario/src/objects/station.rs`.
- Orbit: `station_orbit_velocity(mu, well, position)` reuses `circular_orbit_speed`; `seeded_orbit_velocity` stays private to the rock generator (seed-keyed sign flip). `STATION_ORBIT_TOLERANCE = 0.15`: gravity is a force held constant per fixed step, so an orbit gains about 1.5 % of its binding energy per orbit on the proof's 30 s orbit before any hit (measured, `track_orbit` energy trace), and a 1 % tangential kick adds about 4 % radius. Generator placement is `20261012-100001`.
- Proofs: `system_station_body` (void nudge, 0.0038 rad) and new `system_station_orbit` (200 m planet, mass 4000, station at 450 m on a 30 s orbit; real Serpent torpedo at 5.2 u, authored 0.5 rad / 0.3 rad/s / 1 % kick): radius 45.00..49.30 u, drift 0.0956, 0.0007 rad off attitude after 18.88 rad swept, 71 pools whole.
- `content lint` reports 4 pre-existing example-mod errors (`example_*` ships, disconnected link-point graph at a thruster); not touched here.

- Orchestrator close 2026-10-12: commit 506b8dc57. `STATION_LEVEL_TOLERANCE_RAD = 0.03`, `STATION_ORBIT_TOLERANCE = 0.15` (measured: the 64 Hz constant-per-step gravity gains ~1.5 % binding energy per orbit before any hit; reducing that gain is engine debt, not this task), `station_orbit_velocity` reuses `circular_orbit_speed`; invulnerability gated once in `on_damage` under `StationMarker` (moved to nova_gameplay markers), living or dead; proofs `system_station_body` and `system_station_orbit` (three orbits + torpedo, drift 0.0956, level 0.0007 rad). Follow-up fix commit: the 095959 lint probe had left four example-mod ships with a docking port for a thruster; restored.

## Delivery gate
- Reuse the ship controller for attitude only; show which input the station controller writes and why thrust stays zero. Orbit: compute circular velocity from the planet mass at materialize; state the tolerance.

## Verification
- Asserted examples: a station under a gravity well and a torpedo hit stays level within a stated angle after N seconds; an orbiting station survives three orbits and a torpedo hit without tumbling or decaying into the well. Run them under Xvfb, not only `cargo check`.

## Done when
- Both proofs pass and the self-levelling angle and orbit tolerance are named constants with a reason comment.
