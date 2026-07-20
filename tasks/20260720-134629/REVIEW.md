# Review: fix the 108 broken intra-doc links

- TASK: 20260720-134629
- BRANCH: docs/rustdoc-links

## Round 1

- VERDICT: APPROVE

A mechanical doc-comment sweep with a definitive verification: the strict
`cargo doc` re-run to zero IS the proof the fix is complete and correct.

- **108 -> 0, verified.** `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --no-deps --features debug` exited 0 with no warnings (it errored on 108
  before). This is the DoD, machine-checked, not eyeballed.
- **Pure doc sweep, no code risk.** `git diff` is 106/106 line replacements with
  0 non-`///`/`//!` lines changed - no code, no public API, no behavior touched.
  So the change cannot break a build beyond what `cargo doc` itself checks, and
  it did check.
- **Scripted edits were verified, not trusted.** The un-link script keyed each
  edit on the exact (file, line, name) from the warning and reported 0 "needle
  not found" misses; a sample of edited hunks was re-read (e.g. audio.rs:35 "see
  `compute_thruster_hum_volume`)." reads fine as prose); and the strict re-run
  catches anything the script under- or over-fixed.
- **Honest fixes, no API widening or invented targets.** The 88 private-item and
  11 unresolved refs were UN-LINKED (name kept in prose) rather than making
  internal systems public or guessing a target - the two things the DoD forbids.
  Ambiguous `screen_indicator` -> the module (`mod@`), matching the prose "the
  screen_indicator widget [subsystem]"; redundant explicit targets -> the
  shortcut that already resolves.

- [ ] R1.1 (NIT) A private item referenced from both a public doc (now
  un-linked) and a private-item doc (still `[`X`]`, which rustdoc neither
  renders nor warns on) now reads with mixed link styles in the SOURCE. Purely
  cosmetic - the rendered public docs are consistent and warning-free, and
  un-linking every private-context occurrence too would be churn for no rendered
  benefit. Left as-is deliberately.
