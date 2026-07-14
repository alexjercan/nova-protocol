# Spike: v0.6.0 direction - modding language, better editor, screenshot/example tooling

- DATE: 20260714-081636
- STATUS: RECOMMENDED
- TAGS: spike, roadmap, planning

> SUPERSEDED IN PART (20260714): this is the high-level direction snapshot. After
> user review, the editor collapsed to a single scenario-builder task (ship
> save/load and the UI-overhaul task were closed), the RON work was split and
> detailed, and the optimizations were gated behind a benchmark. The living plan is
> `docs/plans/20260714-v0.6.0-plan.md`; the detailed modding/opt design is
> `tasks/20260714-083224/SPIKE.md`. Trust those over the task IDs in "Next steps"
> below.

## Question

v0.5.2 shipped (radar locking, tutorial, typed damage, main menu, web site with a
full wiki, a 12-example testable curriculum). What should v0.6.0 be, and which
backlog tasks belong in it? The user set three explicit goals for this sprint:

1. **Modding language + optimizations for it.**
2. **A better editor** - sandbox-mode UI/layout, and ideally building scenarios
   and saving/loading them.
3. **More testing/example utilities**, headlined by a screenshot-generation
   example (runs a reel of small scenarios/objectives, spawns things, moves the
   camera to them, captures frames), plus research on an in-engine screen-capture
   crate.

A good answer commits concrete tasks to the sprint, moves the rest to backlog,
and prioritizes. No time constraint - the sprint runs as long as needed.

## Context

- **Modding today:** the scenario engine (`crates/nova_scenario`) is ~90% complete
  as *code* - events, filters, actions, a `variables.rs` expression AST, and a
  `GameScenarios` resource - but it is **not serialized**. There is no `serde` in
  `nova_scenario`, no `assets/scenarios/` dir, no `*.ron` files. Scenarios are
  built programmatically in `crates/nova_assets/src/scenario.rs`. The prior modding
  spike (`tasks/20260708-161726/SPIKE.md`) already decided the direction: **phase 1
  = a declarative RON format + `AssetLoader`; phase 2 = a piccolo scripting VM,
  much later and prototype-gated.** Phase 1 never shipped, so it is the heart of
  goal 1. Tasks 20260525-133029 (format) and 133028 (config resource) are the
  seeded phase-1 work; 133014 (index handlers by event name) is the "optimization".
- **Editor today:** `crates/nova_editor/src/lib.rs` is a single 1978-line file. It
  places sections on a fixed grid by click, shows per-section keybind chips, and
  has a scrollable build panel - but no rotation, copy/paste, templates, or
  persistence. Task 20260708-162014 already frames "editor polish + save/load ship
  blueprints", and explicitly wants the saved ship to be the same serialized
  `SpaceshipConfig` the scenario format uses - so goal 2 dovetails with goal 1.
- **Examples/capture today:** the 12-example curriculum already self-drives under
  `BCS_AUTOPILOT`, and there is a working in-engine screenshot preset -
  `nova_screenshot()` / `ScreenshotPlugin` in `crates/nova_debug/src/harness.rs`,
  gated by `BCS_SHOT`, built on Bevy 0.19's `Screenshot` component + `save_to_disk`
  observer. The web site (`web/src/`) references **~17 named screenshot files that
  do not exist yet** (empty placeholder divs): `feature-editor.png`,
  `feature-autopilot.png`, `feature-gravity.png`, `feature-combat.png`,
  `feature-juice.png`, `feature-hud.png`, `tutorial-menu/-radar-lock/-orbit/
  -combat-lock.png`, `wiki-sections/-radar/-gravity/-hud/-combat/-flight.png`,
  `thumb-devlog-3/-4/-5.png`. That is the concrete target for goal 3.

## Options considered

### Screen-capture: which mechanism? (goal 3 research)

- **A. Bevy 0.19 built-in `Screenshot` + `save_to_disk` (recommended).** Already
  vendored, already wrapped as `nova_screenshot()`, already driven by the autopilot
  harness. Spawn `Screenshot::primary_window()`, attach a `save_to_disk(path)`
  observer, get a PNG. For a series, spawn one per beat. Zero new deps. Windowed +
  real GPU is the simplest robust path; true headless works but needs a software
  Vulkan (lavapipe) on CI, which the current smoke suite does not have.
- **B. `bevy_capture` crate.** Purpose-built: wraps the same API and encodes frame
  series to PNG-sequence / MP4 / GIF / FFmpeg. Right tool if we want *video*, not
  stills. Needs a Bevy-0.19-compat check before adding.
- **C. `bevy_image_export`.** Deterministic offline frame-by-frame export; overkill
  for stills, aimed at rendering non-realtime animations.
- **D. External capture (scrot etc.).** Explicitly rejected by the user - wants
  in-engine.

The web placeholders are stills, and we already have the primitive, so **A** wins
for the sprint. **B** is worth a backlog spike only if a concrete video need
appears.

### How much modding to bite off

- **Phase 1 only (RON format + config-as-asset + handler-index opt) - recommended
  for the sprint.** This is exactly "implement the modding language + optimizations
  for it", it is mostly mechanical (the model already derives `Reflect`), it is
  wasm-safe and low-risk, and it unblocks editor save/load and an authorable
  scenario builder.
- **Phase 1 + piccolo scripting prototype (162010).** Rejected for this sprint. The
  prior spike gated piccolo on the declarative form "provably running out of road"
  - i.e. modders hitting the ceiling of the fixed action set and the arithmetic
  AST. We have not shipped the declarative form yet, so we cannot have hit its
  ceiling. Piccolo is also the high-risk half (WIP crate, stackless binding glue);
  pulling it in now would let the risky half hold the valuable, easy half hostage.
  Keep it backlog, still spike-gated.

### Editor scope

- **UI/layout overhaul + ship save/load (sprint) and scenario builder (stretch,
  spike).** The user asked for both "UI placement for sandbox mode" (concrete,
  self-contained, no dependency) and "maybe allow building scenarios and
  saving/loading them" (the word "maybe" plus a hard dependency on the RON format
  landing). So: commit the UI overhaul and ship-blueprint save/load; keep the
  full scenario-authoring UI as a lower-priority spike that plans once phase-1
  modding exists.

## Recommendation

**v0.6.0 = "Modding & Authoring".** One owning theme: turn the code-only scenario
engine into a data-driven, editable, showcase-able authoring platform. Three
strands, all reinforcing:

1. **Modding language (goal 1).** Ship phase-1 RON: add `serde` to the config
   model (including the object/`SpaceshipConfig` configs, so the editor can reuse
   them), write a `*.scenario.ron` `AssetLoader`, port the built-in scenarios into
   `assets/scenarios/*.ron`, load `GameScenarios` from assets, and index handlers
   by event name for the lookup optimization.
2. **Editor (goal 2).** Overhaul the sandbox UI/layout for legibility, add ship
   blueprint save/load on the same serialized representation, and (stretch,
   spike-gated on strand 1) an in-editor scenario builder that authors and
   round-trips `*.scenario.ron`.
3. **Examples/tooling (goal 3).** Build a screenshot-showcase example that plays a
   reel of framed mini-scenes and captures the ~17 named web-site screenshots via
   the existing `nova_screenshot` harness; add the small backlog example
   (`bevy_common_systems` PD/thruster) and the SFX integration test to round out
   the testing curriculum. Research answer: **no new capture crate is needed** for
   stills; Bevy's built-in screenshot (already wrapped) is enough. `bevy_capture`
   is parked as a backlog spike for future video/GIF.

Sequencing: strand 1's RON format (133029) is the foundation - editor save/load,
the scenario builder, and porting built-ins all consume it, so it carries the
highest priority. The screenshot example and the sandbox UI overhaul are
independent and can run in parallel with it.

## Open questions

- **How much of the object/blueprint model to serialize in phase 1.** The editor
  save/load and the scenario builder both want `SpaceshipConfig`/section data
  serialized, not just the event graph. Scope this when planning 133029 - lean
  toward doing the whole config model at once so there is one representation.
- **Headless capture on CI.** The showcase example runs windowed by default; if we
  want it in the blocking smoke suite headless, that needs lavapipe on the runner.
  Decide during 20260714-081706 whether it is a smoke-gated example or a
  run-on-demand tool.
- **Piccolo readiness.** Re-evaluate piccolo's stdlib/sandboxing when phase 2 is
  actually reached (still backlog, 20260708-162010).
- **Do the objectives/win-lose tasks (133026/133027) still stand?** v0.5.0 shipped
  an objective-conveyance HUD and markers; those two legacy tasks may be largely
  satisfied. Left in backlog; audit before planning them.

## Next steps

### v0.6.0 sprint (prioritized)

Modding language:
- tatr 20260525-133029 (p80) - RON scenario format + `serde` + `AssetLoader`;
  port built-ins to `assets/scenarios/*.ron`. Foundation.
- tatr 20260525-133028 (p70) - load `GameScenarios` from the RON assets.
- tatr 20260525-133014 (p40) - index event handlers by name (the optimization).

Editor:
- tatr 20260714-081700 (p65) - sandbox UI/layout overhaul (new).
- tatr 20260708-162014 (p60) - editor polish + ship blueprint save/load.
- tatr 20260714-081703 (p30, spike) - in-editor scenario builder, save/load RON
  (new; gated on 133029/133028).

Examples & testing:
- tatr 20260714-081706 (p55) - screenshot-showcase example -> web-site screenshots
  (new).
- tatr 20260525-133010 (p25) - minimal `bevy_common_systems` example.
- tatr 20260708-224303 (p20) - SFX event->sound integration test.

### Backlog (seeded / left)

- tatr 20260714-081710 (p0, spike) - evaluate `bevy_capture` for video/GIF (new).
- tatr 20260708-162010 - piccolo scripting VM prototype (phase 2, still gated).
- The other ~18 backlog tasks (gamepad/mobile, keybind icons, settings content,
  diegetic HP, alt-fire, docs passes, low-end visual mode, wasm particles,
  capital-combat vertical slice, etc.) stay `backlog` at p0 - none serve the
  v0.6.0 "modding & authoring" theme.

## Fix record

(Appended by each implementing task as it lands.)
