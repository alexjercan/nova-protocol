# Thruster hum plays in editor build state (gate audio to sim)

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.4.0,audio,bug

## Goal

Bug (user report): the thruster engine hum plays while holding thrust in the
editor's build state, where the ship is not being simulated. Root cause: the
thruster-loop audio systems (`ensure_thruster_loop`, `update_thruster_loop_volume`
in `crates/nova_gameplay/src/audio.rs`) run in plain `Update` with no state
gating and *poll* `ThrusterSectionInput`. That value is set to 1.0 by the global
`on_thruster_input` observer even in the editor build state (thrust physics is
gated to `ExampleStates::Scenario`, but the input observer is not). Every other
consumer of `ThrusterSectionInput` (the impulse system, the exhaust shader) is
gated into `SpaceshipSectionSystems`, which the editor runs only in `Scenario`.
The audio must follow the same pattern.

The four one-shots (explosion/impact/turret/torpedo) are NOT affected: they fire
on real spawn/damage/destroy events, which only occur inside the (gated)
`SpaceshipSectionSystems`, so no projectile spawns / no sound in the editor.

## Steps

- [x] Put `ensure_thruster_loop` and `update_thruster_loop_volume`
      `.in_set(SpaceshipSectionSystems)` (Update) so they inherit the same
      `run_if(ExampleStates::Scenario)` gating the editor applies to the thruster
      physics/shader. Keep `prune_sfx_throttle` ungated (harmless map cleanup).
- [x] Verify scenarios still hum: headless `BCS_AUTOPILOT=1` on a scenario
      example reaches Playing, no panic (SpaceshipSectionSystems runs there,
      ungated). Confirm the editor example (`09_editor`) still boots headless.
- [x] fmt, clippy --all-targets, cargo test --workspace. Shared CARGO_TARGET_DIR.
- [x] Update `docs/2026-07-08-audio-sfx-system.md`: note the hum is gated into
      `SpaceshipSectionSystems` so it is silent outside the running simulation
      (e.g. the editor build state); the one-shots are event-driven and need no
      gating.

## Notes

- Depends on: 20260708-162011 (CLOSED). `SpaceshipSectionSystems` is a public set
  (sections prelude); the editor gates it to `ExampleStates::Scenario`
  (`nova_editor/src/lib.rs`), the base plugin configures it (`plugin.rs`), and a
  plain scenario leaves it ungated (runs every frame) so the hum still works.
- Known edge (accept/note, not fixing here): if you switch Scenario -> Editor
  while holding thrust, the gated volume-update stops and the sink volume freezes
  at its last value until you return to Scenario. This matches the existing
  behaviour of the gated thruster shader and is a rare transition artifact; a
  full fix would despawn the loop on sim exit (needs an editor-visible signal).

## Outcome

Gated the thruster-hum systems (`ensure_thruster_loop`,
`update_thruster_loop_volume`) `.in_set(SpaceshipSectionSystems)` so they inherit
the editor's `run_if(ExampleStates::Scenario)` gating - the same set the thruster
physics and exhaust shader already use. In the editor build state the set is off,
so the hum systems do not run and the engine stays silent; in a plain scenario
the set is ungated, so the hum works normally. `prune_sfx_throttle` stays ungated.
Confirmed the diagnosis: the one-shots are event-driven (spawn/damage/destroy
inside the gated set) and were never at risk; only the polling hum leaked.

Verified: fmt, clippy --all-targets (clean), cargo test --workspace, headless
`10_gameplay` autopilot (Playing, hum path runs, no panic) and `09_editor`
autopilot (boots, reaches Playing, no panic). Could not headlessly hold thrust in
the editor (needs a GUI + audio device); the fix is logically the same gating the
existing thruster shader relies on. Noted a rare Scenario->Editor-while-thrusting
freeze edge (matches the shader's behaviour).
