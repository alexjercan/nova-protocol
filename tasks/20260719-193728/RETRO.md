# Retro: examples reorg (purpose dirs + bevy-style slugs)

- TASK: 20260719-193728
- BRANCH: refactor/examples-reorg (squash-landed as a607eb3b)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs recorded, 1 accepted risk)

## What went well

- Plan-then-decide paid off: filing the task with "names stay" as an
  explicit challengeable KEY DECISION surfaced the user's real preference
  (bevy-style slugs) BEFORE any file moved, and the amendment priced the
  rename consequence in as scope instead of it arriving as review churn.
- The scripted two-pass sweep (path forms before bare names, explicit
  live-file list, built-in leftover grep) made 202 replacements
  reviewable in one screen and left task history/news/CHANGELOG-history
  untouched by construction rather than by care.
- doc-sweep-grep-plus-reread, applied deliberately this time, caught four
  meaning-level spots a clean grep passed: development.md's "four
  blocks"/"all eighteen" counts, guide-add-section's "use the next free
  number" how-to, and the CHANGELOG Unreleased bullets that ship with
  v0.8.0.
- The drift pin turned the design's scariest failure mode (autoexamples
  off + a forgotten [[example]] block = an example silently stops
  building) into a bare-cargo-test failure, and immediately proved its
  worth in-cycle by construction-testing the catalog I had just written.
- Landing protocol clean on the first try this time: sync check, atomic
  squash from the main checkout, rename detection verified in the squash
  stat (all 21 moves as renames, 90-100%).

## What went wrong

- One command violated two promoted ledger lessons at once: `cd wt &&
  Xvfb :N ... & ... cargo test | tail` backgrounded the WHOLE cd chain
  (test ran vacuously in the main checkout) and the trailing pipe ate the
  nonzero exit (xvfb-run was not even installed). The catch came from
  reading the test COUNTS ("0 passed; 1 filtered out" = wrong binary),
  not the green exit - counts are the vacuous-pass tell. Redone via a
  script file (job control scoped per line, recorded-PID Xvfb, bare exit).
- `autoexamples = false` first landed under `[lib]`, where cargo silently
  ignores it - a no-op that self-review caught minutes later. Manifest
  keys have a HOME table; a key that lands without complaint is not
  necessarily a key that took effect.

## What to improve next time

- Any command needing job control (`&`, traps, PID capture) starts life
  as a script file, never an inline compound - the inline form has now
  produced a vacuous pass twice.
- After adding a config key, verify its EFFECT (here: does an uncataloged
  file still build?), not just its presence in the file.

## Action items

- [x] Lessons: bumped `shell-bg-vs-and-chain` (x2, the cd-swallowing
      variant), `piped-cargo-masks-exit-code` (x7, counts-are-the-tell),
      `doc-sweep-grep-plus-reread` (x2, applied-successfully note).
- [ ] Recorded NIT for a future release cut: old example names get
      cargo's bare "no example target" error; the CHANGELOG bullet is
      the migration note (no hint map on purpose).
