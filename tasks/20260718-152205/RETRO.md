# Retro: README overhaul - getting-started HOW-TO + tools reference

- TASK: 20260718-152205
- BRANCH: docs/readme-overhaul
- REVIEW ROUNDS: 1 (APPROVE, one NIT folded in)

## What went well

- **Enumerated bins authoritatively, not by eye.** My first pass grepped
  `[[bin]]` stanzas and reported only 2 of 6 bins - it silently missed every
  default bin (`src/bin/content.rs`, the two `src/main.rs` gens). `cargo
  metadata --no-deps` gave the true set (content, probe, perf_web, meta_gen,
  portal_gen, the root game), so the DoD's "every bin exactly once" was
  checkable instead of guessed.
- **Treated the stale brief as a hypothesis.** The task text named `trace`, a
  separate "HTML report bin", and "Python successors" for meta/portal - all
  three were dead. Grepping the live tree first (verify-stale-brief-against-
  tree) caught them; the README documents the end state and TASK.md records
  the drift.
- **Reused the canonical source.** AGENTS.md already carries a tight 15-crate
  table; reconciling against it beat re-authoring 15 one-liners from scratch.
- **Checked every link target on disk** - which is the only reason the
  pre-existing broken banner (`assets/banner.png`, moved to `assets/base/` in
  d055337a and never swept in the README) surfaced at all.

## What went wrong

- **Did not pin land-scope at the start.** The ask was "sprout a branch and use
  /flow" - branch + /flow, which the promoted `flow-land-scope-when-user-says-
  branch` lesson says to confirm up front. I defaulted to the session's stop-
  before-land cadence and it happened to be right, but I should have stated the
  scope explicitly in my opening restatement rather than leaving it implicit.
- **Uneven command verification.** Only `content -- lint` ran end-to-end; the
  other CLIs were source-verified against their clap/argparse defs. Defensible
  for a docs-of-tools task on a shared box (each other tool is a multi-minute
  cold build), and flagged transparently in TASK.md and REVIEW.md - but it is
  verification-by-reading, not by-running, for 5 of 6 invocations.

## What to improve next time

- To document or audit "every binary/target", enumerate with `cargo metadata`
  (or a find over `src/bin/*.rs` + `src/main.rs`), never by grepping `[[bin]]`
  blocks - default targets carry no stanza.
- In the opening restatement of a flow, state the land-scope decision (branch
  vs default) as a line, even when the session cadence makes it obvious.

## Action items

- [x] LESSONS.md: add `enumerate-bins-via-cargo-metadata`; broaden
  `generated-links-need-real-targets` to authored doc links (x2).
- No follow-up code work: the doc now matches the tree. The sibling dev-wiki
  task 20260718-152214 (project-tour + development.md) should stay consistent
  with this README's crate table and tools list when it runs.
