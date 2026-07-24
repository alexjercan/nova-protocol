# Changelog

All notable changes to this project are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups each release by subsystem (Gameplay & Flight, Combat & Weapons,
Ships & Sections, Scenarios & Objectives, Modding & Mod Portal, Interface & HUD,
Web & Platform, Audio & Visuals, Performance, Fixes, Internals & Tooling) rather
than by Added/Changed/Fixed. Entries are kept SHORT - one commit-title line each;
the per-release News post (`web/src/news/<version>.md`) is where the detail and
narrative live. This project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Breaking changes are
tagged **(breaking)**.

## [Unreleased]

## [0.8.1] - 2026-07-24

### Web & Platform

- Landing hero gains per-OS download buttons (Windows/macOS/Linux) that deep-link the matching release asset, with a fallback to the releases page.

## [0.8.0] - 2026-07-23

### Gameplay & Flight

- Unarmed AI ships are now non-combatants: no target, no chase, no fire - still targetable, so an escort/convoy must be defended.
- Gravity wells pull only PILOTED ships; an unpiloted bystander floats where it sits instead of being dragged in.
- Lifeline's convoy haulers now fly (unarmed non-combatant AI) instead of drifting off when shoved.

### Scenarios & Objectives

- Base-campaign pacing pass: an objective posts a beat AFTER its comms line, never on the same frame, with a breather between objectives (shared scenario-pacing toolbox).
- Shakedown Run opens slower: Capt. Halloran briefs you over comms (~40s) before objective 1; objective texts shrank to the bare goal.
- New scenario **Lifeline** (ch3 pt1): screen a two-hauler convoy through three raider waves to a live `RELIEF mm:ss` countdown - the first shipped ally content.
- New scenario **Final Tally** (ch3 pt2, finale): survey + orbital picket + capital-escort fight in the base chain's first combat gravity well; the campaign now ends properly. New Game arc is five scenarios.
- Broadside found its voice: story moved to the comms panel, objectives shrank to imperative goals, Victory banners track the Ceres Queen's fate.
- The **Asteroid Field** sandbox is back in the Scenarios picker (it had been wrongly hidden as unreachable).
- New `HudReadout` action: show a scenario variable on the HUD (`Number`/`Integer`/`Time`), an Instrument-tier readout, pause- and teardown-safe.
- New `SetAllegiance` action: flip a ship's allegiance mid-scenario - the neutral-until-provoked primitive.
- New reserved `player_speed` variable: the player's live speed, engine-written and read-only, to gate beats on how fast you fly.

### Modding & Mod Portal

- Gauntlet Run is now a TIME-TRIAL (1.3.0): a live `mm:ss.s` clock and a clean-run bonus, built on `HudReadout`.
- The Ledger grew 1.5.0 -> 1.12.0: a campaign-wide pacing pass, a ch3 stealth run (neutral pickets, warn-then-trip overspeed), a forking ch4 finale, and a fifth reward chapter (The Raid); re-published to the portal.

### Interface & HUD

- The Scenarios picker groups the base storyline as a campaign: scenarios declare an optional `campaign` (name + order); mods can group their own chapters the same way.

### Fixes

- Destroying a sectioned ship no longer crashes: the damage-tint tolerates a section the explosion destroyed the same frame.
- The mouse cursor is hidden while flying, dev builds too; the `--features debug` layer now boots OFF and F11 toggles all of it as one.

### Internals & Tooling

- A pre-commit `cargo fmt --check` hook makes rustfmt drift impossible to LAND (arm once with `scripts/setup-hooks.sh`).
- `content lint` is the single content command: `audit` folded in, plus a flight-rig input-overlap check; `lint --target <mod> --report <path>` writes a per-mod Markdown/HTML report pinpointing each finding.
- The `nova_perf` crate became `nova_probe`, the run-harness crate (bin names, `NOVA_PERF_*` env vars and output formats unchanged).
- One front door: `probe run <example>` runs a clean pass + optional `--profile` trace and `--samply` flamegraph, then a self-contained `report.html` + machine-readable `checks.json` with an OK/WARN/FAIL/NO_DATA verdict.
- New run-timeline recorder (`NOVA_PERF_TIMELINE`): ordered JSONL of state transitions, fired events, variable changes and beat markers, flushed per entry.
- Continuous invariant checks (`NOVA_PERF_INVARIANTS`): per-frame health/velocity/variable/entity-count assertions that land on the timeline.
- Probe timelines emit outcome markers at their assertion sites, so the timeline shows the FEATURE working, not just the process surviving.
- The whole example fleet is probe-evaluable: every cataloged example carries the recorder, invariants and frame-time capture (all inert without probe's env).
- Fleet runs: `probe run <list|category|--all>` run sequentially with an aggregated `index.html`/`index.json` status index and worst-row verdict.
- `--fps` is a dedicated capture-only pass; narrative one-shots are fps-EXEMPT via `Cargo.toml` metadata; the capture window and completion deadline are sized to the request.
- `--baseline` works across a group: regression-check a whole fleet against a prior `--all` run (missing captures are SKIPPED, not errors).
- Harness runs are silent: any bcs harness env zeroes the audio output (`NOVA_MUTE` overrides); the volume setting and menu are untouched.
- Examples moved into purpose directories (`sections|gameplay|ui|screenshots|perf/`) with the root `Cargo.toml` `[[example]]` catalog as the single source of truth; per-category smoke tests.
- Profiled pass renders a top-N costliest-systems table (Perfetto chrome trace via a new `trace` feature); `--samply` adds a flamegraph on a dedicated profile.
- Frame-time rows record run metadata (backend, GPU, resolution, preset, git SHA, host) and build profile; pre-metadata files still load.
- Run-harness hardened for unattended use: a cleaned run dir + `probe-run.json` manifest, so a hung or crashed run still produces a FAILing report.
- bevy_common_systems 0.19.1 -> 0.19.2: `GameEvent` gained public `name()`/`info()` read accessors for external observers.
- The lessons ledger moved to `LESSONS.md` at the repo root; `docs/` wipes to only its README at a release (guard-enforced).

## [0.7.0] - 2026-07-18

### Gameplay & Flight

- RCS fine docking thrusters: hold SHIFT to translate along the ship's own axes under a ~2 u/s cap (no rotation) - a per-ship controller verb, withheld in the mainline.
- GOTO/STOP autopilot now settle their arrival on RCS, easing to a stop instead of pulsing on the spot.

### Combat & Weapons

- Turrets fire and aim every muzzle they have: twin-barrel PDCs throw two streams at once, sharing one magazine.
- Hostiles respect line of fire: hold fire (and torpedoes) when hard cover blocks the shot; intangible volumes and PDC-vs-torpedo exempt.
- Weapons auto-reload: a dry magazine refills on its own, so magazine size is a fire-pacing beat and finite ammo is on by default.
- Locking is no longer instant: a lock-on dwell (longer at range) you must hold the target through before it commits.

### Ships & Sections

- Enemy ships show battle damage: a destroyed hostile section reads burnt-black (no intermediate red on the enemy hull).

### Scenarios & Objectives

- Scenarios can declare a win/lose: new `Outcome` action shows a VICTORY/DEFEAT overlay with real buttons and freezes the sim behind it.
- **Broadside**, chapter two of the base storyline: a distress call, a two-corvette ambush, then screen a gunship's torpedoes - chained from Shakedown's victory.
- Broadside plays as two scenarios (checkpoint after the ambush; the gunship retries itself); invulnerable boulders anchor the big fights.
- Campaign + Ledger storytelling pass: fights announce themselves on an arrival grace, comms beats spaced on the scenario clock, closing lines moved into the banner; beat-sheet convention documented and lint-enforced.
- Scenario transitions got a middle gear: `NextScenario` gained a `delay` and Outcome banners an `auto_advance_secs`; lint warns on pairing an Outcome with a non-lingering switch.
- The Ledger chapter two rebuilt around fair fights (single lane, mook guns, a cover field) and split into two checkpointed acts.
- The Ledger finale's Auditor traded its top-tier turret for the light mook gun on both ending branches.
- Every combat scenario now flies real auto-reloading ammo (unlimited stays an authoring/testing option).
- Ships in scenario RON can author an `allegiance` (e.g. `Some(Neutral)` for bystanders).
- The Asteroid Field sandbox joined the outcome frame (Victory on arrival, Defeat + Retry on death).
- The ship-less Demo Scenario was removed; the example mod's arena is the worked hand-authored RON example now.

### Fixes

- The Rust Tally's side mounts now roll to seat against the spine and face outboard (firing arcs fixed, port/starboard ids un-swapped).
- The Ledger Auditor's torpedo bay comes out of the hull (flush on the bow); a new overlap lint catches the whole class at build time.
- Skybox `.meta` sidecars are honored for every asset (base and mod, shipped or downloaded), fixing the WebGL2 oversized-upload crash.
- Retry no longer blanks the objectives panel (the objectives HUD resets on teardown).
- A ship that loses its last section off the damage path no longer lingers as a 0-HP ghost (structural death backstop).
- Scenario `OnUpdate` handlers now freeze while the game is paused.

### Modding & Mod Portal

- Turret mounts are an arbitrary joint tree **(breaking)**: `root` + recursive `children` (offset/axis/render_mesh/muzzle) replace the fixed yaw/pitch/barrel fields; `fire_rate` is per-muzzle; stock content migrated, the lint checks the tree.
- Scenarios can tell time **(breaking)**: reserved `scenario_elapsed` variable (live unpaused seconds) readable from any filter; writing it is a lint error.
- `orbit_hold_secs` / `lock_refire_secs` are author-tunable per ship (default 5s; a non-positive value is a lint error).
- Mods can ship their own art: a `resources` list + `self://` paths that resolve against the mod's own folder.
- Asset paths always carry a scheme **(breaking)**: `self://`, `dep://<id>/`, `dep://base/`; the base game is a normal mod now and a bare path is an error.
- One `example` mod is the single copy-me tutorial (section overlay, new section, arena, shipped art, comms, Outcomes, menu backdrop).
- New `menu_backdrop` flag: the menu picks a flagged scenario at random; New Game moved to the base bundle's `new_game_scenario`.
- Two new menu backdrops: Waystation Traffic and Scrapyard Drift.
- New `StoryMessage` action: speaker-attributed dialog in a HUD comms panel.
- Broken mod content fails loud: reference errors give a FAILED TO START report; the backdrop rotation skips them.
- `content -- lint` lints every scenario for reference bugs (CI-enforced); `--target <mod>` scopes to one mod.
- **The Ledger**, the first campaign mod on the portal: a four-chapter salvage arc with a two-ending finale. Install from Mods > Explore.
- Every world sound is content-owned: weapon/controller/thruster/crate sound fields and per-target `impact_sound`/`destroy_sound` are authorable refs; base sounds ship under `assets/base/sounds/`.

### Interface & HUD

- Comms chatter is readable: lines queue in arrival order, fade in, hold a dwell and yield; newly posted objectives flash gold.
- The diegetic ammo gauge shows reload as a dimmer sweep filling the pips back up.
- The pause menu gained a Retry button (offered only while a scenario is live).
- The Settings menu is real: a master volume slider, a Low/Medium/High graphics preset, and a keybind reference, remembered across restarts.

### Performance

- Low/Medium presets skip heavy visuals for weak machines: Low is spawn-less (no particle bursts), Medium keeps particles.
- Low also renders the world at ~70% internal resolution and upscales (HUD stays crisp); Medium/High untouched.

### Internals & Tooling

- Content tools are one CLI: `content -- <gen|lint|audit>` replaces the three former binaries.
- The content lint checks mount seating (a base face must seat against an occupied neighbor cell), at build time and in the in-game gate.
- The two 5s scenario-event windows now measure against `scenario_elapsed` (one place freezes and resets them on pause/teardown/retry).
- The balance audit learned acknowledgments (`balance_acks.ron`): intended drama acked with a reason + task; errors can't be acked, a stale ack fails CI.
- `content -- audit` grades each combat scenario's fairness sheet and fails CI on an armed hostile that spawns inside its own range of the player.
- Firing no longer spams an avian "no mass or inertia" warning (bullets carry an explicit angular inertia).
- Debug builds bind F12 to a screenshot saved to Downloads.

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

- Enemies can be authored to ARRIVE instead of appearing: a scenario ship with an `engage_delay` grace flies its patrol and holds its fire for those seconds before going hot - shoot it and the courtesy ends instantly and permanently, and its point-defense never stops watching for torpedoes. Paired with a warning comms beat, a spawn now reads as an approach you were told about, not an ambush from nowhere.
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

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.8.1...HEAD
[0.8.1]: https://github.com/alexjercan/nova-protocol/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/alexjercan/nova-protocol/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/alexjercan/nova-protocol/compare/v0.6.0...v0.7.0
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
