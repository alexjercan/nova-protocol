# Add regional ship variety to the open world

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.15.0,world,ships,content

## Goal and boundary

Vary ship designs and appearances across the open world. Human activity already weights a damaged tender versus wreck plating; explore industrial, salvage, and other readable regional styles using current fields or an additional field only if the evidence needs it. Ship variety should include more than the same two hulks, and suitable mod-provided designs should participate without a closed enum. Do not assume that a new skin alone makes a new ship role.

`20260925-190131` owns live enemy, neutral, and allied encounter spawns and their AI. This task owns what ship designs and looks are eligible in which places; it does not itself authorize a full faction system or new combat behavior.

## Decisions

- A derelict remains a ship design, not a separate object kind or spawn-time damage flag.
- Suitability metadata belongs to the ship catalog entry or another owning content interface.
- Unknown design IDs and empty eligible sets must fail loudly. Do not fall back to an arbitrary ship.
- Selection must be deterministic for a world seed, coordinate, and content identity.

## Agent findings

- `block_frame_tender_damaged` and `block_wreck_plate` are existing authored catalog entries.
- `GameShipDesigns` has no role, tag, or suitability metadata for procedural selection.
- The current layered generator uses a hardcoded two-design `DERELICT_DESIGNS` list, weighted by human activity and material density in `crates/nova_world_base/src/clusters.rs`; main-thread materialization validates catalog existence.
- Worker generation cannot inspect the live `GameShipDesigns` resource.

## Delivery

- Review current ship designs and presentation options against industrial, salvage, and other regional looks; propose a small visible first set rather than inventing a closed cast.
- Decide whether explicit catalog suitability and regional weighting are needed for the first set, and define when eligible designs are validated and snapshotted for worker-safe deterministic generation.
- Decide save/mod mismatch behavior when the eligible catalog changes, with `20260925-190156` owning the broader lifetime policy.
- If catalog metadata is approved, migrate the damaged-tender and wreck-plate entries, update content lint, generated-world selection, catalog documentation and editor presentation as needed, and delete the obsolete hardcoded list.

## Verification

- Rendered travel shows distinct ship designs and looks in the selected kinds of regions; the same seed, coordinate and catalog identity repeat the choices.
- Base content exposes a nonempty eligible set for every selected role or region.
- Invalid or missing metadata fails during content lint or world arming, before a sector materializes.
- Same seed, coordinate, and catalog identity select the same design independent of visit order.
- A mod-provided eligible design participates without a Rust enum change.
- Catalog mismatch behavior is asserted rather than silently selecting a different hull.

## Done when

- The owner approves the first regional ship-variety slice and its content/selection policy before runtime edits.
- The selected designs are visibly distinct in rendered world travel, with documented base and mod eligibility and focused deterministic/failure proofs.
- Obsolete hardcoded selection and fallback paths are removed when their owning policy replaces them.
