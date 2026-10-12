# Jobs: recover, escort, distress call and neutralize a named hull

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.16.0,jobs

## User facts
- Epic `20261012-100022`. Four job types from the design page: recover a marked item from a wreck; escort a hauler to the next cluster with pirates scaling with its cargo value; answer a distress call, some fake; neutralize a named hull.

## Agent findings
- Generated bodies need the addressing seam from the world-first strand (`crates/nova_world/src/streaming.rs:176-182`); the events strand supplies `OnPickedUp` and friends.

## Delivery gate
- Each type is one handler template over the open-world events, with its completion check named; no new event kinds. One asserted example per type.

## Verification
- Probe walks: each type completes once and refuses hand-in before completion.

## Done when
- Four types ship and each has a permanent proof.
