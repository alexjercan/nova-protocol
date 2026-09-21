# Review 03: Gameplay integrity and physics

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor finding

## Finding

### MINOR - `crates/nova_gameplay/src/integrity/neutralize.rs:22,82` - Documentation points to a deleted module

The module link targets `super::glue`, and a later ordering comment attributes `on_section_disable` to `glue.rs`. That module was deleted when section adjacency moved to link-point mates. The observer now lives at `crates/nova_ship/src/sections/integrity.rs:216`.

The stale link can produce a rustdoc warning and sends maintainers to the wrong owner for an ordering guarantee. No runtime behavior or new API is involved.

## Adjudication

- Dropped a test assertion's embedded spacing as not material to code quality or behavior.
- No supported performance finding survived. The reviewed hot paths use explicit budgets, pools, reused buffers, caps, and focused invariant tests.
- Checked and cleared observer fan-out around `IntegrityDestroyMarker`, contact and overkill clamps, carve budgets, chunk activation, pyre sampling, emitter pooling, gravity hysteresis, projectile filtering, and settings wiring.

## Coverage

Fully read all files under `nova_gameplay/src/integrity/`, plus `gravity.rs`, `bounds.rs`, `projectile_hooks.rs`, `settings.rs`, `test_support.rs`, and plugin wiring. Targeted cross-boundary checks covered damage signatures, marker definitions, ship section-disable ownership, asteroid debris authoring, and the history of the removed glue module.

Not checked: no Cargo command, content lint, game, probe, wasm build, workspace test, or Clippy run. Full ship and scenario consumers remain assigned to later batches. Performance conclusions are static, not measured.
