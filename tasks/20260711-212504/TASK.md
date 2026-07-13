# Menu ambience: thruster-flown AI orbit replaces ballistic seeding

- STATUS: CLOSED
- PRIORITY: 41
- TAGS: v0.5.0,menu,ai,spike

## Goal

The main menu's ambience ship flies its orbit on real thrusters (flame +
hum) instead of the seeded ballistic orbit; the menu's bespoke orbit math
goes away.

## Steps

- [x] Flip `menu_orbiter` (crates/nova_assets/src/scenario.rs ~95) to
      SpaceshipController::AI(AIControllerConfig { orbit:
      Some("menu_planetoid"), .. }) and update the surrounding comment that
      cites the old editor gate as the reason for ballistic seeding.
- [x] Delete nova_menu's seed_orbiter_velocity system, the OrbitSeeded
      marker, and their scheduling (crates/nova_menu/src/lib.rs); also
      deleted orbit_insertion_velocity, MENU_ORBITER_ID, the LinearVelocity
      import, and the two tests that pinned the seeding
      (orbit_insertion_velocity_is_tangential_v_circ,
      orbiter_is_restaged_and_seeded_once - they tested exactly the deleted
      mechanism). Kept stage_menu_camera. Updated the module doc.
- [x] Check ORBIT_CLEARANCE and related constants in nova_menu for
      leftovers only the deleted seeding used; ORBIT_CLEARANCE stays (the
      camera framing uses it to keep the ring in shot) with a re-scoped doc
      comment; MENU_PLANETOID_ID stays (camera anchor).
- [x] Run the app, watch the menu: thruster flame visible, orbit in
      progress, no wild swing across the camera (evidence rig below). Hum
      verified by mechanism, not by ear: the flame proves
      ThrusterSectionInput > 0, which is the same input the hum's volume
      polls inside the same live set.
- [x] Regression test: config-level pin in nova_assets
      (menu_orbiter_is_an_ai_ship_directed_at_the_planetoid) - the orbiter
      is AI with orbit directive on "menu_planetoid", carries controller +
      thruster sections, and the planetoid exists with authored surface
      gravity (so it gets a well). The behavior chain is covered by
      nova_gameplay's orbit_directive_tests (engage/resume) and
      nova_scenario's config-mapping test from task 20260711-212521; the
      full visual is the run above. (The originally-planned headless
      "engaged autopilot after some frames in MainMenu" app test would need
      the full render + assets stack; the piecewise chain plus the live run
      covers the same claim without a fragile mega-fixture.)
- [x] Close the originating spike task 20260711-185440 (STATUS: CLOSED with
      outcome note) and append the fix record to
      tasks/20260711-212358/SPIKE.md.
- [x] Verify: cargo check + fmt, run the newly written tests.

## Notes
- Spike: tasks/20260711-212358/SPIKE.md
- Third of three seeded tasks; depends on 20260711-212519 (gate re-scope)
  and 20260711-212521 (AI orbit directive) - both landed.

## Close record (2026-07-11)

What changed: menu_orbiter is now an AI ship with an orbit directive on
"menu_planetoid" (nova_assets); nova_menu lost its entire ballistic-orbit
apparatus (seed_orbiter_velocity, OrbitSeeded, orbit_insertion_velocity,
MENU_ORBITER_ID, two seeding tests) - the ORBIT autopilot plans the ring
from the well's runtime geometry, which is exactly the runtime-derivation
work the menu's staging math used to hand-roll. The camera staging is
untouched; ORBIT_CLEARANCE survives only as the framing estimate.

Evidence rig (visual run): built the main app with the debug feature and
ran it under the real display (cargo run --features debug, RUST_LOG=warn);
captured the NovaProtocol window by X id (xprop _NET_CLIENT_LIST +
ImageMagick import) at ~20 s, ~25 s and ~37 s after launch. Observed: ship
left of the planetoid with a visible blue thruster flame at t0, right of
the planetoid at t1 (flame visible), advanced along the ring at t2; camera
fixed, menu panel over the scene, no HUD chrome, no insertion swing across
the camera; log free of panics. Screenshot diffs were localized to the
ship's track (would-it-fail check: a dead ship would have produced
pixel-identical frames, the prior cycle's known false-positive mode).

Difficulties: the first run used the bare binary path, which resolves
assets relative to the executable and loaded nothing - launch via cargo
run from the workspace root. Full-screen scrot also captured the user's
desktop instead of the game window; capturing the window by X id is the
reliable rig (recorded here per record-the-exact-rig).

Self-reflection: deleting the two seeding tests alongside the mechanism
they pinned felt like weakening coverage until the replacement pin
(config-shape test + the sibling tasks' behavior tests + the live run) was
written down explicitly - spelling out the coverage chain in the step is
what kept the deletion honest. The visual step caught nothing wrong this
time, but it is the only place flame/motion/framing are actually seen;
keep it in any scene-facing task.
