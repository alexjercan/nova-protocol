# SetControllerVerb scenario action: enable/disable a flight verb on a ship's controller by id

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.5.0, scenario, controller, verbs, spike

Spike: tasks/20260712-143551/SPIKE.md

Add a scenario action that flips one flight-verb flag on a ship's controller
section at runtime, shaped like `SetSpeedCapActionConfig`
(nova_scenario/src/actions.rs:211-252). New `EventActionConfig::SetControllerVerb`
variant + `SetControllerVerbActionConfig { id, verb, enabled }`: scoped lookup of
the `SpaceshipRootMarker` entity by scenario id (same skeleton as SetSpeedCap),
then find its child `ControllerSectionMarker` section(s) and write the
`ControllerVerbs` flag. If a ship has more than one controller section, write all
of them (the union is the ship capability).

Depends on the ControllerVerbs component from task 20260712-143832. This is the
runtime lever the shakedown task uses. Match the existing action wiring: register
the variant in the `EventActionConfig` match (actions.rs:26,52) and mirror the
`world.push_command`/`queue` deferral SetSpeedCap uses.
