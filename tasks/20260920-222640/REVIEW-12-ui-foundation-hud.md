# Review 12: UI foundation and HUD

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: ECS deferred-command follow-up
- Verdict: major and minor findings

## Findings

### MAJOR - `crates/nova_hud/src/objective_stack.rs:376-405,622-632` - Objective-stack rebuild can panic automated proof runs during same-frame teardown

`sync_objective_chips` queues `despawn_related::<Children>()` on the stack every rebuild. The `On<Remove, PlayerSpaceshipMarker>` observer can queue a despawn of the same stack in the same frame. There is no ordering edge covering every ship-destroy and scenario-unload path.

Unlike plain `despawn`, Bevy 0.19's `despawn_related` uses the configurable fallback command error handler. Automated examples install `FallbackErrorHandler(panic)`. If the stack despawn applies before the previously queued child-despawn command, the latter targets a dead entity and panics the proof process. The repository already guards the same class of teardown race in scenario lifecycle and slider rebuilding, but not here.

The specialist established a deterministic command-level reproduction shape, but did not execute it. Production play normally warns rather than panics because it does not install the panic handler.

Why not BLOCKER: no shipping-player crash was established, and the race was not executed in this review. It can invalidate automated correctness runs.

### MINOR - `crates/nova_ui/src/widget/text_field.rs:365-446` - Text fields rewrite display state every frame

`paint_text_fields` runs unconditionally and writes border, background, color, visibility, and text for every field every frame. It also rebuilds the displayed string even when focus, hover, error, value, and placeholder are unchanged. Sibling widget and HUD reconcilers use change filters or write-on-diff guards.

This marks text changed and can cause avoidable measurement/shaping work in idle forms. The population is small and no frame-time impact was measured, so this remains a minor static performance finding.

## Adjudication

Dropped the broad list of plain HUD `remove`/`despawn` sites. Those commands use a warning handler, and no concrete same-frame race was established for the individual sites. Checked and cleared theme resolution, indicator projection order, HUD visibility, bounded marker reconciliation, layout measurements, missing-player state, and authored theme IDs.

## Coverage

Fully read every Rust file and inline test in `nova_ui` and `nova_hud`. The ECS specialist then traced objective-stack production registration, scenario teardown paths, Bevy deferred-command/error semantics, proof-environment handler installation, and comparable guarded code.

Not checked: no Cargo command, game, probe, GPU measurement, wasm build, workspace test, or Clippy run. The objective-stack panic reproduction was designed but not executed. Text-field cost is unmeasured.
