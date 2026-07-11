# Menu ambience: thruster-flown AI orbit replaces ballistic seeding

- STATUS: OPEN
- PRIORITY: 41
- TAGS: v0.5.0,menu,ai,spike

Goal: the main menu's ambience ship flies its orbit on real thrusters
(flame + hum) instead of the seeded ballistic orbit. Flip `menu_orbiter`
(crates/nova_assets/src/scenario.rs) to SpaceshipController::AI with an
orbit directive on "menu_planetoid", and delete nova_menu's ballistic
seeding + restaging math (seed_orbiter_velocity, OrbitSeeded) - the
autopilot's own orbit insertion replaces it. Keep the menu camera staging.
Verify visually: thruster flame visible, hum audible, no wild insertion
swing across the camera (if the swing is ugly, stage the spawn position
near the target orbit radius in the scenario config).

Notes:
- Spike: docs/spikes/20260711-212358-live-ship-systems-outside-editor-scenario.md
- Third of three seeded tasks; depends on 20260711-212519 (gate re-scope)
  and 20260711-212521 (AI orbit directive).
- Also update the menu_ambience doc comments that cite the old editor gate
  as the reason for ballistic seeding (nova_assets/src/scenario.rs,
  nova_menu/src/lib.rs module docs).
- When done, this closes out the original spike task 20260711-185440.
