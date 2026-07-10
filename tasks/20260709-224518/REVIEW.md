# Review: Balance a single off-center main drive with off-axis counter-torque thrusters

- TASK: 20260709-224518
- BRANCH: feat/off-axis-counter-torque (local branch, per user instruction; no worktree)

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed `git diff master...feat/off-axis-counter-torque` against TASK.md.
Verified independently: the user decision (bounded drift) is recorded in
TASK.md; the solver change is a strict generalization (primary-only inputs
reproduce the old QP exactly - existing balance tests pass unmodified in
substance, only re-expressed through the `main_engine` helper); the
recruit-with-zero-forward convention is sound and the reasoning for it
(saturated demand has zero equality slack) is reproduced in the doc; the seed
being a stationary point on balanced ships means no gratuitous lateral burns,
and this is pinned by `balance_throttles_leaves_off_axis_engines_dark_on_a_balanced_ship`.
Checked the blast radius of widening `manual_burn_system` to all unbound
engines: `FlightIntent` exists only on player ships (`insert_flight_control`
on `PlayerSpaceshipMarker`; grep shows no AI/torpedo use), AI ships
(`input/ai.rs`) write their own thruster inputs on `AISpaceshipMarker` ships
without `FlightIntent`, and bound thrusters stay excluded - no writer
conflict. Recruits cannot fire without a primary burn (`primary_forward <=
1e-6` returns all zeros), so no counter-torque ghost burns while coasting.
Ran `cargo check` (clean), `cargo fmt --check` (clean), and the flight module
suite: 38/38 pass, including the two new physics tests and four new solver
unit tests, all of which assert behavior (hand-computed optima, a
pulls-without-the-lateral control, convergence to rest). Full workspace suite
and clippy deferred to CI per project practice.

- [x] R1.1 (MAJOR) tasks/20260706-162908/TASK.md, tasks/20260708-224303/TASK.md,
  tasks/20260709-225734/TASK.md - three unrelated task re-prioritizations
  (v0.4.0 -> v0.5.0, priority changes) are swept into the feature commit
  3d28e58. These are pre-existing uncommitted working-tree edits (they predate
  the branch), not part of this task; landing them under a
  `feat(flight): ...` squash commit falsifies history and couples an
  unrelated backlog decision to this feature. Rewrite the branch commit to
  exclude these three files and leave them as uncommitted working-tree
  changes where they were found.
  - Response: Fixed. The branch was rebuilt from master: the feature commit
    now contains only flight.rs, the two docs, and this task's folder; the
    three re-prioritization edits are back to uncommitted working-tree
    changes. Verified with `git diff master...HEAD --stat` and `git status`.
- [x] R1.2 (MINOR) docs/2026-07-09-thrust-balancing.md:110 - the
  "Full-throttle asymmetric burns still pull" boundary bullet is now stale:
  on a ship with a usable off-axis engine a full-stick burn no longer pulls
  (the recruit's trim budget is its own throttle box - the new doc's
  headline result). The UPDATE note added in this branch covers only the
  firing-set bullet above it. Add the same style of dated update to this
  bullet, scoping it to ships with no off-axis engine left.
  - Response: Fixed - dated UPDATE added to the full-throttle bullet, scoping
    the pull to ships with no usable off-axis engine left.
- [x] R1.3 (MINOR) crates/nova_gameplay/src/flight.rs:505 - garbled sentence
  in the `balance_throttles` doc: "the objective nulls the net torque while
  keeping the sideways force it buys that with bounded". Rewrite, e.g. "the
  objective nulls the net torque while keeping bounded the sideways force it
  buys that with" or clearer phrasing.
  - Response: Fixed - now reads "the objective nulls the net torque while
    keeping bounded the sideways force that nulling costs".

## Round 2

- VERDICT: APPROVE

Verified against the rebuilt branch:

- R1.1: `git diff master...HEAD --stat` shows only flight.rs, the two docs,
  and this task's folder; the three re-prioritization edits are uncommitted
  working-tree changes again. Confirmed resolved.
- R1.2: the full-throttle boundary bullet in
  docs/2026-07-09-thrust-balancing.md carries a dated UPDATE scoping the pull
  to ships with no usable off-axis engine left. Confirmed resolved.
- R1.3: flight.rs:505 now reads "keeping bounded the sideways force that
  nulling costs". Confirmed resolved.

Re-ran `cargo fmt --check`, `cargo check --workspace`, and the flight module
suite after the fixes: clean, 38/38 pass. No new findings from the rebuilt
commits (the code diff is byte-identical apart from the two doc fixes).
