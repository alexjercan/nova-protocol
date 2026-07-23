# Review: Tag base storyline chapter-heads as Nova Protocol 1/2/3 + regen content

- TASK: 20260723-095909
- BRANCH: content/nova-protocol-campaign-tags

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

No BLOCKER / MAJOR / MINOR findings. Two non-blocking NITs (below).

Verified independently (out-of-context reviewer, re-confirmed in-session): the
three visible chapter-heads carry `campaign: Some((name: "Nova Protocol",
order))` with shakedown_run=1, broadside=2, lifeline=3 (exact, consistent
name); the hidden continuations `broadside_gunship`, `final_tally`,
`asteroid_next` stay untagged (`campaign: None` / no key, `hidden: true`). The
generated-content contract holds: `content_ron_parity` passes 2/2, and
re-running `content -- gen` leaves `git status` CLEAN - the committed RON
already matches the builders, both in the same change. `content -- lint` clean
(0 errors, 0 warnings, 13 scenarios audited); `cargo fmt --check` clean.

The TASK.md close-out's inherited-failure claim was verified: the reviewer ran
`content_lint_gate::target_mode_lints_one_mod_in_repo_or_external` in the main
checkout (confirmed on `master`) and it FAILS identically there. Genuinely
inherited (about the-ledger, untouched by this branch), correctly filed as
20260723-103523.

DoD item 3's `manual: diff review` proof is satisfied by this review's diff
inspection (only the three intended scenarios tagged) - no pending user gate.

- [ ] R1.1 (NIT) tasks/20260723-095909/TASK.md:6 - tatr normalized the TAGS
  line spacing (`v0.8.0,scenario,content` -> `v0.8.0, scenario, content`).
  Cosmetic, harmless; no action.
  - Response: acknowledged, no change (tatr owns the header).
- [ ] R1.2 (NIT) shakedown.rs vs broadside.rs/lifeline.rs - shakedown uses
  `..Default::default()` for trailing fields while the other two spell them
  out. Pre-existing builder-style inconsistency, not introduced here.
  - Response: acknowledged, pre-existing; out of scope for this task.
