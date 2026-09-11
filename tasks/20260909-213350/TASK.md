# HUD, camera and map sizes derive from the hull and the target

- STATUS: CLOSED
- PRIORITY: 84
- TAGS: v0.14.0, bug, hud, camera, review

## Goal

Every HUD, camera, map, and debug size that is fixed in world units or
pixels but really depends on the hull, the target, or the scene derives from
that thing. Owner (2026-09-09): the velocity sphere "needs to scale with the
ship and inscribe around it; probably there are more cases similar to this
which are easy to identify if we look at all hardcoded values."

The velocity and gravity spheres are `20260909-212917`. This task is the
rest of the presentation layer, from the 2026-09-09 sweep of `nova_hud`,
`nova_ship::camera`, `nova_os_ui`, `nova_debug`, `nova_ui`, `nova_menu`.
Sizes to derive from: `HullEnvelopeRadius` (own visible hull), the target's projected
apparent radius (`screen_indicator::indicator_size` already computes it
from the collider AABB union), `BodyRadius` (beacons, bodies), or a live
`ComputedNode` size. Author margins in meters through `nova_events`.

## Implementation decisions

- Depend on `20260909-212917` and use its `HullEnvelopeRadius` whenever a
  presentation element or camera must clear the visible hull. `HullRadius`
  remains the structural arm and is not the containment contract.
- Preserve the current skiff camera compositions. Normal remains the standard
  chase view. Turret/combat remains closer and elevated with its focus ahead so
  the hull leaves the combat area clear. FreeLook/alternate remains the wider
  view. Grow each rig only when the live envelope plus the 5 m visual hull
  clearance requires it.
- Derive burn push from live forward main-drive acceleration: aligned authored
  thruster force divided by ship mass, gated by live thruster input. Scale it
  with the current rig distance and calibrate against the current skiff so its
  full-burn push remains approximately 30 m. Do not let gravity move the rig.
- Size the flip gate's major radius from the hull envelope plus visual
  clearance. Keep its tube thickness an indicator-sized authored value.
- Place an anchored widget at `projected target radius + visual gap + widget
  half-size`. Calibrate each fixed gap to preserve the current skiff/small-target
  composition rather than inventing a new baseline.
- Dynamically cluster overlapping ammo gauges of the same kind in screen space.
  Keep individual gauges while they fit. A cluster shows its mount count and
  the lowest member ammo fraction, then separates again when screen space
  permits.
- Plot map bodies at their projected physical radius. Give them a separate
  minimum-size interaction target and outline, so planets read to scale while
  small contacts remain selectable.
- The ship inspector has no `Special` section category. Keep a clickable dot
  for every section, retain the existing class glyphs, and show a text label
  only for the selected section.
- Missing presentation data hides the entity but does not disable the systems
  that measure it and make it ready. Hidden UI is not pickable. Any handler
  that can produce an action must also validate semantic readiness; do not use
  generic ECS `Disabled`, which could prevent layout or readiness processing.
- Debug barrel lines use authored weapon reach. Projectile lines use velocity
  over one authored time interval. Thruster lines use authored force times
  live input. Marker radii derive from their collider or a small metric visual
  clearance.

## Camera (player-visible first)

- [x] `crates/nova_ship/src/camera/framing.rs:131-141` `mode_camera_rig`:
      chase offsets fixed at (0,5,-20) Normal, (0,10,-30) FreeLook, (0,5,-10)
      Turret world units. `block_carrier` has a hull reach of ~17 u, so Turret
      mode parks the camera INSIDE the hull. Derive from `HullEnvelopeRadius`
      plus an authored margin.
- [x] `crates/nova_ship/src/camera/chase.rs:92` `ChaseCamera::default()`
      repeats the fixed offset; a respawned camera wears it for its first
      frame, so a respawn in a big hull opens from inside the ship.
- [x] `framing.rs:126,160-165` `SURVEY_MAX_DISTANCE` 250 u caps the orbit
      survey dolly below a 400 u ring; include `HullEnvelopeRadius` in both
      bounds.
- [x] `framing.rs:116,235` `BURN_PUSH_DISTANCE` 3 u: imperceptible on the
      carrier, a lurch on a skiff. Make it a fraction of the rig distance.

## World-anchored HUD

- [x] `crates/nova_hud/src/holo_instruments.rs:31` `GATE_RADIUS` 4 u torus,
      built once into `HoloAssets::gate_mesh`. The carrier is 11 cells across
      the shoulders and swallows the flip gate. Size per ship from
      `HullEnvelopeRadius`; rebuild or scale the mesh.
- [x] `allegiance_markers.rs:85` `MARKER_OFFSET` -40 px: the triangle meant to
      float above the hull sits amidships on a big target. Use the target's
      projected radius.
- [x] `objective_markers.rs:28` `CHIP_OFFSET` -36 px, and
      `beacon_chips.rs:26` `CHIP_OFFSET` -28 px: labels land on the mesh of a
      planetoid, the carrier, or a 50 m beacon (`BodyRadius` published at
      `nova_scenario/src/objects/beacon.rs:86`, never read).
- [x] `keybind_dock.rs:62` `CUE_OFFSET` 48 px: a verb cue inside a big
      silhouette reads as a lock marker.
- [x] `lock_dwell_ring.rs:29` `RING_PX` 39.2 px around a reticle that grows
      with the target: a 39 px dot floats inside a 300 px reticle. Track the
      reticle node's live size.
- [x] `lock_crosshairs.rs:72-73` `GHOST_TRAVEL_PX` / `GHOST_COMBAT_PX`: the
      unlatch ghost pops at a fixed size off a crosshair that is
      `ApparentSize`. Stamp from the crosshair node at unlatch.
- [x] `component_lock.rs:22,25` `MARKER_PX` 10 / 16 px, one per section, no
      cap: locking the carrier paints a solid slab and hides the fine-lock
      highlight. Size per section extent and declutter by count.
- [x] `ammo_readout.rs:244` per-mount gauge offset 16.8 px with a 28 px ring:
      ten or more mounts pile up at range. Scale or group.
- [x] `flight_status.rs:51,55` `SPEED_CHIP_OFFSET` / `MODE_CHIP_OFFSET` are
      justified by the sphere radius and the chase distance, both of which
      become hull-dependent. Re-derive with `20260909-212917`.
- [x] `bore_sight.rs:89,267` `MAX_TRACE_LAYERS` 24 while the round is
      uncapped: the sight under-reports a lance down the carrier's 33-cell
      spine. Bound by the same power rule the round uses.

## Layout that measures another widget by hand

- [x] `objective_stack.rs:81,198` `STACK_TOP_PX` 96 px "one two-line readout":
      a mod with two `HudReadout` slots stacks chips on the run clock.
      Measure the strip's live height.
- [x] `target_inset.rs:77` `INSET_TOP_PX` 44 px hand-read from
      `nova_ui/src/status_bar.rs:47,227`. Measure the node.
- [x] `maneuver_instruments.rs:38` `READOUT_OFFSET` 28 px hand-matched to
      `DESTINATION_MARKER_PX` 24 in `flight_status.rs:33`. Derive.
- [x] `cinematic_prompt.rs:90-91` fixed 180 px pill with a -90 px margin
      around a runtime glyph label: a long binding label overflows. Content-
      size and centre with a transform.

## NOVA OS

- [x] `crates/nova_os_ui/src/map/mod.rs:63-68` `MAP_RING_RADII`,
      `MAP_RADIUS_DEFAULT` 170 u, `MAP_RADIUS_MAX` 520 u: a contact 20 km out
      can never be reached by zoom. Derive from the live contact spread that
      `MapContacts::collect` already has.
- [x] `map/scene.rs:484,84` `MAP_BLIP_PX` 12 px for every contact and a fixed
      16 m focus hub: a planetoid, the carrier and a torpedo plot identically.
      Size from `BodyRadius` / `HullEnvelopeRadius`.
- [x] `ship/scene.rs:602,712,653` one 12 px blip plus a label per live
      section: the carrier's inspector is a thousand overlapping labels.
      Count-aware declutter: label the selection and the specials.

## Debug gizmos

- [x] `crates/nova_debug/src/sections.rs:36,46` `DEBUG_LINE_LENGTH` 100 u per
      turret barrel: derive from the mount's authored reach.
- [x] `sections.rs:63,85-86,96,114` fixed 0.2 u marker, 2 u stubs, and a
      `RoundVelocity` normalised to 1 u: a railgun slug and a PDC round draw
      the same stub. Scale with speed.

## Judged correct, leave alone

Target inset framing (collider AABB union), turret lead and bore reach
(read the section), `ApparentSize` floors, the ship schematic's opening
framing (`extent * 2.6`), map pan speed, menu backdrop camera (authored
`SetCamera`), gravity SOI gizmos, harness deadlines, `RADAR_BOX_PX`, the
settings `CHIP_WIDTH`.

## Proof

- Extend the sphere range from `20260909-212917` (skiff and carrier as the
  player hull) with an invariant per camera mode: the camera sits outside
  `HullEnvelopeRadius` plus the margin. Add an invariant for each world-anchored
  chip: its screen offset exceeds the target's projected radius.
- `screenshot_combat_hud`, `screenshot_nova_os_apps` and the inset loop re-
  shot on the carrier; the wiki stills updated where they change.
- The status bar and readout layout ranges (`system_ui_scale`,
  `system_hud_indicators`) extended for the measured-not-assumed layouts.

## Verification

- Run the affected `nova_ship`, `nova_hud`, `nova_os_ui`, `nova_ui` and
  `nova_debug` unit tests.
- Run `system_hud_shell` live on both hulls and read every per-mode and
  per-chip verdict line.
- Run `system_hud_indicators` and `system_ui_scale` live and read the measured
  layout numbers at 1x and 2x.
- Re-shoot the shipped stills through `scripts/capture-web-shots.sh` and
  inspect the HUD and NOVA OS frames at native resolution against the shipped
  ones.

## What the audit found that the sweep did not list

- `flight_status.rs:51,55` needed no change. `20260909-212917` had already
  re-derived both chip offsets from the eased hull envelope, which is what this
  entry asked for.
- There is no inset LOOP. `scripts/capture-web-media.sh` names no producer that
  shoots the target inset; the inset frame is the `inset_shot.png` beat inside
  `system_hud_indicators`, which is re-shot on every run of that range.

## Two defects the re-shoot found (2026-09-11)

Both were introduced by this task's own first pass and are fixed in it.

- Component-lock markers scaled at half the section's bounding SPHERE cover
  0.87 of the section's WIDTH, because a sphere around a box is 1.73 times as
  wide as the box. Adjacent markers met, and the HUD still on a hauler at 340 m
  came back with two solid red blocks where the ship was - the same slab the
  entry existed to remove, moved from the carrier onto a small hull. The
  unselected scale is now a fifth of the sphere, about a third of the section,
  with a test that fails any scale that can tile.
- The NOVA OS inspector's class glyph does not read at dot size. An 8 px glyph
  inside a 12 px dot came back as a blank dot through the CRT pass, so the
  schematic no longer said which block was a thruster. The glyph is back in the
  code pill, which now appears only on the selection, so the declutter and the
  kind both survive.

## Result (2026-09-11)

Done. Every entry on the sweep is ticked and every proof line is met.

Sixteen commits, `d58750b4b` through `90705cad6`.

Ranges, live under Xvfb on the RTX 3060 Ti:

- `system_hud_shell`, both hulls. Carrier: FreeLook 315.9 m, Turret 199.6 m and
  Normal 205.8 m out against a 194.3 m envelope; chips pushed 111 / 123 / 120 px
  clearing their silhouettes by 64 / 66 / 12 px. Skiff: 315.9 / 112.1 / 205.8 m
  against 48.3 m; chips 116 / 128 / 125 px clearing by 67 / 69 / 12 px. Before
  this task the Turret rig stood at a fixed 10 u - 100 m - which is inside the
  carrier.
- `system_hud_indicators`: the objective stack measures 172 px under a 142 px
  three-slot readout strip, and the target inset 46 px under a 42 px status
  bar. Both gaps identical at a doubled scale factor. The stack was authored at
  a fixed 96 px, which a three-slot strip reaches past.
- `system_ui_scale`: the status bar's live bottom edge reads 42 px at 1024x768,
  at 2x, at 1280x600 and at 760x600 - the same number the in-flight inset hangs
  from, and the same one `system_hud_indicators` measures in flight.

Unit tests, the touched crates: `nova_hud` 288, `nova_ship` 890, `nova_os_ui`
123, `nova_ui` 59, `nova_debug` 23. All pass.

`cargo fmt --all --check` and `cargo check --examples --features debug`: clean.
The workspace test and Clippy sweeps are left to CI, per the standing
instruction.

Stills, through `scripts/capture-web-shots.sh` (base-only mods, clean config):

- `wiki-hud.png` re-shot. The component-lock markers read as an overlay on the
  target instead of the two solid blocks the first pass produced.
- `wiki-nova-os-map.png` re-shot. Contacts plot at their own size and the rings
  range from the live contact spread.
- `wiki-nova-os-ship.png` re-shot. One code pill, on the selection.
- Every other shipped still was re-captured in the same sweep and restored: the
  frames differ run to run (fights resolve differently, counters move), and
  nothing this task changed is in them. They refresh on the next full capture
  pass like always.
- `web/src/wiki/nova-os.md` corrected while it was in front of me: it still
  described a per-blip integrity bar and ammo pips, which moved into the
  inspector several releases ago.
