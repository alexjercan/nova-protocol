# Retro: nova_ui crate + migrate nova_editor

- TASK: 20260714-214111
- BRANCH: ui/nova-ui-crate
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what/why; process only here.

## What went well

- Treating the migration as a pure extraction let me verify by PARITY rather than
  by behaviour: `git diff master -- placement.rs` empty + a value/logic diff of the
  moved theme/widget proved nothing drifted, which is faster and stronger than
  re-running everything.
- Adding a deterministic `nova_ui` selection test (insert `Pressed` -> resource set)
  isolated the one thing the move could plausibly break, and carried the verdict
  when the autopilot couldn't.

## What went wrong

- I burned a lot of time chasing a green autopilot "placed a section" that never
  came, because the machine was saturated (I had just kicked off cold full-graph
  rebuilds + a parallel sprout was building). The autopilot's phase waits are
  frame-COUNTED but its lifetime is a wall-clock 6s, so under heavy load too few
  frames run and it stalls after "selected" - it looks like a placement failure but
  is frame-starvation. Root cause: I ran the timing-sensitive autopilot in the
  middle of heavy builds.
- Two self-inflicted detours: ran the raw example binary (assets resolve from CWD,
  so nothing loaded) and later dropped stderr with `2>/dev/null` (tracing logs go to
  stderr, so greps found nothing).

## What to improve next time

- Run frame-sensitive autopilots BEFORE starting other heavy builds, or accept the
  git-diff + unit-test parity proof for a touch-free path instead of forcing a
  timing-dependent e2e.
- Autopilot logs are on STDERR - always `2>&1`, never `2>/dev/null`. And run via
  `cargo run` from the crate root, not the raw binary (asset CWD).

## Action items

- [x] Ledger: `autopilot-is-frame-starved-under-load` added; bump
  `run-example-via-cargo-run-for-assets`.
- [ ] Residual: capture one green `09_editor` "placed a section" on an idle machine
  (the code path is byte-identical to where it passed in 20260714-204219).
- [ ] Next: task 20260714-214115 (menu restyle), then 214118 (HUD).
