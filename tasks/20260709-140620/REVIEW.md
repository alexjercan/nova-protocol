# Review: Center of mass does not shift when sections are destroyed

- TASK: 20260709-140620
- BRANCH: fix/com-section-destroy

## Round 1

- VERDICT: APPROVE (no BLOCKER/MAJOR; MINOR/NIT findings below addressed on
  the branch before merge - see Responses)

Independent fresh-eyes pass over `git diff master...fix/com-section-destroy`,
TASK.md, the bcs chase-camera/PD/health sources, avian's mass-property plugin,
and the editor preview code. Verified clean: the core math
(`ComputedCenterOfMass` is body-local, ship roots are top-level unscaled, so
`transform_point` lifts correctly); `update_chase_camera_input` is the only
writer of `anchor_pos` and all three camera modes route through it; camera
shake and HUD projections are not origin-anchored; the physics tests assert
concrete before/after values against a deterministic manual-timestep sim; the
harness additions match how the real app provides the same resources; the
exact-health damage trick is sound and the dodged overkill propagation is
honestly filed (20260709-144906); TASK.md/docs/CHANGELOG match the diff;
conventions match the sibling examples.

- [x] R1.1 (MINOR) camera_controller.rs (fallback comment; echoed in TASK.md
  and the docs file) - the "editor preview" rationale for the
  `Option<&ComputedCenterOfMass>` fallback is false: the preview deliberately
  never carries `SpaceshipRootMarker`/`PlayerSpaceshipMarker`, and every real
  matching root has `RigidBody`, which requires `ComputedCenterOfMass`.
  Suggested: reword to "roots without a RigidBody (unit tests / defensive)".
  - Response: fixed - comment now says the fallback is defensive (marker-only
    test roots; every real root's RigidBody requires the component), and the
    editor-preview claim is corrected in TASK.md and the docs file too.
- [x] R1.2 (MINOR) input/ai.rs:83-166, input/player.rs:117-145,~217 - AI ships
  aim/fly at the player ROOT ORIGIN and the player turret aim/lock-on ray is
  origin-anchored: same root-cause family as the camera (enemies converge on
  the empty build-spot after front sections die). Not the reported symptom;
  suggested: file a follow-up task.
  - Response: filed as task 20260709-150711 (AI aim + turret lock-on anchor at
    the root origin), with the file:line evidence from this review.
- [x] R1.3 (NIT) camera_controller.rs anchor math - `transform_point` applies
  scale; avian works in unscaled space, so
  `rotation * com + translation` is the scale-proof form.
  - Response: fixed - anchor is now `rotation * com + translation`, with the
    scale rationale in the comment.
- [x] R1.4 (MINOR) examples/11_com_range.rs (script assert block) - the camera
  assertion is inside `if let Some(...)`, so a refactor that breaks the
  ChaseCameraInput query makes the check vacuously pass. Panic when the camera
  is missing.
  - Response: fixed - the camera lookup is now `.expect(...)`; a missing camera
    fails the run.
- [x] R1.5 (MINOR) examples/11_com_range.rs (script timing) - the timeline
  gates on TOTAL elapsed while the autopilot exits at 6.0s; a slow Loading
  (>~5.3s) skips every assertion and still exits Success, and a Playing entry
  between ~3.8-5.3s can assert before despawn/recompute settles. Suggested:
  Playing-relative timeline plus a "script actually asserted" guard before
  exit.
  - Response: fixed - the example holds an 8s window, the timeline is relative
    to entering Playing (`playing_since`), and a pre-gate backstop panics at
    7.5s if the assertion never ran.
- [x] R1.6 (MINOR) examples/11_com_range.rs - `ComRangeScript` and
  `com_snapshot` are only consumed by the cfg(debug) script: two dead-code
  warnings without `--features debug` (siblings compile clean). Gate them.
  - Response: fixed - `ComRangeScript`, its init, and `com_snapshot` are gated
    behind the debug feature; `cargo check --example 11_com_range` (no
    features) is warning-free.
- [x] R1.7 (NIT) examples/11_com_range.rs:64 - "also reused as the hotkey
  latch" is wrong; the hotkeys never touch `ComRangeScript`.
  - Response: fixed - comment no longer claims the hotkeys use the latch.
- [x] R1.8 (MINOR) examples/11_com_range.rs (draw_com_gizmos) - the GRAY
  gizmo's `Local` freeze captures `com.0` on the first matching frame, which
  can be the pre-settle default/NaN, pinning the marker to garbage for the
  whole run. Guard the freeze on finite, settled mass properties.
  - Response: fixed - the GRAY freeze waits for finite COM and settled mass
    (`mass.value() > 0.5`), and the marker is only drawn once frozen.
- [x] R1.9 (NIT) integrity/glue.rs (exact-damage comment) - "a larger amount
  would also zero the root's aggregate" implies only overkill propagates; the
  exact 100 also propagates (200 -> 100), it just does not zero. Add the
  clause.
  - Response: fixed - the comment now states the exact amount also propagates
    (200 -> 100) and only overkill would zero the root.
- [x] R1.10 (NIT) integrity/glue.rs - `max_element()` shrink is the weakest
  inertia assertion; expected principal values for two unit cubes are
  computable.
  - Response: fixed - the shrink check is replaced with sorted-principal
    comparisons against the analytic cuboid values ([1/3, 5/6, 5/6] before,
    1/6 across the board after, tolerance 0.02).
- [x] R1.11 (NIT) docs/2026-07-09-com-section-destroy.md - the "~43 vs ~120
  rad/s^2" figures are not reproducible from this branch's ships (they assume
  inertia ~2.3); name the ship and axis or use figures from the 11_com_range
  ship.
  - Response: fixed - the doc names both ships and axes (game 3-section ship:
    ~43 vs ~120 rad/s^2; 11_com_range line ship: ~9 vs ~40 transverse) with
    flip times, and no longer overstates "faster than perception".

## Round 2

- VERDICT: APPROVE

Verified each fix on the updated branch: the anchor math is scale-proof and the
comments/docs no longer claim the editor-preview rationale; the smoke script is
Playing-relative with a mandatory camera check and an asserted-at-exit
backstop (re-ran headless: physics drift 0.000, camera drift 0.000, PASS, and
`06_torpedo_range` regression still green); the no-feature build of the example
is warning-free; the strengthened inertia assertions pass against the analytic
values; task 20260709-150711 covers the remaining origin-anchored consumers.
No new findings.
