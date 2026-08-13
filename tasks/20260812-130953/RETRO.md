# Retrospective

## What worked

- One pure derivation function kept runtime publication, lint, tests, and NOVA OS aligned.
- Exact old/new graph parity protected all built-in cube ships during the migration.
- Strict content lint found migration gaps before runtime.
- Keeping collider geometry non-structural produced a clear ownership boundary.

## What changed during delivery

- Ship-specific integrity moved fully into `nova_ship`; generic lifecycle stayed in
  `nova_gameplay`.
- Runtime graph publication moved from per-collider observers to one update per
  completed section spawn batch. The Raid playtest exposed false errors from
  intermediate collider states.
- The example mod needed explicit sockets because its section override replaces
  the complete base prototype.

## Next time

- Test complex spawn order, not only graph topology. A bridge-spawned-last ship
  would have exposed the per-collider timing defect before playtesting.
- Treat observer seams as incremental unless the API explicitly guarantees batch
  completion.
