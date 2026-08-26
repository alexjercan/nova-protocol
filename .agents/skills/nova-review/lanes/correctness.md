# Lane: correctness and tests

Judge whether the change is right at its edges, and whether its tests would
catch it being wrong.

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
  - A fixture must spawn every component production spawns, or the test proves
    nothing.
  - A `Local`-guarded reconciler needs an `Added<Marker>` override.
- State and ordering: a transition that can run twice, a message two systems
  read where only one drains it, a system pair with no explicit order.
- A test that asserts the implementation instead of the behavior. A test whose
  name does not read as a behavior statement.
- Behavior the change adds with no test and no example range. A substantial
  feature earns a harnessed range, not only a unit test.
- An `outcome: <slug>` marker missing beside a range assertion, or missing from
  `crates/nova_probe_cli/tests/catalog_drift.rs`.
- A bug fixed with no test that fails without the fix.

## Running

```bash
nix develop --command cargo test -p <crate> --lib <filter>
nix develop --command cargo run --features debug probe run <name> --correctness-only
```

Run only what the change touches. Never the workspace suite.
