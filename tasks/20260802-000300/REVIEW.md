# Review: Release v0.9.1

- TASK: 20260802-000300
- BRANCH: master

## Round 1

- REVIEWER: Codex, in-context release verification
- VERDICT: APPROVE

No findings. Release metadata changed only `Cargo.toml`, `Cargo.lock`, and
`CHANGELOG.md`; the lightweight tag resolves to that commit. The v0.9.0 News
post contains the point-release note, its rendered HTML was inspected, and web
CI passed.

Review exception: already landed release bookkeeping; no separate cold review
branch remained.
