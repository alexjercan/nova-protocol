# Review: wire sccache into the nix devshell (fast, safe worktree builds)

- TASK: 20260721-000229
- BRANCH: tooling/sccache-devshell

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. The load-bearing claim (sccache is active and the warm build is fast)
was INDEPENDENTLY reproduced: the reviewer re-ran `sccache --zero-stats` +
`cargo clean` + `cargo build` and measured 39s wall (cargo 38.21s) with sccache
reporting 633 requests / 517 executed / 517 hits / 0 misses (100% hit rate) -
non-zero requests prove `RUSTC_WRAPPER=sccache` is genuinely in effect, not a
no-op. This matches the implementer's 38s / 100% hits to the second, so the
result is double-verified. Devshell env confirmed (`sccache` on PATH,
`WRAP=sccache`, `INCR=0`); flake.lock UNCHANGED; safety smoke
(`content -- lint`) exit 0 on current code (not stale); the never-share
`CARGO_TARGET_DIR` rule PRESERVED and strengthened in AGENTS.md + LESSONS.md;
`npm run ci` green.

- [x] R1.1 (NIT) tasks/20260721-000229/TASK.md - the reviewer did not re-run the
  COLD build (evicting the shared cache is expensive); the 405s cold number is
  therefore not independently re-verified, only internally consistent.
  - Response: accepted, no change. The DoD's before/after is the WARM build,
    which two independent runs reproduced to the second; the cold number is a
    reference point, not the load-bearing claim.
- [x] R1.2 (NIT) AGENTS.md / development.md - the "shared cache at
  `~/.cache/sccache`" claim was verified literally true (`sccache --show-stats`
  reports that location).
  - Response: accepted, no change - the doc claim is accurate as written.

No BLOCKER/MAJOR. The wrapper is active, the warm build is genuinely ~10x
faster, the safety rule is intact, flake.lock is untouched, CI is unaffected.
