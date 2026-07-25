# Review: Bug: drawer tabs scroll instead of overflowing

- TASK: 20260725-163835
- BRANCH: fix/drawer-scroll-tabs

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Verification notes:

- `nix develop --command cargo test -p nova_gameplay drawer` passed.
- `nix develop --command cargo fmt --check` passed.
- `nix develop --command cargo check` passed.
- `npm run ci` in `web/` passed.
- `git diff --check master...HEAD` passed.
- `tatr check --ledger LESSONS.md` only reported lifecycle gaps before this
  review file and the retro existed.

Pending manual acceptance: open an overlong Flight Log and Objectives list and
confirm both stay inside their panels while scrolling.
