# Lane: correctness and tests

Judge edge cases and whether tests catch the change being wrong.

Read `examples/systems/README.md` before you judge a range.

## Look for

- The edge the change does not handle: an empty collection, one element, a
  missing resource, an entity despawned between two systems, a zero or negative
  delta, a value already at its clamp.
- `unwrap`, `expect`, indexing, and integer casts on data that comes from
  content, a save, or a player.
- Bevy traps this repository has already hit:
  - `Changed<T>` needs a detector system and a counter. `is_changed()` on an
    `EntityRef` outside a system is silently always false.
  - The first update of a `ManualDuration` app has a delta of zero. A
    single-tick test needs a warm-up `app.update()`.
  - A fixture must spawn the components that production spawns.
  - A `Local`-guarded reconciler needs an `Added<Marker>` override.
- State and ordering: a transition that can run twice, a message two systems
  read where only one drains it, a system pair with no explicit order.
- A test that asserts the implementation instead of the behavior. A test whose
  name does not read as a behavior statement.
- Important stable behavior with no sufficient proof. A unit test can prove
  pure logic; cross-system claims need the affected app path; player flows need
  `nova-bench`. Select the check from the claim, not the size of the diff.
- A new test with no named behavior, failure, or invariant. Do not demand tests
  for coverage, prose, paths, inventories, implementation details, or tests.
- An `outcome: <slug>` marker missing beside a range assertion, or missing from
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- A reproduced defect whose important stable behavior has no regression proof.

## Running

```bash
nix develop --command cargo test -p <crate> --lib <filter>
nix develop --command cargo run --features dev probe run <name> \
  --correctness-only
```

Run only what the change touches. Never the workspace suite.
