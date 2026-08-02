# Add checkpoint-driven Nova automation scripts

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.10.0, tooling, autopilot, testing
- KIND: STORY
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120019

## Story

Replace bespoke elapsed-time closures with a small Nova-first checkpoint runner.
Scripts wait for observable game state, perform player-path actions, emit probe
markers, optionally request a capture, and fail with the named stalled
checkpoint. This becomes the shared seam for correctness, screenshots, and
profiling loops.

## Steps

- [ ] Inventory duplicated stage/wait/deadline/completion logic in `playable`,
      `broadside`, `lifeline`, UI examples, and screenshot producers.
- [ ] Design the minimum code-authored step API required by at least three
      existing scripts: named wait predicate, action, settle frames, capture
      request, probe marker, per-step timeout, finish, and loop reset.
- [ ] Implement the runner in `nova_autopilot`; keep Nova integrations behind
      adapters so the crate boundary stays acyclic.
- [ ] Convert `playable`, one full campaign walk, and one UI/capture producer as
      proof that the same runner supports all three downstream uses.
- [ ] Add failure diagnostics containing script, checkpoint, elapsed time, and
      the last observed state. Preserve real input scheduling and production
      scenario events as the success signal.
- [ ] Document script authoring next to the example catalog and link it from the
      development wiki.

## Definition of Done

- One checkpoint model drives gameplay, UI, and capture scripts without
  wall-clock success assumptions. (test: `checkpoint_runner_advances_only_after_observation`)
- A stalled checkpoint fails loudly with its name and observed state.
  (test: `checkpoint_timeout_names_the_stalled_step`)
- Capture checkpoints negotiate completion with probe/FPS collectors instead of
  exiting early. (test: `capture_checkpoint_waits_for_registered_collectors`)
- The three migrated examples complete through the shared checkpoint runner.
  (test: `migrated_checkpoint_examples_complete`)

## Notes

- No serialized automation DSL in v0.10.0. Rust scripts already need direct
  world access for real input and scenario predicates.
- Keep scenario-specific actions in examples. The crate owns sequencing,
  deadlines, markers, capture requests, and completion only.
