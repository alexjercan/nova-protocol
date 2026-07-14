# Retro: Editor UI rework (baseline)

- TASK: 20260714-204219
- BRANCH: editor/ui-rework
- REVIEW ROUNDS: 1 (APPROVE, both NITs addressed)

See TASK.md for what/why and NOTES.md for the module map; this is process only.

## What went well

- Reusing the existing selection path for the new cards was the single best
  decision. Cards are themed buttons carrying `EditorButton` +
  `ButtonValue<SectionChoice>`, so `button_on_setting` drives selection and the
  two example autopilots (which find UI by `Name` and insert `Pressed`) passed
  with ZERO example edits. Reaching for the shared mechanism instead of a bespoke
  card handler avoided a whole class of regressions.
- De-risking the picking footprint up front paid off: I read the window
  resolution (1024x768) BEFORE sizing the panels, kept rail+drawer under half the
  width, and the autopilot's screen-centre placement click landed first try. The
  alternative - discover it via a failed run - would have cost a full cold rebuild.
- The out-of-context adversarial review was worth it on a 12-file change: it
  independently re-derived the old-vs-new plugin-wiring parity and caught a latent
  tooltip despawn-ordering fragility I had only half-noticed. Both NITs were cheap
  and became real robustness fixes.
- Splitting a 2001-line file went cleanly because I cross-checked wiring parity
  mechanically (grep old vs new `add_systems`/`add_observer`) rather than trusting
  the move by eye.

## What went wrong

- First autopilot run exercised nothing: I ran the built binary directly
  (`./target/.../examples/09_editor`), so `assets/` resolved against the wrong CWD,
  every asset 404'd, the editor never left Loading, and the run still printed
  "cycle complete, no panic". A green-looking run that loaded zero assets. Re-ran
  via `cargo run --example` (CWD = crate root) and it actually drove placement.
- A Write path slip: I first wrote the TASK.md close-out to
  `crates/nova_editor/tasks/...` (nested inside the crate) instead of the worktree
  root `tasks/...`, creating a stray directory I had to `rm -rf`. Caught it
  immediately because the STATUS grep showed the real file still IN_PROGRESS.

## What to improve next time

- To verify a headless example, run it through `cargo run --example` from the
  crate root (or set the asset root), never the raw `target/` binary - the CWD
  determines whether `assets/` loads, and a no-panic run proves nothing if nothing
  loaded. (New ledger lesson `run-example-via-cargo-run-for-assets`.)
- When a hover-out despawns a shared singleton, tag it with its owner so an
  out-of-order enter/leave between siblings can't kill the fresh one. (Ledger
  `despawn-by-owner-not-all-on-cross`.)

## Action items

- [x] Two NITs fixed on the branch before merge (tooltip owner-tag; planetoid
  gravity assertion).
- [x] Ledger: `run-example-via-cargo-run-for-assets`, `despawn-by-owner-not-all-on-cross`,
  `ui-footprint-vs-3d-picking` added; `sweep-then-delete` bumped to x7.
- [ ] Follow-up task 20260714-212139 filed: unify the whole game UI to the
  web-app theme (the editor is now the template; menu/HUD still ad-hoc).
- [ ] "The rest" stays on task 20260714-081703 (export/load, objects, events,
  factions, modifications, real icons).
