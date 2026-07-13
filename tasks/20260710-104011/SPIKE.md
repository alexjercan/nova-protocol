# Spike: Target inset view - a zoomed close-up of the locked ship for easier section targeting

- DATE: 20260710-104011
- STATUS: RECOMMENDED
- TAGS: spike, hud, targeting, camera, backlog

## Question

Should nova get a "radar"/minimap-style inset panel that shows a magnified
live view of the currently locked enemy ship, so the player can see and
select individual sections more easily than squinting at 10 px markers on a
distant silhouette? A good answer picks the rendering approach (real 3D
close-up vs schematic diagram vs plain camera zoom), defines how the inset
interacts with the existing component fine-lock (snap + cycle), and scopes it
as a backlog item - explicitly NOT for the v0.4.0 sprint.

## Context

The component fine-lock arc (spike 20260709-192358, doc
tasks/20260709-192358/NOTES.md) already shipped everything the inset would
sit on top of:

- **Lock + fine-lock state exists.** `SpaceshipPlayerTargetLock`,
  `SpaceshipPlayerLockFocus` (1.5 s dwell) and
  `SpaceshipPlayerComponentLock` (Snap | Pinned, `[`/`]` cycle keys) live in
  `input/targeting.rs`. The inset is a pure consumer of this state; no new
  targeting mechanics are needed.
- **Sections are discrete entities.** Every section is a `SectionMarker`
  child of the ship root with its own `Health`, `GlobalTransform` and
  cuboid `Collider`; destroyed sections despawn, so any live render of the
  ship shows battle damage for free. `sections/mod.rs::live_structure_anchor`
  gives the COM-correct center of the surviving structure.
- **The HUD is bevy_ui via the screen-indicator widget** (`hud/screen_indicator.rs`).
  Section markers (hot-red, 10/16 px) already appear on the locked ship when
  focused (`hud/component_lock.rs`). The pain the inset addresses: at
  1-20 km, sections are sub-pixel and the markers overlap into a blob; snap
  and cycle work, but the player cannot see WHAT they are selecting.
- **Camera constraints.** One `Camera3d` per scenario
  (nova_scenario/loader.rs) carrying `PostProcessingCamera` and a skybox.
  A second WINDOW-targeting camera blacks out the 3D scene on Bevy 0.19
  (comment in hud/screen_indicator.rs:17); this is why the HUD is UI-pass.
  Render-to-texture (`RenderTarget::Image`) is a different path and is not
  used anywhere in the project yet - it is standard Bevy and works on
  WASM/WebGL2 (the project ships a Trunk/WASM build), but it is unverified
  in THIS codebase and must reconcile with `PostProcessingCamera` and the
  skybox.
- **Input model.** The mouse drives aim; there is no free-cursor mode in
  flight. Any "click a section in the inset" interaction implies a new
  cursor-release mode, which is a real input-design cost.

## Options considered

- **A. Render-to-texture inset (recommended).** A second `Camera3d` with
  `RenderTarget::Image`, spawned only while focused on a lock, positioned
  relative to the locked ship (framed on `live_structure_anchor` with the
  section-AABB union setting the distance), shown in a bevy_ui `ImageNode`
  panel (corner inset, ~256-320 px). The player sees the actual ship,
  actual orientation, actual damage state - sections visibly missing.
  Section selection stays on the existing snap/cycle mechanic; the inset
  makes it legible. Highlighting the fine-locked section is done IN-SCENE
  (emissive tint / outline on the selected section's material), which shows
  up in both the main view and the inset with zero projection code, and is
  a combat-juice win on its own.
  - Pros: exactly the requested "close-up of the enemy"; damage readout for
    free; no bespoke drawing code; highlight doubles as main-view juice.
  - Cons: renders the scene a second time (mitigate: small texture, spawn
    only while focused, optional `RenderLayers`/far-plane trim); must
    verify no interaction with the 0.19 blackout, `PostProcessingCamera`
    and the skybox (de-risk step 1); inset camera framing/orbit is a feel
    knob to tune.
  - Unknowns: RTT + this project's post-processing stack; perf on WASM.
- **B. Schematic panel (hologram-style 2D diagram).** No second camera:
  project each section's local position/AABB onto a plane in the ship's
  local frame and draw one bevy_ui node per section in a panel - an
  FTL/Elite-subsystems-style diagram, optionally rotated to match the
  ship's apparent orientation.
  - Pros: cheap to render; readable at any range; pure bevy_ui, all
    existing tech.
  - Cons: bespoke projection/layout code that must handle arbitrary
    modded ship shapes; loses the "live view of the real ship" feel the
    request asks for; a cuboid diagram of cuboid ships adds little over
    the markers it replaces.
- **C. Focus-zoom on the main camera.** A held "zoom" input that narrows
  the main camera FOV toward the lock (sniper-scope style). Zero new
  render tech; existing markers scale up via ApparentSize.
  - Pros: trivial; no second view to maintain.
  - Cons: not a minimap - the player loses all situational awareness while
    zoomed, which fights the dogfight loop; does not satisfy "inset" at
    all. Worth having someday as a cheap complement, not as the answer.
- **D. Do nothing.** Snap + cycle already select sections correctly; the
  markers show membership. Deferring costs nothing structurally - the inset
  is additive. But at range the mechanic is blind (you cycle markers on a
  blob), and the fine-lock depth the VATS arc built stays illegible.

## Recommendation

Option A, as a backlog item (post-v0.4.0). It is the only option that
delivers what was actually asked for - a close-up of the targeted ship -
and the codebase is unusually well set up for it: the inset consumes
existing lock/focus/fine-lock state, frames on the existing
live-structure-anchor helper, and the selected-section highlight lands
in-scene where both views (and the moment-to-moment game) benefit.

Shape it in two phases so the risky part is small and first:

1. **View-only inset.** De-risk RTT against the 0.19
   blackout/post-processing/skybox first (a spike-sized probe inside the
   task). Then: RTT camera spawned/despawned with the focused lock (the
   hud/mod.rs observer pattern), framed on the locked ship, ImageNode panel
   in a corner, in-scene emissive highlight on the fine-locked section.
   Selection still happens via the existing snap/cycle - the inset only
   makes it visible.
2. **Direct picking (optional, separate decision later).** Cursor-release
   mode + UV-to-ray raycast into the inset to click sections. Real input
   redesign; do not commit until phase 1 proves the inset earns its screen
   space.

Rejected: B duplicates in bespoke 2D what the engine renders for free in 3D
and loses the requested feel; C solves a different problem (magnification,
not an inset); D leaves the shipped fine-lock mechanic hard to read at
range.

## Resolved (task 20260710-104421, 2026-07-12)

- RTT coexistence: CONFIRMED. `RenderTarget::Image` (a standalone component on
  Bevy 0.19, not a `Camera { target }` field) coexists with the main camera's
  `PostProcessingCamera` and skybox with no blackout and no crash - both are
  per-camera components, and every camera query in the codebase is
  marker-filtered so an unmarked second camera trips no `Single<Camera>`.
  Option A shipped as planned (Option B not needed). Details in
  tasks/20260710-104421/NOTES.md. WASM/WebGL2 is standard RTT territory
  but was not profiled in-browser here (follow-up if it ever feels heavy).

## Open questions

- ~~Does `RenderTarget::Image` coexist with `PostProcessingCamera` and the
  skybox on Bevy 0.19 in this app, and does the WASM build handle it?~~
  RESOLVED above (desktop confirmed; WASM unmeasured but expected to work).
- Inset camera pose: fixed offset in the target's local frame (stable,
  hologram-like) vs player-relative bearing (matches what you see) vs slow
  orbit (shows all faces). Feel knob; decide at plan time, keep it a
  constant.
- Panel placement/size vs the existing HUD (readout column, focus meter)
  and whether the inset replaces or coexists with the on-ship section
  markers while focused.
- Whether the in-scene selected-section highlight (emissive/outline) should
  land earlier than the inset as a standalone juice task - it has value
  without the panel.
- Phase 2 picking: is a cursor-release mode acceptable mid-combat, or is
  cycle-with-legibility enough? Defer until phase 1 is playable.

## Next steps

Direction-level task this spike seeded, for /plan to break into steps when
it is picked up (parked at priority 0 per the roadmap convention for
post-sprint work, spike 20260708-203517):

- tatr 20260710-104421: target inset view - render-to-texture close-up
  panel of the locked ship (phase 1: view-only + in-scene selection
  highlight)
