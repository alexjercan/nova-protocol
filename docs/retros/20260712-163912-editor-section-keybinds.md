# Retro: Editor visible + editable section keybinds

- TASK: 20260712-163912
- BRANCH: feature/editor-section-keybinds (landed as 640aaca)
- REVIEW ROUNDS: 1 (APPROVE, 2 NITs; 1 addressed)

What/why: TASK.md. Implementation write-up: docs/retros/20260712-editor-section-keybinds.md.
Process only here.

## What went well

- **Verified the reuse substrate's PRECONDITIONS before planning on it.** The
  gameplay `screen_indicator` was the obvious tool for "label anchored to a world
  entity", and it would have COMPILED in the editor - but its
  `ScreenIndicatorCamera` is attached only to the spaceship chase camera, which
  the editor doesn't have, so it would have silently shown nothing. Checking that
  before committing the plan turned a would-be dead-end into a ~15-line
  editor-local `world_to_viewport` projection. Verifying an API's wiring/
  preconditions, not just its signature, is the save here.
- **Designed the rebind's sharp edges up front**: dual-write to the live
  component AND the config map the scenario reads, preserve non-keyboard binds,
  Escape cancel, and a stale-entity guard for a section deleted while armed - all
  first-pass, all tested. The review found no correctness issues.
- **Was honest about the untested part** (the pixel projection needs a rendered
  viewport) in the docs and task record rather than pretending coverage.
- **Kept the fast path untouched**: the hold-key-while-placing arms are
  byte-identical; only the previously-empty `None` click arm changed.

## What went wrong

- Nothing of substance. One doc/impl mismatch (NIT R1.1): the plan said the
  positioning system would run in `PostUpdate` after transform propagation, but I
  implemented it in `Update` (a deliberate one-frame-lag choice for a near-static
  editor scene) without updating the wording. Fixed by documenting the Update
  choice + rationale in the function so code and comment agree. Lesson: when I
  deviate from a plan step during work, update the step's wording / note it in the
  record in the same edit, not leave it for review to catch.

## What to improve next time

- When reaching for an existing subsystem to reuse, verify its runtime
  PRECONDITIONS in the new context (does the required camera/marker/resource
  exist here?), not just that its API fits - a compile-clean reuse can be a
  silent no-op.

## Action items

- [x] Retro written; ledger note added under advertised-but-unwired (verify a
  reused subsystem's preconditions hold in the new context).
- [ ] Playtest the editor UX: rebind entered from no-tool-selected mode, and
  whether a "Select" palette button / armed-section highlight would help
  discoverability (noted as optional follow-ups in the docs).
