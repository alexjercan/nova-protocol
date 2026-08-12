# Define destruction and neutralization event lifecycle

- STATUS: IN_PROGRESS
- PRIORITY: 0
- TAGS: backlog, scenario, modding

## Goal

Define clear destruction, despawn, and combat-neutralization event semantics
before changing `OnDestroyed` or `OnNeutralized`.

## Scope

- Trace every destruction and despawn path.
- Separate hull destruction, combat neutralization, scripted despawn, and
  lifecycle cleanup.
- Decide which events compose with lock, orbit, and area end edges.
- Document ordering and payload guarantees before implementation.
