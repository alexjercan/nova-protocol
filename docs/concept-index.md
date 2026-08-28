# Concept index

The routing table. Search a concept (the search icon or `S`), open the named
files, start reading at the entry symbol. Paths are repo-relative; every row
was verified against the tree. For API detail run
`cargo doc --open -p <crate>` locally - every crate exposes a `prelude`. For
prose depth, follow the chapter linked at each group heading.

## Boot and frame flow

Depth: [Architecture](architecture.md).

| Concept | Crate(s) | Key files | Entry symbol | Orientation |
| --- | --- | --- | --- | --- |
| App boot, app assembly, plugin order | `nova_core` | `crates/nova_core/src/lib.rs` | `AppBuilder` | `build()` wires the whole plugin stack; game binary + CLI flags in `src/main.rs`. |
| Game states, pause | `nova_gameplay`, `nova_menu` | `crates/nova_gameplay/src/lib.rs`, `crates/nova_menu/src/pause.rs` | `GameStates`, `PauseStates` | State enums + `GameMode` live in gameplay; ESC overlay, `toggle_pause` and the clock freeze in menu. |
| Frame flow: Update vs FixedUpdate | `nova_ship`, `nova_gameplay` | `crates/nova_ship/src/lib.rs`, `crates/nova_gameplay/src/plugin.rs` | `SpaceshipSystems` | `NovaShipPlugin` chains the set brackets identically in both schedules; avian physics runs in `FixedPostUpdate`. Which schedule your system goes in: the Architecture chapter. |
| Asset loading gate, mod merge | `nova_assets` | `crates/nova_assets/src/plugin.rs`, `crates/nova_assets/src/merge.rs` | `GameAssetsPlugin` | `GameAssetsStates` gates entry to the menu/game; `register_bundles` merges base + enabled mods into the `Game*` resources. |

## The ship

Depth: [Ship sections internals](sections.md).

| Concept | Crate(s) | Key files | Entry symbol | Orientation |
| --- | --- | --- | --- | --- |
| Ship building from sections | `nova_scenario`, `nova_ship` | `crates/nova_scenario/src/objects/spaceship.rs`, `crates/nova_ship/src/sections/base_section.rs` | `insert_spaceship_sections` | Observer on the ship root spawns one child per `SectionKind` config, resolving prototypes; `SpaceshipConfig` is the authored shape. |
| Section integrity: damage, disable, destroy | `nova_gameplay` | `crates/nova_gameplay/src/integrity/mod.rs`, `crates/nova_gameplay/src/damage.rs` | `NovaIntegrityPlugin` | `Health` store and the disable/destroy chain; typed `DamageType` + the travel rule (`apply_damage`, `pierce_remainder`). Health decides WHEN a body dies; the two readings below decide what it looks like. |
| How far gone a body looks | `nova_gameplay` | `crates/nova_gameplay/src/integrity/erosion.rs` | `DamageLevel` | One scalar, 0..1, derived from the entity's OWN `Health`. Grades every whole-body effect. |
| Where a body was hit (marks, carve cost, merge) | `nova_gameplay` | `crates/nova_gameplay/src/integrity/carve.rs` | `DamageMarks`, `mark_radius` | Spheres in the body's local frame, priced by what the hit ABSORBED at `DAMAGE_PER_UNIT_VOLUME` (8 hp per cubic unit); `record_blast_marks` cuts one crater per body. |
| Authored per-section damage looks | `nova_ship` | `crates/nova_ship/src/sections/damage_effects.rs`, `damage_cracks.rs`, `damage_sparks.rs`, `damage_plume.rs` | `DamageEffects`, `fit_damage_effects` | `Cracks`/`Sparks`/`Plume`, one component each, default `[Cracks]`. No ship section ever loses geometry. |
| Carve debris: dust and severed pieces | `nova_gameplay` | `crates/nova_gameplay/src/integrity/spew.rs`, `crates/nova_gameplay/src/integrity/chunk.rs` | `CarveSpew`, `ShardLook`, `spawn_carved_chunk` | Shards are keyed on the WEAPON CLASS - kinetic and pierce chip, explosive does not; only a cut that SEVERED material spawns a real body, floored at `CHUNK_MIN_VOLUME`. |
| Severing, wreck fragments, structural collapse | `nova_ship` | `crates/nova_ship/src/sections/integrity.rs` | `ShipIntegrityPlugin` | Builds the section graph, splits disconnected structure (`sever_disconnected_structures` -> `ShipWreckFragmentMarker`), runs `cascade_structural_collapse`. |
| Neutralization (combat-dead) | `nova_gameplay` | `crates/nova_gameplay/src/integrity/neutralize.rs` | `NeutralizedMarker` | An armed ship that loses all weapons, or the flight computer it had, fires `OnNeutralizedEvent`; the hull may survive. |
| Ship skins, styles, greebles | `nova_ship` | `crates/nova_ship/src/sections/shell_skin.rs`, `crates/nova_ship/src/sections/skin_style.rs`, `crates/nova_ship/src/sections/skin_decor.rs` | `ShipSkinPlugin` | Cladding derived from structure; styles resolve by id (`ShipStyleConfig`, `GameStyles`); deterministic greeble scatter (`scatter_decor`). |
| Turrets, aiming | `nova_ship` | `crates/nova_ship/src/sections/turret_section/mod.rs`, `crates/nova_ship/src/sections/turret_section/aim.rs` | `TurretSectionPlugin` | Authored joint tree; lead-intercept aim (`update_turret_aim_point`, `muzzle_on_target`); arcs and firing in sibling files. |
| Torpedoes, point defense | `nova_ship` | `crates/nova_ship/src/sections/torpedo_section/mod.rs`, `crates/nova_ship/src/input/point_defense/mod.rs` | `TorpedoSectionPlugin`, `SpaceshipPointDefensePlugin` | Bay + guided round lifecycle (`TorpedoGuidance`, `TorpedoBlast`); PD splits target assignment from mount authority (borrowed player mounts). |
| Flight autopilot verbs (STOP, GOTO, ORBIT), PD attitude controller | `nova_ship` | `crates/nova_ship/src/flight/state.rs`, `crates/nova_ship/src/sections/controller_section.rs`, `crates/nova_ship/src/physics/pd_controller.rs` | `FlightVerb`, `Autopilot` | Controller sections grant verbs; the flight layer flies them (`NovaFlightPlugin`); `PDControllerPlugin` is the attitude loop. |
| Radar locking, targeting | `nova_ship` | `crates/nova_ship/src/input/targeting/mod.rs`, `crates/nova_ship/src/input/targeting/state.rs` | `SpaceshipTargetingPlugin` | Radar search writes the two sticky lock slots on the ship root: `TravelLock`, `CombatLock`. |
| Gravity wells | `nova_gameplay` | `crates/nova_gameplay/src/gravity.rs` | `GravityWell` | Inverse-square wells with an SOI cutoff; anchors and asteroids publish one. |

## Scenario and modding

Depth: [Scenario engine](scenario-system.md).

| Concept | Crate(s) | Key files | Entry symbol | Orientation |
| --- | --- | --- | --- | --- |
| Scenario engine, mission scripting | `nova_scenario`, `nova_events` | `crates/nova_scenario/src/lib.rs`, `crates/nova_events/src/engine.rs` | `NovaScenarioPlugin` | A scenario is handlers = event + filters + actions over `NovaEventWorld`; the generic queue/dispatch (`EventHandler`) is `nova_events`. |
| Scenario events, filters, actions (modding events) | `nova_scenario` | `crates/nova_scenario/src/events.rs`, `crates/nova_scenario/src/filters.rs`, `crates/nova_scenario/src/actions/mod.rs` | `EventConfig`, `EventFilterConfig`, `EventActionConfig` | One dispatch enum each; actions fan out to the flow/mission/sequence/ship/spawn/timer/view submodules beside `mod.rs`. |
| Scenario variables, expressions, watches | `nova_scenario` | `crates/nova_scenario/src/variables.rs`, `crates/nova_scenario/src/world.rs` | `VariableExpressionNode`, `NovaEventWorld` | Typed literals + expression tree (`VariableConditionNode` for filters); watches sample typed world queries into variables. |
| Scenario objects (asteroid, spaceship, beacon, crate, light, anchor) | `nova_scenario` | `crates/nova_scenario/src/objects/mod.rs`, `crates/nova_scenario/src/actions/spawn.rs` | `ScenarioObjectsPlugin`, `ScenarioObjectKind` | One module per kind under `objects/`; the spawn action dispatches on the kind enum. |
| Scenario loading, lifetime scoping, teardown | `nova_scenario` | `crates/nova_scenario/src/loader/mod.rs`, `crates/nova_scenario/src/loader/lifecycle.rs` | `ScenarioLoaderPlugin` | `LoadScenario`/`UnloadScenario` observers; everything tagged `ScenarioScopedMarker` dies at teardown; `scenario_is_live` gates the ship sets. |
| Mod formats, bundles, portal | `nova_mod_format`, `nova_modding`, `nova_assets` | `crates/nova_mod_format/src/lib.rs`, `crates/nova_modding/src/lib.rs`, `crates/nova_assets/src/portal/mod.rs` | `BundleManifest`, `Content`, `PortalPlugin` | Engine-free serde wire types -> RON asset loaders -> portal fetch/verify/install. The static portal is generated by `scripts/gen-portal.py` over `webmods/`. |
| Content generation (base RON, builders, lint) | `nova_authoring` | `crates/nova_authoring/src/cli.rs`, `crates/nova_authoring/src/generation.rs` | `cli::main` | `cargo run content gen` serializes the `base_content` builders into the committed base `*.content.ron`; `content lint` validates any content tree. Never hand-edit generated RON. |

## Interface

| Concept | Crate(s) | Key files | Entry symbol | Orientation |
| --- | --- | --- | --- | --- |
| NOVA OS (terminal, shell, ship computer apps) | `nova_os`, `nova_os_ui` | `crates/nova_os/src/command.rs`, `crates/nova_os/src/app.rs`, `crates/nova_os_ui/src/lib.rs` | `NovaOsUiPlugin` | Pure model in `nova_os` (`NovaOsCommandRegistry`, `NovaOsAppRuntime`); the Tab CRT monitor and the map/ship apps in `nova_os_ui`. |
| Keybinds: the action table, rebinding, the capture | `nova_input`, `nova_menu` | `crates/nova_input/src/registry.rs`, `crates/nova_input/src/poll.rs`, `crates/nova_menu/src/settings.rs` | `InputBindings`, `InputSources` | One table of named actions; each owner registers its own defaults, every rig is BUILT from it, and every rebind surface reads it. Settings' Controls tab takes the next press through `InputSources`; overrides persist in `settings_store`. |
| Flight HUD widgets | `nova_hud` | `crates/nova_hud/src/lib.rs` | `NovaHudPlugin` | One module per instrument (crosshairs, ammo, markers, comms, keybind dock); reads the ship, never drives it. |
| Main menu, menu backdrops | `nova_menu` | `crates/nova_menu/src/lib.rs`, `crates/nova_menu/src/ambience.rs` | `NovaMenuPlugin` | The backdrop is a random `menu_backdrop`-flagged scenario drawn live (`load_menu_ambience`); `NOVA_MENU_BACKDROP` pins one id for capture. |
| Shared UI theme, widgets, skins | `nova_ui` | `crates/nova_ui/src/lib.rs`, `crates/nova_ui/src/widget/mod.rs` | `NovaUiPlugin` | Theme tokens, the `UiSkin` switch, and the widget factories every UI-drawing crate consumes. |
| Ship editor, link points, placement | `nova_editor`, `nova_ship` | `crates/nova_editor/src/lib.rs`, `crates/nova_editor/src/snap.rs`, `crates/nova_ship/src/sections/link_points.rs` | `NovaEditorPlugin` | Sandbox build scene; `snap_placement` mates socket frames; the editor solver decides or refuses a drop. |

## Tooling

Depth: [Automation harness](automation-harness.md),
[Measuring performance](performance.md) and
[Building and running](development.md).

| Concept | Crate(s) | Key files | Entry symbol | Orientation |
| --- | --- | --- | --- | --- |
| Automation harness, autopilot scripts, screenshot capture | `nova_autopilot`, `nova_debug` | `crates/nova_autopilot/src/autopilot.rs`, `crates/nova_debug/src/harness.rs` | `AutopilotPlugin` | Env-armed step driver (`NOVA_AUTOPILOT`, `NOVA_CAPTURE`); Nova presets and the `shoot` capture idiom live in the debug harness module. |
| Probe, run reports, what an example claims | `nova_probe`, `nova_probe_cli` | `crates/nova_probe/src/capabilities/mod.rs`, `crates/nova_probe/src/contract.rs`, `crates/nova_probe_cli/src/native.rs` | `NovaProbePlugin` | One bundle wires every capability; `probe run`/`scenario`/`report` (subcommands of the game binary, `debug` feature) spawn and grade runs. Web capture app: `crates/nova_perf_web/src/main.rs`. |
| Frame cost, scene census, the capture window | `nova_probe` | `crates/nova_probe/src/capabilities/frametime.rs`, `framecost.rs`, `census.rs` | `nova_frametime`, `nova_framecost`, `nova_census` | Wall-clock deltas over a fixed window, plus where the milliseconds went and what the scene contained. Knob table: `cargo doc -p nova_probe`. |
| World-state snapshot (read the world, not a render) | `nova_probe` | `crates/nova_probe/src/capabilities/snapshot.rs` | `nova_snapshot`, `probe_snapshot` | One JSON object per snapshot: ships, sections, fixtures, weapon state, rounds in flight. Sorted and rounded, so two snapshots of one frozen frame are byte-identical. |
| WFC arena, generated hulls | examples | `examples/playable/wfc_arena.rs`, `examples/playable/shared/wfc.rs` | `wfc_hull` | Wave-function collapse over real section prototypes into flyable hulls; the arena's lobby/pause/result match flow sits in `examples/playable/wfc_arena/`. |
| Debug tooling (inspector, overlays, F12 screenshots) | `nova_debug` | `crates/nova_debug/src/lib.rs` | `DebugPlugin` | Compiled only under the `debug` feature; F11 overlay toggle, F12 screenshot, `--norender`, `--debugdump`. |
| Web preview, this book at /dev/ | scripts, web | `scripts/serve-web.sh`, `scripts/preview-web.sh`, `web/webpack.config.js` | `serve-web.sh` | Live-serves site + `/play/` + `/mods/` + this book at `/dev/`, all watched; the preview script builds the static deploy shape. |
