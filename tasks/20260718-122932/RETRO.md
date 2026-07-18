# Retro: autopilot RCS terminal-settle

- TASK: 20260718-122932
- BRANCH: feat/rcs-autopilot (landed as master 8b0a610e)
- REVIEW ROUNDS: 1 (REQUEST_CHANGES -> APPROVE; a MAJOR drift bug found + fixed)

Process only; what/why live in TASK.md + NOTES.md.

## What went well

- Reading the primitive's gate BEFORE building caught that ORBIT is fundamentally
  incompatible: `rcs_burn_system` caps ABSOLUTE speed at 2 u/s, orbits run at
  ~2.5-6 u/s, so RCS would brake the orbit, not trim it. Surfaced to the user,
  scoped to GOTO/STOP, split ORBIT to a redesign task - instead of discovering it
  after building the wrong thing (`verify-first-plan-steps` in action).
- Running the WHOLE `flight::` suite (not just the new tests) caught the 11-test
  regression immediately - the change touched the core autopilot loop, so the
  module suite was the right blast radius to check.
- The self-review found a real MAJOR bug the tests missed: a residual `RcsIntent`
  left at disengage keeps pushing the ship (rcs_burn_system isn't
  autopilot-gated), drifting it to the cap AFTER arrival. My own test stopped
  observing at disengage and missed it; the review asked "what happens past the
  handoff?" and the off-ramp test now guards it.

## What went wrong

- First cut regressed 11 autopilot tests: I under-weighted that `Rcs` is granted
  by DEFAULT, so the terminal-settle hijacked EVERY ship's STOP/GOTO, not just
  RCS-opt-in ones. Root cause: didn't reason about "what does this do to every
  existing default entity" before touching a path all of them run. New lesson
  `new-default-on-capability-changes-tested-behavior`.
- The drift bug (residual intent after disengage) is the same class: a shared
  side-effecting component needs each driver to CLEAN UP when it stops driving.
  I built the on-ramp and forgot the off-ramp. New lesson
  `shared-primitive-clear-on-handoff`.
- Hit `piped-cargo-masks-exit-code` AGAIN (x6 now) - grep-piped cargo runs
  returned empty, hiding results, twice. Finally switched to `> file` then grep
  the file. This lesson is well past promotion; I keep re-learning it live.
- My own two new tests failed first on too-tight thresholds (settle to the
  deadband 0.75, not to 0.3) - a modeling error about where the autopilot
  releases.

## What to improve next time

- Before adding a capability that a shared/default path will honor, ask "what
  does every EXISTING default entity now do differently?" and expect to opt the
  legacy tests out (or make the capability opt-in).
- For any component multiple systems write, test the OFF-ramp (run past the
  handoff / disengage), not just the on-ramp.
- Stop piping cargo through grep. Write to a temp file, grep the file.

## Action items

- [x] Ledger: added `new-default-on-capability-changes-tested-behavior` and
  `shared-primitive-clear-on-handoff`; bumped
  `changed-shared-observer-run-the-module-suites` (x2) and
  `piped-cargo-masks-exit-code` (x6).
- [ ] Follow-ups seeded and OPEN: `20260718-151102` (error-relative RCS mode for
  ORBIT + tighten the terminal creep), `20260718-144939` (cap ring).
- [ ] Still in this flow (user's newer ask): show SHIFT in the keybind hints
  only when RCS is granted (like the other verb hints), and disable RCS in the
  mainline campaign (withhold the verb). Then close the parent (20260717-105406).
