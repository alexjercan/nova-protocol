# Fix stale content_lint_gate test: ledger ch4 'mutually exclusive' warn gone after ch4 rework

- STATUS: OPEN
- PRIORITY: 5
- TAGS: v0.8.0,bug,testing,content

## Story

As a developer, I want `cargo test -p nova_assets` green on master, so the
content lint gate is a trustworthy guard. The integration test
`content_lint_gate::target_mode_lints_one_mod_in_repo_or_external` currently
FAILS on master (confirmed 20260723): it asserts the-ledger lint surfaces a
ch4 warn whose message contains "mutually exclusive", but `lint_target("the-
ledger")` now returns `[]` for that expectation, panicking at
`crates/nova_assets/tests/content_lint_gate.rs:48`.

Root cause (to confirm): the ch4 ending rework on 2026-07-22 (commit 33dc293f,
"diverging ch4 endings - burn avoids the Auditor", task 20260722-214110)
restructured `webmods/the-ledger/ledger_ch4.content.ron` and removed whatever
produced the "mutually exclusive" warn; the test's expectation was not updated
in the same change. This is INHERITED, unrelated to the campaign-metadata run
(umbrella 20260723-093914) that discovered it - task B's own guards
(content_ron_parity, content lint) pass.

## Steps

- [ ] Reproduce: `cargo test -p nova_assets --test content_lint_gate
      target_mode_lints_one_mod_in_repo_or_external` fails at line 48.
- [ ] Determine whether the ch4 "mutually exclusive" warn SHOULD still exist:
      did the rework legitimately remove the mutually-exclusive branch (test is
      stale -> update the assertion to the warn ch4 actually emits now, or to
      another known the-ledger warn), or did it accidentally drop a real lint
      signal (fix the content/lint)?
- [ ] Update the test to assert a currently-real the-ledger finding (keeping
      the "target mode returns only the-ledger, non-Error findings" intent), or
      fix the content if a real warn was lost.
- [ ] Green `cargo test -p nova_assets`.

## Definition of Done

- `cargo test -p nova_assets --test content_lint_gate` passes. (cmd: that test)
- Full `cargo test -p nova_assets` green. (cmd: `cargo test -p nova_assets`)
- The test asserts a finding the-ledger actually emits today, not a stale one.

## Notes

- Discovered during umbrella 20260723-093914 task B (095909) verify step; that
  branch does not touch the-ledger and is not the cause.
- Fails identically on master (checked out master, same panic at line 48).
