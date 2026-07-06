# Improve spaceship brain

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Clean up refactor pass on the spaceship brain. Legacy #66.

## Steps

- [x] Locate the spaceship "brain" (the AI controller, input/ai.rs).
- [x] Clean it up without changing behavior: name magic numbers, dedupe, fix style.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

The "brain" is SpaceshipAIInputPlugin (crates/nova_gameplay/src/input/ai.rs): chase/brake
steering, thrust, turret aim, and firing. It worked but had readability problems. Did a
behavior-preserving cleanup pass:

- Named the magic numbers as constants: AI_CHASE_SPEED_GAIN (0.2),
  AI_MIN/MAX_CHASE_SPEED (2.0/20.0), AI_BRAKE_SPEED_MARGIN (1.0),
  AI_THRUST_ALIGNMENT / AI_FIRE_ALIGNMENT (0.95). The steering intent is now readable.
- Extracted the duplicated steering computation (target speed -> chase-or-brake ->
  desired direction) that appeared verbatim in both update_controller_target_rotation_torque
  and on_thruster_input into a single `ai_desired_direction(to_player, velocity)` helper.
- Fixed a non-ASCII apostrophe in a comment (repo style is ASCII-only).

Behavior preservation: the two inlined copies differed only in that
update_controller had a zero-direction fallback and on_thruster did not. That fallback is
unreachable in practice (it only triggers when `too_fast` and velocity is zero, but
`too_fast` requires speed > min_chase_speed + margin > 0), so unifying them under the
helper - which keeps the fallback - changes nothing in any reachable state. Constants use
the exact previous literal values.

Verified: build --all-targets, clippy, fmt green. Runtime not exercised (no display), but
the change is a pure readability refactor with identical numeric behavior.

Self-reflection: "clean up the brain" with no specific defect is an invitation to
speculative churn. Kept it disciplined: only readability changes provably equivalent to
the original (named constants at the same values, a dedup helper that matches in every
reachable state). Verified the seemingly-different fallback was actually dead code before
unifying, rather than assuming the two copies were identical.
