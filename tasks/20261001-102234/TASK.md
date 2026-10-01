# Replace SalvageCrate with canister cargo and location triggers

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0

## User facts
- Remove the legacy `SalvageCrate` object. Keep the useful "reach this location" event without pretending that proximity awards inventory stock. Explore real cargo canisters plus independent trigger volumes (including bounded regions) to preserve location objectives.
- Track this on the M5 branch as the remaining item-loop cleanup; it is not proof that PR #98 already delivers it.

## Agent findings
- `crates/nova_scenario/src/objects/salvage.rs` defines a static sphere-sensor pickup that emits `OnEnter` and lets scripts despawn/count it; it does not transfer inventory. `ScenarioAreaMarker` in `crates/nova_scenario/src/objects/area.rs` already owns deduplicated location enter/exit events. `CargoCanister` is a separate physical, capacity-bounded inventory object collected by ship intake, not by location entry.
- `examples/playable/first_shift_map.rs` authors a SalvageCrate. The object kind and type name also have editor, event/filter, scenario, docs, and base-asset consumers. A shared sound file is used by cargo intake: deleting the crate must not delete that sound by accident. Legacy released scenario/mod RON may contain SalvageCrate; assess migration policy rather than silently dropping it.

## Delivery
- Map all runtime-ID consumers, authored scenarios/builders, example/lesson objectives, editor surfaces, and published scenario/mod content that still rely on `SalvageCrate`. Show exact replacement of each: real canister pickup through intake when stock matters, independent location area when arrival matters, or a composed objective with both gates when both matter. Decide which event identifies the actual canister transfer; location `OnEnter` alone must never mint items or complete a pickup objective.
- Gate the trigger shape/geometry and ownership before implementation: existing scenario areas are spherical, while a bounding-box trigger may need an explicit validated shape. Decide persistence/migration of released SalvageCrate content and id/event behavior before deleting the old object kind, marker, plugin, editor picker, and obsolete documentation. Do not retain an unshipped compatibility adapter.
- Preserve the existing New Game lootable derelict and the approved session-only boundaries. Do not add a market, save/claim-once logic, free inventory, or an intake bypass to make the objective pass.

## Verification
- Assert that entering a location alone fires its location event once and changes no inventory; actual canister intake transfers the authored quantity exactly once, observes capacity/refusal, and provides the correct pickup event. Assert combined objectives need their chosen gates, and scenario cleanup/revisit does not duplicate items within a session.
- Validate migrated real content and examples with generation/lint and affected tests; inspect a rendered location cue and a real canister pickup/objective flow. Check editor authoring, event filters, docs, and released-format migration or documented refusal explicitly.

## Done when
- No live runtime/editor/authoring SalvageCrate path remains; approved shipped-content migration or explicit load error is proven. Location-only and inventory-pickup objectives are distinct, and real canister transfer plus rendered flow pass. Parent M5 task records the result; PR readiness requires separate review and CI.
