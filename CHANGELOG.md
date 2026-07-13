# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- @alexjercan radar playtest round (2026-07-13): the radar now LOCKS LIVE - after the 0.25s hold threshold it grabs the first body under your look and retargets instantly as you sweep; releasing just makes it stick (no more wait-for-release), and sweeping over empty space keeps the last target. The slot (nav vs combat) is chosen at the threshold from your stance at that moment. The HUD shows instead of telling: the target inset appears the moment a combat lock exists and acts as the sweep's viewfinder (with a text-free NO-SIGNAL panel across non-scopeable bodies like beacons), its frame turns red with armed corner ticks while weapons are hot, and the "WEAPONS HOT / TORP" status text is gone - the inset's presence IS the guided-torpedo signal. Lock clears pop a wordless unlatch ghost with a lock-off sound; lock-on, safety-on and radar-denied placeholder cues join the sound bank (a computer without the LOCK capability now buzzes + flashes instead of silently ignoring CTRL). The on-object lock language is purely slot-colored: RED bracket = combat lock, WHITE bracket = nav lock (the relation tint and the reticle corner pips are retired). Turrets stay on the combat lock even while RMB is held - moving the cursor no longer pulls them off; tap CTRL to clear the lock and get manual aim back. The inset panel moved below the FPS status bar. Polish from the next round: [CTRL] RADAR joined the keybind cluster; the nav crosshair renders a step larger than the combat reticle so an overlapped pair stays concentric at any target size; the inset shows a relation-colored faction line ("SCAVENGER - HOSTILE") for the locked target; the sweep label next to the bracket is distance-only and identical for both lock types. The keybind cluster went contextual (Arma-style): rows appear only while their verb can actually do something (a scenario-emphasized key still shows early, pulsing gold), instead of idling greyed out, and the [`] HUD row is gone from the list (the key still works)
- @alexjercan deliberate radar locking replaces all passive targeting (supersedes the interim sticky-lock/CTRL+scroll model that shipped earlier in this cycle): nothing locks by itself anymore. Hold CTRL to run the radar - it live-tracks the best body under your look (hollow box + name) - and release to lock it: a white NAV crosshair when lowered, the combat reticle when raised (RMB); the slot is chosen at the moment you press, so a mid-gesture stance change cannot re-route it. Releasing onto empty space changes nothing (the abort). Tap CTRL to clear, staged: combat lock first, then the nav lock (which also disengages an engaged GOTO); while raised a tap only ever clears the combat lock. Locks are sticky until the target dies, leaves range, turns non-hostile, or (combat) 30 s pass without any combat activity. GOTO flies to the NAV lock captured at [G]; guns/torpedoes/focus/component fine-lock/inset follow the COMBAT lock; the wheel still cycles sections. Some ship computers may not provide the LOCK capability at all (same flag family as GOTO). Debris is now only radar-visible within ~5 m (was 15)

### Fixed

- @alexjercan the target inset now scopes only combat bodies (ships, torpedoes, asteroids), not nav beacons: a new `InsetZoomable` flag gates what the scope will zoom, so locking a waypoint no longer opens a pointless close-up. The framing also handles bodies without ship sections (a torpedo/asteroid) by using their collider bounds. The inset stays hidden outside the full (`ALL`) HUD mode
- @alexjercan debug inspector egui no longer renders into a render-to-texture camera: bcs's inspector assigns its egui context to the first camera added, so a second camera that draws to a texture (the new target inset) could steal it and paint the inspector inside the inset instead of the window. The debug crate now keeps the inspector pinned to a window-targeting camera regardless of spawn order
- @alexjercan editor rebinding now accepts mouse buttons, not just keys: after clicking a section to rebind it, press any key OR mouse button (e.g. LMB) to bind it - the arming click is ignored (it waits for release first), and the new binding replaces the old key/mouse while keeping any gamepad bind
- @alexjercan the editor build panel now scrolls with the mouse wheel, so its lower entries (section palette + Play) are reachable instead of overflowing the fixed-height panel
- @alexjercan editor keybind rebinding is now reachable and the chips are readable: a "Select / Rebind" palette button deselects the build/delete tool so you can click a section to rebind it (previously no way back to select mode), and the keybind chips now sit on a dark rounded background instead of floating unreadably over the scene
- @alexjercan the debug ammo-count number (the `rounds/capacity` text on the readout, F11-only) no longer lingers on screen while debug mode is off: its F11 toggle was gated to gameplay while the global debug toggle is not, so pressing F11 in the menu/editor desynced them and left the number showing in normal play. The toggle is now ungated, staying in phase with debug mode
- @alexjercan PDC turret retuned to a point-defense profile (playtest: it "destroyed asteroids and objects with one bullet"): per-hit damage dropped from ~20 to 4 so the 100-round/s stream chips a target down over a visible burst (~0.25s to clear a 100-HP asteroid) instead of popping it instantly. Still clearly the strongest gun by rate; also lengthens ship fights slightly
- @alexjercan the lock reticle on a beacon no longer wraps the beacon's (invisible) 70u trigger volume - apparent-size HUD elements now measure non-sensor colliders only, so a locked beacon gets the small minimum reticle (playtest: "beacons have a really big target thingy")
- @alexjercan conveyance gold text readability (playtest): the emphasized keybind row now pulses pure gold with only its alpha breathing (the old cyan-to-gold cross-fade washed through an unreadable near-white every cycle), and the objective marker label keeps constant full alpha with a dark text shadow - the diamond and chevron carry the motion
- @alexjercan objective sound pacing: when a beat completes and posts the next objective in one step, the posting blip now waits a configurable moment (default 1.0s, ObjectiveFeedbackSettings) after the completion chime instead of playing over it
- @alexjercan Shakedown Run playtest round 3: objective changes now have feedback - a completion chime plus the finished objective lingering in green for a moment, and a posting blip for new objectives (two new placeholder sounds); the scavenger is territorial (new AI leash: combat breaks off beyond 150u of its patrol ground, so the fight stays at the debris field; being under fire overrides the tether); [X] stop is taught in the very first objective
- @alexjercan Shakedown Run playtest round 2: the planetoid moved out so its gravity (worst-seed SOI 960u) can no longer reach the debris field during the salvage beat; turret bullets no longer physically shove what they hit - rounds are now sensor projectiles that deal identical kinetic damage and expend on the first contact (a single 0.1-mass round at 100 u/s used to knock a ship ~3 u/s off course, and they bounced) - as a side effect bullets also stop pushing debris around; the training speed governor now releases once you reach beacon 1 (new SetSpeedCap scenario action), and the objective text says so
- @alexjercan Shakedown Run playtest round 1: beacon triggers now contain the GOTO park point (the autopilot used to stop 10u outside the objective); the orbit beat completes by HOLDING the orbit for 5s (new OnOrbit scenario event driven by autopilot state - the old position gate was unreachable on many asteroid seeds); the scavenger spawns only after making orbit instead of ambushing mid-tutorial; the ships' hull gap is closed (turret sits directly behind the controller); the planetoids (shakedown and menu) are invulnerable so the orbit beat cannot be shot away; objective text is smaller and wraps in a fixed-width panel; the starter ship carries a soft 25 u/s manual speed cap so a missed brake does not sail a new pilot out of the play area
- @alexjercan F1 no longer drops a New Game scenario into the ship editor; the back-to-editor key is sandbox-only (the pause menu is the way out of a campaign)

### Added

- @alexjercan target inset (scope of the locked ship): once the lock focus dwell completes, a corner panel shows a live, magnified 3D close-up of the target ship, rendered by a second camera into a texture - so you can see which section the component fine-lock is selecting (and watch it take damage / explode) instead of squinting at sub-pixel markers at range. The fine-locked section also glows in-scene (an emissive shell that reads in both the main view and the inset). The inset only renders while you are actually scoping a focused lock. Phase 1 is view-only; clicking sections in the inset is a later decision
- @alexjercan editor section keybinds are now visible and editable: each placed thruster/turret/torpedo shows its bound key as a chip over the section, and clicking a section (with no build tool selected) lets you rebind it - press the new key, Escape cancels. The old hold-key-while-placing shortcut still works
- @alexjercan turret free-aim: manual gunnery detaches the turrets from the target lock and aims them at the crosshair directly - release to snap back to the lock. Lets you place a manual shot without dropping your lock. (Superseded within this release by the deliberate-radar model: manual aim now rides the raised RMB stance with NO combat lock - a lock holds the turrets until tapped clear; CTRL is the radar)
- @alexjercan typed damage (combat depth, phase 1): weapons now deal a typed damage (Kinetic / Armor-Piercing / EMP / Explosive) scaled by a per-section resistance table, so a section can be weak to one type and tough against another (EMP wrecks the command core but barely dents hull; AP bites armored turrets but over-penetrates thin thrusters; explosives shred exposed thrusters but bounce off hardened mounts). Turret rounds are Kinetic (authored per-hit damage that reproduces the old feel, since Kinetic is neutral against everything) and torpedoes are Explosive. Groundwork for reloadable bullet types and alt-fire
- @alexjercan ammo-type foundation: each turret carries a loaded-ammo "slot" that decides its rounds' damage type, and the diegetic ammo readout is now color-coded by that type (Kinetic amber, AP steel-blue, EMP cyan, Explosive red-orange) - so torpedo-bay readouts now read red-orange. Catalog weapons are all Kinetic for now, so nothing else changes; the slot is the seam a future ship-management menu uses to load/switch bullet types and reload
- @alexjercan objective conveyance visuals (Shakedown Run upgrades in place): a gold objective marker chip rides the current leg's target (label + live distance, clamps to the screen edge as a direction chevron; new ObjectiveMarkerAttach/Detach scenario actions; while a beacon is marked its cyan chip yields so one entity shows one chip), salvage crates advertise themselves (emissive glow pulse on the mesh plus an apparent-size HUD bracket that tightens as you close in, breathing on one shared clock), and the keybind cluster can spotlight one verb row (new HintEmphasisSet/Clear actions; beat 4 pulses [G] GOTO toward gold until orbit is made; teardown clears any leftover emphasis)
- @alexjercan turret rounds now feel gravity wells like ships and torpedoes do: PDC fire curves toward a nearby planetoid instead of flying dead straight. The curve is subtle except on close passes to a strong well, and since the target ship already falls, letting rounds fall too keeps the auto-aim honest rather than fighting it
- @alexjercan New Game now drops into "Shakedown Run", a five-beat starter scenario: burn to a beacon (W), freelook to find the next one (Alt), recover 3 supply crates from a debris cluster (X to stop), hand the ship to the computer (G GOTO a locked beacon, O ORBIT the planetoid), then drive off a lone scavenger that snuck into the debris field (passive until provoked, scavenger-grade hull and turret); dying restarts the run (Enter confirms)
- @alexjercan scenario content primitives: nav beacons (emissive blinking waypoints with a HUD chip showing label + live distance that clamps to the screen edge as a direction chevron; aim-lockable, so GOTO works on them) and salvage crates (bright tumbling proximity pickups); scenarios can now despawn objects by id (crate pickup consumes the crate)
- @alexjercan web landing site (new `web/` project: TypeScript + Webpack + Tailwind): a themed marketing/content site with a hero landing page, a "Play" gate, and blog/tutorial/wiki pages, styled from the `assets/banner.png` key art (space-navy field, neon-cyan and amber glow). README rewritten to match, with the banner embedded
- @alexjercan the deploy now fronts the game: `deploy-page.yaml` builds the landing site to the Pages root (`/nova-protocol/`) and the Bevy WASM game under `/nova-protocol/play/`, instead of publishing the raw game at the root

- @alexjercan pause menu: ESC freezes the game (virtual + physics clocks, spaceship input gated off) behind a dimmed overlay with Resume / Back to Main Menu / Exit; Back returns to the main menu cleanly (scenario unloaded, editor scene torn down, cursor released while paused and re-grabbed on resume during scenario play)

- @alexjercan HUD visibility levels: the grave/tilde key cycles ALL -> MINIMAL (flight and combat instruments only, chrome hidden) -> NONE (clean screen for cinematic shots); every HUD widget carries a tier, the main menu drives the level to NONE while it is up, and the keybind hint cluster documents the key

- @alexjercan the main menu plays a live ambient scene behind the panel: an AI ship flying a real thruster-driven orbit around a gravity-well planetoid (visible flame, engine hum), framed by a fixed cinematic camera (WASD controller stripped, fps/version status bar hidden while the menu is up)
- @alexjercan the game boots into a main menu (new `nova_menu` crate): a bottom-right "Nova Protocol" panel with New Game (loads the asteroid-field scenario with the canned player ship), Sandbox (the ship editor), Settings (placeholder) and Exit (hidden on wasm); examples with custom game plugins keep the direct Loading -> Playing flow, and `AppBuilder::with_main_menu(bool)` overrides the default

### Changed

- @alexjercan the editor's play-test scenario ship is now a passive target instead of an AI combatant, matching the sandbox's build-and-fly scope

- @alexjercan the velocity sphere tints to the flight computer's nav-cyan family while the autopilot flies and reverts to white/blue in manual, so engaged-vs-manual reads from the instrument itself; the gravity sphere stays yellow in both states
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
