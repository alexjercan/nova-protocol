# Review: pre-commit fmt guard (20260722-183022)

- VERDICT: APPROVE

Round 1, out-of-context reviewer (fresh context, re-derived every load-bearing
claim independently; ran the test and read the sprout source directly).

## Verified

- **Correctness (hook / setup / test): clean.** The staged-`.rs` detection
  `git diff --cached --name-only --diff-filter=d -z | grep -qz '\.rs$'` behaves
  under `set -euo pipefail` (grep's no-match exit is consumed by `if !`, so a
  no-Rust/empty-staging commit hits `exit 0`). `fmt_check` is assigned in every
  reachable branch before expansion (no `-u` risk). The
  `if ! (cd "$repo_root" && "${fmt_check[@]}")` subshell's non-zero exit is
  caught by the `if`, not fatal. Hook + scripts are executable.
- **Land-path coverage: confirmed.** sprout `cmd_land` squashes with
  `git merge --squash` then commits with a plain `git commit --quiet`
  (no `--no-verify`/plumbing), so the pre-commit hook fires at land; on hook
  failure it rolls the main checkout back (`git reset --merge || --hard`).
- **CI parity: identical.** Both CI "Formatting" and the hook run
  whole-workspace `cargo fmt --check` from repo root. The known `cargo fmt`
  module-reachability limitation is therefore the same in both - not a
  regression vs CI. Docs call it "parity," not "stronger" - accurate.
- **Test is real:** `scripts/test-fmt-hook.sh` drives the shipped hook in a
  throwaway repo, cleans up via `trap ... EXIT`, asserts refuse/accept/skip.
  Ran green (exit 0).
- **CI step ordering:** "Fmt hook self-test" runs after the toolchain install,
  so `cargo fmt` is available.
- **Docs accuracy:** AGENTS.md / CHANGELOG.md / LESSONS.md / hook comments all
  match observed behavior. No overclaims.

## Non-blocking (NIT, no action required)

- `.githooks/pre-commit` nix-develop fallback is not exercised by the self-test
  (CI and this env both have bare cargo on PATH); it reads correctly but is
  unverified in practice.
- `scripts/setup-hooks.sh` `chmod +x .githooks/*` is belt-and-suspenders over
  already-correct tracked exec bits.

No BLOCKER/MAJOR/MINOR findings. The guard closes the land-time gap, the test
drives the real hook and passes, and the docs are honest.
