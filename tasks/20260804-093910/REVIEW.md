# Review: Retire the mainline and POC example runs, reduce screenshots to capture-only

- TASK: 20260804-093910
- BRANCH: refactor/retire-example-runs

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (MINOR) .claude/skills/probe/SKILL.md:25 - the probe skill still
  advertises `gameplay` as a live PROBED category (`today:
  sections|gameplay|ui|perf`) after this diff deleted the `"gameplay"` row from
  `CATEGORY_POLICIES`, so a documented command (`probe run gameplay`) now
  errors; drop `gameplay` from that list.
  - Response:
- [ ] R1.2 (MINOR) .claude/skills/probe/SKILL.md:146 - the "extra depth" table
  still carries a `gameplay/broadside | a marker per script stage (11: picker
  -> defeat -> Retry -> acts -> victory)` row for an example this branch
  deleted; remove the row. With R1.1 this is the residual half of the doc
  sweep the close-out's Reflection already names - `web/src/wiki/dev/
  development.md` was caught, `.claude/skills/` was not.
  - Response:
- [ ] R1.3 (NIT) examples/screenshots/screenshot_orbit.rs:44 -
  `capture_settle_frames(capturing: bool) -> u32` is copied verbatim into three
  files (orbit:44, juice:47, combat:56) while `screenshot_nova_os` writes the
  same idea as `let after_capture = if capturing { 20 } else { 2 }` and
  `screenshot_ui` as a three-field `Settle` struct - three shapes for one idea
  in a change whose stated win is one idiom for `screenshots/`; collapse the
  three copies to the `let` form.
  - Response:
- [ ] R1.4 (NIT) examples/screenshots/screenshot_reel.rs:42 - deleting
  `SCENARIO_ID` left `const REEL_CONTENT_RON` butted against `fn main` with no
  blank line (rustfmt does not restore it); add the blank line.
  - Response:
- [ ] R1.5 (NIT) examples/ui/menu_scenarios.rs:70 - the `broadside` removal left
  two badly wrapped comment lines at :70 and :152; re-wrap to the file's width.
  - Response:

### Verification

- All four `cmd:` proofs re-run by both the out-of-context reviewer and this
  pass: no hits, all exit 0.
- `cargo fmt --check` clean; `cargo check --examples --features debug` clean
  (re-run by this pass).
- `cargo test --test examples_smoke --features debug` under Xvfb :99 - 7
  passed, 0 failed, including `screenshots_reach_playing_without_panic`,
  `catalog_matches_disk`, `every_category_has_a_probe_policy`.
- DoD 7 independently reproduced: the five converted producers all exit 0 and
  write exactly the 15 non-empty PNGs the record lists.
  `hud-nav-chips.png` and `nova-os-ship.png` opened and inspected - both chips
  present, RTT schematic renders, no black or torn frames, so the eager-
  completion risk the close-out flags is genuinely handled by the trailing
  capture-step holds.
- Every converted timeline re-derived beat-for-beat against master: `combat`
  preserves all 14 offsets and the 0.7s CTRL release gap, keeps beacon despawn
  after the radar-lock capture, and keeps one capture per step; `orbit`,
  `juice`, `ui` and `nova_os` carry their settle frames and waits over
  verbatim.
- Deletions confirmed on disk: `examples/gameplay/`, `examples/ui/
  nova_os_rtt_poc.rs`, `assets/shaders/nova_os_rtt_poc.wgsl`,
  `[package.metadata.nova_probe]`. Surviving `broadside`/`lifeline` hits
  repo-wide are shipped scenario CONTENT and its `nova_assets` tests - correct
  to keep.
- Re-derived independently by this pass: the `"gameplay"` row is gone from
  `CATEGORY_POLICIES` while `.claude/skills/probe/SKILL.md` still names it
  (R1.1/R1.2).
- Not verified here: `cargo clippy --examples` and `cargo test -p nova_probe`
  (CI owns those per repo rule); the close-out's clippy-clean and 102-test
  claims are unchecked by this review.
- Process signal: DECISION 1 pulls the `fps_exempt` deletion out of sibling
  task `20260804-094006`'s Steps into this branch. Recorded, with the
  consequence for `094006` spelled out - disclosed scope transfer, not creep -
  but `094006`'s Steps still list the edit and need re-reading as a
  verification.
