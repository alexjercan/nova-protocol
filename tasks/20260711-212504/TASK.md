# Menu ambience: thruster-flown AI orbit replaces ballistic seeding

- STATUS: OPEN
- PRIORITY: 41
- TAGS: v0.5.0,menu,ai,spike

## Goal

The main menu's ambience ship flies its orbit on real thrusters (flame +
hum) instead of the seeded ballistic orbit; the menu's bespoke orbit math
goes away.

## Steps

- [ ] Flip `menu_orbiter` (crates/nova_assets/src/scenario.rs ~95) to
      SpaceshipController::AI(AIControllerConfig { orbit:
      Some("menu_planetoid"), .. }) and update the surrounding comment that
      cites the old editor gate as the reason for ballistic seeding.
- [ ] Delete nova_menu's seed_orbiter_velocity system, the OrbitSeeded
      marker, and their scheduling (crates/nova_menu/src/lib.rs ~86, ~340);
      keep stage_menu_camera. Update the module doc (~6) that describes the
      ballistic workaround.
- [ ] Check ORBIT_CLEARANCE and related constants in nova_menu for
      leftovers only the deleted seeding used; keep what the camera staging
      still needs.
- [ ] Run the app, watch the menu: thruster flame visible, hum audible,
      orbit settles without a wild swing across the camera. If the
      insertion looks bad, stage the spawn position near the target orbit
      radius in the scenario config.
- [ ] Regression test in nova_menu or nova_assets (headless): loading
      menu_ambience in MainMenu yields the orbiter with AISpaceshipMarker +
      AIOrbitDirective, and an engaged Orbit autopilot after some frames
      (needs tasks 20260711-212519 and 20260711-212521 landed).
- [ ] Close the originating spike task 20260711-185440 (STATUS: CLOSED with
      outcome note) and append the fix record to
      docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md.
- [ ] Verify: cargo check + fmt, run the newly written tests.

## Notes
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- Third of three seeded tasks; depends on 20260711-212519 (gate re-scope)
  and 20260711-212521 (AI orbit directive).
- Also update the menu_ambience doc comments that cite the old editor gate
  as the reason for ballistic seeding (nova_assets/src/scenario.rs,
  nova_menu/src/lib.rs module docs).
- When done, this closes out the original spike task 20260711-185440.
