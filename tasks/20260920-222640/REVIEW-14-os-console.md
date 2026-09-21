# Review 14: NOVA OS and console

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: ECS/input follow-up
- Verdict: major finding

## Finding

### MAJOR - `crates/nova_os_ui/src/terminal/input.rs:235-243` - App launch leaves the prompt handler active for the rest of the buffered input batch

`handle_terminal_keyboard` computes `nova_os_prompt_active` once before reading all `KeyboardInput` messages. An Enter event can call `terminal.submit()` and switch `active_mode` from Prompt to App. Later messages already buffered in the same frame still pass the cached guard and mutate or submit the now-hidden prompt.

This is reachable deterministically: autopilot text input writes multiple keyboard messages into one batch without an update between characters. Native input can also batch events during a slow frame or key repeat. The app-side handler explicitly drops transition-frame input, but the prompt-side handler does not.

The result is hidden prompt state that differs when the player exits the app, or a phantom second submission from a message the app transition should have consumed or discarded. No crash or permission bypass was found. A proof can use the existing terminal fixture by enqueueing launch Enter plus one character before one `app.update()`, but it was not executed here.

Why not BLOCKER: the defect corrupts prompt UI state at an input edge; it does not bypass command permissions, corrupt game state, or crash.

## Adjudication

The ECS specialist confirmed a structural lack of ordering among Dispatch/Simulate and Map/Ship sets, but found no current shared state that makes it a live defect. It was dropped.

Checked and cleared parser/execution agreement, cheat arming, command dispatch exhaustiveness, shell-control ownership, pause transitions, app invocation ownership, per-ship section IDs, map IDs, pointer transforms, and terminal revision-based painting.

## Coverage

Fully read every source and test file in `nova_os`, `nova_os_ui`, and `nova_console`. The specialist traced Bevy message batching, terminal mode mutation, autopilot injection, app-side transition clearing, and the complete Monitor/Console set graph.

Not checked: no Cargo command, game, probe, rendered UI run, GPU measurement, wasm build, workspace test, or Clippy run. The proposed same-batch input proof was not executed. Native multi-event frequency was not measured.
