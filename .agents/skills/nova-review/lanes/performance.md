# Lane: performance

Judge what the change costs a frame.

Read the probe sections of `docs/performance.md` before you make any timing
claim. You hold the measurement slot: no other lane runs a rendered example
while you work.

## Look for

- Per-frame work that scales with entity count: a query that walks everything to
  find one thing, a `Vec` or `String` built every frame, a `format!` in a hot
  system, a large asset cloned.
- A system in `Update` that belongs in `FixedUpdate`, and gameplay in
  `FixedUpdate` that a render frame drives instead. The fixed loop is
  single-threaded on purpose.
- Missing change detection: a reconciler that rebuilds its tree every frame, a
  system with no `Changed<T>`, `Added<T>`, or run condition where the state
  changes rarely.
- Archetype churn: a component inserted and removed every frame; spawn and
  despawn where a `Visibility` flip or a pool would do.
- Assets loaded or re-read on a hot path. Blocking IO inside a system.
- Rendering: a mesh or material handle created per frame, gizmo and UI entities
  respawned rather than mutated, an entity kept visible only to keep it
  pickable.
- A cost that lands on the frame a player is looking at, where a cover or a
  spread across frames would hide it.

## Evidence

Judge a repeat set against a named reference. Never assert a single timing.

```bash
nix develop --command cargo run --features debug probe run <name>
nix develop --command cargo run --features debug probe run <name> --repeat 5
```

Read `checks.json` and `report.html` in `probe-runs/<short-commit>/<name>/`.
`SKIPPED` means unmeasured, not passed.

Measure only when the change plausibly moves a frame and a range already covers
it. Otherwise reason from the code and say the claim is unmeasured.
