# Review: Bullets affected by gravity wells

- TASK: 20260712-105505
- BRANCH: bullets-gravity

## Round 1

- VERDICT: APPROVE

Delivers the spike's Option C1: turret rounds opt into `GravityAffected` via a
third observer mirroring the torpedo one, ride the existing
`gravity_well_system`, and the measurement-driven perf fix (scratch-buffer
reuse) makes the shared force path cheap at bullet scale. Tests are meaningful:
the curve regression carries its own gravity-free control as an in-run A/B, so
it fails if the opt-in is removed (not a presence-only check). Independently
re-verified two load-bearing claims rather than trusting the summary:
- The perf refactor is behaviour-equivalent: both `candidates` and `pulls` are
  `clear()`ed before use at the top of each entity iteration, so no state leaks
  between entities - only capacity is reused. The four pre-existing tests that
  exercise the shared system (bounded orbit, SOI hand-off, well-death dominance
  release, ship pull through the plugin) still pass.
- The "ride the shared system" decision rests on `DominantWell` churning only
  on owner *change*; confirmed by the `if current.map(|d| **d) != Some(owner)`
  guard - steady state issues no command.

Full `nova_gameplay gravity::` suite: 15 passed, 1 ignored (the perf bench).
Wider suite/clippy left to CI per project policy.

No BLOCKER or MAJOR findings. NITs below are the implementer's discretion.

- [x] R1.1 (NIT) crates/nova_gameplay/src/gravity.rs:660 - the
  `gravity_system_marginal_cost` bench spawns bodies already inside the SOI, so
  it measures steady-state per-tick force cost, NOT the per-crossing
  `DominantWell` insert/remove (archetype-move) churn that streaming rounds
  incur at SOI boundaries. That churn is real but bounded by SOI-crossings/sec
  and argued small in the task notes. Suggest one sentence in TASK.md / the
  bench doc making explicit that the ~0.1 ms/tick figure is steady-state force
  cost and the crossing churn is unmeasured-but-bounded, so the number is not
  read as covering everything.
  - Response: fixed - added a "Scope:" paragraph to the bench doc comment and a
    matching sentence to the TASK.md perf note. Verified both now state the
    figure is steady-state force cost and the crossing churn is bounded-but-
    unmeasured.
- [x] R1.2 (NIT) examples/08_turret_range.rs:250 - `anchor_shooter_against_gravity`
  strips `GravityAffected` from the ship a frame or two after spawn, by which
  point the FixedUpdate force system may have inserted a `DominantWell` on it;
  that component then lingers (the system only cleans up `DominantWell` for
  entities it still sees as affected). Harmless here - the range reads no
  `DominantWell` - but if it bothers you, remove both components together in the
  strip system.
  - Response: fixed - the strip now removes `DominantWell` alongside
    `GravityAffected`, with a doc line explaining why. Best-effort (a same-frame
    insert-after-strip race can still leave it one frame) but harmless and
    documented. Example rebuilds clean.
- [x] R1.3 (NIT) examples/08_turret_range.rs:14 - the spike path in the header
  doc is split across a line break (`docs/spikes/20260712-\n112113`), which
  breaks click-to-open. Keep the path on one line.
  - Response: fixed - the full spike path now sits on its own line
    (`Spike: tasks/20260712-112113/SPIKE.md`).
