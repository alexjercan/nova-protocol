# Improve input system for spaceship components

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Thruster and turrets should use bevy_enhanced_input. Legacy #114.

## Resolution (CLOSED - already resolved)

The spaceship input system already uses bevy_enhanced_input idiomatically for every
controllable section (crates/nova_gameplay/src/input/player.rs):

- Each section type declares a proper action, e.g.
  `#[derive(InputAction)] #[action_output(bool)] struct ThrusterInput;`
  (and TurretInput / TorpedoInput).
- SpaceshipPlayerInputPlugin registers an input context per type
  (`add_input_context::<ThrusterInputMarker>()`, TurretInputMarker, TorpedoInputMarker).
- When a section's `Spaceship*InputBinding` is added, an observer attaches the context
  and its actions via the `actions!` macro with `Action::<..>::new()` and
  `Bindings::spawn(bindings)`.
- Input is consumed through enhanced-input action events (`On<Start<ThrusterInput>>`
  etc.), which drive the section input components.

So the concrete goal - thruster and turrets (and torpedo) driven by bevy_enhanced_input -
is fully in place. The remaining rough edge that the code's own TODO flagged
("NEED TO REFACTOR THIS ... scuff it out") is the `SpaceshipPlayerTorpedoTargetEntity`
*global resource* used for torpedo targeting, which is a torpedo-targeting concern rather
than section input. Re-pointed that TODO at the torpedo extraction ticket
(20260706-162913) so it stays tracked. Closed as already-resolved.

Verified: comment-only annotation change; build clean.
