# Retro: Units - display 1 u = 10 m everywhere (m/km, m/s)

- TASK: 20260728-175731
- BRANCH: feat/units-x10-display
- REVIEW ROUNDS: 1 (APPROVE, two non-blocking findings addressed in-round)

Process observations only; what/why/evidence live in TASK.md, findings in
REVIEW.md.

## What went well

- Applying the ledger forward paid off. `pin-each-caller-not-just-shared-core`
  and `test-the-wiring-system-not-just-its-pure-helpers` were both flagged
  BEFORE writing tests, so the harness proofs drive real systems at three
  distinct callers (speed chip -> `50.0 m/s`, combat lock -> `DST 1.50 km` +
  `CLS +200.0 m/s`, new map range test) rather than only the pure helper.
  The out-of-context reviewer confirmed a no-op or dropped x10 would fail them.
- Editing existing sibling tests in place as the harness proofs (rather than
  spinning up new rigs) kept the diff small and reused known-good fixtures.
- `keep-docs-in-sync`: a whole-doc-tree sweep caught six wiki pages, not just
  the glossary, and correctly left the dev authoring guide's "units per
  second" (raw RON) and past-release news (dated history) verbatim.

## What went wrong

- Pre-sprout TASK.md edits stranded. The Flow State (`PLANNED`,
  `PLAN STATUS: APPROVED`) and the precision Note were written to the MAIN
  checkout's TASK.md at the plan gate, before sprouting. The sprout branched
  from committed master, so those edits never reached the branch; had to
  re-apply them on the branch and `git checkout --` the main checkout to clean
  it. Root cause: flow writes the PLANNED markers at the gate, which is before
  the worktree exists - same family as
  `tatr-new-then-sprout-strands-the-task-file`, but for EDITS to an existing
  task, not a `tatr new` stub.
- Scoped test run silently exercised a subset. `cargo test -p nova_gameplay
  --lib -- f1 f2 f3 ...` with eight positional module filters reported ok but
  flight_status / lock_crosshairs / beacon_chips tests never appeared in the
  output; only re-running each module's filter alone actually ran them. A
  fresh occurrence of `validate-proof-command-shape-at-plan-time` - "ok" is
  not "the named tests ran"; a non-zero count per intended module is.
- R1.1 (the `1000 m` seam) escaped implementation. The m/km switch compared
  the raw pre-format metres, so 999.5..999.9 m rounded to a four-digit
  `1000 m` the km branch exists to avoid. Root cause: a display-unit threshold
  must switch on the ROUNDED displayed value, not the raw input. Cheap fix
  (`metres.round() < KM_THRESHOLD_M`), but it took a reviewer to see it.

## What to improve next time

- Flow: write Flow State / PLANNED markers to TASK.md AFTER sprouting on the
  branch (or commit them to master first). A gate-phase edit to the
  main-checkout task file is orphaned by the next sprout.
- When scoping tests by many `--` filters, grep each intended module name in
  the output (or read a per-module "N passed") and re-run any missing module
  alone; do not trust a single "test result: ok" to mean all filters ran.
- For any value-display helper with a unit/format switch, pin the switch on
  the rounded output the player sees, and add a boundary test that the switch
  never emits the string the other branch is meant to own.

## Action items

- [x] R1.1 + R1.2 addressed in review round 1 (units boundary on rounded
  metres; beacon chip width 140 -> 168 px).
- [x] Ledger: bump `validate-proof-command-shape-at-plan-time` (many-`--`
  filters run a subset) and `tatr-new-then-sprout-strands-the-task-file`
  (flow's pre-sprout marker edits); add `display-threshold-on-rounded-value`.
