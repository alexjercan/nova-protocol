# Notes

## Implementation

- Kept `screenshot_combat` as one example and one autopilot story.
- Added private `rock_hollow_ordnance` scenario content in Rust.
- The ordnance boundary calls `LoadScenario`, which tears down the prior
  scenario-owned fight before spawning a fresh player, raider, lance, hollow,
  and lighting rig.
- Readiness requires the new scenario id, exactly the three required ships,
  exactly two torpedo bays, and no bullets or old torpedoes.
- The salvo step waits for exactly two projectiles. Commit asserts exactly two
  uncommitted torpedoes before targeting the raider.
- Tracking now fails immediately if the complete salvo disappears, rather than
  waiting for the generic step deadline.
- Existing injected aftermath damage remains unchanged.

## Verification

- `nix develop --command cargo check --example screenshot_combat --features debug` - passed.
- `nix develop --command cargo fmt --check` - passed.
- Four consecutive correctness probes passed with both `DISPLAY` and
  `WAYLAND_DISPLAY` unset.
- Every run logged:
  - ordnance chapter loading and readiness;
  - two torpedo bays firing;
  - two torpedoes committed;
  - successful tracking, capture, and detonation steps.

## Rendered review

User ran the example with a display and approved the output. The ordnance
background, camera continuity, torpedo framing, and aftermath need no further
chapter split for this fix.
