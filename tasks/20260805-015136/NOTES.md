# Notes

## Finding

The `ScriptBuilder` rename did not remove the defect. `StepBuilder::on_enter`
still assigned `Some(Arc::new(f))`, so each call discarded the previous action.
The reproduction observed only `second` after registering `first`, then
`second`.

## Change

- Store entry actions as `Vec<Arc<EnterFn>>`.
- Append each `on_enter` action.
- Execute the list in registration order.
- Convert the three stress reload steps from manually combined closures to two
  independent `on_enter` calls.

A list is simpler and safer than nested composed closures. Clone behavior stays
cheap because each action remains behind an `Arc`.

## Verification

- `nix develop --command cargo test --lib -p nova_autopilot` - 50 passed.
- `nix develop --command cargo fmt --check` - passed.
- `nix develop --command cargo check --example many_projectiles --example many_sections --example many_bodies` - passed.
- `nix develop --command cargo run --features debug -- probe run stress` - all
  four probes OK; all three stress capture windows completed.
