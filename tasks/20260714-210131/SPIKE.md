# Spike: screenshot-showcase pipeline - how to generate the web site's missing screenshots

- DATE: 20260714-210131
- STATUS: RECOMMENDED
- TAGS: spike, example, screenshot, web, testing

> Focused implementation spike for the screenshot-showcase example. Supersedes the
> scope of the earlier seed task `tasks/20260714-081706/TASK.md` (same goal, no
> detail); that task can be closed as absorbed here, or this task folded back into
> it - a decision for the planner. The high-level direction lived in the roadmap
> spike `tasks/20260714-081636/SPIKE.md`; this doc is the concrete design.

## Question

The web site references ~18 screenshot PNGs (plus 5 section icons) that do not
exist yet - empty placeholders. How do we generate them from the game, in-engine,
reproducibly, and package them into `web/src/assets/` at the right resolutions? The
user's framing: author it as a **scenario mod loaded in an autopilot example**,
where the reel is "a chain of events, and on each event we capture a screenshot
from a WASD camera moved to the subject", plus a Python step that validates
format/resolution and moves the files. And: if a screenshot primitive is generally
useful, it should land in the game itself, not as a dev-only dependency.

## Context - what already exists (verified in code)

**Capture primitive (no new crate needed).** Bevy 0.19's
`bevy::render::view::screenshot::{Screenshot, save_to_disk}` is already used in the
tree (`examples/11_hud_range.rs:672` spawns `Screenshot::primary_window()` and
observes `save_to_disk("inset_shot.png")`). `bevy` is pulled with default features
(render + png), including in `nova_scenario` (`crates/nova_scenario/Cargo.toml:9`),
so the primitive is reachable from the game crates - **no Cargo change and no new
dependency**. This confirms the roadmap answer: `bevy_capture` stays a backlog
spike for video/GIF; stills need nothing new.

**The single-shot harness is not a reel.**
`bevy_common_systems::debug::harness::ScreenshotPlugin` (wrapped as
`nova_screenshot()` in `crates/nova_debug/src/harness.rs:94`) advances to one state,
settles N frames, captures **one** PNG, and exits. It cannot drive a multi-beat
reel. So the reel needs its own sequencer that reuses the underlying
`Screenshot + save_to_disk` primitive per beat.

**The autopilot harness is a usable engine.**
`AutopilotPlugin::input(|world, elapsed| ...)` (autopilot.rs) runs a closure every
frame in `PreUpdate` with `&mut World` and elapsed seconds, gated on `BCS_AUTOPILOT`.
`11_hud_range` and `12_menu_newgame` already drive multi-second scripted timelines
(menu clicks, combat locks, GOTO, component locks, mid-run captures) through it.
This is the proven pattern for stepping through contexts and capturing.

**The RON scenario + mod system is SHIPPED** (the roadmap's "never shipped" note is
stale). `nova_modding` loads `*.bundle.ron` manifests -> `*.content.ron` files
(`Vec<Content>`, `Content::Scenario(ScenarioConfig)` / `Content::Section`), listed
in `assets/mods.catalog.ron`, merged by `EnabledMods` (base first, later bundles
overlay by id). A working `assets/mods/demo/` proves the path. All `ScenarioConfig`
types derive serde behind the `serde` feature. So "author the reel as a scenario
mod" is real: drop `assets/mods/screenshot-reel/{screenshot-reel.bundle.ron,
reel.content.ron}` and a catalog entry.

**The scenario action model** (`crates/nova_scenario/src/actions.rs`): 14
`EventActionConfig` variants, each an `EventAction<NovaEventWorld>` that queues work
via `world.push_command(|commands| ...)` and, when it needs world access,
`commands.queue(|world: &mut World| ...)`. Adding a variant is mechanical: enum arm
+ payload struct + `EventAction` impl + `action()` match arm + prelude export (the
`DespawnScenarioObject` block at actions.rs:191-243 is the template). **Gaps for the
reel: there is no camera action, no screenshot action, no Wait/Delay, and no
built-in time/frame variable** (`OnUpdate` fires every frame; expressions only read
the variable map).

**The camera.** `on_load_scenario` (loader.rs:455) spawns the scenario camera with
`ScenarioCameraMarker` + `WASDCameraController` + `PostProcessingCamera` +
`SkyboxConfig`. The WASD controller's `sync_transform` (bevy_common_systems
`camera/wasd.rs`) **overwrites `Transform` every frame** from `WASDCameraState` (on
`Changed<WASDCameraState>`). So a scripted camera pose must either update
`WASDCameraState` or remove `WASDCameraController` (the player-spawn observer at
loader.rs:584 already does the remove-and-swap dance). When a player ship spawns the
camera swaps to `SpaceshipCameraController` (chase cam).

## The reframe that drives the design

The ~18 shots are **not one scene filmed from a moving camera** - they span
different game contexts and UI:

- **Menu / backdrop** (`tutorial-menu.png`): the real `MainMenu` state over the
  ambience scenario.
- **Editor + section icons** (`feature-editor.png`, `wiki-sections.png`,
  `icon-hull/controller/thruster/turret/torpedo-bay.png`): the Sandbox editor UI and
  per-section closeups.
- **3-up HUD tiers** (`feature-hud.png`): the HUD at full/minimal/clean chrome.
- **Combat / radar / inset / juice** (`feature-combat.png`,
  `tutorial-combat-lock.png`, `feature-juice.png`, `devlog5-*`): scripted locks,
  the viewfinder inset, a section blown off - exactly what `11_hud_range` already
  drives.
- **Pure-3D scenes** (`feature-autopilot.png`, `feature-gravity.png`,
  `tutorial-orbit.png`, `wiki-gravity.png`, and the asteroid-field backdrop for
  thumbs): a ship maneuvering / orbiting / a gravity well - **these are the shots
  where "a WASD camera posed at the subject" is exactly right.**
- **Devlog thumbnails** (`thumb-devlog-3/4/5.png`): thematic framings reusing the
  above scenes.

So a single scenario mod cannot produce all of them. The deliverable is an
**autopilot capture example that steps through several contexts**, reusing the
existing menu/editor/combat drivers, and the **scenario mod is the right tool for
the pure-3D subset**. The user's mental model is correct - for the shots it fits.

## Options considered

### A. Capture engine: how the reel steps and captures

- **A1. Everything in the example's autopilot closure (code-driven beats).** One
  `ReelDriver` state machine in the example: per beat, bring up the context (set
  state / load scenario / drive UI), settle K frames, pose camera, capture to the
  named path, advance. Reuses `12_menu_newgame` (menu+editor) and `11_hud_range`
  (combat) verbatim. Pro: handles ALL contexts, not just 3D scenes; proven pattern;
  deterministic. Con: choreography is code, not data.
- **A2. Pure-data reel in RON** (new `Screenshot` + `SetCamera` actions, a counter
  variable ticked on `OnUpdate` for settle). Pro: the "chain of events, screenshot
  per event" the user pictured; fully authorable. Con: only covers the pure-3D
  scene shots (no menu/editor/HUD/combat-UI); expressing "wait K frames then
  capture" in RON needs a manual `OnUpdate` counter (no time primitive), which is
  awkward and fragile.
- **A3 (recommended). Hybrid.** Add the two scenario actions (they double as an
  in-game **photo mode** - the "put it in the game" the user asked for), author the
  pure-3D showcase beats as a RON scenario mod that uses them, AND drive the
  cross-context beats (menu, editor, HUD, combat) from the example's autopilot. A
  small reusable `ScreenshotReelPlugin` in `nova_debug` owns the settle-then-capture
  cadence so both the RON beats and the code beats share one timing path and one
  output-path convention. Best of both: data where data fits, code where UI state
  is unavoidable, and the primitive lands in the game.

### B. Scripting the camera pose (the WASD fight)

- **B1 (recommended). `SetCamera` action removes `WASDCameraController` and sets
  `Transform` + `WASDCameraState`** on the `ScenarioCameraMarker` entity (mirrors
  the player-spawn swap at loader.rs:584). The pose then holds - `sync_transform`
  won't run without a state change. Re-enable WASD on teardown. Deterministic
  framing, no input needed (autopilot has no live WASD input anyway).
- **B2. Keep WASD, write `WASDCameraState.position/yaw/pitch`.** Lighter, but you
  must decompose a `look_at` into yaw/pitch and trust the controller's next
  `sync_transform`; more coupling to the controller internals.

Recommend B1: an explicit pose is what a screenshot reel and a future photo mode
both want.

### C. Where the screenshot action lives

- **C1 (recommended). `Screenshot { path }` in `nova_scenario`** (a real game
  feature / photo mode). `push_command -> queue -> spawn(Screenshot::primary_window())
  .observe(save_to_disk(path))`. Guard against a headless-without-render context by
  degrading to a warn (like `HintEmphasisSet` checks for its resource). Path is
  resolved relative to a configurable capture dir (default cwd, overridable by env
  so the example/python can redirect to a staging folder).
- **C2. Screenshot only in the dev harness** (`nova_debug`). Rejected: the user
  explicitly said a useful capture primitive belongs in the game, and a photo-mode
  action is genuinely useful (community mods, marketing, the attract mode).

### D. Resolution / packaging

Capture at the hero resolution and let a **Python packaging script**
(`scripts/gen-web-screenshots.py`, matching the existing `scripts/*.py` +
`preview-web.sh` convention) do format/size normalization:

- **Heroes / figures / thumbs**: 16:9 (`.figure__img` and `.post-card__media` are
  `aspect-ratio: 16/9`). Capture at `1920x1080` via `BCS_SHOT=1920x1080` window
  sizing (or the reel's own resolution override); thumbs are the same frames
  downscaled to ~`640x360`.
- **Section icons** (`icon-*.png`, 44x44): capture a per-section closeup at high res,
  then center-crop square + resize to 44x44 in the script.
- The script validates each expected filename exists, checks dimensions/aspect,
  resizes/crops per target, and copies into `web/src/assets/`. It **fails loudly**
  on a missing or mis-sized shot (no silent partial runs). Pillow is the obvious
  dependency; keep it optional/documented like `gen-placeholder-sounds.py`.

### E. Headless / CI

The examples open a real window (`DefaultPlugins`); CI runs them under Xvfb +
lavapipe (`tests/examples_smoke.rs`, `.github/workflows/ci.yaml`). The reel example
should join `HARNESSED_EXAMPLES` so a `BCS_AUTOPILOT` run (no file output, just
"reach every beat and exit clean") gates it in the smoke suite, while the actual
capture is a `BCS_SHOT`/on-demand run the Python script invokes. Generating the real
PNGs stays a **run-on-demand tool**, not a blocking CI step (capture correctness is
visual; committing binaries in CI is undesirable). This answers the roadmap's open
question "smoke-gated or on-demand": **both** - smoke-gated for "does it run", on-demand for "produce the images".

## Recommendation

Ship option **A3 (hybrid)** with **B1** camera posing and **C1** in-game actions:

1. **Two in-game scenario actions** (photo mode), added to `nova_scenario`:
   `Screenshot { path }` and `SetCamera { position, look_at }` (+ a `settle`/`hold`
   convention). Mechanical enum + `EventAction` additions, unit-tested like the
   existing actions, serde round-tripped.
2. **A reusable `ScreenshotReelPlugin`** in `nova_debug` that owns the
   "context -> settle -> pose -> capture -> advance" cadence and the output-path /
   capture-dir convention, env-gated so it is inert in a normal run.
3. **A `screenshot-reel` scenario mod** (`assets/mods/screenshot-reel/`) authoring
   the pure-3D showcase beats (orbit, gravity well, autopilot maneuver, asteroid
   field, section-blown juice) as a chain of events using the new actions - the
   fully data-driven part the user pictured.
4. **One autopilot example** (`examples/13_screenshot_reel.rs`) that runs the whole
   reel: loads the mod for the 3D beats, and reuses the menu/editor/HUD/combat
   drivers (from `editor_app`, `12_menu_newgame`, `11_hud_range`) for the
   UI/state-dependent beats, capturing each to its web filename.
5. **A Python packaging script** (`scripts/gen-web-screenshots.py`) that runs the
   example, normalizes format/resolution (16:9 heroes, downscaled thumbs, cropped
   44x44 icons), and moves the files into `web/src/assets/`, failing on any missing
   or mis-sized shot.
6. **A randomized attract-mode variant** (from the task): the same reel with a seed
   that scatters/varies the scene and sweeps the camera - for ad-hoc marketing
   capture. Lowest priority; a flag on the example.

Sequencing: the two actions + the reel plugin are the foundation; the mod and the
example consume them; the Python script and web wiring come last; attract mode is a
stretch. Full step breakdown in `TASK.md`.

## The shot inventory (target: everything the site references)

Grouped by the context that produces it. Filenames land in `web/src/assets/`
(webpack copies to `dist/assets/`). All 16:9 except the 44x44 icons.

**Feature (home, hero 16:9):** `feature-editor.png` (editor UI, sections bolted on),
`feature-autopilot.png` (ship mid GOTO/ORBIT, plume lit), `feature-gravity.png`
(ship in stable orbit + radius spoke), `feature-combat.png` (red combat lock +
lead pips), `feature-juice.png` (section blown off, hit rings + shake),
`feature-hud.png` (3-up HUD tiers).

**Tutorial (16:9):** `tutorial-menu.png` (main menu + ambience backdrop),
`tutorial-radar-lock.png` (NAV crosshair mid-sweep, brackets snapping),
`tutorial-orbit.png` (clean ORBIT circle + spoke), `tutorial-combat-lock.png` (red
reticle on hulk + viewfinder inset).

**Wiki (16:9):** `wiki-sections.png` (built ship, sections called out),
`wiki-gravity.png` (well diagram or orbit).

**Section icons (44x44):** `icon-hull.png`, `icon-controller.png`,
`icon-thruster.png`, `icon-turret.png`, `icon-torpedo-bay.png`.

**Devlog thumbnails (16:9, ~300px display):** `thumb-devlog-3.png` (torpedo/blast),
`thumb-devlog-4.png` (guided torpedo / targeting), `thumb-devlog-5.png` (radar
locking + menu). Devlogs 1-2 use YouTube thumbnails - not ours.

**Devlog-5 post figures (16:9):** `devlog5-radar-stance-slots.png` (NAV vs combat
reticle side by side), `devlog5-target-viewfinder.png` (viewfinder inset + fine
lock). Nice-to-have; reuse the combat/radar beats.

## Open questions (for the planner / user)

- **Task structure.** This spike created a fresh task (`20260714-210131`) as
  instructed. The older seed `20260714-081706` covers the same goal - close it as
  superseded, or keep it as the umbrella and demote this to its child? Recommend
  closing 081706.
- **Scope cut.** ~25 images across 6 contexts is a lot for one `/work` session. The
  `TASK.md` steps are phased so each phase is a coherent stopping point (actions +
  plugin -> 3D mod + example -> UI/combat beats -> icons -> python + web wiring ->
  attract mode). Ship in phases, or all at once? Recommend phased, with the first
  milestone being the two actions + reel plugin + the pure-3D beats (the part that
  proves the whole pipeline end to end).
- **Capture resolution.** `1920x1080` heroes assumed. If the site wants sharper
  (2x) or a different hero aspect, set it before authoring poses. (Banner is
  currently 768x512, a different asset.)
- **Section icons.** 44x44 is tiny; a rendered ship-section closeup cropped to 44
  may read poorly. Alternative: author simple 2D/diagram icons instead of captures.
  Decide during the icon phase.
- **Committing binaries.** The PNGs are build output but the site needs them in
  git. Confirm they are committed to `web/src/assets/` (they are content, like
  `banner.png`), not generated at deploy time.

## Fix record

(Appended by the implementing task as it lands.)
