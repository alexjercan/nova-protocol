# Review: AI engagement flight: standoff orbit/strafe envelope

- TASK: 20260709-225729
- BRANCH: feature/ai-standoff-flight

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...feature/ai-standoff-flight` (commit 6d4066c)
against TASK.md. The change is exactly the planned seam: everything lives
in `ai_desired_direction` plus tuning constants, both call sites (rotation
command and thrust gate) pick it up unchanged, so the two systems keep
agreeing on the desired direction.

Verified with fresh eyes:

- Envelope math: radial term signed by the range error, tangential term
  with stable handedness and a working polar fallback (los near +/-Y is
  the only case where los x Y fails to normalize, and los x X is
  well-defined there). The blend cannot degenerate to zero for a nonzero
  line of sight (orthogonal unit terms, weights summing to 1), and the
  final normalize_or_zero plus the face-the-target fallback cover the
  rest. The speed budget scaling with RANGE ERROR (floored at
  AI_ORBIT_SPEED) is what actually breaks the park-at-zero behavior; the
  overshoot brake branch is byte-identical in spirit to the old regime
  and pinned by a test.
- Constants: AI_STANDOFF_RANGE 250 m sits inside fire discipline's
  effective range (450 m * 0.9 gate) as the task Notes require.
- Harness updates: flip_world's player to z=800 and the swing test's
  player to x=1000 move both setups outside the band so their
  nose-on-target assertions keep their original meaning - updated, not
  weakened; the assertions themselves are untouched.
- Tests assert behavior: the five unit tests pin each regime (approach,
  orbit, extend, brake, polar) with directional dot-product assertions,
  and the physics test runs the full diegetic loop for 45 simulated
  seconds, asserting the final second holds within 2x the band AND the
  ship never dove under 100 m - pure pursuit fails both on this harness.
- Checks on the branch: cargo fmt --check clean, cargo check --workspace
  clean, input::ai suite 33/33 green (full suite deferred to CI per repo
  policy).
- TASK.md Resolution matches the code; the global-handedness and
  torpedo-interplay scope cuts are recorded in Notes, not hidden.

- [ ] R1.1 (NIT) crates/nova_gameplay/src/input/ai.rs:375 - the
  distance <= EPSILON early return hands Vec3::ZERO to
  Quat::from_rotation_arc at the rotation call site, which expects
  normalized inputs. Unreachable in practice (both anchors coincident)
  and strictly better than the old code's NaN in the same case, but if
  touched again consider returning early at the CALLER when the anchor
  vector is degenerate, so the command simply freezes like the other
  guard paths.
  - Response:
