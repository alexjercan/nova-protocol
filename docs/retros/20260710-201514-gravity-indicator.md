# Retro: Yellow gravity indicator, SOI shell removed

- TASK: 20260710-201514
- BRANCH: gravity-indicator (squashed to master as b88f3fd)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 2 NIT, fixed in-round)

## What went well

- The generalize-instead-of-copy call paid off: velocity.rs already split
  input (feeder) from rendering (shader/observers), so the gravity variant
  was a source enum + palette component, not a second widget module. The
  velocity path is byte-for-byte unchanged and a test pins that.
- `cargo check --workspace --examples` caught the 05_directional initializer
  break immediately - third cycle in a row this habit paid; it is earning
  its place as a standing verify step.
- Removing the SOI shell in the same branch as its replacement kept master
  from ever having both or neither gravity display.

## What went wrong

- R1.1 (spawn flash): the bundle hardcoded `Visibility::Visible` and relied
  on the feeder's first run to hide the gravity variant - a ship spawned in
  flat space flashes yellow for a frame. Root cause: I wrote the toggle in
  the feeder and never asked what the state is BEFORE the feeder runs.
  Initial state is part of a toggle's contract; derive it from the same
  predicate the runtime toggle uses.
- Two stale doc comments in holo_instruments.rs still described the removed
  shell. Grep for the identifiers was done; grep for the concept ("shell")
  in comments was not. When removing a feature, sweep prose too.

## What to improve next time

- When a system toggles a component at runtime, make the spawn-time value an
  explicit decision from the same condition, and assert it in the test.
- Feature-removal checklist: identifiers, registrations, tests, prelude,
  docs - and a case-insensitive grep for the feature's NAME across comments
  and docs, not just its symbols.

## Action items

- [ ] Playtest the 5.6/5.0 nesting of gravity vs velocity spheres by eye
  (noted in REVIEW.md; adjust radius or sharpness if they read as one).
- [ ] Cosmetic: the two On<Add, VelocityHudMarker> observers disagree on
  bare-marker handling (error vs default-palette fallback); unify whenever
  velocity.rs is touched again.
