# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- @alexjercan the bottom-left flight status text is replaced by diegetic readouts: a speed chip and an engaged-mode chip (verb + phase) anchored to the ship beside the velocity sphere, and an ORBIT radius spoke holo (well-to-ship line with the current radius riding it); the GRAV coasting cue is retired in favor of the gravity sphere and the keybind hint cluster re-docks to the freed corner; the ring's planned `r | v_circ` chip is retired as redundant with the new readouts

## [0.4.1] - 2026-07-10

### Fixed

- @alexjercan release-flow: install the `x86_64-apple-darwin` std for the pinned nightly toolchain, fixing the macOS universal binary build (E0463 in the x86 half)

### Changed

- @alexjercan CI: one `--features debug` feature set across clippy, tests and the examples smoke step, so the tree builds once instead of rebuilding Bevy per feature flip; cache also saves on failed runs
- @alexjercan CI: the windowed examples smoke test is a separate non-blocking step (with full backtraces and a wider failure log tail) while a runner-only taffy panic is investigated (task 20260710-143138); `cargo test --workspace` remains the blocking gate

## [0.4.0] - 2026-07-10

### Added

- @alexjercan add proportional-navigation guidance to torpedoes so they track maneuvering targets
- @alexjercan projectiles inherit the ship's rotational muzzle velocity when fired
- @alexjercan add example test ranges: torpedo bay (`06_torpedo_range`), PDC turret (`08_turret_range`), and a minimal `nova_gameplay` showcase (`10_gameplay`)
- @alexjercan add live turret-range tuning sliders and an FPS/version overlay to the examples
- @alexjercan wire the `bevy_common_systems` autopilot + screenshot harness into the examples for headless smoke testing
- @alexjercan add torpedo target aim-assist (angular lock-on cone, easier than a raycast) and a reticle that sizes itself to the locked target
- @alexjercan add an expanding-sphere blast-radius visual on torpedo detonation (wasm-safe mesh effect, shows the actual area of effect)
- @alexjercan add `11_com_range` example: live center-of-mass / attached-centroid gizmos, spin and kill-section hotkeys, and a headless assertion that mass properties follow section destruction
- @alexjercan the scenario-loading smoke tests (`03_scenario`, `10_gameplay`, `07_torpedo_guidance`) observe `ScenarioLoaded` via a shared `assert_scenario_loaded` harness helper and fail the headless run if scenario init is trivial (wrong id, zero handlers/objects) or never fires
- @alexjercan first audio: placeholder SFX for explosions, impacts, turret fire, torpedo launches and a throttle-tracking thruster loop, with distance attenuation and per-source throttling; placeholder WAVs are committed so the game is audible out of the box
- @alexjercan combat juice: subtle trauma-model camera shake on impacts and destructions, plus wasm-safe expanding gizmo hit/impact flash rings, both distance-attenuated and per-area throttled
- @alexjercan flight-assist overhaul: the flight computer turns pilot intent into forces the surviving sections can actually produce - assisted mode holds a commanded velocity (WASDQE nudges, X brake latch, 30 u/s soft cap) through the spooled thrusters plus a controller-bound RCS budget, Z toggles a direct-thrust Newtonian mode, and the HUD gains an FA/speed readout
- @alexjercan HUD screen-projected-indicator substrate (entity/point anchors, apparent-size sizing, clamp-to-edge with direction arrow); the torpedo reticle and autopilot GOTO marker become thin consumers of it
- @alexjercan HUD turret lead/intercept pips (one amber pip per player turret on its computed aim point) and a locked-target info readout beside the reticle (range, closing speed, health bar)
- @alexjercan player targeting arc: a dedicated targeting module with close-range signature auto-acquisition (550 m), a focus dwell (1.5 s) that unlocks a per-section component fine-lock with aim-snap and cycle inputs (bracket keys, dpad, scroll wheel), and HUD component-lock markers with selection highlight and a focus meter
- @alexjercan player turrets auto-aim from the target lock through a three-tier feed: fine-locked section, else the locked ship's live structure, else the camera ray - with the lock's velocity feeding a true intercept lead
- @alexjercan minimal faction/relation model (hostile/neutral/own): ship markers carry an allegiance, projectiles copy the shooter's at spawn, signature acquisition only grabs hostiles, and the lock reticle tints by relation
- @alexjercan AI combat-behavior wave: a behavior state machine (Idle/Patrol/Engage/Evade/Retreat) with threat-tiered target selection over the relation model; fire discipline (turret lead feed, leaded aim-point and range gates, staggered burst cadence); point-defense turret priority that pulls the guns onto inbound torpedoes in every state; a standoff orbit/strafe envelope replacing pure pursuit; patrol waypoint routes and idle station-keeping flown through the real autopilot (new position-goal GOTO variant); evasion under fire driven by a threat memory of recent attackers (jink maneuvers, attacker-biased targeting); and torpedo launches from Engage gated by a launch envelope and per-bay cadence
- @alexjercan the flight computer balances thrust through the live center of mass: differential throttle across the firing set nulls burn torque, and off-axis thrusters are recruited for counter-torque so a single surviving drive on a damage-shifted hull still holds heading
- @alexjercan torpedo bays emit a launch particle burst on fire, mirroring the turret muzzle flash
- @alexjercan CI workflow: cargo fmt, clippy and the full workspace test suite (windowed smoke examples under Xvfb) run on every PR and push to master

### Changed

- @alexjercan ship handling is now mass-legible: rotation commands slew at a turn rate derived from the flight computer's torque budget and the hull's live inertia (stripped ships snap, heavy builds lumber); ship controller max_torque retuned 100 -> 40
- @alexjercan the chase camera gained weight: smoothing on all gameplay modes and a burn push-back that leans the camera with the spooled main drive
- @alexjercan consume the integrity, health, blast and mesh-slicer systems from `bevy_common_systems` instead of the in-tree copies
- @alexjercan split the torpedo section into its own module with config-driven blast parameters
- @alexjercan turret now leads moving targets with intercept aim
- @alexjercan enrich the `ScenarioLoaded` event with init status (scenario id, handler count, object count) so the smoke harness can assert on it and scenario init is easier to debug
- @alexjercan the player lock reaches out to 20 km (up from 2 km) so distant bodies can be designated for GOTO legs and torpedo launches; the AI's 2 km sensor scan is unchanged
- @alexjercan the SFX/juice listener is an explicit `SfxListenerMarker` on the gameplay camera instead of "any Camera3d", so the editor camera never grows shake and attenuation follows the right camera

### Fixed

- @alexjercan reinterpret the stacked skybox cubemap into a 6 layer array at load time (via `.meta` loader settings) instead of on camera spawn: the renderer eagerly uploads loaded images, and the raw 24576 px tall form exceeds the 16384 texture limit of smaller GPUs (CI's llvmpipe), which killed the app with a render validation error depending on frame timing
- @alexjercan make blast damage reach every body overlapping the blast instead of only one
- @alexjercan ships, asteroids and torpedoes interpolate their transforms between fixed physics ticks, fixing the camera twitch the new chase smoothing exposed
- @alexjercan the chase camera anchors on the ship's live center of mass instead of the root origin, so a ship that lost its front sections no longer appears to tumble around an empty point in space
- @alexjercan projectiles no longer contact-collide with the ship that fired them: torpedoes (like turret rounds) get an owner collision filter, so they stop taking and dealing hull damage at launch
- @alexjercan despawn the leftover `RigidBody` husk when an asteroid's collider child is destroyed
- @alexjercan stop the one-frame origin snap when switching camera modes
- @alexjercan torpedoes: add an arming gate so they no longer self-detonate on spawn, keep flying when their target dies, and stop vanishing when there is no lock
- @alexjercan fix the turret resting position
- @alexjercan make the editor preview controller inert to stop physics 'root not found' log spam
- @alexjercan enemy fire, the player turret aim ray and the lock cone anchor on the surviving hull (shared `live_structure_anchor`) instead of the empty build spot after front sections die
- @alexjercan absorb section overkill instead of forwarding it: a 1000-damage hit on a 100 hp section no longer drags an otherwise-healthy ship through disable and destroy
- @alexjercan a controller section disabled in place no longer keeps torquing the hull toward its frozen command - a dead computer means adrift
- @alexjercan solve bullet lead in the shooter's frame: the lead solve subtracts the muzzle point velocity the bullet inherits, so a moving shooter's rounds land instead of drifting by exactly the ship's motion
- @alexjercan the AI helm writes an absolute, slewed rotation command through a shared `ship_turn_rate` (also used by the player path and autopilot) instead of rewriting an unslewed delta every frame
- @alexjercan a torpedo whose body section is destroyed dies whole and blast-free, so point defense actually stops it; the shot-down despawn is deferred one tick to avoid a command-flush panic
- @alexjercan one hit plays one cue: the audio and juice damage observers ignore propagation re-entry, fixing doubled impact sounds, doubled camera trauma and phantom flash rings at the ship root

## [0.3.1] - 2026-07-07

### Added

- @alexjercan add a post-processing camera component

### Changed

- @alexjercan upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10) and migrate the source to the new API
- @alexjercan externalize `bevy_common_systems` as a git dependency and remove the vendored copy

### Docs

- @alexjercan add `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, Bevy 0.19 migration notes)

## [0.3.0] - 2025-11-29

### Added

- @alexjercan implement OnEnter and OnExit events
- @alexjercan implement torpedo bay section and blast damage
- @alexjercan implement new health system - add health for each spaceship section
- @alexjercan added new event trigger for entering a zone

### Changed

- @alexjercan improve directional shader to make the forward direction more visible
- @alexjercan improve the thruster shaders to allow more complex shapes and animations

## [0.2.1] - 2025-11-15

### Chores

- @alexjercan improve modding documentation and examples
- @alexjercan refactor event system for better performance and scalability

## [0.2.0] - 2025-11-08

### Added

- @alexjercan implement game events and a queue system that handles them
- @alexjercan scenario and modding capabilities added
- @alexjercan asteroids with procedural mesh and dynamic destruction for objects added

## [0.1.0] - 2025-10-21

### Added

- @alexjercan basic spaceship sections added
- @alexjercan editor and simulation scenes added

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/alexjercan/nova-protocol/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/alexjercan/nova-protocol/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
