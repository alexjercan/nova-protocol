# Game state snapshot: a nova_probe capability

- STATUS: CLOSED
- PRIORITY: 71
- TAGS: v0.11.0, harness, debug, probe

## Goal

Debug the game by reading its state, not by staring at screenshots. Owner's
framing: "I like this idea of being able to export JSON/RON of the ship+skin +
also including the health and stats, like you would take a snapshot of the game".

Every ship-skin defect this sprint was caught by rendering a picture and looking
at it. Twice a rule was adopted from a render and later disproved by another
render. A machine-readable snapshot ends that loop.

## Shape

A capability in `nova_probe`, following the existing pattern: `capabilities/`
holds one module per kind of evidence, each an env-gated plugin an example wires
(`frametime`, `timeline` - a JSONL sink, `invariants` riding that sink).
`contract.rs` declares what an example claims to collect.

Contents, per ship: identity, transform and velocity, aggregate health and mass,
per section (id, prototype, local pose, class, health, alive), per
`SectionFixture` (what, attached to what, health, alive), per turret (magazine,
rounds, reload, target), and ordnance in flight. Plus a header with scenario id,
frame, elapsed time and a schema version.

Deterministic ordering and fixed float precision, so two snapshots of one state
are byte-identical. Without that a diff is useless.

## Why build it properly

The owner eventually wants a headless JSON mode - actions on stdin, per-frame
state on stdout, so a harness or MCP tool can drive the game. **This serializer
is the output half of that.** Build a reusable serializer over the world, not a
debug printf welded to one example.

## Ruled out by the owner

Save/load and checkpoints. Each scenario is already fully replayable and acts as
its own checkpoint. This is READ ONLY.

## Definition of done

- a snapshot from a real run, read and attached
- two snapshots of one frozen state diff byte-identical
- the skin-dump lane can extend the ship record without a second serializer

## Lane

sprout `probe-snapshot`.
