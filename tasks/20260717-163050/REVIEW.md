# Review: Transition pacing

- TASK: 20260717-163050
- BRANCH: feature/transition-pacing

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_scenario/src/actions.rs:542, crates/nova_menu/src/lib.rs:811 - an
  authored out-of-range duration panics `Timer::from_seconds` at runtime.
  `Duration::from_secs_f32` panics on non-finite or too-big values ("can not
  convert float seconds to Duration"), and both new fields reach it unguarded:
  `NextScenario((.., delay: Some(1e30)))` parses fine (finite f32) and crashes
  inside the action apply the frame the event fires; `Outcome((.., auto_advance_secs:
  Some(1e300)))` crashes `auto_advance_outcome` (`secs.max(0.0)` guards NaN and
  negatives only - `1e300 as f32` is `inf`). This is moddable content crashing the
  app mid-frame, in a codebase that just hardened the identical case: the sibling
  `dwell` field from the same pacing initiative (task 20260717-163033) got BOTH a
  runtime clamp before `Timer::from_seconds` (comms_panel.rs:253-257) and a range
  lint (lint.rs:283-295). Suggested change: clamp at both construction sites
  (mirror the comms clamp; e.g. `delay.min(SOME_MAX)` after the `> 0.0` filter,
  and clamp the f64 before the cast in the menu), plus a range lint arm for
  `delay` and `auto_advance_secs` mirroring the dwell lint.
  - Response: fixed at both sites - the action apply finite-checks and caps
    at NEXT_SCENARIO_DELAY_MAX_SECS (300), the menu system finite-checks and
    clamps at OUTCOME_AUTO_ADVANCE_MAX_SECS; panic-regression tests
    (absurd_delays_are_capped_not_panics) and range lint arms with tests.

- [x] R1.2 (MINOR) crates/nova_scenario/src/world.rs:136-139 - the load-bearing
  no-early-return invariant has no regression test. Mutation probe M5: replacing
  the `if !still_waiting` guard with an early `return` while waiting (exactly the
  refactor the comment warns against - it starves every queued spawn/effect for
  the whole delay window) passes the ENTIRE nova_scenario suite green (110 + 1).
  NOTES.md honestly admits this was "caught by reading the system's tail before
  committing, not by a test". Suggested change: extend
  `a_delayed_cut_holds_then_switches` (or add a sibling) to `push_command` during
  the delay window and assert the command applies BEFORE expiry.
  - Response: fixed - the_command_flush_runs_through_the_delay_window pins
    the invariant (a command queued mid-window applies on the next sync
    while the cut still holds); your M5 early-return mutation now fails it.

- [x] R1.3 (MINOR) crates/nova_scenario/src/world.rs:254-262 -
  `release_lingering_next` ignores an armed `next_scenario_delay`, which makes an
  explicit "advance now" input a silent no-op in two reachable shapes: (a) during
  an un-overlaid delay window, Enter/DPadDown routes `decide_advance ->
  ReleaseQueued` (loader.rs:953; `has_queued` is true because the request now
  survives in `next_scenario` for the whole window) and flips an
  already-false `linger` to false - the player pressed advance and nothing
  happens; (b) cross-handler Outcome + delayed non-lingering cut (handler A fires
  the outcome, handler B the delayed cut): the overlay's pause freezes the timer,
  and Continue - the same `release_lingering_next` path - is a dead button; only
  the always-present Main Menu button escapes. The new lint only covers the
  same-handler pair. Suggested change: have `release_lingering_next` also set
  `next_scenario_delay = None` (release means "go now"), which makes Enter
  fast-forward a delayed cut and revives Continue in shape (b); alternatively
  document both no-op shapes if holding the authored beat against player input is
  intended.
  - Response: fixed - release_lingering_next clears next_scenario_delay:
    Enter during a window skips the beat, and a cross-handler overlay's
    Continue works (release -> no timer -> the next sync switches).
    Pinned by release_skips_the_pending_delay.

- [x] R1.4 (NIT) crates/nova_scenario/src/actions.rs:470 vs 528 - the two new
  authored durations disagree on width: `delay: Option<f32>` vs
  `auto_advance_secs: Option<f64>`. The f64 is immediately cast to f32 for the
  Timer (lib.rs:811), so the extra width buys nothing. Suggested change: make
  both f32.
  - Response: declined with reasoning - auto_advance_secs stays f64 to match
    the sibling orbit_hold_secs/lock_refire_secs fields the parallel task
    landed this afternoon (config-side f64 is now the local convention);
    the cast is capped and finite-checked.

- [x] R1.5 (NIT) crates/nova_scenario/src/lint.rs:179-198 - `delay` on a
  `linger: true` request is silently ignored (the apply's `_ => None` arm; the
  field doc at actions.rs:521-523 says as much, and behavior matches: a released
  linger switches instantly with no timer). A dead authored field is the same
  class of trap the new lint exists for. Suggested change: a cheap lint arm that
  WARNs on `linger: true` + `delay: Some(_)`.
  - Response: fixed - the dead-field lint arm warns on linger + delay.

- [x] R1.6 (NIT) web/src/wiki/dev/scenario-system.md:184-185,
  crates/nova_scenario/src/actions.rs:520 - "ticks on the scenario's
  (pause-frozen) clock" is loose: the delay ticks on generic/virtual `Time` in
  `state_to_world_system`, not on the `scenario_elapsed` clock variable this
  repo elsewhere names with precision (task 20260717-151537 built a whole
  discipline around which clock derives what). Behaviorally equivalent under
  pause (both freeze), and the player-facing claim is verified true, but an
  author could read it as gate-on-`scenario_elapsed` semantics. Suggested
  change: "ticks on virtual (pause-frozen) time".
  - Response: fixed - the wiki says virtual (pause-frozen) time.

### Verification record

Adversarial re-derivations (all against the code as written):

- Consume-once + second request: the apply assigns `next_scenario_delay`
  UNCONDITIONALLY (actions.rs:540-546, `_ => None`), so last-wins is wholesale.
  Delayed replaced by undelayed: timer cleared, instant switch, no stale defer.
  Delayed replaced by delayed: fresh timer, clock restarts (documented). Delayed
  replaced by lingering: timer cleared, `request.filter(|r| !r.linger)` skips the
  whole block, overlay path untouched. Verified by reading every arm.
- Teardown: `clear()` nulls both fields (world.rs:208-209), pinned by
  `clear_drops_the_pending_delayed_cut`.
- No-early-return: the command flush is the same fn's tail (world.rs:168-181),
  outside the `if !still_waiting` block; it runs every pass during the window.
  The tick's `resource_mut` deref also keeps the PostUpdate chain's
  `resource_changed` run-condition warm, so the timer keeps ticking even if the
  OnUpdate pulse were quiet. No test guards the invariant (R1.2).
- linger+delay authored, then released: apply arms the timer only for `!linger`,
  so after `release_lingering_next` the non-lingering request meets a None timer
  -> instant switch. Matches the field doc ("meaningless with linger: true").
- Clock claims, from bevy_time 0.19 source (registry, lib.rs:146-186,
  virt.rs:280-284): `time_system` updates `Time<Real>` unconditionally every
  frame, then `update_virtual_time` derives virtual from real (pause zeroes the
  virtual delta) and sets generic `Time = virt.as_generic()`. So the delayed cut
  (generic Time in PostUpdate) freezes under `pause_clocks` /
  `sync_outcome_pause` (both routes pause `Time<Virtual>`), and the timed banner
  (`Time<Real>`) keeps advancing under the overlay's pause. Both wiki claims
  ("a player pausing holds the cut", "the pause stops virtual time, not the
  wall clock") verified TRUE. The world.rs test's 0.25s-per-update comment is
  the `Time<Virtual>` default `max_delta` clamp on the 0.5s manual steps -
  checked, the arithmetic holds (4 updates = 1.0s < 2.0s hold; +8 = 3.0s fired).
- Timed banner idle shapes: Local clock resets on `outcome.is_changed()` AND on
  the missing/non-lingering chain branch; no queued chain (end-of-story Victory)
  idles forever, matching the field doc. NaN/negative `auto_advance_secs` are
  guarded by `max(0.0)` (f64::max returns the non-NaN arg); non-finite/huge are
  NOT (R1.1). Zero-duration Timer finishes on its first tick: immediate advance,
  sensible.
- Lint: `EventActionConfig` is a flat enum (no nested action containers), so the
  per-event `any()` scan is complete; ONE warn per offending handler (single
  push per event), pinned by the `issues.len() == 1` asserts.

Mutation probes (each applied, run, reverted; tree clean after):

- M1 (delete the delay tick, `still_waiting` always false):
  `a_delayed_cut_holds_then_switches` FAILED at the FIRST assert - "the delayed
  cut must NOT switch inside its window (the pre-change instant cut fails
  here)", left: 1, right: 0. The fail-first claim is real.
- M2 (delete the arming in the action apply): 2 FAILED -
  `a_delayed_cut_holds_then_switches` + `clear_drops_the_pending_delayed_cut`
  (108 passed).
- M3 (disable the lint arm): `outcome_with_hard_switch_in_one_handler_warns`
  FAILED.
- M4 (delete `release_lingering_next()` in `auto_advance_outcome`):
  `auto_advance_releases_the_lingering_switch_after_real_seconds` FAILED.
- M5 (early-return while waiting, the starvation refactor): ALL GREEN, 110
  passed - the invariant is untested (R1.2).

Command runs (worktree, verbatim result lines):

- `cargo test -p nova_scenario --features serde`:
  `test result: ok. 110 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.70s`
  (+ skybox e2e: `test result: ok. 1 passed; 0 failed; ...`). Without
  `--features serde` the lib test target does not compile (known quirk,
  confirmed while probing).
- `cargo test -p nova_menu`:
  `test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s`
- `cargo run -p nova_assets --bin content_lint`:
  `content_lint: clean (1 warning(s))` (the pre-existing ledger_ch4 auditor
  double-spawn warning; nothing from this change).
- `cargo test -p nova_assets --test content_ron_parity`:
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- `cargo run -p nova_assets --bin gen_content`: rewrote the content files
  byte-identical; `git status --porcelain` empty afterwards - the
  `skip_serializing_if` claim holds, committed RON unchanged.
- `cargo check --workspace --all-targets`: `Finished \`dev\` profile` (green).
- `cargo fmt --check`: clean.

Docs honesty: wiki three-gears section matches verified behavior; CHANGELOG
entry accurate; NOTES' queue-starvation claim confirmed by reading (and its
"not by a test" admission confirmed by M5). Per standing instruction the full
suite runs on CI; locally only the targeted packages above were run.

## Round 2

- VERDICT: APPROVE

Per-finding verification (fixes at cc13dcc6, re-verified against the code and
by re-running the Round 1 probes):

- R1.1 CONFIRMED. Action apply guards with `delay > 0.0 && delay.is_finite()`
  then `.min(NEXT_SCENARIO_DELAY_MAX_SECS)` (actions.rs:549-551) - NaN fails
  `> 0.0`, inf fails `is_finite`, 1e30 caps at 300. The menu system
  finite-checks first (`!secs.is_finite()` resets and returns, catching the
  f64 NaN/inf before any cast) then `clamp(0.0, OUTCOME_AUTO_ADVANCE_MAX_SECS)
  as f32` (lib.rs:810-817) - every f64 input now reaches
  `Timer::from_seconds` finite and in [0, 300]. Pinned by
  `absurd_delays_are_capped_not_panics` (1e30 capped, inf arms nothing); the
  lint range arms warn on the same values (`pacing_field_ranges_warn`: 1e30
  delay and inf auto_advance both warn, sane values clean, warn-only - no new
  errors on existing content). Boundary check: `0.0..=MAX` admits an authored
  0.0 while the warn text says "(0, N]" - divergent only at exactly 0.0,
  which never triggers the message and is documented-instant, so no
  user-visible inconsistency. The delay warn threshold (60) vs runtime cap
  (300) split is stated in the warn text itself. Sound.
- R1.2 CONFIRMED by mutation. Re-ran the M5 probe (early `return` while
  waiting): `the_command_flush_runs_through_the_delay_window` now FAILS -
  "a command queued mid-window must apply long before the cut (an early
  return while waiting starves the flush)" - and the suite is green with the
  mutation reverted. The invariant is pinned.
- R1.3 CONFIRMED. `release_lingering_next` now clears `next_scenario_delay`
  (world.rs:255-265): Enter during a delay window fast-forwards the cut, and
  the cross-handler overlay's Continue can no longer be a dead button. Pinned
  by `release_skips_the_pending_delay`. Interplay re-checked: the apply still
  overwrites both fields wholesale, so last-wins is unaffected, and the timed
  banner's release path only ever meets a None timer (lingering requests never
  arm one).
- R1.4 DECLINE ACCEPTED. Verified the claimed convention:
  `orbit_hold_secs: Option<f64>` and `lock_refire_secs: Option<f64>` in
  crates/nova_scenario/src/objects/spaceship.rs:65,118 - config-side f64 is
  the established shape, and the cast site is now capped and finite-checked
  (R1.1), which was the substantive risk.
- R1.5 CONFIRMED. The dead-field arm warns on `linger: true` + `delay:
  Some(_)` (lint.rs, NextScenario arm), covered by
  `pacing_field_ranges_warn` ("dead" message asserted).
- R1.6 CONFIRMED for the wiki (scenario-system.md:184-185 now says "virtual
  (pause-frozen) time"). Residual, non-blocking: the field doc at
  crates/nova_scenario/src/actions.rs:529 still says "the scenario's
  (pause-frozen) clock" - same NIT-level looseness, one line, fine to fold
  into any future doc pass. Accepted.

No new findings introduced by the fixes: the new lint arms are warn-only
(content_lint on the shipped tree stays clean at the one pre-existing
ledger_ch4 warning), the menu's nova_scenario prelude import is an existing
dependency edge, and the release-clears-delay change only strengthens the
explicit-advance paths.

### Verification record

- M5 rerun (early-return mutation, applied and reverted):
  `test world::tests::the_command_flush_runs_through_the_delay_window ... FAILED`
  with the starvation message above; tree clean after revert.
- `cargo test -p nova_scenario --features serde`:
  `test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.68s`
  (+ skybox e2e `1 passed`) - 110 -> 114, exactly the four new pins
  (flush-window, absurd-delays, release-skips, pacing-ranges).
- `cargo test -p nova_menu`:
  `test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s`
- `cargo run -p nova_assets --bin content_lint`:
  `content_lint: clean (1 warning(s))` (pre-existing ledger_ch4 auditor
  warning only - the new range/dead-field arms fire nothing on shipped
  content).
- `cargo fmt --check`: clean.
