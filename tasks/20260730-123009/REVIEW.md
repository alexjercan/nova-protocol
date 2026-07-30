# Review: combat lock lets go of locked enemies (intended decay or defect?)

- TASK: 20260730-123009
- BRANCH: fix/combat-lock-decay

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

Reviewer ran, in the worktree: `git diff master...HEAD` (read in full);
`cargo test -p nova_gameplay --lib input::targeting::tests` (54 passed);
`cargo test -p nova_gameplay --lib hud::torpedo_target` (15 passed);
`cargo check --all-targets` (clean); the DoD 6 cmd proof
`grep -rn "firing joins in" crates/` (no output, exit 1 - PASSES); and a doc
sweep of `web/src/wiki`, README, CHANGELOG and crate docs. Cross-checked the
NOTES.md numbers against what the rigs actually assert. Did not run the probe
(DoD 5) - the implementer's run was already in flight, so its verdict is
unconfirmed by the reviewer.

Pending user checks, not findings: DoD 2 (owner reads NOTES.md) and DoD 4's
manual half (owner sees the cue in flight).

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/torpedo_target.rs:368 - the
  wind-down pulse used absolute session time as its phase (`elapsed_secs * hz`)
  while `hz` itself swept 1.5 -> 6.0 Hz, so the instantaneous rate is
  `hz + elapsed * d(hz)/dt` and grows with uptime: ~29 pulses over the window
  at t=0, 10 at t=60 s (a smooth slide, no pulse), 118 at t=300 s (frame-rate
  aliasing). A real playthrough reaches its first decay window well after
  t=0, so the shipped cue was essentially never the described one - and
  CHANGELOG, wiki and NOTES all claimed behaviour the code did not produce.
  Fix: drive the phase from the decay clock, integrating the chirp.
  - Response: Confirmed independently before adopting - simulated the shipped
    formula at 60 fps and counted local maxima over the window: 29 at t=0, 74
    at t=10 s, 10 at t=60 s, 118 at t=300 s. Real. `wind_down_alpha` now takes
    `idle_secs` alone and integrates the linear chirp
    (`CALM * x + (URGENT - CALM) * x^2 / (2 * WINDOW)`), so the sweep is
    genuinely 1.5 -> 6 Hz, 18 pulses across the window, identical at 60/144/600
    fps and at any uptime. `wind_down_reticle_on_decay` no longer takes `Time`.
- [x] R1.2 (MAJOR) crates/nova_gameplay/src/hud/torpedo_target.rs (the
  `the_wind_down_*` tests) - neither test could catch R1.1: the pure-function
  test only sampled `elapsed_secs` in `[0, 1)`, the one regime where the
  formula behaved, and the live-node test ran at `Time::default()`
  (`elapsed = 0`, `dip = 0`) so it never exercised the pulse at all.
  - Response: Fixed. Added
    `the_wind_down_pulse_is_the_same_at_any_uptime_or_frame_rate`, which
    counts pulses across the whole window at 60/144/600 fps against the count
    the constants promise and asserts the pulse quickens (more maxima in the
    window's second half than its first); rewrote the envelope test to check
    successive pulse PEAKS fall; and extended the live-node test to walk the
    last second frame by frame at 0 s and 300 s uptime, asserting the same
    5 pulses both times. Verified the bar: with the old formula restored all
    three tests fail (`at 60 fps the window showed 148 pulses, not the ~18 the
    1.5 -> 6 Hz sweep promises`), and pass with the fix.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/input/targeting.rs:26 - the doc
  claimed every drop is "answerable from a log", but nothing reads
  `CombatLockDropped` and nothing logs it, so in a shipped build the answer is
  in no log.
  - Response: Made the doc true rather than weakening it. The three write
    sites now go through `report_combat_lock_drop`, which emits a `debug!`
    naming the target, the branch and the idle clock alongside the message.
    Module doc and NOTES.md reworded to say exactly that.
- [x] R1.4 (NIT) crates/nova_gameplay/src/input/targeting.rs:713-717 - the
  `OutOfRange`/`TargetGone` split infers from `q_candidates.get`, but
  `collect_lockable` also rejects for non-range reasons, so those would be
  mislabelled `OutOfRange` while the enum doc promises the range gate
  specifically.
  - Response: Widened the doc to "no longer passes the candidate gate - in
    practice its range gate", noting that the gate's other rejects (the ship
    itself, an uncommitted torpedo) cannot hold a lock in the first place and
    so never reach the branch. Kept the classification as is: re-running the
    distance check would duplicate `collect_lockable`'s gate for a case that
    cannot occur.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

Reviewer ran `git show 4ef68731` (read in full),
`cargo test -p nova_gameplay --lib hud::torpedo_target` (16 passed, up from
15), `... --lib input::targeting::tests` (54 passed),
`cargo check --all-targets` (clean) and `git status --porcelain` (empty), and
re-derived the chirp maths and the R1.2 bar numerically from the shipped
constants rather than trusting the response commit.

Per-finding confirmations (the round-1 checkboxes are ticked on these):

- R1.1 CONFIRMED RESOLVED. `wind_down_alpha` takes `idle_secs` alone and
  `wind_down_reticle_on_decay` no longer has a `Res<Time>` param, so a
  session-time dependency is structurally unrepresentable. The maths checks
  out independently: `d(cycles)/dx = CALM + (URGENT - CALM) * x / W` is
  exactly 1.5 Hz at x=0 and 6.0 Hz at x=W, `cycles(W) = 18.75` matches the
  mean-rate closed form, the boundary is continuous in both directions, and
  alpha is bounded in [0.1375, 1.0] (measured minimum 0.1418).
- R1.2 CONFIRMED RESOLVED and the bar is met. Independently measured 18
  pulses at 60, 144 and 600 fps against the test's `expected = 18` (so the
  `<= 1` tolerance is not doing the work), 6 early vs 12 late maxima for the
  quickening assertion, and a strictly falling 18-peak envelope. With the old
  formula restored the reviewer measures 135 / 141 / 141 pulses and 11 at
  300 s uptime - each failing by a wide margin, so all three assertions are
  load-bearing. (The reviewer's 135 at 60 fps differs from the implementer's
  148; that is a different choice of how the restored `elapsed` is fed and
  does not change the verdict.)
- R1.3 CONFIRMED RESOLVED. All drop sites funnel through
  `report_combat_lock_drop`, so the module doc's claim is now true of a
  shipped build.
- R1.4 CONFIRMED RESOLVED.

No new findings.

- The probe evidence in NOTES.md was taken at 17:26, before the R1.1 response
  commit landed at 17:38, so it exercised the pre-fix tree. The reviewer
  judged this low risk (the response commit changes only the reticle's alpha
  maths and adds a `debug!`) but flagged it as a one-command re-run; the
  implementer re-ran `cargo run -p nova_probe -- run broadside` on the final
  tree rather than ship evidence from a superseded commit. Result recorded in
  NOTES.md.
- The live-node test's `for uptime in [0.0, 300.0]` loop is tautological
  against the current code (nothing reads `Time` any more); it stands as a
  forward-guard against reintroducing a render-clock dependency.
- Open user checks, not findings: DoD 2 (owner reads NOTES.md) and DoD 4's
  manual half (owner sees the wind-down in flight).
