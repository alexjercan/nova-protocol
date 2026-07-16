# Changelog

All notable changes to this project are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups each release by subsystem (Gameplay & Flight, Combat & Weapons,
Ships & Sections, Scenarios & Objectives, Modding & Mod Portal, Interface & HUD,
Web & Platform, Audio & Visuals, Performance, Fixes, Internals & Tooling) rather
than by Added/Changed/Fixed. This project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Breaking changes are
tagged **(breaking)**.

## [Unreleased]

### Combat & Weapons

- Locking is no longer instant: once the radar settles on a target you now hold steady through a short lock-on dwell before it commits, and the dwell is longer the farther away the target is. Sweeping off the target before it fills cancels the acquisition, and re-designating to a new target earns a fresh dwell while the old lock holds - so a lock is something you earn by keeping the target under your aim, not granted the moment you point at it.

### Scenarios & Objectives

- Scenarios can declare a win/lose: the new `Outcome` action shows a VICTORY/DEFEAT overlay with real buttons (Continue/Retry riding a queued lingering `NextScenario`, Main Menu always) instead of the old silent press-Enter-to-restart; dying in Shakedown Run now presents Defeat + Retry. The overlay freezes the simulation the same way the pause menu does - physics, AI, weapons and timers stop behind the banner (Enter/Continue/Retry stay live) instead of the world running on underneath it.
- Broadside, chapter two of the base storyline: answer a neutral hauler's distress call, break a two-corvette ambush, then screen the gang gunship's torpedoes with your PDC and break it apart - the capital-combat slice, in the Scenarios picker and chained from Shakedown Run's victory screen.
- Ships in scenario RON can author their side: an optional `allegiance` field (`Some(Neutral)` for bystanders like Broadside's hauler) overrides the controller default.
- The Asteroid Field sandbox joined the outcome frame: reaching the zone declares Victory (Continue reruns the field) and dying declares Defeat + Retry, replacing its silent switches.
- The base Demo Scenario (the ship-less RON authoring demo) was removed from the game and the Scenarios picker; the example mod's arena is the worked hand-authored RON example now.

### Fixes

- Restarting a scenario (Retry, from the pause menu or the Victory/Defeat frame) no longer loses the objective text: the objectives HUD is now reset on scenario teardown, so a restart that re-posts the same objectives repaints the panel instead of leaving it blank.
- A ship that loses its last section outside the damage path no longer lingers as a targetable 0-HP ghost hull: the health aggregate now carries a structural death backstop (no living sections = dead), closing a soft-lock where Broadside's act gates waited on a kill that never registered.
- Scenario `OnUpdate` handlers now freeze while the game is paused (the pause menu or the Victory/Defeat frame): the per-frame pulse is gated on the unpaused state, so an already-satisfied handler no longer keeps re-applying its action every frame behind the pause.

### Modding & Mod Portal

- Mods can ship their own art: a bundle manifest gains a `resources` list of the binary files (textures, skyboxes, models, audio) it packages, and content references them with a `self://` path (`cubemap: "self://textures/nebula.png"`) that resolves against the mod's OWN folder instead of the base game - so a mod's look is its own, whether shipped or downloaded. A `self://` ref must name a declared resource or the mod fails its gates.
- One `example` mod is now the single copy-me tutorial mod, folding together the earlier scattered demonstrations: it shows a section overlay, a brand-new section, a playable arena scenario, its own shipped skybox and texture via `self://`, a `StoryMessage` comms beat, Victory/Defeat `Outcome`s, and a `menu_backdrop` scene - a little of everything a modder can do, in one self-contained folder to copy.
- Scenarios gained a `menu_backdrop` flag: on every menu entry the game picks one flagged scenario at random, so mods can ship their own menu ambience scenes (the base backdrop is just the first of them). The New Game start moved from code into the base bundle's `new_game_scenario` declaration - honored only from the base game, so mods cannot redirect it.
- Two new menu backdrops join the rotation: Waystation Traffic (a hauler convoy circling a freight stop under amber dock lights) and Scrapyard Drift (a quiet salvage yard - drifting crates, two wrecks and a lone tug).
- Scenarios can speak: the new `StoryMessage` action shows speaker-attributed dialog in a HUD comms panel (`OKONO > Strip it clean.`) - the story-text surface campaign mods build on.
- Broken mod content fails loud, not weird: scenarios with reference errors (unknown prototypes, dangling chapter chains) refuse to start with a FAILED TO START report naming each problem, and the menu backdrop rotation skips them.
- `cargo run -p nova_assets --bin content_lint` lints every scenario in the repo (base, shipped mods, portal mods) for the reference bugs the loaders cannot see - unknown section prototypes, dangling chapter chains, filter targets nothing spawns; CI enforces it. Mod developers can check just their own mod with `-- --target <mod dir or id>`.
- The Ledger, the first campaign mod on the portal: a four-chapter salvage arc (strip the wreck, break the claim jumpers, run the quiet channel, pick the buyer - or the buoy) with comms-driven story beats and a two-ending finale. Install from Mods > Explore.

### Interface & HUD

- The pause menu (Esc) gained a Retry button: restart the current scenario from scratch without going back through the main menu. Offered only while a scenario is live, so the editor's build mode does not show it.
- The Settings menu is real content instead of a placeholder: a draggable master audio volume slider, a Low/Medium/High graphics-quality preset, and a read-only reference of the current keyboard and gamepad bindings (flight, targeting, camera, pause). Reachable from BOTH the main menu and the pause menu (the same modal, both entry points), and remembered across restarts - a config file on native, browser storage on the web. The graphics preset tunes the combat juice (High keeps camera shake and hit flashes, Medium drops the shake, Low turns the juice off) and now also gates the heavier visuals for low-end machines (see Performance): Low is spawn-less (no particle bursts) and thins dense asteroid/debris fields, Medium thins the fields a little while keeping particles.

### Performance

- The Low/Medium graphics-quality presets now skip expensive visuals for low-end machines, not just the combat juice: Low is spawn-less (torpedo blast/launch and turret muzzle particle bursts are not spawned at all) and thins dense scatter fields to half, and Medium keeps particles but still thins the fields a quarter. High is unchanged. The exact thinning fractions are provisional pending the gameplay frame-time baseline; the tiers stay observably distinct today.

### Internals & Tooling

- Firing no longer spams the log with an avian "no mass or inertia" warning per round: turret bullets, which use a near-zero mass and a sensor collider, now carry an explicit angular inertia, removing the warning and its NaN risk.
- Debug builds (`--features debug`) bind F12 to a screenshot that saves the primary window to your Downloads directory as `<timestamp>.png`.

## [0.6.0] - 2026-07-16

### Scenarios & Objectives

- Scenarios picker on the main menu: two-pane overlay listing every base and mod-added scenario, with a details pane and Play button; scenarios gained optional `thumbnail` and `hidden` fields.
- New `SetSkybox` action swaps the skybox cubemap mid-scenario, deferred until the image loads so a bad path leaves the sky unchanged.

### Modding & Mod Portal

- Mod dependencies resolve end to end: installs auto-pull missing deps, enabling a mod auto-enables its transitive deps, disabling a still-depended-on mod is refused, and merge order is dependency-respecting topological (ids only, no version constraints yet).
- Static mod portal now publishes on every deploy: validated, sha256-hashed bundles under `/mods/<id>/<version>/` with a generated `catalog.json` (first mod: Gauntlet Run).
- Local mod cache foundation: a `mods://` asset source, a RON installed index, and downloaded bundles that load and merge through the same pipeline as shipped ones.
- Portal client fetches `catalog.json` and installs/uninstalls over the wire on native and web, staged installs verified against size + sha256 and committed only once every file checks out.
- Explore online tab is real: browse the portal in-game and install/update/uninstall on native and web, with per-file progress, offline catalog fallback, and enabled-state-preserving updates.
- Mods menu is a two-pane Factorio-style screen: `Installed` | `Explore online` tabs over the scrollable list with per-row enable checkboxes and a details panel from bundle meta.
- Mods menu hides dev/tooling mods: `hidden: true` entries stay installed and code-enableable by id but drop out of the player list.
- **(breaking)** Mod metadata moved into each `*.bundle.ron` as a `meta` block; `assets/mods.catalog.ron` slimmed to a thin pointer list (catalog and out-of-tree bundle format break).

### Web & Platform

- Particle effects (muzzle flash, projectile trail, torpedo launch/detonation bursts) now render in the web build after moving from WebGL2 to WebGPU.
- Browsers without WebGPU get a clear "WebGPU required" message instead of a black canvas, plus a heads-up under the landing page's "Play in browser" button.

### Performance

- Modding event dispatch is indexed by event name (upstreamed to bevy-common-systems rev 4c81117): 17-24% faster under bursts of 500-5000 handlers, neutral at the realistic one-event-per-frame rate.
- Added a criterion benchmark for the scenario-dispatch hot path (`cargo bench -p nova_scenario --bench scenario_dispatch`); the measure-first gate is documented in `tasks/20260714-083331/modding-perf-report.md`.
- Sibling filter-key-interning and condition-eval-compile optimizations were measured and deferred: at realistic event rates their per-handler cost is noise, kept as documented insurance.

### Fixes

- Scenarios picker no longer crashes the renderer on a non-2D thumbnail: such thumbnails are skipped with a warning, and images mount only once loaded.
- Local mod-portal web testing no longer needs a cross-origin `?portal=` override: `scripts/preview-web.sh` serves the portal same-origin as the game, matching production.

### Internals & Tooling

- Screenshot Reel capture set no longer ships in the game assets: its scenario moved into the example that films it, so players and the web build stop downloading a capture tool.

## [0.5.2] - 2026-07-14

### Gameplay & Flight

- Gamepad bindings rounded out: ORBIT -> South, scenario-advance confirm -> DPadDown, and HUD cycle / pause / back-to-editor gained buttons.

### Web & Platform

- The site grew a full wiki (gameplay, ship sections, keybinds, world and meta pages), two new devlogs, and a tutorial trimmed to first-scenario onboarding.

### Fixes

- Thruster hum now attenuates with distance per ship, so another ship's or torpedo's burn no longer plays at full volume from anywhere (your own ship stays exempt).
- Scenario teardown no longer logs an "Entity despawned" command error when a ship despawns with autopilot engaged: the telemetry-cleanup observer uses the fallible remove variant, pinned by regression tests.
- Debug inspector's window-camera placement fix moved upstream into bevy-common-systems (rev 4a743b2), deleting nova's local workaround.

### Internals & Tooling

- CI's example smoke suite is BLOCKING again: the GitHub-runner-only taffy panic is gone, so the 12-example suite gates every push.
- Examples are a testable curriculum: twelve numbered examples from controller PD through the boot flow, each self-driving under BCS_AUTOPILOT with behavior assertions and a completion backstop, all on the CI smoke list.
- Example smoke now fails on ANY logged command error, closing the gap where handled remove/despawn warnings (the stale-entity teardown race) sailed past the panic gate.
- Weapon test ranges fire again: the weapons safety had silently disarmed both, so their scripts raise the stance first, with new assertions pinning the fire -> hit chain.

## [0.5.1] - 2026-07-13

### Fixes

- Web build no longer quits with a fatal render validation error on New Game / editor Play: the target inset dropped its `view_formats` override (unsupported on WebGL2) for a plain sRGB target.
- Skybox cubemap's `.meta` loader settings now actually apply in the shipped app: `AssetMetaCheck::Never` had silently ignored them, resurrecting the oversized-upload race on GPUs with a 16384 texture limit.

## [0.5.0] - 2026-07-13

### Gameplay & Flight

- Diegetic flight readouts replace the bottom-left status text: speed and engaged-mode chips beside the velocity sphere, an ORBIT radius spoke, and a nav-cyan tint while the autopilot flies.
- Keybind cluster is contextual: rows appear only while their verb can act; scenario-emphasized keys show early, pulsing gold.

### Combat & Weapons

- Deliberate radar locking replaces all passive targeting: hold CTRL to sweep and live-lock what you look at, stance picking the slot (lowered NAV vs raised combat) and a tap to clear; locks stick until the target dies, leaves range, or goes cold.
- Typed damage (Kinetic / AP / EMP / Explosive) against per-section resistance tables; each turret carries a loaded-ammo slot setting its rounds' type, with a color-coded ammo readout.
- Lock language is slot-colored: RED bracket = combat lock, WHITE = nav lock; relation tint and reticle pips retired, turrets hold the combat lock even during manual aim.
- Turret rounds curve through gravity wells, like ships and torpedoes.
- PDC retuned to point defense: per-hit damage 20 -> 4, so the stream chips targets down over a visible burst instead of one-shotting them.

### Ships & Sections

- Editor: placed sections show their bound key as a chip and can be clicked to rebind (keys or mouse buttons); the build panel scrolls.
- Editor play-test ship is now a passive target instead of an AI combatant.

### Scenarios & Objectives

- Shakedown Run: New Game starts a ~12-beat tutorial (burn, freelook, stop, salvage, GOTO, gravity coast, ORBIT, radar lock, live-fire rehearsal, scavenger fight); each beat teaches one gesture and completes the instant it lands.
- Objective conveyance: gold marker chip with live distance to the current target, salvage-crate glow and brackets, keybind emphasis pulses, completion chime and posting blip.
- Scenario primitives: nav beacons and salvage crates (authorable radar signatures), despawn-by-id, and new events/actions (OnOrbit, OnTravelLock/OnCombatLock, SetSpeedCap, objective markers, hint emphasis).

### Interface & HUD

- Target viewfinder: a corner inset renders a live magnified 3D view of the combat lock via a second camera; red armed frame while hot, NO-SIGNAL for non-scopeable bodies, and a ~2 s freeze-frame kill cam on death, the fine-locked section glowing in both views.
- Main menu (live ambient scene: an AI ship flying a thruster-driven orbit) and ESC pause menu; the game now boots into the menu (new `nova_menu` crate).
- HUD visibility levels: grave/tilde cycles ALL -> MINIMAL -> NONE.

### Web & Platform

- Web landing site (`web/`: TypeScript + Webpack + Tailwind); the Pages deploy serves it at the root with the game under `/play/`.

### Fixes

- Debug builds no longer crash when a scenario advances (the smoke assertion now covers only the boot load), and teardown no longer warns about despawned entities.
- Asteroid kills emit OnDestroyed under the scenario id so scripts can react (this had soft-locked the derelict beat).
- Target inset zooms combat bodies only (ships, torpedoes, asteroids), not beacons, and frames section-less bodies by collider bounds.
- Debug inspector stays on a window camera instead of leaking into render-to-texture cameras.
- HUD apparent-size elements (reticles, brackets) measure real colliders, not invisible trigger volumes.
- Turret bullets are sensor projectiles: same damage, no physical shove, expended on first contact.
- Many Shakedown playtest fixes: park points inside beacon triggers, orbit-hold completion, scavenger spawn timing and a 150u combat leash, invulnerable planetoids, speed-cap teaching, objective text wrapping and sound pacing, readable gold pulses.
- F1 back-to-editor is sandbox-only; the debug ammo readout no longer lingers when debug mode is off.

## [0.4.1] - 2026-07-10

### Internals & Tooling

- release-flow: install the `x86_64-apple-darwin` std for the pinned nightly, fixing the macOS universal build.
- CI: one `--features debug` feature set across clippy/tests/examples (one Bevy build), cache saved on failure, windowed examples smoke as a separate non-blocking step.

## [0.4.0] - 2026-07-10

### Gameplay & Flight

- Flight-assist overhaul: assisted velocity-hold mode (WASDQE nudges, X brake latch, soft cap), Z direct Newtonian mode, RCS budget, FA/speed readout.
- Flight computer balances thrust through the live center of mass: differential throttle nulls burn torque, off-axis thrusters recruited for counter-torque.
- Mass-legible handling: turn rate derived from the torque budget and live inertia (stripped ships snap, heavy builds lumber); max_torque 100 -> 40.
- Chase camera: smoothing on all gameplay modes plus a burn push-back lean.

### Combat & Weapons

- Torpedo guidance: proportional navigation, angular lock-on aim-assist, arming gate, blast-radius visual, launch particle burst.
- Player targeting arc: signature auto-acquisition (550 m), focus dwell, per-section fine-lock with aim-snap and cycling, HUD lock markers and focus meter.
- Turret auto-aim with true intercept lead, fed by fine-locked section, else live structure, else camera ray.
- Faction/relation model (hostile/neutral/own) drives acquisition, projectile allegiance, and reticle tint.
- AI combat wave: behavior state machine (Idle/Patrol/Engage/Evade/Retreat), fire discipline, point-defense priority on inbound torpedoes, standoff orbit/strafe, patrol routes, threat-memory evasion, enveloped torpedo launches.
- Player lock range 2 km -> 20 km (AI sensor scan unchanged).

### Interface & HUD

- HUD substrate: screen-projected indicators (entity/point anchors, apparent-size sizing, clamp-to-edge arrows), turret lead pips, and a locked-target readout (range, closing speed, health).

### Audio & Visuals

- First audio: placeholder SFX (explosions, impacts, turret fire, torpedo launch, throttle-tracking thruster loop) with distance attenuation and throttling.
- Combat juice: trauma-model camera shake and expanding hit/impact flash rings, distance-attenuated and throttled.
- The SFX/juice listener is an explicit marker on the gameplay camera, not "any Camera3d".

### Fixes

- Skybox cubemap reinterpreted into a 6-layer array at load time; the raw 24576 px image exceeded smaller GPUs' texture limit and killed the app.
- Blast damage reaches every body overlapping the blast, not just one.
- Ships, asteroids and torpedoes interpolate between physics ticks (no camera twitch); the chase camera anchors on the live center of mass.
- Projectiles no longer collide with their shooter; shot-down torpedoes die whole and blast-free; destroyed asteroids no longer leave rigid-body husks.
- Section overkill is absorbed instead of propagated; a disabled controller stops torquing the hull.
- Bullet lead solved in the shooter's frame so a moving shooter's rounds land; the AI helm writes slewed absolute rotation commands.
- One hit plays one cue: audio/juice observers ignore propagation re-entry.
- Misc: editor preview controller made inert, turret resting position, one-frame origin snap on camera-mode switch.

### Internals & Tooling

- Example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`, `11_com_range`) with live tuning sliders, FPS/version overlay, and a headless autopilot + screenshot smoke harness that asserts scenario init.
- CI workflow: fmt, clippy, and the workspace test suite (windowed examples under Xvfb) on every PR and push to master.
- Integrity, health, blast and mesh-slicer systems consumed from `bevy-common-systems` instead of in-tree copies; torpedo section split into its own module.

## [0.3.1] - 2026-07-07

### Audio & Visuals

- Post-processing camera component.

### Internals & Tooling

- **(breaking)** Upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10); anything built against nova, including from-source mods, must move to the matching versions.
- `bevy_common_systems` externalized as a git dependency; vendored copy removed.
- Added `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, migration notes).

## [0.3.0] - 2025-11-29

### Combat & Weapons

- Torpedo bay section and blast damage.

### Ships & Sections

- Per-section health system.

### Scenarios & Objectives

- OnEnter/OnExit zone events.

### Audio & Visuals

- Improved directional and thruster shaders.

## [0.2.1] - 2025-11-15

### Modding & Mod Portal

- Modding documentation and examples; event system refactor.

## [0.2.0] - 2025-11-08

### Modding & Mod Portal

- Game events and a queue system; scenario and modding capabilities.

### Scenarios & Objectives

- Asteroids with procedural mesh and dynamic destruction.

## [0.1.0] - 2025-10-21

### Ships & Sections

- Basic spaceship sections.

### Scenarios & Objectives

- Editor and simulation scenes.

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/alexjercan/nova-protocol/compare/v0.5.2...v0.6.0
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
