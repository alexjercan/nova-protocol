# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Particle effects (turret muzzle flash, projectile trail, torpedo launch and detonation bursts) now render in the web build: it moved from the WebGL2 render backend to WebGPU, which is what `bevy_hanabi`'s compute-shader particles require. Native was already running them. The thruster exhaust plume is a shader (not particles) and rendered on the web already
- Browsers without WebGPU (e.g. Firefox on Linux, which the WebGPU switch above needs) now see a "WebGPU required" message on the game page instead of a crash/black canvas, and a heads-up under the landing page's "Play in browser" button. The check probes `navigator.gpu` and then `requestAdapter()`, so it also catches browsers that expose the API but cannot actually get an adapter
- A criterion benchmark for the modding scenario-dispatch hot path (`cargo bench -p nova_scenario --bench scenario_dispatch`): entity-filter and condition-eval micro groups plus the full event-dispatch loop over a synthetic hundreds-of-handlers scenario, with a realistic one-event-per-frame group and a scan-isolating burst group. `samply` joins the dev shell for sampling profiles. This is the measure-before-optimizing gate for the modding perf work; the full writeup is `docs/modding-perf-report.md`
- Scenarios can swap the skybox cubemap mid-scenario with a new `SetSkybox` action: a beat authors a new cubemap path (the same asset-path layer the scenario's initial cubemap uses) and the sky changes when it finishes loading. The install is deferred until the image is present so the skybox setup never reads a half-loaded texture; a bad path leaves the sky unchanged with a warning
- A static mod portal now publishes alongside the game on every deploy: `webmods/` sources are validated, hashed (per-file sha256) and copied to `/mods/<id>/<version>/` with a generated, schema-versioned `catalog.json` - never hand-maintained (docs/mod-portal.md). First published mod: Gauntlet Run, a beacon slalom course. The generator (`nova_portal_gen`) is engine-free so the deploy job builds it in seconds; a CI test loads every portal bundle through the real game loaders as the deep publish gate. The game-side Explore/download flow comes next

### Changed

- The Mods menu no longer lists dev/tooling mods: catalog entries can be marked `hidden: true`, which keeps them out of the player-facing list while staying installed and enableable by id from code
- The Screenshot Reel capture set no longer ships in the game assets at all: its scenario moved out of `assets/mods/` into the example that films it (`examples/data/reel.content.ron`, embedded at compile time), so players and the web build stop downloading a capture tool. The `hidden` flag stays for future dev/tooling mods
- Mod metadata now lives in the mod's own `*.bundle.ron` as a `meta` block (name, description, author, version, dependencies, icon/screenshots - the Factorio info.json analog), and `assets/mods.catalog.ron` slimmed down to a thin pointer list (id, bundle path, base/hidden flags). One source of truth per mod, ready for the mod portal and the richer mods UI. Format break for the catalog and any out-of-tree bundles (none known); meta-less bundles remain valid
- Modding event dispatch is now indexed by event name (moved upstream into bevy-common-systems, rev bump to 4c81117). The dispatcher used to scan every spawned handler for every fired event; it now jumps straight to the handlers registered for that event and walks them from a contiguous snapshot, touching neither the ECS nor scattered memory. Measured on the synthetic benchmark: 17-24% faster dispatch under bursts (a wave of entities all firing in one frame) at 500-5000 handlers, and neutral at the realistic one-event-per-frame rate. First-party scenarios (1-19 handlers) are unaffected; this is insurance for large community mods. A first entity-id index that looked handlers up per-dispatch was measured, caught regressing at scale (random-access cache thrash), and replaced by the snapshot version before landing
- The sibling filter-key-interning and condition-eval-compile optimizations (task 20260714-083339) were measured and deferred: at realistic event rates their per-handler costs (13 ns entity filter, 26 ns condition eval, and condition filters run only once per frame) are noise, so they stay documented insurance rather than serde/data-model churn

## [0.5.2] - 2026-07-14

### Added

- The web site grew a full wiki (gameplay systems, ship sections, a keybinds page with controller button glyphs, world and meta pages), two new devlogs, and a tutorial trimmed to first-scenario onboarding

### Changed

- Gamepad bindings rounded out: ORBIT moved to South, the scenario-advance confirm to DPadDown, and HUD cycle / pause / back-to-editor gained buttons

- CI's example smoke suite is BLOCKING again: the GitHub-runner-only taffy panic did not survive the examples rework, so the 12-example suite (behavior assertions, command-error gate and all) now gates every push
- The examples are a testable curriculum now - sections (01 controller PD, 02 thruster burn + plume, 03 hull damage pipeline, 04 turret range, 05 torpedo range, 06 guidance, 07 COM), scenario language (08: variables, filters, actions asserted live), editor (09), and playable (10: lock, gun kill, GOTO, arrival driven through the real input pipeline and watched by the scenario's own handlers; 11 HUD range with the velocity sphere; 12 the boot flow) - every one self-drives under BCS_AUTOPILOT with behavior assertions and a completion backstop, and the CI smoke list runs all twelve
- The example smoke suite now fails on ANY logged command error ("Encountered an error in command"), closing the gap where handled remove/despawn warns - the stale-entity teardown race class - sailed past the panic gate
- The weapon test ranges fire again: the weapons safety had silently disarmed both (a cold press never latches and a held key never re-edges); their scripts and controls now raise the stance first, and new outcome assertions pin the fire -> hit chain so it cannot regress quietly

### Fixed

- The debug inspector's window-camera placement fix moved upstream into bevy-common-systems (rev bump to 4a743b2): the per-frame reconcile that keeps the egui context off render-to-texture cameras now ships with the plugin, and nova's local workaround is deleted - one implementation instead of two
- The thruster hum now attenuates with distance per ship: another ship's (or a torpedo's) burn no longer plays at full volume in your ear from anywhere in the scene. Your own ship is exempt, so the hum does not fade when the camera pulls back (orbit survey). Side effect: the main-menu backdrop ship's hum now fades with its orbit distance instead of droning at full throttle gain
- Scenario teardown no longer logs an "Entity despawned" command error when a ship despawns with its autopilot engaged: the telemetry-cleanup observer queued a remove against the very entity whose despawn triggered it; it now uses the fallible variant. The two suspected cross-entity variants of the race (camera handover, dominant-well strip) were probed and refuted - regression tests pin the command ordering that keeps them safe

## [0.5.1] - 2026-07-13

### Fixed

- The web build no longer quits with a fatal render validation error on New Game / editor Play: the target inset's render target used a view-format override (`view_formats`), which WebGL2 does not support; it is now a plain sRGB target
- The skybox cubemap's `.meta` loader settings now actually apply in the shipped app: `AssetMetaCheck::Never` had silently ignored them on every platform since the fix landed, resurrecting the oversized-upload race on GPUs with a 16384 texture limit (and the "single layer image" warning); the app now reads meta files for exactly the cubemap's path

## [0.5.0] - 2026-07-13

### Added

- Deliberate radar locking replaces all passive targeting: hold CTRL to sweep, the radar live-locks what you look at; your stance picks the slot (lowered = white NAV crosshair feeding GOTO, raised = red combat reticle feeding guns/torpedoes/fine-lock); tap CTRL to clear, staged (combat, then nav). Locks stick until the target dies, leaves range, or goes cold. LOCK is a ship-computer capability, like GOTO
- Target viewfinder: a corner inset renders a live magnified 3D view of the combat lock via a second camera; red armed frame while weapons are hot, NO-SIGNAL panel for non-scopeable bodies, and a ~2 s freeze-frame kill cam when the target dies. The fine-locked section glows in both views
- Shakedown Run: New Game starts a ~12-beat tutorial (burn, freelook, stop, salvage, GOTO, gravity coast, ORBIT, radar lock, live-fire rehearsal on a derelict, scavenger fight); each beat teaches one gesture and completes the instant the gesture lands
- Typed damage (Kinetic / AP / EMP / Explosive) against per-section resistance tables; each turret carries a loaded-ammo slot that sets its rounds' type, and the ammo readout is color-coded by type
- Main menu (live ambient scene: an AI ship flying a thruster-driven orbit) and ESC pause menu; the game boots into the menu (new `nova_menu` crate)
- HUD visibility levels: grave/tilde cycles ALL -> MINIMAL -> NONE
- Objective conveyance: gold marker chip with live distance on the current target, salvage-crate glow and brackets, keybind emphasis pulses, completion chime and posting blip
- Scenario primitives: nav beacons and salvage crates (with authorable radar signatures), despawn-by-id, and new events/actions (OnOrbit, OnTravelLock/OnCombatLock, SetSpeedCap, objective markers, hint emphasis)
- Editor: placed sections show their bound key as a chip; click a section to rebind it (keys or mouse buttons); the build panel scrolls
- Web landing site (`web/`: TypeScript + Webpack + Tailwind); the Pages deploy serves it at the root with the game under `/play/`
- Turret rounds curve through gravity wells, like ships and torpedoes

### Changed

- Diegetic flight readouts replace the bottom-left status text: speed and engaged-mode chips beside the velocity sphere, an ORBIT radius spoke, and the velocity sphere tints nav-cyan while the autopilot flies
- The keybind cluster is contextual: rows appear only while their verb can do something; scenario-emphasized keys show early, pulsing gold
- Lock language is slot-colored: RED bracket = combat lock, WHITE = nav lock; relation tint and reticle pips retired. Turrets hold the combat lock even during manual aim
- PDC retuned to point defense: per-hit damage 20 -> 4, so the stream chips targets down over a visible burst instead of one-shotting them
- The editor's play-test ship is a passive target instead of an AI combatant

### Fixed

- Debug builds no longer crash when a scenario advances (the smoke assertion now covers only the boot load), and scenario teardown no longer warns about despawned entities
- Asteroid kills now emit OnDestroyed under the scenario id, so scripts can react to them (this soft-locked the derelict beat)
- The target inset zooms combat bodies only (ships, torpedoes, asteroids), not beacons, and frames section-less bodies by collider bounds
- The debug inspector stays on a window camera instead of leaking into render-to-texture cameras
- HUD apparent-size elements (reticles, brackets) measure real colliders, not invisible trigger volumes
- Turret bullets are sensor projectiles: same damage, no physical shove, expended on first contact
- Many Shakedown playtest fixes: park points inside beacon triggers, orbit-hold completion, scavenger spawn timing and a 150u combat leash, invulnerable planetoids, speed-cap teaching, objective text wrapping and sound pacing, readable gold pulses
- F1 back-to-editor is sandbox-only; the debug ammo readout no longer lingers when debug mode is off

## [0.4.1] - 2026-07-10

### Fixed

- release-flow: install the `x86_64-apple-darwin` std for the pinned nightly, fixing the macOS universal build

### Changed

- CI: one `--features debug` feature set across clippy/tests/examples (one Bevy build), cache saved on failure, windowed examples smoke as a separate non-blocking step

## [0.4.0] - 2026-07-10

### Added

- Torpedo guidance: proportional navigation, angular lock-on aim-assist, arming gate, blast-radius visual, launch particle burst
- Player targeting arc: signature auto-acquisition (550 m), focus dwell, per-section component fine-lock with aim-snap and cycling, HUD lock markers and focus meter
- Turret auto-aim with true intercept lead, fed by fine-locked section, else live structure, else camera ray
- HUD substrate: screen-projected indicators (entity/point anchors, apparent-size sizing, clamp-to-edge arrows), turret lead pips, locked-target readout (range, closing speed, health)
- Faction/relation model (hostile/neutral/own): drives acquisition, projectile allegiance, reticle tint
- AI combat wave: behavior state machine (Idle/Patrol/Engage/Evade/Retreat), fire discipline, point-defense priority on inbound torpedoes, standoff orbit/strafe, autopilot-flown patrol routes, threat-memory evasion, enveloped torpedo launches
- Flight-assist overhaul: assisted velocity-hold mode (WASDQE nudges, X brake latch, soft cap), Z direct Newtonian mode, RCS budget, FA/speed readout
- The flight computer balances thrust through the live center of mass: differential throttle nulls burn torque; off-axis thrusters recruited for counter-torque
- First audio: placeholder SFX (explosions, impacts, turret fire, torpedo launch, throttle-tracking thruster loop) with distance attenuation and throttling
- Combat juice: trauma-model camera shake and expanding hit/impact flash rings, distance-attenuated and throttled
- Example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`, `11_com_range`) with live tuning sliders, FPS/version overlay, and a headless autopilot + screenshot smoke harness that asserts scenario init
- CI workflow: fmt, clippy, and the workspace test suite (windowed examples under Xvfb) on every PR and push to master

### Changed

- Mass-legible handling: turn rate derived from the torque budget and live inertia (stripped ships snap, heavy builds lumber); max_torque 100 -> 40
- Chase camera: smoothing on all gameplay modes plus a burn push-back lean
- Integrity, health, blast and mesh-slicer systems consumed from `bevy-common-systems` instead of in-tree copies; torpedo section split into its own module
- Player lock range 2 km -> 20 km (AI sensor scan unchanged)
- The SFX/juice listener is an explicit marker on the gameplay camera, not "any Camera3d"

### Fixed

- Skybox cubemap reinterpreted into a 6-layer array at load time; the raw 24576 px image exceeded smaller GPUs' texture limit and killed the app
- Blast damage reaches every body overlapping the blast, not just one
- Ships, asteroids and torpedoes interpolate between physics ticks (camera twitch); the chase camera anchors on the live center of mass
- Projectiles no longer collide with their shooter; shot-down torpedoes die whole and blast-free; destroyed asteroids no longer leave rigid-body husks
- Section overkill is absorbed instead of propagated; a disabled controller stops torquing the hull
- Bullet lead solved in the shooter's frame, so a moving shooter's rounds land; the AI helm writes slewed absolute rotation commands
- One hit plays one cue: audio/juice observers ignore propagation re-entry
- Misc: editor preview controller made inert, turret resting position, one-frame origin snap on camera-mode switch

## [0.3.1] - 2026-07-07

### Added

- Post-processing camera component

### Changed

- Upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10)
- `bevy_common_systems` externalized as a git dependency; vendored copy removed

### Docs

- `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, migration notes)

## [0.3.0] - 2025-11-29

### Added

- OnEnter/OnExit zone events
- Torpedo bay section and blast damage
- Per-section health system

### Changed

- Improved directional and thruster shaders

## [0.2.1] - 2025-11-15

### Chores

- Modding documentation and examples; event system refactor

## [0.2.0] - 2025-11-08

### Added

- Game events and a queue system
- Scenario and modding capabilities
- Asteroids with procedural mesh and dynamic destruction

## [0.1.0] - 2025-10-21

### Added

- Basic spaceship sections
- Editor and simulation scenes

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.5.2...HEAD
[0.5.2]: https://github.com/alexjercan/nova-protocol/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/alexjercan/nova-protocol/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/alexjercan/nova-protocol/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/alexjercan/nova-protocol/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/alexjercan/nova-protocol/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
