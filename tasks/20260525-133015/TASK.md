# Create changelog.md

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.4.0, chore

Populate from git history. Legacy #95 (note: CHANGELOG.md exists; verify completeness).

## Resolution (CLOSED - 2026-07-08)

CHANGELOG.md existed but its [Unreleased] section was an empty placeholder. Populated it
from the 36 commits in `v0.3.1..HEAD`, grouped Keep-a-Changelog style:

- Added: torpedo proportional-navigation guidance, projectiles inheriting rotational muzzle
  velocity, the new example test ranges (06_torpedo_range, 08_turret_range, 10_gameplay),
  turret tuning sliders + FPS/version overlay, and the BCS autopilot/screenshot harness.
- Changed: consuming integrity/health/blast/mesh-slicer from bevy_common_systems, torpedo
  module split with config-driven blast params, turret intercept aim.
- Fixed: blast reaching every overlapped body, asteroid RigidBody-husk despawn, camera
  origin-snap, the torpedo arming/keep-flying/no-lock fixes, turret resting position, editor
  preview 'root not found' spam.

Also fixed the stale `[unreleased]` compare link (`v0.3.0...HEAD` -> `v0.3.1...HEAD`).

Test-only, chore/task-tracking, style and internal-doc commits were intentionally omitted as
not user-facing. Kept under [Unreleased] (no 0.4.0 release cut yet).
