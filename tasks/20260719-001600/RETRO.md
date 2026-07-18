# Retro: Make the CI clippy job warning-clean

- TASK: 20260719-001600
- BRANCH: chore/ci-clippy-clean (landed on master as 5e40facc)
- REVIEW ROUNDS: 1 (APPROVE, out-of-context pass; 2 NITs, both record-keeping)

## What went well

- The split "cargo clippy --fix first, then re-read the produced diff line
  by line" carried the cycle: --fix did over half the sites mechanically,
  and the re-read caught its one WRONG fix (blockquote markers baked into
  shakedown.rs prose) before anything was built on it
  (verify-scripted-edits-applied paying off).
- Verify-before-design on the engine constraint: reading bevy_reflect
  0.19's impls BEFORE attempting to box SectionSource::Inline turned a
  would-be dead-end compile cycle into an accurate one-line allow
  justification (verify-engine-guarantees-in-source).
- Compiler-as-enumerator for the Content::Section boxing: full grep sweep
  dumped to a file, then check --all-targets - zero missed call sites,
  first-try green.
- Pipelining: the cold baseline clippy run executed in the background while
  the task file, pre-reads and the call-site sweep happened - the cycle's
  wall-clock was essentially one cold build plus one test build.

## What went wrong

- clippy --fix produced a semantically wrong doc fix. Root cause: the
  doc-lint family's machine-applicable suggestions optimize for silencing
  the lint (indent it, mark it as a quote), while the actual defect is
  prose wrapped so a line starts with a markdown marker (`-`, `+`, `>=`);
  the correct fix is a rewrap no tool offers.
- NOTES.md miscounted the boxing clone-out sites (said 2, diff has 4) -
  written from memory of making the edits instead of from the final diff.
  Caught by the out-of-context reviewer. This is the third occurrence of
  prose-from-diff-not-intent; bumped to Pending promotions in the ledger.

## What to improve next time

- On lint-cleanup tasks, budget the manual pass for prose/doc lints from
  the start; treat --fix output on them as adversarial input, not a fix.
- Write fix-record counts by counting the diff, not the memory of edits -
  then re-read asking "does the prose claim anything the diff does not?".

## Action items

- [x] Ledger: new lesson `doc-lint-autofix-misreads-prose` (x1); bumped
      `prose-from-diff-not-intent` to x3 -> Pending promotions.
- [x] tatr 20260719-004908 (backlog): decide pin-nightly + -D warnings vs
      advisory clippy gate.
