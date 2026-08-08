# Add input mapping to PlayerControllerConfig

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

HashMap<Action, Input> per section. Legacy #117.

## Resolution (CLOSED - already resolved)

PlayerControllerConfig already carries a per-section input mapping:
`input_mapping: HashMap<SectionId, Vec<Binding>>`
(crates/nova_scenario/src/objects/spaceship.rs). When a spaceship is built, each
thruster/turret/torpedo section looks its bindings up by section id and gets a
Spaceship{Thruster,Turret,Torpedo}InputBinding component (spaceship.rs ~105-145).

The task envisioned `HashMap<Action, Input>` per section, but each section type has a
single action (thrust / fire / launch), so a per-section list of bevy_enhanced_input
`Binding`s is the more idiomatic shape - the Binding already encodes the input, and
there is no second action to key on. The intent (configure inputs per section on the
player controller config) is fully in place. Closed as already-resolved.
