# Classify ship designs for procedural derelict selection

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, world, content, ships

## User facts

- Procedural ship landmarks should read as hulks or derelicts, not repeat one live hauler design.
- The base game already has damaged-tender and wreck-plate designs that can seed the first production policy.
- Procedural selection should later support appropriate mod-provided designs without adding a closed hulk enum.

## Decisions

- A derelict remains a ship design, not a separate object kind or spawn-time damage flag.
- Suitability metadata belongs to the ship catalog entry or another owning content interface.
- Unknown design IDs and empty eligible sets must fail loudly. Do not fall back to an arbitrary ship.
- Selection must be deterministic for a world seed, coordinate, and content identity.

## Agent findings

- `block_frame_tender_damaged` and `block_wreck_plate` are existing authored catalog entries.
- `GameShipDesigns` has no role, tag, or suitability metadata for procedural selection.
- The current layered generator stores one `anchorage_design` and validates catalog existence only during main-thread materialization.
- Worker generation cannot inspect the live `GameShipDesigns` resource.

## Delivery

- Design explicit ship-catalog metadata for procedural roles such as derelict or wreckage suitability.
- Define when the eligible design set is validated and snapshotted for worker-safe deterministic generation.
- Define save and mod mismatch behavior when the eligible catalog changes.
- Migrate base damaged-tender and wreck-plate entries to the approved metadata.
- Update content lint, generated-world selection, catalog documentation, and editor presentation if it exposes the metadata.
- Delete any temporary private base-design list once catalog-driven selection owns the behavior.

## Verification

- Base content exposes a nonempty derelict-eligible set.
- Invalid or missing metadata fails during content lint or world arming, before a sector materializes.
- Same seed, coordinate, and catalog identity select the same design independent of visit order.
- A mod-provided eligible design participates without a Rust enum change.
- Catalog mismatch behavior is asserted rather than silently selecting a different hull.

## Done when

- Procedural derelict selection reads owning catalog metadata instead of a hardcoded design list.
- Base and mod extension behavior is documented.
- Determinism and failure semantics have focused proofs.
- Temporary compatibility and fallback paths are absent.
