# Typed scenario queries and watches

## Accepted design

- Replace engine-owned reserved variables with typed queries.
- A watch evaluates a query each live update and publishes it as an auto-updating variable.
- Watched and mutable variables share the authored `Name("...")` lookup namespace.
- Runtime storage remains separate. `VariableSet` cannot write watched names.
- An inline `Query(...)` expression reads the current query snapshot.
- `VariableSet` plus `Query(...)` captures a one-shot value.

Initial query family:

```rust
enum QueryConfig {
    Scenario(ScenarioQuery),
    Entity(EntityQuery),
}
```

- `ScenarioQuery { property: ScenarioProperty::Elapsed }`
- `EntityQuery { filter: EntityQueryFilter { id }, property: EntityProperty::Speed }`
- Query kinds own separate property enums. Invalid cross-kind properties are unrepresentable.
- `Entity` has strict single-result semantics: zero is unavailable; multiple is an error.
- Missing velocity is unavailable, not zero.
- Future plural queries are a separate `Entities` query kind with `aggregate`, not `property`.

Authored shape:

```ron
watches: [
    (variable: "elapsed", query: Scenario((property: Elapsed))),
    (
        variable: "courier_speed",
        query: Entity((filter: (id: "courier"), property: Speed)),
    ),
]
```

```ron
Term(Factor(Query(Entity((
    filter: (id: "courier"),
    property: Speed,
)))))
```

## Compatibility

- Clean breaking migration. No runtime aliases for `scenario_elapsed` or `player_speed`.
- Lint legacy names with direct migration guidance.
- Migrate base, examples, Gauntlet, and The Ledger.
- Bump bundled mod versions and changelogs.

## Implementation slices

1. Query types, scenario elapsed watch, and expressions.
2. Strict entity-speed queries and watched values.
3. Inline query snapshots and one-shot capture coverage.
4. Content migration, docs, player-path coverage, and validation.

## Implementation record

- Added `queries.rs` with typed scenario and strict-single entity query families.
- `NovaEventWorld` keeps mutable variables, watched values, query snapshots,
  and the internal scenario clock separate.
- `Name` resolves mutable and watched values. Watched names reject writes.
- Query sampling runs before timer edges and `OnUpdate` under the live/unpaused gate.
- Inline `Query(...)` factors use the same coherent snapshot as watches.
- Entity speed is unavailable for zero matches, duplicate ids, or missing velocity.
- Removed reserved engine-variable constants and runtime aliases.
- Migrated generated base content, the example mod, Gauntlet, and The Ledger.
- Gauntlet version: 1.7.0. The Ledger version: 1.20.0.
- Updated modding and developer documentation.

## Verification

- `nix develop --command cargo check`
- `nix develop --command cargo fmt --check`
- `nix develop --command cargo test --lib -p nova_scenario` - 162 passed
- `nix develop --command cargo run content lint` - 0 errors, 0 warnings
- `cd web && npm run ci`
- Portal generation publishes Gauntlet 1.7.0 and The Ledger 1.20.0.
- `NOVA_AUTOPILOT=1 ... player_path --features debug` completed two rounds.
  Installed portal copies were older and produced expected legacy-name lint
  warnings; source portal generation and content lint used the migrated copies.

## Follow-up fix

- The initial speed sampler scanned every `EntityId`, including ship section
  children. Section ids are ship-local and repeat across ships, which produced
  false strict-single errors for ids such as `cube_i0_j0_k1`.
- Speed sampling now includes only entities with `LinearVelocity`. Duplicate
  ids outside the `Speed` property domain are ignored. Duplicate moving entity
  ids still make the speed query unavailable.
- Added focused regression coverage with duplicate section-local ids.
