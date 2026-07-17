# Review: HUD health percent rounds a living sliver to 0%

- TASK: 20260716-165617
- BRANCH: fix/hud-health-percent-ceil

## Round 1

- VERDICT: APPROVE

Cross-repo change. The logic + tests land in bevy-common-systems (master,
commit fca377d, tag v0.19.1); nova's branch bumps the pin from v0.19.0 to
v0.19.1 across all five crates and the lockfile.

Verified:

- bcs `src/ui/health_display.rs` extracts a pure `display_percent(current,
  max) -> i32` with both guards: `max <= 0.0` returns 0% (kills the NaN%
  divide-by-zero on a section-less `Health{0,0}` root), and `0 < percent < 1`
  ceils to 1% (a living sliver never reads dead). Independently re-derived all
  seven boundary cases (0.4/230 -> 1, 2.29/230 -> 1, 2.3/230 -> 1, 3.45/230 ->
  2, 0/0 -> 0, 5/0 -> 0, -0.5/230 -> 0); every one matches its assertion.
- The four new unit tests are meaningful - each would fail with the fix
  deleted (old code renders "0" for the sliver and "NaN" for the zero-max
  root). `cargo test --lib health_display` in bcs: 4 passed.
- No display-format regression: old code formatted `f32::round()` (e.g.
  "100"), new formats the equal `i32` ("100").
- nova pin bump is complete - no stray `v0.19.0` bcs reference remains in any
  Cargo.toml or Cargo.lock; lockfile source resolves
  `tag=v0.19.1#fca377dd...`, matching the pushed bcs commit.
- CHANGELOG got a `[0.19.1]` Fixed section and the crate version bumped to
  0.19.1, consistent with the Bevy-minor-tracks-crate-minor scheme (a Bevy
  0.19.x-compatible patch stays on 0.19.x).
- `cargo fmt --check` clean; `cargo check --workspace` green. Full test suite
  deferred to CI per project policy.

No BLOCKER/MAJOR/MINOR/NIT findings. Clean diff.
