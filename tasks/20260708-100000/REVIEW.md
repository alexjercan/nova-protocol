# Review: Harnessed editor example for smoke testing

- TASK: 20260708-100000
- BRANCH: feat/editor-harness-example

## Round 1

- VERDICT: APPROVE

Diff: `editor_app(render)` added to nova_core (+ prelude), `src/main.rs` refactored onto it, new
`examples/09_editor.rs`, and `09_editor` registered in `examples_smoke`.

Verified:

- Delivers the ask. The example runs the *same* editor the binary runs - both now go through the
  single `editor_app` constructor, so there is no divergent copy. The harness is attached the same
  way the gameplay examples do it (env-gated, inert in a normal run).
- The autopilot exercises a real editor action, not just a boot. It waits for `GameStates::Playing`
  (correct - the editor enters its inner Editor state there), finds the create-with-controller
  button by `Name`, and triggers `Activate` once (guarded by `EditorAutopilotDone`). The headless
  run confirms the whole chain: `reached Playing` -> `created a ship with a controller` ->
  `cycle complete, no panic`.
- Bonus regression value: the created ship is exactly the controller-preview path from task
  20260706-212909, and the run shows no `root not found in q_root` - so this example would now
  catch a regression of that fix in CI.
- `main.rs` refactor is behavior-preserving: `render` resolves to `!norender` under `debug` and
  `true` otherwise, matching the previous `with_rendering`; `debugdump` path is unchanged. The
  binary builds in both feature modes.
- Clippy is clean in both feature modes (the `bevy::prelude` import is correctly gated to `debug`,
  since only the harness code names bevy types); the only workspace warning is the pre-existing
  `hull_section.rs` one, outside this diff.
- `examples_smoke` runs `09_editor` and passes (49s for the five examples under Xvfb); the whole
  `cargo test --workspace` is green (58 nova_gameplay, etc.).

Scope is honest (TASK.md): only the create-with-controller action is driven; section placement
needs pointer picking, left as a future extension. The `Activate`-by-`Name` seam is a clean,
minimal way to reach editor UI from the autopilot without adding test-only hooks to the editor.

No BLOCKER/MAJOR/MINOR findings.
