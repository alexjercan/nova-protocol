# Add an animated docking section and baseline break-on-move joints

- STATUS: DONE
- PRIORITY: 0
- TAGS: v0.14.0, feature, docking, ship, spike, rendering

## Goal

Add the first useful docking mechanic in two phases:

1. Run a visual spike for a new animated docking-section mesh and select one
   candidate from screenshots.
2. Implement baseline explicit docking as a temporary fixed joint that releases
   when either ship starts moving again.

Keep this version deliberately small. It proves the section, animation, joint,
interaction, and disengagement. It does not solve station services or flight
control of a docked assembly.

Coordinate the new section types with task `20260915-161020`, so docking enters
the accepted ship-design and typed-patch model rather than the section
modification model being removed.

## Accepted baseline

- Docking requires an explicit contextual `DOCK` command.
- The currently locked ship is the docking target.
- The game automatically chooses the nearest eligible free port pair.
- Stable section IDs break candidate ties deterministically.
- Each docking port supports one connection.
- Ports are cylindrical and rotationally symmetric.
- Port outward axes must oppose each other within a configured angle.
- Roll around the docking axis is ignored during eligibility checks.
- Distance is measured as the gap between the retracted outer faces, not between
  section origins.
- The initial maximum face gap is `1.0` engine unit.
- Relative linear and angular speed must be below configured safe thresholds.
- Accepted docking immediately creates a fixed joint between ship rigid-body
  roots and reserves both ports.
- Physics constrains relative motion. Do not manually zero rigid-body velocity.
- Both tubes then animate to their fully extended state.
- New translation or rotation intent from either ship removes the connection
  before applying that intent and retracts both tubes.
- Destroying either port or ship also cleans up the connection and joint.

Do not transfer authority, union controllers, disable ship systems, or clear the
other ship's commands. Do not add docking permissions or capabilities yet.

## Phase 1: docking mesh spike

Generate several complete docking-section candidates. Put them together in one
registered screenshot example with a fixed camera, lighting, labels, and scale
reference. Show every candidate in both retracted and extended states.

Common constraints:

- retracted section envelope is `1 x 1 x 1`;
- the cylinder points along the section's documented local outward axis;
- extension changes its axial depth from `1.0` to `1.5`;
- the inner end remains anchored while the outer end gains `0.5` reach;
- two facing ports therefore bridge up to a `1.0` face gap;
- extension travel is fixed and never fitted to the actual gap;
- at smaller gaps, the two extended tubes overlap intentionally;
- uniform rotationally symmetric geometry makes overlap read as one tunnel;
- avoid keyed details, exposed end decoration, or seams that reveal overlap;
- the extension is visual only and does not grow a collider.

Produce a small but meaningfully different set, such as plain industrial,
armored-ring, recessed, and skeletal candidates. Do not implement gameplay for
each candidate.

Spike procedure:

1. Build the candidates with the real section rendering and animation path.
2. Register the screenshot example in `Cargo.toml`.
3. Capture matched overview and close-up frames.
4. Inspect the rendered output, not only a headless report.
5. Present the images for owner selection.
6. Keep the selected candidate and remove rejected candidate code/assets before
   completing the implementation phase.

The visual selection is a required stop. Do not choose a candidate on the
owner's behalf.

Phase 1 is done. The owner selected `dock_flush` on 2026-09-15. Phase 2 uses
that mesh; `SPIKE.md` records the candidates, the checked geometry contract and
the captured frames.

## Phase 2: baseline docking logic

Start from this model and adjust names only for concrete crate conventions:

```rust
pub struct DockingSectionConfig {
    pub capture_distance: Meters,
    pub capture_angle: f32,
    pub maximum_relative_speed: MetersPerSecond,
    pub maximum_relative_angular_speed: f32,
}

pub struct DockingConnection {
    pub first_ship: Entity,
    pub first_section: Entity,
    pub second_ship: Entity,
    pub second_section: Entity,
}

pub enum DockingSectionState {
    Retracted,
    Extending,
    Extended,
    Retracting,
}
```

Add `Docking` to `SectionKind`, section spawning, preview rendering, authored
base content, preload/lint, design patch handling, and editor section handling.
Use the selected mesh and existing section-animation architecture where it
fits. Do not add a second generic animation system only for docking.

Candidate selection for `DOCK`:

1. Find free docking sections on the initiating and locked target ships.
2. Reject occupied pairs.
3. Reject pairs beyond face-gap, opposing-axis, and relative-speed thresholds.
4. Rank remaining pairs by face gap, then alignment, then stable section IDs.
5. Expose `DOCK` only while a candidate exists.
6. Revalidate the selected pair when the command executes.
7. Reserve both ports and create one canonical connection and fixed joint.

Create the joint before extension animation, so the ships cannot drift apart
while the tubes extend. The fixed joint preserves the relative roll present at
capture. The visual tubes may overlap without changing collision behavior.

On movement intent or endpoint destruction, remove only the owned connection
and joint, free both ports, and transition surviving sections toward
`Retracted`. Avoid broad cleanup queries that can remove unrelated joints.

## Deferred on purpose

Do not include these in the baseline:

- host, guest, or assembly flight authority;
- combined controller, thruster, mass, or capability calculations;
- station-specific behavior;
- repair, rearm, trade, cargo, crew, or mission services;
- automatic approach or docking autopilot;
- explicit port selection UI;
- male/female or keyed port compatibility;
- roll alignment;
- break force or joint-stress simulation;
- general multi-ship docking graph and cycle policy;
- persistence of pre-authored docked assemblies unless required by the focused
  example.

The existing station-services idea remains separate in task
`20260824-125943`.

## Blast radius

Expected areas:

- `nova_ship`: new section kind, render/animation state, connection ownership,
  fixed-joint lifecycle, movement-intent disengagement, destruction cleanup.
- `nova_scenario`: section serialization, preload/lint, contextual action/event,
  spawn and cleanup paths.
- `nova_authoring`: selected mesh/config builder and regenerated base content.
- `nova_editor`: section preview and basic inspection for the new section kind.
- `nova_hud` or input UI: contextual `DOCK` availability and activation.
- examples and probe catalog: visual candidate gallery and asserted behavior.

This is new and breaking internal content. Update repository content directly.
Do not add compatibility aliases or readers for intermediate section formats.
Edit Rust builders and regenerate generated RON; never hand-edit generated base
content.

## Verification

Visual spike evidence:

- matched screenshots show all candidates at useful scale;
- each candidate is visible retracted and extended;
- the selected mesh remains readable when two fully extended tubes overlap at
  several face gaps below `1.0`;
- selected animation keeps the inner end fixed and moves only outward;
- rendered frames are inspected before selection.

Focused behavior assertions:

- invalid distance, facing direction, relative speed, and occupied ports reject
  `DOCK`;
- roll difference alone does not reject cylindrical ports;
- multiple valid pairs select deterministically;
- one command creates exactly one connection and one root fixed joint;
- docking does not manually zero shared world drift;
- both ports extend after connection;
- translation and rotation intent from either endpoint remove the joint before
  movement applies;
- endpoint destruction cleans up the connection and frees the surviving port;
- disengagement retracts surviving ports;
- visual extension does not change collision shape;
- generated content is stable after a second generation pass.

Use a focused systems example for assertions and the screenshot example for
appearance. Register each example and each `outcome: <slug>` marker as required.
Run affected crate tests, content generation/lint, and those examples only. Do
not claim appearance from headless output.

## Phase 2 record, 2026-09-16

Done. `SPIKE.md` carries the Phase 1 record and the Phase 2 cleanup; this is
what Phase 2 built and what was decided along the way.

### Owner decisions taken during the check

1. The PD controller stands down while a hull is docked, the same way it does
   under ORBIT. `sync_controller_section_forces` excludes a `DockedShip` root,
   so the loop keeps computing and nobody applies it - the two hulls stop
   pushing against the constraint that already holds them.
2. No shipped ship design gets a port. `docking_port_section` is in the editor
   palette and the catalog only.
3. `dock_flush` is shown by the existing all-sections gallery. The dedicated
   docking gallery is deleted.

### The mechanic

- `crates/nova_ship/src/sections/docking_section/`: `mod.rs` the config and the
  sleeve state machine, `port.rs` the candidate search, `connection.rs` the
  connection's life, `render.rs` the mesh.
- `SectionKind::Docking` through spawning, preview, the editor, the typed
  patch, preload and lint, and the authored `docking_port_section`.
- `DOCK` is a FLIGHT action on `K`, keyboard only: every pad button the flight,
  camera and targeting rigs can reach is already spoken for, which is the call
  `ship_repair` made before it. It is gated on the new `dock_enabled` ship
  capability and offered only while `best_candidate` finds a pair.
- The HUD carries an eighth verb chip. Its hot state is `HudSituations.docked`,
  because the offer is retired the instant the joint exists: without that the
  chip would vanish exactly when docking becomes worth reporting.
- One `FixedJoint` with an explicit `with_anchor` / `with_basis` frame, so the
  pose the two hulls met in - roll included - is what the solver holds. Nothing
  on the path writes a velocity.

### Verification

- `crates/nova_ship/src/sections/docking_section/tests.rs`: 15 tests over the
  envelope, the ranking, the reservation, the sleeves and the three releases.
- `examples/systems/system_docking_ports.rs`: 8 `outcome:` markers, green
  through `probe run system_docking_ports --correctness-only` (all eight
  emitted, 0 invariant violations over 151 frames).
- `screenshot_section_gallery` run live under Xvfb and the `docking` row
  INSPECTED: retracted sits flush in the cell, extended shows the sleeve out
  with the inner end anchored.
- `content gen` + `content lint` clean; `gen-section-parts.py --check` reports
  26 parts byte for byte.
- Docs: CHANGELOG, `docs/sections.md`, `docs/concept-index.md`,
  `/create/sections|objects|actions|base-content`, the player wiki
  (`sections`, `sections/docking`, `keybinds`, `hud`, `glossary`). `web npm run
  ci` and `mdbook build` green.

### Deliberately not done

Everything under "Deferred on purpose" stands. Two notes for whoever picks the
next phase up:

- There is no `SetShipCapabilityDock` action: `dock_enabled` is authored at
  spawn only, because the task defers docking permissions. Both creator pages
  say so.
- A docked pair has no shared flight authority, so each pilot still flies their
  own hull - and the first one who does breaks the dock.

## Phase 3 record, 2026-09-16

The owner flew Phase 2 in the editor sandbox and reported three things: the
dock broke almost instantly because mouse movement is intent, `K` is a poor
key, and the approach gives no feedback to fly against. This phase answers all
three. It does NOT add flight authority for a docked assembly - that stays
deferred, with the decaying-station rescue as its motivating case.

### Owner decisions taken during the playtest

1. The `lock` requirement STAYS. You will GOTO a station and then begin the
   dock, so naming the target first is the honest order.
2. A dock is MODAL, like ORBIT: everything is disabled until `DOCK` is pressed
   again. "Fly and you are free" is gone.
3. `DOCK` moves from `K` to `D`.
4. The alignment feedback is a GIZMO, not numbers: parallel plates on the two
   port tips, a perpendicular line between them for angle and distance.
5. Roll stays unassisted. The struggle is the mechanic, and no autopilot is
   coming for it.

### What changed

- **The modal gate.** `DockedShip` on a root is read at the FORCES, not at the
  keys: `thruster_impulse_system` skips the impulse,
  `sync_controller_section_forces` the torque, `manual_burn_system` and
  `rcs_burn_system` exclude the root outright, and
  `update_controller_target_rotation_torque` writes no helm command. A bound
  throttle (`on_thruster_input`) refuses to light a plume it cannot spend. One
  rule covers pilot, AI and scripted order at once.
- **`release_docking_connections_on_intent` is gone.** Its replacement is
  `park_docked_helms_and_release_maneuvers`, which re-parks each docked hull's
  attitude command on the rotation it actually HAS every tick, so the helm a
  released ship inherits is the direction it is already pointing. The same pass
  drops a pair where either hull has engaged an autopilot, before the maneuver
  is flown.
- **`DockingReleaseRequest`** is addressed to a SHIP, because that is what the
  pilot pressing `DOCK` again knows about. `on_docking_release_request` is
  ungated: a capability withdrawn mid-dock must never strand a clamped hull.
- **The docking sight** (`crates/nova_hud/src/docking_sight.rs`): a flat cross
  over each port face, a line between the two faces, and a tick per
  `capture_distance` along it. Plates green on `facing_holds()`; line and ticks
  green on `gap_holds() && motion_holds()`. It draws inside an 8 unit face gap,
  only while locked, capable and not already docked, and reads its poses from
  `DockingPorts::nearest_pair` - the mechanic's own search with the gate off -
  so the instrument and the verb can never disagree.
- **`examples/playable/docking_approach.rs`**: a scenario-local tender with a
  port on its nose and a drifting spar to dock with, and nothing else in the
  scene. The systems range is autopilot-only, which is why the owner had to
  test in the editor.

### Verification

- `cargo test -p nova_ship --lib docking`: 18 tests. The new ones are the modal
  rule itself - a dock that holds through a burn and a helm because only the
  verb ends it, a release asked for by the ship that did NOT press `DOCK`, a
  re-parked helm read off the live `Rotation`, and a maneuver taking the dock.
- `cargo test -p nova_hud --lib docking_sight`: 6 tests over the gate, the two
  plates, the gap line and its ticks, and the colour change.
- `examples/systems/system_docking_ports.rs`: 10 `outcome:` markers, green.
  The two new force-level claims are read back to back on the SAME held burn -
  inert while docked, biting the moment the dock lets go - so neither can pass
  on a dead engine.
- `examples/playable/docking_approach.rs` run under the harness: the lock and
  the sight asserted, and `docking-approach.png` INSPECTED - both plates, the
  gap line and its ticks are on the screen in cyan at 72 m.
- `cargo check --workspace --all-targets` with and without `--features debug`,
  `cargo fmt --check`, `content gen` + `content lint` clean, `web npm run ci`
  and `mdbook build` green.

### A trap worth recording

`SpaceshipSectionSystems` is gated on `scenario_is_live`
(`configure_scenario_gating`). A range that hand-builds its hulls and loads no
scenario runs NO section systems, so "a docked hull's throttle is inert" passes
for the wrong reason. `system_docking_ports` now loads an empty scenario for
the sole purpose of opening that gate.

### Still deferred

Real flight authority for a docked assembly - the case where a player ship
gains control and flies the pair, as in a decaying-station rescue. Recorded
here on the owner's word, not scheduled.
