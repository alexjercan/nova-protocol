# NOVA OS map app: 3D minimap launched from the terminal - v0.9.0 STRETCH

- PRIORITY: 30
- TAGS: v0.9.0, stretch, spike, feature, ui, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Goal

Build the `map` app for the one-screen NOVA OS drawer. The 3D minimap is still a
v0.9.0 stretch item, but it no longer lives in a permanent center drawer panel
between left logs and right objectives. Post-feedback direction is one inset
cockpit monitor: terminal commands either print inline output or launch an app
that swallows the same monitor until exited. This task owns the `map` app that
opens from the terminal command.

v0.9.0 STRETCH, still LAST in Strand C and cut first if the terminal OS core runs
long. The original minimap design came from
`tasks/20260721-211512/SPIKE.md` option C; the current direction is superseded by
`tasks/20260725-104330/SPIKE.md` and the visual PoC at
`examples/ui/nova_os_terminal_poc.html`.

Scope THIS SPRINT (direction-level; /plan breaks into steps at pickup):

- Add a `map` command in NOVA OS that launches a map app, replacing terminal
  scrollback inside the same monitor until the app is closed.
- Show a downsized 3D or schematic map view of the local game space. A small
  dedicated camera/render-to-texture path is acceptable if it proves clean in
  Bevy 0.19; a schematic proxy view is acceptable if the real render path runs
  long.
- Include placeholder markers for map contents: player ship, allies, enemies,
  asteroids and objective/area-of-interest markers. Simple proxy meshes/blips at
  scaled world positions are fine.
- Give the map app its own input ownership while active. WASD or similar camera
  controls belong to the app, not to the terminal prompt.
- Provide a way back to the terminal that matches the NOVA OS app runtime (for
  example app chrome close plus the chosen keyboard chord), without making Tab
  close the drawer from inside the terminal.

LATER (out of scope this sprint, captured for the reader): zoom levels, panning
to plan flights, richer marker filters, route planning, map-boundary gameplay and
ship commands that act from the map. The render mode stays a swappable back layer
so a 2D top-down plot is a valid interim if the 3D view runs long.

## PoC-derived requirements (2026-07-26 fidelity review)

The in-game map will look different from the PoC mock, but these interaction
patterns from `examples/ui/nova_os_terminal_poc.html` carry over and should be
in the plan at pickup:

- Contact inspection: selecting a contact (click, or a keyboard cycle) fills a
  readout with kind / name / range / bearing plus a one-line flavor note (the
  PoC `.contact-readout` + `contacts` table). Range/bearing come from live
  world positions relative to the player ship.
- Contact semantics + color language: OWN SHIP phosphor, HOSTILE red with a
  pulse, ally/objective/terrain in the PoC's blue/amber/plain treatment -
  consistent with the allegiance-marker palette from 20260723-233446.
- App chrome parity: the appbar shows `APP / MAP / LOCAL SPACE`, the LATEST
  flight-log line (the PoC `.applast` - the log stays glanceable while an app
  covers the terminal), and the Close App control the runtime already ships.
- Keyboard-first: the app is fully usable without the mouse mid-flight; footer
  hints swap to the map's hint set while it is active (hint plumbing lands in
  20260726-214708).
- Launch degauss/transition + map-open sound land via 20260726-193233 /
  20260726-214639; do not block on them.

## Notes

- Original spike: `tasks/20260721-211512/SPIKE.md` (RECOMMENDED) captured the
  minimap options and recommended the schematic/proxy approach over rendering
  the real scene as the safer first path.
- Superseding feedback spike: `tasks/20260725-104330/SPIKE.md` changes the shell
  model from multiple drawer panels to one NOVA OS monitor. This task should
  plan against that newer spike.
- Depends on the NOVA OS app runtime task `20260726-115334`; do not implement
  the map as a separate permanent drawer panel.
- Visual reference: `examples/ui/nova_os_terminal_poc.html` shows the intended
  app takeover behavior with mocked map data. Treat it as a design target, not
  production code.

## Understanding (2026-07-27)

Verified against the current tree:

- The app runtime this plugs into has LANDED (`20260726-115334`): apps implement
  `NovaOsAppRuntime` (`crates/nova_os/src/app.rs:54`) and register into
  `NovaOsAppRegistry`; the drawer owns chrome, the `TerminalMode::App` swap,
  input ownership, uniform Escape/close, and footer-hint swap. No `map` stub
  exists yet (`crates/nova_os/src/shell.rs:238` keeps `map` deferred).
- CRITICAL: the trait only hands the app discrete `handle_key` events (Escape +
  Ctrl stripped) and NO mouse. Interactive orbit/pan/zoom/click must run as the
  map's OWN systems gated on `TerminalMode::App { id: "map" }`.
- The NOVA OS screen is itself an offscreen image composited through
  `nova_os_crt.wgsl`; `forward_nova_os_pointer` keeps UI buttons inside it
  clickable, so UI blip markers are pickable but a nested 3D mesh is not.
- Reusable pieces: RTT setup (`nova_os.rs` `Image::new_target_texture` +
  `RenderTarget::Image` + `RenderLayers`); orbit camera
  (`bevy-common-systems/src/transform/sphere_orbit.rs`); allegiance palette
  (`nova_ui::theme::semantic` + `hud/allegiance_markers.rs:83 allegiance_color`);
  contacts (`SpaceshipRootMarker`, `PlayerSpaceshipMarker`, `AISpaceshipMarker`,
  `Allegiance{Player,Enemy,Neutral}`, `ObjectiveMarkerTarget`); GOTO
  (`Autopilot::engage(AutopilotAction::Goto{target}|GotoPos{position})`).
- World scale ~50-100 units across a scenario; asteroids/objectives 40-60 units
  from the player.

See `DECISION.md` for the resolved artifact shape (hybrid 3D scene + projected
UI blips; app-gated input; GOTO via autopilot; `map` app + `map view` CLI).

## Approach

A PoC vertical slice living in `nova_gameplay` (a new `hud/nova_os_map.rs`
module beside `nova_os.rs`), registered into `NovaOsAppRegistry` at plugin
build. One shared contact model feeds both the `map view` CLI text and the
visual app. The app spawns a Camera3d RTT schematic scene (orbit rings + hub +
ground grid) driven by a `SphereOrbit` camera, with contacts drawn as projected
clickable UI blips over it, a selection readout, and a GOTO action that sets
flight autopilot on the player ship.

## Steps

- [x] 1. Contact model: `map_contacts(...)` in `nova_os_map.rs` enumerates
      player / allies / enemies / asteroids / objective markers into a
      `MapContact { entity, kind, allegiance, label, world_pos, range, bearing,
      note }`, computing range and bearing relative to the player ship. Shared
      by the CLI and the app. Unit/harness test with a scripted scene.
- [x] 2. `map view` CLI: this spans BOTH crates (reviewer R1.5). In `nova_os`
      add `"map view"` to `TERMINAL_COMMANDS` (`shell.rs:53`) and a `map_rows`
      field to `TerminalCommandSnapshot` with a `"map view" =>` arm in
      `terminal.rs:~349`; in `nova_gameplay` populate `map_rows` from the shared
      `map_contacts()` inside `terminal_snapshot_from_world` (`nova_os.rs:~1005`).
      Prints KIND / NAME - range / bearing / note. Test the resolver: `map` ->
      App, `map view` -> built-in (mirrors the existing `ship`/`ship view` test);
      test the rendered lines.
- [x] 3. `MapApp: NovaOsAppRuntime` (id `map`, title `MAP`, summary, `hints`
      WASD/orbit/zoom/select/GOTO/ESC, arity None). Register at plugin build.
      NOTE (reviewer R1.1): `spawn_body(&self, body, font)` has NO world/Commands/
      Assets access, so it only spawns the static UI shell - a `MapViewportMarker`
      `ImageNode` (patched with the RTT handle by a follow-up system) + a
      `MapReadoutMarker` line. The Camera3d, RTT image and 3D scene are created by
      the transition system in Step 4, not here. Lifecycle test (open/close
      spawns/despawns the scene, terminal scrollback intact).
- [x] 4. Schematic 3D scene + RTT via an app-transition system (reviewer R1.7):
      a system gated by comparing `terminal.active_mode()` (public,
      `terminal.rs:192`) to a `Local`. On enter-map: create
      `Image::new_target_texture` sized 1:1 to the viewport node's computed px
      (copy the resize logic from `reconcile_nova_os_target`), store it in a
      `MapRtt { image }` resource, patch the `MapViewportMarker` `ImageNode`,
      spawn a `Camera3d` (`is_active` on) with `RenderTarget::Image` +
      `RenderLayers(MAP_LAYER)`, and spawn orbit-ring / ground-grid / central-hub
      proxy meshes on `MAP_LAYER` using UNLIT/emissive materials (no light setup
      needed). On exit-map: deactivate the camera and despawn the scene + blips.
      Headless-guarded on `Option<ResMut<Assets<Image>>>` like the nova_os RTT.
- [x] 5. App-gated camera controls: reuse the already-registered
      `SphereOrbitPlugin` (`plugin.rs:96`). A glue system reads
      `SphereOrbitOutput` (a bare position Vec3) and sets the map Camera3d
      `Transform::translation` + `.looking_at(orbit.center, Y)` (reviewer R1.3 -
      the plugin does not move a camera itself). Input systems gated on
      `TerminalMode::App{"map"}` write `SphereOrbitInput` theta/phi (mouse drag
      orbit), `SphereOrbit.radius` (`MouseWheel` zoom), `SphereOrbit.center` (WASD
      pan), and `R` (reset). Never fire outside the map app.
- [x] 6. Projected UI blips - BESPOKE two-space projection (reviewer R1.6, do
      NOT copy `screen_indicator`, which targets the main camera+window). Each
      frame: `map_camera.world_to_viewport(world_pos)` -> map-RTT texture px ->
      remap into the `MapViewportMarker` ImageNode's computed local rect (a direct
      scale because the RTT is kept 1:1 to the node px in Step 4). Position a UI
      blip (allegiance color via `allegiance_color`, enemy pulse, amber objective)
      + label. Blips are `Button`s (clickable through the pointer-forwarding) and
      keyboard-cyclable; clean them up on close.
- [x] 7. Selection + readout: clicking a blip or cycling (`[`/`]`) selects a
      contact and fills the readout `KIND / NAME - range X, bearing Y. note`
      with the picked/amber treatment.
- [x] 8. GOTO: a key (`G`) / on-screen action on the selected contact inserts
      `Autopilot::engage(Goto{target}|GotoPos{pos})` on the `PlayerSpaceshipMarker`
      entity; shows "GOTO SET" feedback; persists after the computer closes. Add a
      one-line comment that this intentionally bypasses the `FlightVerb::Goto`
      grant check (reviewer R1.4, fine for a PoC). Harness test that the component
      lands on the player ship.
- [x] 9. Verify + hand off: `cargo check`; run the new tests; capture
      screenshots of the map app (scene + blips + readout) and `map view`
      output; `nova_probe` a run to confirm no regressions. Present for the
      owner's in-game review.

## Definition of Done

1. `cargo check` is clean. (cmd: `cargo check`)
2. Contact model + range/bearing proven by a harness test on a scripted scene.
   (test: `cargo test -p nova_gameplay map_contacts`)
3. Resolver proves `map` -> App and `map view` -> built-in; `map view` renders
   the contact lines. (test: `cargo test -p nova_os map` and the render test)
4. App lifecycle: opening `map` swaps the terminal for the app and closing it
   (Escape / chrome) restores the prompt with scrollback intact. (test:
   `cargo test -p nova_gameplay nova_os_map`)
5. GOTO inserts an `Autopilot` on the player ship from a map selection. (test:
   the GOTO harness test)
6. In-game (owner review): `map` opens a 3D schematic view you orbit with the
   mouse, pan with WASD, and zoom with the wheel; contacts show in allegiance
   colors with labels; click or cycle selects and fills the readout; `G` sets a
   GOTO the ship then flies; `map view` prints the CLI list; Escape returns to
   the terminal. Screenshots attached. (manual: owner runs the game)

Gate note: the owner explicitly delegated the plan gate to an out-of-context
plan-review loop ("review it instead of my input 'yes, build this' and loop
until happy") rather than a manual approval. Reviewer R1 (out-of-context)
returned APPROVE after verifying every load-bearing claim against the tree; its
non-blocking corrections are folded into Steps 2-8. The owner's real acceptance
checkpoint is the in-game PoC review (DoD #6).

## PoC status (2026-07-27)

Steps 1-9 IMPLEMENTED on branch `feature/nova-os-map` (not yet landed - the
owner reviews in-game first, per their instruction). Automated DoD 1-5 GREEN:
`cargo check` + `cargo fmt` clean; `cargo test -p nova_os` (11) and
`cargo test -p nova_gameplay --lib nova_os_map` (5) pass - contact model +
range/bearing, `map view` rows + empty state, resolver (`map` app vs `map view`
builtin), app lifecycle, headless RTT scene build/drive/teardown, and GOTO
insertion.

DoD 6 (in-game visual) is OUTSTANDING and is the owner's review: this sandbox's
GPU segfaults on shader compile (`NVVM compilation failed`) at Playing, so no
real-pixel screenshot could be captured here. Run to review:

```
BCS_AUTOPILOT=1 BCS_REEL=1 NOVA_SHOT_DIR=target/reel \
  cargo run --example screenshot_nova_os --features debug   # scripted: opens map, shots
# or, live, in a populated scenario (enemies/objectives/asteroids):
cargo run --example playable            # Tab -> type `map` (or `map view`)
```

Open items for the in-game pass: blip/label placement + scale against the RTT,
orbit/zoom feel, ring framing, whether `map`/`map view` naming should swap.
