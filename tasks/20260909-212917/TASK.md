# Velocity and gravity spheres enclose the live hull

- STATUS: CLOSED
- PRIORITY: 86
- TAGS: v0.14.0, bug, hud

## Defect

The velocity sphere and the gravity sphere have a fixed radius. A big hull
pokes through them.

- `crates/nova_hud/src/lib.rs` `setup_hud_velocity`: `radius: 5.0` for the
  velocity widget and `radius: 5.6` for the gravity widget, world units
  (50 m and 56 m), regardless of hull size. The shipped skiff has 21 sections;
  the 33-cell-long carrier has 2,081 sections and cannot fit inside the fixed
  112 m outer diameter.
- `crates/nova_hud/src/velocity.rs` `VelocityHudConfig::radius` is the orbit
  radius of the cone and the sphere shell.
- `crates/nova_hud/src/flight_status.rs:43` documents the 5.6 u shell as a
  constant; the speed chip parks beside it.

Owner report (2026-09-09): "it has a fixed radius, so a really big ship would
not be able to display it properly, we would see it clipping into the ship, so
it needs to scale with the ship and inscribe around it."

## Plan

`HullRadius` (`crates/nova_ship/src/sections/hull_radius.rs`) is not the
containment contract. It measures from the centre of mass to the outer face
along each section's radial ray, explicitly not to the furthest collider point,
because flight uses it as a structural arm and arrival distance. Changing that
meaning would retune flight and still leave the HUD contract implicit.

Add a separate `HullEnvelopeRadius`: the smallest radius this section-collider
model can publish that contains every live section collider about the live
centre of mass. Publish it beside `HullRadius` in the same one-pass fixed-tick
scan so the two values cannot disagree about which sections are live. Compute
the exact furthest point for each supported `SectionCollider` shape in its local
frame, rather than using a rotated AABB whose empty corners would unnecessarily
inflate shells around rotated long sections. The authored HUD clearance remains
outside this physical envelope to cover the visual skin and keep the projection
clear. Use a fixed 5 m hull clearance and retain the current 6 m separation
between the velocity and gravity shells. These are fixed physical gaps, not
percentages of hull size.

The shell must share the envelope's centre. Centering a COM-relative radius on
the ship root is incorrect after asymmetric construction or damage moves the
COM.

1. Establish the visual reproduction before changing production code:
   - add a dedicated `screenshot_hud_shell` example and its explicit workspace
     `[[example]]` entry first;
   - stage `block_carrier` as the player ship with nonzero velocity and a
     dominant gravity well, so both fixed shells are visible around the same
     hull;
   - use one pinned broadside camera, fixed capture resolution, deterministic
     setup, and the normal `AppBuilder` path so the before and after shots differ
     only because of the fix;
   - capture `hud-shell-carrier.png` from the unfixed build into
     `target/hud-shell-comparison/before/`, inspect it, and record in this task
     that the 50 m and 56 m shells clip through the carrier;
   - keep the example unchanged through the implementation so it is the visual
     test fixture, not a separately restaged after-shot.
2. Add and export `HullEnvelopeRadius` in `nova_ship`:
   - define it beside `HullRadius` as a derived world-unit component;
   - rename the private publisher to `publish_hull_radii`, since its one pass
     now publishes both contracts;
   - add a `SectionCollider` helper that returns the exact furthest distance
     from an arbitrary COM-relative point for cuboid, sphere, capsule, and
     cylinder shapes, including section rotation;
   - derive `HullRadius` and `HullEnvelopeRadius` together from the same live
     section query and `ComputedCenterOfMass`;
   - update an existing component with change detection and insert it when
     absent, matching `HullRadius` lifecycle and dead-root behavior;
   - register and export it through the ship preludes.
3. Define the shell contract in `nova_hud`:
   - velocity radius = live `HullEnvelopeRadius` + `Meters(5.0)`;
   - gravity radius = velocity radius + `Meters(6.0)`;
   - convert the `nova_events` `Meters` values only at the render boundary;
   - grow immediately so an expanding radius cannot expose the hull;
   - shrink with a frame-rate-independent 150 ms exponential half-life using
     `alpha = 1 - 2^(-dt / half_life)`, then snap to the target within 1 cm, so
     repeated damage retargets without restarting a fixed-duration transition;
   - ease one shared envelope radius and derive both shells from it every frame,
     so their 6 m separation cannot drift during convergence.
4. Extend the directional HUD's live state instead of baking radius only at
   spawn. Each frame, read the target's `HullEnvelopeRadius`, `Position`,
   `Rotation`, and `ComputedCenterOfMass`. Transform the local COM into world
   space and use it immediately as the common centre of the velocity sphere,
   gravity sphere, and their orbiting cones. Do not ease COM movement: exact
   centering and containment take precedence over smoothing a damage-driven COM
   shift.
5. Synchronize every radius consumer from the eased value:
   `DirectionalSphereOrbit.radius`, the sphere child's translation and scale,
   and `DirectionSphereMaterial.radius`. Keep the gravity shell at the exact
   authored separation so the shells do not z-fight.
6. Handle startup without showing the carrier through the old 50 m shell. Do
   not reveal a shell until the first `HullEnvelopeRadius` is available.
   Preserve the gravity widget's separate rule that hides it in flat space.
7. Move the speed and mode chips with the outer gravity shell:
   - anchor each chip from the live COM to the screen-right edge of the current
     outer gravity radius, including while the gravity shell itself is hidden in
     flat space;
   - add a fixed 12 px horizontal gap beyond the projected shell edge;
   - retain the current -90 px speed-chip and -114 px mode-chip vertical
     offsets and existing off-screen behavior;
   - calculate the world edge after the chase camera updates and before screen
     indicator projection, so placement uses the camera pose rendered that
     frame and follows both hull size and camera zoom.
8. Keep the generic directional-orbit mechanism in `nova_gameplay` independent
   of ships. Ship radius, COM, metric margins, child mesh synchronization, and
   chip placement remain HUD responsibilities.
9. Add focused unit coverage:
   - each `SectionCollider` shape reports its exact furthest point under
     translation and rotation;
   - `HullEnvelopeRadius` contains all live colliders, follows a moved COM, and
     shrinks when an outer section becomes inactive without changing the
     existing `HullRadius` result;
   - the velocity shell stays 5 m outside the physical envelope and the gravity
     shell stays exactly 6 m outside the velocity shell;
   - metric conversion, COM centering, immediate growth, 150 ms shrink
     half-life, 1 cm completion snap, child transform/material updates, startup
     visibility, chip placement 12 px outside the projected outer shell with
     the existing vertical offsets, and unchanged gravity visibility.
10. Add a dedicated `system_hud_shell` range in `examples/systems/`. Spawn
   `block_skiff` and `block_carrier` as the player hull in turn and assert that
   both shells clear `HullEnvelopeRadius` from the live COM. Sever the live
   section that determines the envelope, wait for convergence, and assert that
   the envelope and shell shrink while remaining nested. Assert that both
   flight chips remain outside the projected gravity shell. Put every
   `outcome:` slug on the roster in
   `crates/nova_probe_cli/tests/catalog_drift.rs`.
11. Re-run the unchanged `screenshot_hud_shell` example after the fix into
    `target/hud-shell-comparison/after/`. Compare the two captures at native
    resolution and side by side. The after-shot must show the carrier wholly
    inside both COM-centred shells, the gravity shell outside the velocity
    shell, and both chips clear of the outer edge. Keep `screenshot_combat_hud`
    unchanged so its armed-hull combat showcase retains its existing purpose.
12. Register `screenshot_hud_shell` as the producer of
    `web/src/assets/wiki-hud-shell.png` and add that focused figure under Flight
    readouts in `web/src/wiki/hud.md`. State that the shells follow the live
    hull, remove the fixed 56 m claim from `flight_status.rs`, and add one
    concise Interface & HUD entry to `[Unreleased]`.

## Verification

- Run the affected `nova_ship` and `nova_hud` unit tests.
- Run `system_hud_shell` correctness-only, then rendered, and inspect
  `checks.json` and `report.html`.
- Run the `nova_probe_cli` catalog-drift test.
- Capture `screenshot_hud_shell` before and after into separate directories;
  inspect both at native resolution and as a side-by-side comparison.
- Run the screenshot producer-map checks after registering
  `wiki-hud-shell.png`.
- Run `cd web && npm run ci` after the wiki and screenshot mapping changes.

## Before shot (2026-09-11)

`target/hud-shell-comparison/before/hud-shell-carrier.png`, captured from the
unfixed build through the new `screenshot_hud_shell` example at 1920x1080.
`block_carrier` is the player hull, on station under 140 m/s of held velocity,
inside a 4.9 km ice-world SOI so the gravity shell is up.

Inspected at native resolution:

- The 50 m velocity shell is not visible at all. It is entirely buried inside
  the 360 m hull.
- The 56 m gravity shell shows only as a small yellow bulge poking through the
  keel near frame centre. The rest of it is inside the ship.
- The speed chip (`140.0 m/s`) sits ON the hull at its fixed 120 px offset,
  because the offset was authored for a shell 56 m across.

This is the defect the owner reported, in one frame: neither sphere encloses
the hull, and the readout parked "beside the sphere" is parked on the ship.

## After shot (2026-09-11)

`target/hud-shell-comparison/after/hud-shell-carrier.png`, from the fixed build
through the same unchanged `screenshot_hud_shell` example and the same
1920x1080 framing. Both captures inspected at native resolution and side by
side; the dark hemisphere was also read under a shadow boost, because the shell
surface only lights where a cone rides it.

- Both spheres now enclose the hull. The halo is centred on the carrier's
  centre of mass and stands clear of the nose and the stern, where the before
  shot has no shell outside the hull at all.
- The blue velocity cone rides the shell OUTSIDE the stern, in open sky. In the
  before shot it is inside the ship.
- The yellow gravity cone hangs well below the keel, pointing down at the well,
  instead of bulging through the plating.
- The gravity shell is outside the velocity shell: the yellow cone stands
  further out than the blue one by the authored 6 m.
- The speed chip (`140.0 m/s`) is off the hull and clear of the outer shell
  edge, where the before shot parks it on the superstructure.

The mode chip is blank in both captures. The carrier flies manually here, and
`drive_mode_chip` shows nothing without an engaged maneuver - a quiet HUD is
the manual look. Both chips are graded together in the `system_hud_shell`
range, which engages a GOTO leg for exactly that reason.

## Scripted-camera indicator ordering

The first after shot came back with no chips at all. `ScreenIndicatorSystems`
and `anchor_flight_chips` were ordered after `ChaseCameraSystems::Sync`, which
sits in `CameraAuthoritySystems::Solve`, but a `ScriptedCameraPose` - photo
mode, a capture script, a cinematic - overwrites the solved pose afterwards in
`CameraAuthoritySystems::Override`. Every scripted frame therefore projected
its indicators through a camera that was not the one rendering them: here the
chase pose stands 26 m off the carrier's stern, so a 205 m shell edge falls 43
degrees off axis against a 36 degree half-angle and the widget hid it.

Both sets now order after `CameraAuthoritySystems::Override`, which is what the
existing comment already asked for: sample the same camera pose the frame
renders with.

## Result (2026-09-11)

Done. The shells are sized off `HullEnvelopeRadius`, published beside
`HullRadius` from the same live section scan, and both spheres and the two
flight chips are driven from one eased envelope per hull.

Verification run:

- `cargo test -p nova_hud --lib`: 270 pass.
- `cargo test -p nova_ship --lib sections::`: 397 pass.
  `cargo test -p nova_ship --lib flight::`: 126 pass.
- `cargo test -p nova_probe_cli --test catalog_drift`: 2 pass.
- `probe run system_hud_shell --correctness-only`, then rendered: OK, 7/8
  measured, 0 invariant violations over 391 frames, `capture_simulated` PASS,
  `log_clean` PASS. All four outcomes fire on BOTH hulls - skiff envelope
  48.3 m (shells 53.3 / 59.3 m), carrier 194.3 m (shells 199.3 / 205.3 m);
  after the sever, 40.6 m (45.6 / 51.6 m) and 187.9 m (192.9 / 198.9 m).
- `screenshot_hud_shell` captured before and after, inspected at native
  resolution and side by side: see the two sections above.
- `scripts/gen-web-screenshots.py` packaged `wiki-hud-shell.png` into
  `web/src/assets/`; producer map checks pass.
- `cd web && npm run ci`: pass, the new asset emitted.
- `cargo fmt --check`: clean.

## CI gates (2026-09-11)

Every job in `.github/workflows/ci.yaml` reproduced locally.

- `cargo fmt --check`, the seven generated-art `--check` gates and the
  probe-matrix step: clean.
- `cargo clippy --workspace --all-targets --features debug -- -D warnings`:
  clean. It caught a `single_match` in `publish_hull_radii`, now a `let ... else`.
- `cargo test --workspace --features debug`: clean.
- `RUSTFLAGS=-D warnings cargo check --workspace --all-targets` (DEFAULT
  features): clean. It wanted `#[cfg(feature = "debug")]` on the two example
  constants that only debug-gated code reads.
- `CLIPPY_CONF_DIR=ci/wasm-clippy cargo clippy ... --target
  wasm32-unknown-unknown -- -D warnings`: clean.
- `cd web && npm run ci`: clean.

### The probe shards, under the rasterizer CI uses

Both new examples were run the way the probe job runs them - lavapipe
(`llvmpipe`, `device_type: Cpu`), Xvfb, four cores, `NOVA_AUTOPILOT_DEADLINE=280`,
`--correctness-only --timeout 300`. Both OK, 0 invariant violations, 0 offending
log lines, and all eight outcome markers fire with the same numbers as on the
GPU.

The first pass of the range took 167 s in-process against the 280 s run
backstop, and 99 s of that was the ONE appended screenshot: thirty settle frames
of 2 081 sections cost ~3 s each under a software rasterizer. The round order is
now carrier first, skiff second, so the picture is taken of the small hull. Same
two rounds, same four invariants on both, 77 s in-process. The per-beat
deadlines are `SHELL_STEP_DEADLINE_SECS` (90 s) against a 16 s worst beat,
instead of the bare 20 s they were.

### One unrelated flake fixed

`nova_ship::camera::handback::tests::handback_blends_the_anchor_instead_of_snapping`
failed inside the full workspace run and passed on its own. It drives
`MinimalPlugins`, so the handback blend advances on the WALL clock: one update
slowed by a loaded machine carries the anchor past the "did not snap" band
(0.339 rad against 0.05). It now runs on
`TimeUpdateStrategy::ManualDuration(1/60 s)`, the pattern the `nova_hud` tests
already use. Not caused by this task - it would fail on a busy runner either
way.
