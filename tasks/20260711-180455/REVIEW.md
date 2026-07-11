# Review: Ambient menu background scenario

- TASK: 20260711-180455
- BRANCH: feature/menu-ambience

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_menu/src/lib.rs:113,147 - the two systems
  that carried all three development bugs (stage_menu_camera,
  seed_orbiter_velocity) have no committed tests; the evidence harness was
  throwaway. Regressions the suite cannot catch: pose write moved back into
  the controller-removal frame (bug 1 returns), re-stage dropping
  well.body_radius, OrbitSeeded not inserted (reseed every frame). Add two
  headless App tests: camera staging (controller gone + pose at
  well + (0, 0.75r, 2.5r)) and orbit seeding (re-staged position,
  tangential LinearVelocity, OrbitSeeded present, exactly once).
  - Response: fixed - menu_camera_is_staged_from_the_wells_geometry (also
    asserts the blank-while-controlled behavior from R1.5) and
    orbiter_is_restaged_and_seeded_once (position, tangential v_circ,
    OrbitSeeded, seeded exactly once). 7/7 nova_menu tests pass.
- [x] R1.2 (MINOR) crates/nova_menu/src/lib.rs:26 - `use
  nova_assets::prelude::*` is now unused (setup_menu_camera deleted);
  cargo check warns. Delete the import.
  - Response: fixed - import deleted; cargo check --workspace is warning-free
    (only the pre-existing proc-macro-error2 future-incompat note).
- [x] R1.3 (MINOR) crates/nova_menu/src/lib.rs:334 - the ambience teardown
  is special-cased in on_sandbox; every future MainMenu exit (pause menu's
  Back path, task 20260711-185156) must remember its own unload or the
  backdrop simulates forever. Move the UnloadScenario trigger to a uniform
  OnExit(GameStates::MainMenu) system (ordering is safe: OnExit runs before
  OnEnter(Playing), so New Game's LoadScenario still lands after).
  - Response: fixed - unload_menu_ambience on OnExit(MainMenu); on_sandbox is a
    plain mode+state handler again; the sandbox test now enters the menu
    first and asserts the OnExit teardown fired; doc comments updated.
- [x] R1.4 (MINOR) crates/nova_menu/src/lib.rs:124,163 -
  wells.iter().next() is order-arbitrary and only correct because the
  backdrop has exactly one well; a second big rock would silently give an
  arbitrary camera target and a wrong-well orbit seed. Select the well by
  EntityId "menu_planetoid".
  - Response: fixed - both systems select the well by MENU_PLANETOID_ID (new
    const documenting the why).
- [x] R1.5 (MINOR) - one-to-two frames of the menu render from inside the
  planetoid (loader spawns the camera at (0,10,20); staging waits a frame
  for the controller removal). Deactivate the camera while the controller
  is still attached and re-enable it when staging the pose.
  - Response: fixed - camera.is_active false while the controller is attached,
    true when the staged pose is written; asserted in the staging test.
- [x] R1.6 (MINOR) tasks/20260711-180455/TASK.md close record - the
  "stable orbit" evidence is 4.7s of samples while the ticked step demanded
  "holds a stable orbit for minutes"; cite the bounded 70s v_circ orbit
  integration test in gravity.rs as the stability evidence and state the
  observed duration honestly.
  - Response: fixed - close record states 4.7s observed, cites the bounded 70s
    v_circ orbit integration test in gravity.rs for the long horizon, and
    notes what each covers.
- [x] R1.7 (NIT) nova_menu tests app() doc comment still claims OnEnter
  (MainMenu) systems never run in tests; the ambience test now enters
  MainMenu. Update the comment.
  - Response: fixed - comment rewritten to describe the actual contract.
- [x] R1.8 (NIT) nova_assets/src/scenario.rs ring comment - "can never
  cross the orbit" holds by ~10u in the worst collider seed, not by
  construction. Soften the wording or widen the ring floor.
  - Response: fixed - comment states the ~10u worst-case clearance and the
    regrow-with-the-planetoid instruction.

Round 1 notes: verified clean - state-transition ordering end to end
(Activate observers pre-transition; single teardown on New Game; no
DespawnOnExit/ScenarioScopedMarker overlap), the thrusters-cannot-fire
claim, query disambiguations, Single skip behavior, orbiter-death and
re-entry degradation, long-idle energy boundedness (70s gravity test, no
ship damping), wasm parity. cargo fmt --check clean; cargo check clean
except R1.2.

## Round 2

- VERDICT: APPROVE

Verified against the new diff (fix commit + responses):
- R1.1: both regression tests present and meaningful - the staging test
  asserts the deactivate-then-stage two-frame contract (bug 1's exact
  failure mode) and the seeding test asserts runtime-radius restage,
  tangential v_circ, and seed-exactly-once. 7/7 nova_menu tests pass.
- R1.3: uniform OnExit teardown confirmed; the reworked sandbox test walks
  the real MainMenu -> Playing path and delivery-guards the unload.
- R1.2/R1.4/R1.5/R1.6/R1.7/R1.8 confirmed in the diff; check is
  warning-free, fmt clean.
- Re-ran: 09_editor smoke green (menu -> Sandbox -> editor with the new
  teardown); Xvfb captures 10s apart differ (live orbit) with the panel
  over the scene and no status bar.

No new findings. APPROVE.
