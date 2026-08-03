# Review: Migrate nova_debug, nova_probe, and the example fleet onto nova_autopilot

- TASK: 20260802-183403
- BRANCH: refactor/autopilot-migration

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (MINOR) crates/nova_debug/src/harness.rs:83 - the new
  `nova_autopilot` dependency makes every bare ``[`nova_autopilot`]`` intra-doc
  link ambiguous (function vs crate); `cargo doc -p nova_debug --no-deps` emits
  four new `broken_intra_doc_links` warnings (lines 83, 108, 128 and the module
  header) that master did not have. Line 76 was already spelled
  ``[`nova_autopilot`](nova_autopilot())``; spell the other four
  ``[`nova_autopilot()`]``.
- [ ] R1.2 (MINOR) crates/nova_debug/src/harness.rs:266 - `nova_reel`'s public
  doc links ``[`reel_freeze_bodies`]``, which is private (line 314), producing a
  new `rustdoc::private_intra_doc_links` warning. Drop the link brackets (plain
  `reel_freeze_bodies`) or make the fn `pub`.
- [ ] R1.3 (NIT) crates/nova_debug/src/harness.rs:290 - when `beats` is empty
  the crate's `ScreenshotReelPlugin` warns and registers nothing, but
  `reel_freeze_bodies` is still added, so an empty reel silently statics the
  whole scene. Guard the `app.add_systems(Update, reel_freeze_bodies)` with
  `if !self.beats.is_empty()`.
- [ ] R1.4 (NIT) crates/nova_debug/src/harness.rs:301 -
  `scenario_camera_present` replaces an archetype-filtered query with
  `world.iter_entities().any(..)`, a full-world scan re-run every frame for the
  whole reel. Add a one-line comment noting the `&World` signature forbids
  `query_filtered` and the scan is deliberate, so a future reader does not read
  it as an oversight.
- [ ] R1.5 (NIT) crates/nova_debug/src/harness.rs:437 -
  `reel_beat_carries_the_output_path` asserts `beat.settle_frames ==
  NOVA_SCREENSHOT_SETTLE_FRAMES`. `reel_beat` never sets that field; the value
  comes from the crate's own `DEFAULT_SETTLE_FRAMES`, which is 30 by
  coincidence, not by contract. Assert against `ReelBeat::new("x").settle_frames`
  or drop the line, so an independent change to either constant does not fail a
  test about `reel_beat`.
- [ ] R1.6 (NIT) tests/examples_smoke.rs:196 - `DRIVERS` includes
  `ScreenshotReelPlugin`, but `bevy_common_systems` has no reel module at all
  (`src/debug/harness/` is `autopilot.rs` + `screenshot.rs`), so it cannot be
  one of the "names the bcs prelude ALSO exports" the comment above the list
  describes. Drop it from `DRIVERS`, or amend the comment to say the list also
  pins the crate-side reel type out of examples.

Round-1 reviewer: an out-of-context subagent with no sight of the implementing
session, given only the task ID, branch, worktree path, default branch, the
review dimensions and the finding format.

The recording pass re-derived the diff's load-bearing claim independently rather
than accepting it: `bevy_common_systems`'s `src/debug/harness/mod.rs:79-85`
glob-exports exactly `AutopilotLoop`, `AutopilotPlugin`, `HarnessCompletion` and
`ScreenshotPlugin`, and `nova_debug`'s prelude exports none of them - so a bare
name in an example did resolve to the bcs twin, as `DECISION.md` addendum 2
describes, and the new `examples_name_drivers_through_the_nova_harness` guard
covers precisely that set. The five rustdoc warnings in R1.1/R1.2 were
reproduced directly with `cargo doc -p nova_debug --no-deps`.

Proofs re-run by the recording pass, all green:

| Proof | Result |
| --- | --- |
| absence grep | exit 1, no hits |
| `rg '^nova_autopilot' <both Cargo.toml>` | both present |
| `cargo check --workspace --all-targets --features debug` | clean |
| `cargo test --test examples_smoke` (Xvfb) | 6 passed, 0 failed, 190s |
| `cargo run -p nova_probe -- run playable --fps` | verdict OK, 6/6 PASS |

- Process signal: the diff carries scope the plan did not list - ten examples
  moved from a bare `AutopilotPlugin` to the qualified path, four completion
  reach-ins rerouted, and a new `examples_smoke` guard test. All three are
  disclosed in Step 6 and `DECISION.md` addenda 1-2, and both breaks were found
  by RUNNING the fleet, not by `cargo check`. The plan's absence grep could not
  express either class; that is the reusable lesson, not the extra scope.
- Process signal: the DoD's absence grep had to be corrected mid-work (its bare
  `debug::harness` alternative also matched the `nova_debug::harness::` paths
  the task's own Notes require to survive, so it could never have gone green).
  A plan-time red-on-base check of that proof would have caught it.
- Step 4's literal text names `screenshot::SCREENSHOT_ENV` among the consts
  `nova_probe/native/env.rs` should write through, but `nova_probe` never sets
  `NOVA_SHOT`; the clause is vacuous rather than undelivered. No change needed.
- Stale `BCS_*` prose survives in `AGENTS.md`, `CHANGELOG.md`,
  `web/src/wiki/dev/*`, `.claude/skills/probe/SKILL.md`,
  `.github/workflows/ci.yaml` and `.gitignore`. This task's Notes deliberately
  defer the doc sweep to `20260802-183406`, whose repo-wide DoD grep covers all
  six. Not a finding here.

Pending user check (does not block the verdict):

- `manual:` run `NOVA_SHOT_DIR=target/reel NOVA_REEL=1 cargo run --example
  screenshot_reel --features debug` under Xvfb and confirm three PNGs land,
  framed as before. Not self-confirmed. Corroborating evidence only: the three
  expected PNGs are staged at 1920x1080 in the worktree's `target/reel/`, and
  `wiki-sections.png` reads as a clean game render with no dev overlays and no
  HUD chrome.
