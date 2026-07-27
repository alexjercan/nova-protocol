# Decision: NOVA OS `map` app - concrete render + interaction shape

- TASK: 20260724-102320
- STATUS: ACCEPTED (via owner-delegated out-of-context plan review; see Flow note)
- DATE: 20260727

## Context

The `map` app is the terminal-launched 3D minimap. The owner's post-PoC
direction is explicit and supersedes the flat 2D radar plane shown in
`examples/ui/nova_os_terminal_poc.html`:

- a SOLAR SYSTEM style view you can actually MOVE AROUND in (WASD + an orbit
  camera you drive with the mouse: orbit/pan/zoom), not a static top-down plot;
- areas of interest / contacts you can CLICK (or keyboard-cycle) to inspect
  details (kind / name / range / bearing / note);
- the ability to set a GOTO nav target from the map that the flight autopilot
  then flies to after you close the computer (works "on the outside of the ship
  too");
- a CLI counterpart: `map` launches the visual app, `map view` prints a text
  contact list inline in the terminal.

Two load-bearing constraints were discovered in research and force the shape
below (they are not free-choice forks, they are what the runtime allows):

1. The `NovaOsAppRuntime` trait only delivers discrete `handle_key` events
   (Escape + Ctrl are stripped by the runtime) and NO mouse input. An
   interactive orbit/pan/zoom + click map therefore cannot be driven through
   the trait. It must run its OWN Bevy systems gated on
   `TerminalMode::App { id: "map" }`.
2. The whole NOVA OS screen is rendered to an offscreen image and composited
   through the `nova_os_crt.wgsl` CRT shader. `forward_nova_os_pointer` /
   `mirror_nova_os_hover` already forward the real cursor onto that RTT so UI
   buttons inside the screen stay clickable, but a 3D mesh inside a SECOND
   nested Camera3d RTT is NOT pickable that way (it would need a manual raycast
   unproject through two RTT hops plus the barrel-warp).

## Decision 1: hybrid render - 3D schematic scene + projected UI blips

Render mode is the older spike's option C2 (schematic orrery), split into two
layers that each play to their strength:

- A small dedicated 3D scene on its own `RenderLayers(MAP_LAYER)` rendered by a
  `Camera3d` to an offscreen image (cloning the proven NOVA OS RTT setup at
  `crates/nova_gameplay/src/hud/nova_os.rs` `Image::new_target_texture` +
  `RenderTarget::Image` + `RenderLayers`). This scene is the "solar system"
  frame you move around in: concentric orbit rings / a ground disc grid and a
  central hub marker, giving real parallax and depth as the camera orbits. The
  image is shown as an `ImageNode` filling the app body slot.
- The interactive CONTACTS are UI blip markers (colored dot + label) laid over
  that image, their screen positions recomputed every frame by projecting each
  contact's world position through the map camera into viewport space (the same
  world->screen projection the HUD's allegiance/objective markers already use).
  Blips are real UI `Button` nodes, so they are clickable through the existing
  pointer-forwarding AND keyboard-cyclable, and they carry the allegiance color
  language directly.

Rejected:

- Pure 3D proxy-mesh blips picked by raycast: the double-RTT + CRT-warp makes
  pixel-accurate 3D picking a research project; keyboard-first is the task's
  stated requirement and UI blips satisfy click too. The 3D scene stays as the
  spatial frame; contacts ride on top as UI.
- Real-scene render-to-texture (spike option C1): second full render pass over
  live geometry, culling/scale headaches, rejected by both prior spikes.
- Flat 2D radar (PoC / spike option C3): the owner explicitly wants 3D movement.

The render mode stays a swappable back layer (per both spikes): the contact
model and UI blips do not depend on the 3D scene, so the scene can deepen later
without touching interaction.

## Decision 2: input via app-gated systems, keyboard-first, mouse-augmented

Interaction runs as dedicated systems gated on the map app being active:

- Orbit: mouse drag (a held button + `MouseMotion`) rotates a `SphereOrbit`
  (bevy-common-systems `transform/sphere_orbit.rs`: theta/phi/radius/center with
  smoothing) whose output drives the map `Camera3d`. This is the "orbit camera
  we have".
- Zoom: `MouseWheel` adjusts `SphereOrbit.radius` (also +/- keys).
- Pan / move around: WASD moves `SphereOrbit.center` across the map plane.
- Select: keyboard cycle (`[` / `]` or arrows) is the primary, robust path;
  clicking a UI blip selects too. Selection fills the contact readout
  (KIND / NAME - range X, bearing Y. note) with the amber-highlight treatment.
- Reset view: `R`.
- Escape / chrome close still exit to the terminal (owned by the runtime).

Global mouse/keyboard reads are gated behind
`terminal.active_mode() == TerminalMode::App { id: "map" }` so they never fire
while the terminal prompt or another app owns the screen.

## Decision 3: GOTO sets flight autopilot on the player ship

Setting GOTO on the selected contact inserts
`Autopilot::engage(AutopilotAction::Goto { target })` (or `GotoPos { position }`
for a fixed area-of-interest) on the player ship entity - the same component the
flight autopilot already consumes (`crates/nova_gameplay/src/.../flight.rs`).
The GOTO persists after the computer closes, so it "works on the outside of the
ship too". Clearing reuses the existing autopilot disengage path.

## Decision 4: `map` = app, `map view` = CLI text (swappable naming)

`map` launches the visual app (app registry). `map view` is a built-in text
command (same family as `log` / `objectives` / `ship`, task 20260726-115330)
that prints the contact list inline. Both read the SAME contact-collection
function, so the CLI and the app never drift. The resolver already prefers the
longest matching name (`ship` builtin vs `ship view` app), so `map view` as a
2-word builtin resolves ahead of `map` app + stray arg.

NAMING is a soft choice, called out for the in-game review: if the owner would
rather `map view` open the visual view and bare `map` print the list, it is a
one-line swap - the two behaviors and the shared model do not change.

## Scope: this is a PoC for in-game review

Per the owner's instruction, the plan gate is delegated to an out-of-context
plan review loop (in place of the usual "yes, build this"), then the PoC is
implemented directly for the owner to review from the running game. The PoC
delivers the vertical slice above; polish (CRT integration niceties, richer
markers, route planning, map-boundary gameplay) stays deferred per the task's
LATER section.
