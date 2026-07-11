# Review: Menu ambience: thruster-flown AI orbit replaces ballistic seeding

- TASK: 20260711-212504
- BRANCH: feat/menu-thruster-orbit

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; the findings below were addressed on
  the branch before landing)
- Method: fresh-context agent review (out-of-context pass) which re-ran the
  nova_assets and nova_menu suites, swept the workspace for every deleted
  symbol, audited the legitimacy of the two test deletions (both pinned
  only the deleted mechanism; the replacement config test fails on
  controller flip-back, wrong directive id, or missing/gravity-less
  planetoid), and independently re-derived the physics: spawn r=140 sits
  deep inside the SOI (~640-728u) and inside the stable band
  (~122-138..~490-557), so the ORBIT plan always exists - no
  drift-dead-ship risk; the ring stays comfortably inside the camera
  frustum (~22 deg vs ~36 deg half-hfov). In-session visual run separately
  confirmed flame + motion + steady framing.

- [x] R1.1 (MINOR) crates/nova_assets/src/scenario.rs (menu_ambience fn
  doc) - still says "one passive ship that nova_menu puts on a ballistic
  circular orbit"; the mechanism this branch deletes. Reword to the AI
  thruster-flown orbit.
  - Response: fixed in the round-1 fixup commit.
- [x] R1.2 (MINOR) CHANGELOG.md (Unreleased) - "a passive ship on a real
  ballistic orbit" would ship as a false description. Amend to the
  thruster-flown AI orbit.
  - Response: fixed in the round-1 fixup commit.
- [x] R1.3 (NIT) crates/nova_assets/src/scenario.rs (~36) - pre-existing mu
  comment computes from the nominal 20u radius, but the well derives mu
  from the runtime geometric radius (~80-91u), so the numbers are off
  16-20x. Fix while here since the branch leans on runtime-geometry.
  - Response: fixed in the round-1 fixup commit; the comment now says the
    runtime well derives mu from the geometric radius and the nominal
    numbers are only the authored inputs.
