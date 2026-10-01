# Replace SalvageCrate with canister cargo and location triggers

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.15.0

## Decision and result
- This is a location-only objective, not an inventory pickup. Replace the sole authored Maintenance Crate in `first_shift_map` with a `Beacon` area that fires `OnEnter`. Entering the area does not grant stock. Cargo canisters remain a separate intake-based transaction; no canister prop or combined objective is required here.
- The Ledger uses no `SalvageCrate` and needs no mod version bump. No released authored RON instance needs migration; obsolete `SalvageCrate` variants fail to load rather than silently becoming another object. Preserve `salvage_pickup.wav` for cargo intake.
- Commit `acd232a1b` on master removes the runtime, authoring, editor, event, and ItemHighlight paths. The first-shift arrival is a beacon area. This task does not claim PR #98 delivered the removal.

## Source findings before removal
- The former `crates/nova_scenario/src/objects/salvage.rs` defined a sphere-sensor `OnEnter` object, not inventory transfer. `ScenarioAreaMarker` owns location enter/exit; `CargoCanister` and ship intake own cargo transfer.
- `first_shift_map.rs` contained the only authored crate. The removed kind also had editor, event/filter, scenario, docs, and asset references. `salvage_pickup.wav` remains because cargo intake uses it.

## Verification
- `nix develop --command cargo test -p nova_scenario --lib entering_a_beacon_area_fires_one_on_enter_and_moves_no_cargo` passes: the real ECS ship enters the beacon area, fires exactly one `OnEnter`, and keeps its inventory unchanged.
- The first-shift render shows amber beacon markers, not crate boxes. Maintenance label and objective-text legibility were not established by that render. A combined arrival-and-pickup flow was not requested or tested; canister intake has separate coverage.
- The removal spans scenario, editor, event/filter, examples, docs, and web catalog. `salvage_pickup.wav` remains in use. This task does not add persistence, a market, or intake bypass.

## Done when
- The approved location-only replacement is committed, obsolete `SalvageCrate` paths are removed, and real-ECS area entry is proved not to move stock. Rendered label legibility and broad validation are separate follow-ups if needed.
