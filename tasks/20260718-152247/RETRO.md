# Retro: port nova_portal_gen to a Python build-time script

- TASK: 20260718-152247
- BRANCH: refactor/portal-gen-python (landed 36ffcff6)
- REVIEW ROUNDS: 2 (R1 out-of-context REQUEST_CHANGES: 2 MAJOR + 1 MINOR + 2 NIT; R2 in-session APPROVE)

See TASK.md for what changed + the parity evidence; process only here.

## What went well

- The parity-diff-as-oracle made the DoD self-verifying: `diff -r` of the Python
  output against the Rust generator over the real webmods, plus 10 rejection
  cases matched on exit code AND stderr, turned "byte-for-byte" from a claim
  into a check both the implementer and the reviewer re-ran independently. The
  finicky serde_json-pretty parity (field order, `ensure_ascii=False`, no
  trailing newline) held because it was diffed, not eyeballed.
- Delegating the implementation to a subagent in an isolated worktree kept the
  orchestration context lean, and the out-of-context reviewer then caught the
  subagent's two blind spots - exactly the structural value of the
  out-of-context default.

## What went wrong

- R1.1 (MAJOR): the removal-plan note claimed `webmods_validation` was the only
  test touching the generator and it needed no change - but the REAL consumer,
  `nova_assets/tests/portal_install.rs` calling `nova_portal_gen::generate()`
  via a dev-dep, was missed. Root cause: the "who drives the generator" sweep
  stopped at the deploy path + the obviously-named test, and did not grep the
  crate NAME across `Cargo.toml` dev-deps and `tests/`.
- R1.2 (MAJOR): the doc sweep updated mod-portal.md + README but missed
  guide-make-a-mod.md - the primary author-facing "how to publish" guide - which
  still told authors to run the Rust bin. Root cause: swept the portal-specific
  docs, not every doc that instructs the publish command.

## What to improve next time

- Before recording that a crate/symbol can be removed, grep the crate NAME
  across the whole workspace - `Cargo.toml` (deps AND dev-deps) and `tests/` -
  not just the production/deploy path; a dev-dependency test driver is a real
  consumer that blocks removal.
- A "replace command X with Y" doc sweep must grep the command/crate token
  across ALL doc surfaces (every wiki page, not just the topic page) - the same
  discipline as `doc-sweep-covers-source-doc-comments` from the prior task.

## Action items

- [x] LESSONS.md: added `removal-sweep-includes-dev-deps-and-test-drivers` (x1).
- [ ] Follow-up (recorded in TASK.md, not yet filed as a task): removing
  `nova_portal_gen` needs `portal_install.rs` ported to shell out to
  gen-portal.py first. File at the next release cut.
