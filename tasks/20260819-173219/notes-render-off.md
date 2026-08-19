# Running without a GPU: what it costs, and what it buys

Scoped 2026-08-19, NOT started. Owner had begun this once and set it aside; this
records the findings so a third attempt does not start from nothing.

## Why it is worth having, beyond one measurement

Owner: "it would be useful to perf test game logic."

That is the durable reason. Every simulation number in this epic is currently
measured THROUGH the renderer, on a case whose worst-frame spread is ~30%
run to run. A GPU-less run gives simulation cost with the render schedule -
27.5% of the traced 4v4 - removed outright, so a gameplay change can be graded
without render noise on top of it.

It also answers the one question nothing else we have can: **the floor.** If
simulation alone is 70 ms of an 83 ms frame, no rendering work saves the arena.
If it is 8 ms, rendering is the whole story. Resolution scaling separates GPU
fill from CPU render PREPARATION; only a GPU-less run removes the preparation
too.

## The seam already exists

- `AppBuilder::with_rendering(bool)` (`crates/nova_core/src/lib.rs:177`) already
  threads `render` into the gameplay, ship and scenario plugins, and gates the
  HUD and NOVA OS. `editor_app(render, startup)` takes it too.
- `bevy_hanabi` - the one GPU-coupled dependency in game code - is ALREADY
  behind that flag (`crates/nova_gameplay/src/plugin.rs:78`).
- `render_plugin()` (`lib.rs:388`) returns the `RenderPlugin` value for `.set()`,
  which is where the change lands.

What `with_rendering(false)` does NOT do is stop bevy creating a device and
running the render sub-app. That is the entire missing piece:

```rust
RenderPlugin {
    render_creation: RenderCreation::Automatic(WgpuSettings {
        backends: None,          // no adapter -> no render sub-app at all
        ..default()
    }),
    synchronous_pipeline_compilation: true,
    ..default()
}
```

With `backends: None` bevy's `RenderPlugin` bails: no `RenderDevice`, no
extract/prepare/queue. The main schedule still ticks, so the probe still counts
frames - CPU-only ones.

## Why the risk is lower than it looks

**No game crate uses `RenderDevice`, `RenderQueue`, `RenderApp` or
`ExtractSchedule`** - checked across `crates/**`. No first-party system can
panic for want of a GPU. The exposure is third-party only.

Open unknowns, all cheap to discover by running it: whether winit still wants a
window (`primary_window: None` under the same flag if so), and whether any bevy
internal assumes an adapter.

## The design call

**Do not overload `with_rendering(false)`.** It means "no visual game plugins"
today and callers depend on that. Add an orthogonal switch - an env var read
inside `render_plugin()` - so no existing caller's meaning changes.

## Estimate

A 20-30 minute spike, not a project. Write the field, run `wfc_arena`, see what
breaks. Either it boots and the floor is known, or the panic IS the estimate.
Anything past an hour means a genuine surprise worth reporting.

## Note

`synchronous_pipeline_compilation: true` becomes moot without a device. Do not
delete it - `lib.rs:390-397` records that flipping it back to bevy's async
default SIGSEGVs one run in five, and that reason survives this change.
