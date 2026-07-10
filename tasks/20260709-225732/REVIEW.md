# Review: AI torpedo usage from Engage: launch envelope + cooldown

- TASK: 20260709-225732
- BRANCH: feature/ai-torpedo-usage (local branch by user request)

## Round 1

- VERDICT: APPROVE

Verified against TASK.md: the envelope (blast-derived min, constant max,
loose alignment), the per-bay cadence on top of the section's fire-rate
timer, launch-truthful cadence reset through TorpedoSectionPartOf, and the
AI-side commit-on-launch sibling are all present and tested through the
real systems (not convenience calls). The held-trigger model was checked
against shoot_spawn_projectile: fire_state.reset() on launch plus the
next-frame cadence reset (commit ordered before the trigger write) bounds
extra launches to fire intervals shorter than one frame - not a real
config. Player bays are structurally out of reach of the AI trigger
system (the query requires AITorpedoBay, which only AI-parented bays
receive), and the two commit systems partition torpedoes by owner. The
suite for the touched modules passes (71 ai + 27 torpedo_section);
workspace check is clean.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/ai.rs:1436 - the launch
  gate requires a SHIP target, but the commit system attaches the owner's
  CURRENT AITarget unfiltered. The launch and the commit are one frame
  apart: if the ship target dies on exactly that frame and a committed
  hostile torpedo is in acquisition range, pick_ai_target flips AITarget
  to the torpedo and the fresh torpedo commits to chasing ordnance - the
  thing the launch gate exists to prevent. Suggested change: in
  update_torpedo_target_input, filter the committed target through
  q_ship_root (like the trigger side does); a non-ship target commits as
  a dumb-fire shot instead.
  - Response: Fixed as suggested - the commit filters the owner's AITarget
    through q_ship_root and a non-ship target commits as dumb-fire, with
    the rationale documented on the system. Regression test
    a_torpedo_target_at_commit_time_dumb_fires covers the exact flip
    scenario. (Reviewer: verified - the filter matches the trigger side's
    gate and the test fails without it.)
- [x] R1.2 (NIT) crates/nova_gameplay/src/input/ai.rs torpedo_tests - no
  test that the trigger write is per-ship (ship A engaged, ship B idle:
  B's bay must stay released). The ChildOf filter pattern is shared with
  the thruster/turret systems, but those have no such test either, and
  this system adds the lazy-insert path where a cross-wire would be
  easiest to introduce. Take it or leave it.
  - Response: Taken - the_trigger_is_per_ship: two AI ships with their own
    bays, one engaged in envelope, one with nothing to fight; asserts the
    engaged ship's trigger pull does not leak onto the other's bay.
    (Reviewer: verified, the assertion is on the second ship's bay state
    after both ships' passes ran.)
