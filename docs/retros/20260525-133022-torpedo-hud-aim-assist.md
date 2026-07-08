# Retro: Torpedo target reticle sizing + angular aim-assist lock-on

- TASK: 20260525-133022
- BRANCH: torpedo-hud-133022 (squash-merged as 231677c)
- REVIEW ROUNDS: 2

See TASK.md for what shipped and REVIEW.md for the findings. This is about how the
work went.

## What went well

- Reading the targeting/HUD code and the avian API before writing paid off. The
  two decisions that shaped the change - "use `ColliderAabb` for the reticle size"
  and "enumerate `With<RigidBody>` roots instead of a new cross-crate marker" - both
  came from grepping the actual crate boundaries first (nova_gameplay cannot import
  nova_scenario, so a marker-based enumeration would have forced edits across three
  spawn sites for no gain). Confirming `ColliderAabb`/`RigidBody`/`SystemState`
  signatures in the vendored source up front avoided guess-and-recompile churn.
- Extracting the selection rule into a pure `pick_target(origin, aim, ...)` helper
  made item 2 unit-testable without a physics/camera world - five cheap tests that
  actually assert behaviour (nearest wins, cone/range/behind exclusions).
- Thinking about feel, not just correctness, caught the "lock snaps onto your own
  turret bullets" regression before it was ever run: bullets are dynamic bodies
  streaming down the aim ray, so the wider cone would have locked them. Excluding
  `TurretBulletProjectileMarker` was a same-crate one-liner.

## What went wrong

- R1.1 (MAJOR): the first reticle-sizing implementation projected all 8 AABB
  corners and bailed to minimum size if any corner was behind the camera - which is
  exactly the close/large-target case item 1 exists for. Root cause: I reached for
  the "obviously correct" exact-bbox projection without reasoning about the
  near-plane failure mode, i.e. I optimised for geometric accuracy over the robust
  common case. The fix (project centre + one bounding-radius offset) is both simpler
  and more robust; it should have been the first choice.
- Minor churn: `SystemState::get` returns a `Result` in Bevy 0.19, which cost one
  compile cycle on the new tests. A small instance of not verifying an unfamiliar
  API before using it - the same habit that went well elsewhere, missed here.

## What to improve next time

- For any screen-projection / camera-space sizing, reason about the near-plane and
  off-screen cases first and prefer a centre-plus-offset (needs one point to
  project) over corner/bbox projection (needs all points). Robust-common-case beats
  exact-but-fragile for HUD sizing.
- When a change widens an assist/selection heuristic (bigger cone, larger radius,
  looser filter), explicitly list what newly qualifies and whether any of it is
  self-inflicted noise (own bullets, own debris, sensor volumes). That checklist is
  what caught the bullets and the static areas here.

## Action items

- [ ] Next in this flow: run the "HUD for weapons" /spike (TASK.md follow-up) for
  lead indicators, lock-on cue, range/closing-speed readout, etc.
- [x] Lesson captured here (centre+offset over corner projection); not general
  enough for AGENTS.md yet - revisit if a second projection-sizing task repeats it.
</content>
