# Screenshot showcase pipeline: photo-mode scenario actions + reel capture example + web asset packaging

- STATUS: IN_PROGRESS
- PRIORITY: 55
- TAGS: v0.6.0, example, screenshot, web, testing

Spike: tasks/20260714-210131/SPIKE.md
Supersedes the scope of: tasks/20260714-081706/TASK.md (close as absorbed, or fold this in).

Goal: generate the ~18 screenshots (+ 5 section icons) the web site references as
empty placeholders, in-engine and reproducibly, and package them into
`web/src/assets/`. Built as a chain of framed beats captured by a WASD-posed camera,
authored partly as a scenario mod and partly as an autopilot example, plus a Python
normalization/move step. The capture primitive is Bevy 0.19's built-in
`Screenshot + save_to_disk` - no new crate. See SPIKE.md for the full design,
shot inventory, and the constraints that shaped it.

Phased so each phase is a coherent, shippable stopping point.

## Phase 1 - In-game photo-mode primitives (foundation)

1. Add a `Screenshot { path }` scenario action to `crates/nova_scenario/src/actions.rs`:
   new `EventActionConfig::Screenshot(ScreenshotActionConfig)` variant, payload
   struct with serde, `EventAction` impl that `push_command -> queue -> spawn(
   Screenshot::primary_window()).observe(save_to_disk(path))`, match arm, prelude
   export. Resolve `path` under a capture dir (default cwd; env override e.g.
   `NOVA_SHOT_DIR` so the example/python redirect to a staging folder). Degrade to a
   warn if render is unavailable (headless-without-render), like `HintEmphasisSet`.
   Follow the `DespawnScenarioObject` template (actions.rs:191-243).
2. Add a `SetCamera { position, look_at }` scenario action: pose the
   `ScenarioCameraMarker` entity by removing `WASDCameraController` and setting
   `Transform` (+ syncing `WASDCameraState` so a later re-enable is consistent),
   mirroring the player-spawn swap at loader.rs:584. Re-enable WASD on teardown.
3. Unit-test both actions like the existing ones (fire into a `NovaEventWorld`,
   drain via `state_to_world_system`, assert): SetCamera moves the marked camera and
   drops the WASD controller; Screenshot queues without panicking headless. Add a
   serde round-trip test for both configs (mirror `scatter_objects_config_round_trips_through_ron`).
4. Document the two actions in `docs/scenario-system.md` (they are photo mode; note
   the WASD-swap and the capture-dir env).

## Phase 2 - Reusable reel sequencer + pure-3D scenario mod + example (proves the pipeline)

5. Add a `ScreenshotReelPlugin` to `crates/nova_debug/src/harness.rs` (sibling to
   `nova_autopilot`/`nova_screenshot`), env-gated (reuse `BCS_SHOT` or a new
   `BCS_REEL`), that owns the per-beat cadence: bring up context -> settle K frames
   -> (pose camera) -> capture to the beat's path -> advance -> exit clean after the
   last beat. It shares the output-path/capture-dir convention with the Screenshot
   action. Keep the single-shot `nova_screenshot()` intact.
6. Author the pure-3D showcase beats as a scenario mod:
   `assets/mods/screenshot-reel/screenshot-reel.bundle.ron` +
   `reel.content.ron` (a `Scenario` using `SpawnScenarioObject`/`ScatterObjects` for
   each mini-scene, and `SetCamera` + `Screenshot` per beat), plus a catalog entry in
   `assets/mods.catalog.ron`. Beats: asteroid field, ship in stable orbit + gravity
   well (`feature-gravity`, `tutorial-orbit`, `wiki-gravity`), ship mid GOTO/ORBIT
   maneuver (`feature-autopilot`), a section blown off (`feature-juice`).
7. Add `examples/13_screenshot_reel.rs`: an autopilot example that enables the
   `screenshot-reel` mod, loads its scenario, runs the reel plugin, and exits. Wire
   it into `tests/examples_smoke.rs` `HARNESSED_EXAMPLES` so `BCS_AUTOPILOT` proves
   it reaches every beat and exits clean (no file output in the smoke path).
8. Verify a real capture run locally (windowed + GPU, or Xvfb+lavapipe): the pure-3D
   PNGs land in the staging dir at the target resolution. Eyeball framing.

## Phase 3 - UI / state-dependent beats (the shots a scenario alone can't make)

9. Extend the example to drive the cross-context beats through the autopilot closure,
   reusing existing drivers: main menu backdrop (`tutorial-menu`) via `editor_app`
   + `MainMenu`; the Sandbox editor with sections bolted on (`feature-editor`,
   `wiki-sections`) via the `12_menu_newgame` editorplay path; the 3-up HUD tiers
   (`feature-hud`); combat/radar/inset/juice (`feature-combat`,
   `tutorial-radar-lock`, `tutorial-combat-lock`, `devlog5-*`) via the
   `11_hud_range` lock/GOTO/inset script. Capture each to its web filename.
10. Devlog thumbnails (`thumb-devlog-3/4/5`): reuse the combat/torpedo/radar+menu
    beats framed for a thumbnail.

## Phase 4 - Section icons + Python packaging + web wiring

11. Section icons (`icon-hull/controller/thruster/turret/torpedo-bay`, 44x44):
    DECIDED - author simple diagram/vector icons (not rendered captures; ship
    closeups read poorly at 44x44). Produce 5 small PNGs (or SVG->PNG) directly into
    `web/src/assets/`; these are outside the reel/capture path.
12. `scripts/gen-web-screenshots.py`: run the example (headless via Xvfb+lavapipe or
    windowed) into a staging dir, then validate every expected filename, normalize
    format/resolution (16:9 heroes at 1920x1080, thumbs downscaled ~640x360, icons
    center-cropped + resized to 44x44), and move into `web/src/assets/`. Fail loudly
    on any missing or mis-sized shot. Pillow dependency; document like
    `scripts/gen-placeholder-sounds.py`.
13. Confirm the web pages pick up the real files (placeholders replaced), commit the
    PNGs to `web/src/assets/` (content, like `banner.png`), and note the regeneration
    command in `docs/development.md` and/or `web`'s docs.

## Phase 5 - Attract mode (stretch)

14. A randomized attract-mode variant of the reel (seeded scene scatter + a camera
    sweep) behind a flag on the example, for ad-hoc/marketing capture. Not wired into
    the web packaging.

## Notes / constraints (from the spike)

- No new capture crate: Bevy built-in `Screenshot + save_to_disk`; `bevy` already
  has render+png by default in `nova_scenario`.
- The single-shot `ScreenshotPlugin`/`nova_screenshot()` cannot do a reel - hence
  the new `ScreenshotReelPlugin`.
- WASD camera overwrites `Transform` each frame - `SetCamera` must remove the
  controller (loader.rs:584 pattern).
- The RON scenario/mod system is shipped (`nova_modding`, `mods.catalog.ron`, the
  `demo` mod) - the mod is data, no engine change to load it.
- Not all shots are pure-3D scenes - menu/editor/HUD/combat-UI beats must be
  code-driven in the example (reuse `12_menu_newgame` / `11_hud_range`).
- Smoke: `BCS_AUTOPILOT` gates "does it run" in `examples_smoke.rs`; real capture is
  a `BCS_SHOT`/on-demand run the Python script invokes (not a blocking CI step).
