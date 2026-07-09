# Review: AI rotation path: adopt slew_rotation and hull_turn_rate

- TASK: 20260709-155921
- BRANCH: fix/ai-rotation-slew

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...fix/ai-rotation-slew` against TASK.md; ran the
full nova_gameplay suite on the branch: 197/197 green. Both bugs from the
task land correctly: the command is now an absolute world rotation
(command-anchored goal, same roll-regulation rationale as the autopilot,
with the reasoning inlined at the write site) and it slews at the derived
turn rate with the exact player-path derivation, including the dead-helm
freeze. The own-side live-structure anchor from the 150711 review note is
in for both the rotation and thruster systems, keeping the two systems'
chase vectors consistent. The test story is strong: the one-frame slew step
pins hull_turn_rate's wiring (not just "some slew"), the convergence test
discriminates delta-vs-absolute directly, and the physics test honestly
scopes its acceptance around the pre-existing bcs roll-damping bug
(20260709-125640) with an explicit bound to tighten later - the TASK.md
Resolution documents that diagnosis well.

Non-blocking finding:

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/ai.rs:128-139 - the
  "strongest live computer -> max principal inertia -> hull_turn_rate"
  derivation now exists in three copies (player.rs:118-128, flight.rs
  autopilot ~650/686, and here). Extract a shared helper in flight.rs,
  e.g. `pub(crate) fn ship_turn_rate(torques: impl Iterator<Item = f32>,
  inertia: &ComputedAngularInertia, settings: &FlightSettings) ->
  Option<f32>`, and fold all three call sites onto it so the next retune
  cannot drift them apart.
  - Response: fixed in d22668e - added flight::ship_turn_rate, all three
    sites (player, autopilot, AI) fold onto it; suite re-run green.
