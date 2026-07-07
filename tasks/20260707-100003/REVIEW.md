# Review: Torpedo self-detonates on spawn; add arming delay

- TASK: 20260707-100003
- BRANCH: feature/torpedo-arming

## Round 1

- VERDICT: APPROVE

Delivers the Goal: a torpedo can no longer detonate on the muzzle. The latched
`TorpedoArming` component (time OR distance from the spawn point) is ticked by
`update_torpedo_arming` immediately before `torpedo_detonate_system`, which now
skips any un-armed torpedo. Params are config-driven (`arm_time` / `arm_distance`)
with sensible defaults, and the in-game section config is updated. Design matches
the task's suggested `TorpedoArming { min_time, min_distance }` shape.

Verified independently in the worktree:

- `cargo test -p nova_gameplay torpedo`: 6/6 pass - unarmed on spawn; arms via
  time even when stationary (point-blank); arms via distance before min_time (fast
  shot); latches once armed; and the two system-level tests proving an un-armed
  on-target torpedo survives while an armed one detonates. The tests assert
  behavior, not just execution, and directly cover the reported bug.
- `cargo clippy -p nova_gameplay -p nova_assets`: clean.
- `cargo build` (full game): green - the new config fields are set at every
  `TorpedoSectionConfig` construction site (the `Default` impl and the one literal
  in `nova_assets/src/sections.rs`).

TASK.md notes match the code; the un-buildable "verify in the range" step was
honestly redirected to tests since task 20260707-100001 does not exist yet.

No BLOCKER/MAJOR. One NIT, left as a note for the range task, not a change here.

- [ ] R1.1 (NIT) crates/nova_gameplay/src/sections/torpedo_section.rs - the default
  `arm_distance` (5.0) is well below the proximity-fuze radius `BLAST_RADIUS * 0.5`
  (15.0), so a target 5-15 units directly ahead can still detonate the torpedo soon
  after it arms, some distance from the target. This is a tuning artifact of the
  large fuze radius (tracked by blast-param work 20260706-162913 and guidance
  20260525-133021), not the spawn-self-detonation bug this task fixes, which is
  resolved and tested. Best confirmed/tuned interactively once the torpedo test
  range (20260707-100001) exists; no code change warranted now.
  - Response:
