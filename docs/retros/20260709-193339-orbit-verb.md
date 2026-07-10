# Retro: ORBIT autopilot verb - circularize and station-keep

- TASK: 20260709-193339
- BRANCH: orbit-verb (squash-merged to master as 51c9a3d)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 1 BLOCKER + 3 MINOR,
  round 2 APPROVE)

Second half of the v0.5.0 gravity arc; closes the spike's recommendation D.

## What went well

- **One seam-map pass, zero new actuator code.** Reading the whole
  autopilot/input/HUD seam map before touching anything meant the verb
  landed as pure goal computation (sticky plan + desired velocity) on the
  existing machinery. The "one rule flies every maneuver" architecture
  absorbed a third verb without a single change to allocation, alignment,
  spool, or breakout.
- **Playtest reports were folded in at the right altitude.** The lock-on
  regression (static well sources unlockable) was diagnosed to the
  previous task's review fix, fixed as a prerequisite step on this branch
  with a regression test, and the ORBIT approach flow depends on it - the
  bug report and the feature belonged together.
- **The adversarial review caught a CI-red blocker the local policy
  hides.** autopilot_system gained a Res<GravitySettings> dependency; the
  AI patrol physics test builds the flight plugin standalone and panics on
  the missing resource. The affected-modules test run
  (flight/gravity/targeting/HUD) did not include input::ai, so
  implementation missed it; the reviewer reasoned from "who else builds
  this plugin" and the failing test confirmed it in one run.
- **Cross-context input collision found by reading, not playing.** The
  pad South binding collided with the scenario-advance confirm in a
  different crate (nova_scenario). Greping every binding in the workspace
  before picking a button is cheap; shipping a parking maneuver that skips
  scenarios is not.

## What went wrong

- **R1.1 (BLOCKER): a new system dependency, an old test harness.** Root
  cause: when a system gains a resource parameter, every app that
  registers that system needs the resource - and the flight system is
  registered by three harnesses (game, flight tests, AI tests). The
  implementation updated the two it was already editing and missed the
  third. The skip-local-tests policy makes this class of miss invisible
  until CI unless the reviewer hunts for it.
- **R1.4: the "collapse to clearance" fallback was tested but not
  thought through.** The tiny-well unit test faithfully pinned a behavior
  (ring outside the SOI) that is physically incoherent - a test asserting
  a bad design decision makes it look deliberate. Writing the test forced
  the numbers out but not the question "is this outcome sane?".

## What to improve next time

- **When a system function gains a parameter, grep for every place the
  system (or its plugin) is registered** - game wiring, its own test
  harness, and other modules' harnesses - and run those modules' tests.
  "Affected modules" means callers of the changed signature, not files in
  the diff.
- When a helper has a degenerate-input fallback, ask what the player
  experiences in that regime before pinning it with a test; a fallback
  that produces coherent numbers but incoherent gameplay is a bug with
  green tests.

## Action items

- [ ] Playtest knobs to watch: orbit_hold_enter/exit (0.8/1.2 u/s),
  orbit_clearance_factor 1.5, orbit_band_safety 0.9, and the DPadDown pad
  binding's reachability mid-flight.
- [ ] The [O] ORBIT cue is the first hand-placed keybind hint; the
  diegetic-instruments task (20260709-103454) should absorb and
  systematize it (user request: Arma Reforger-style hints).
- [x] Lock-on regression fixed with a test on this branch (b01a76b).
