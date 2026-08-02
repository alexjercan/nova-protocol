# Retro: Move the screenshot reel driver into nova_autopilot behind caller hooks

- TASK: 20260802-183349
- BRANCH: feature/autopilot-reel-port
- REVIEW ROUNDS: 1

## What went well

Round 1 APPROVE with no BLOCKER or MAJOR. The cause is upstream of the
implementation: DECISION.md pre-answered, at plan time, every design question a
reviewer would raise (D1 drop `ReelCamera` rather than wrap it, D3 no second
timeout beside the completion deadline, D4 no stand-down, D5 the separate test
binary), each with the rejected alternative and its cost. Review had nothing
left to relitigate and could spend the round on port fidelity and falsifying
the tests instead.

Planning also caught an UNRUNNABLE DoD guard before any code was written: the
old `! rg -n "nova_" Cargo.toml` boundary check matches the crate's own
`name = "nova_autopilot"` line, so it was red on base for the wrong reason and
could never go green. Replaced with the anchored `^(nova_|...)` form plus a
`test -f` so a missing manifest cannot pass it vacuously.

## What went wrong

Nothing lifecycle-shaped. Two implementation-time surprises, both in the test
rig and both recorded in TASK.md's close-out rather than discovered in review:

- Bevy's render app despawns a served `Screenshot` entity. A headless test that
  only triggers `ScreenshotCaptured` leaves the request behind, so an "exactly
  one capture outstanding" assertion silently counts stale entities. The rig's
  `land_capture` had to despawn as the render app does.
- The frame after a capture lands only ADVANCES the index; the next beat's
  `apply` runs the frame after that. The first serialization test was off by
  one frame per beat.

The two open review findings are docs/test-hygiene, not defects: the `ready`
docs say "gates the first beat" while `reel_drive` re-evaluates the predicate
every frame (a faithful port of the old camera probe, but undocumented), and
the `capture_path` unit test skips its relative-path assertion outright when
`NOVA_SHOT_DIR` is ambiently set instead of asserting against the observed env.

## What to improve next time

A hook whose evaluation cadence differs from the thing it gates needs that
cadence in its doc line at design time. D3 settled *what* the predicate takes
(`&World`) and *what happens on never-ready* (the completion deadline), but not
*how often it is asked* - so the docs inherited the source comment's framing
("wait for the first beat") while the code kept the source's every-frame check.
One sentence in the decision record would have carried into both doc sites.

## Action items

- Fold review R1.1 (document the `ready` predicate's every-frame evaluation at
  both doc sites) and R1.2 (make the `capture_path` test assert against the
  observed env instead of skipping) into `20260802-183403`, which already owns
  the reel's caller migration and touches these files.

## Process signals

- Context: no threshold crossing, compaction warning, or delegation. The
  implementation and the review ran in separate sessions by design (the review
  skill's out-of-context default), which is what let round 1 be an independent
  read rather than a self-check.
- Breadth: ~750 lines, of which 311 is a new test binary and 386 the ported
  module. A port of one cohesive driver; no independently landable split was
  missed. Deliberately deletes nothing from `nova_debug`, which is what keeps
  the branch landable alone.
- Churn: none. See "What went well".
