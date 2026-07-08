# Retro: Minimal example for nova_gameplay crate

- TASK: 20260525-133009
- BRANCH: feat/gameplay-minimal-example
- PR: #50 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260525-133009/TASK.md`. A "showcase" example whose value came out only after
running it and watching the mechanics actually fire.

## What went well

- Weighed the user's steer against the task and picked the clearer vehicle. The suggestion was to
  improve the scenario example; reading `03_scenario` showed it loads a named scenario (the
  ship is hidden in the catalog and it is really a nova_scenario demo). A new example that builds
  the ship inline documents nova_gameplay far better than overloading `03`, so I did that and said
  why. Honouring the intent (test more mechanics via an example) mattered more than the literal
  "edit that file".
- Ran it and read the log instead of trusting "it compiled". The first run showed `0 asteroids`
  from the very first frame - the readout + the DEBUG collision logs made it obvious the targets
  had self-destructed on spawn, not that the turret was slow. A green compile would have shipped
  a broken demo.
- Diagnosed from the data, not a guess. The impact-damage logs showed the two *close* asteroid
  pairs collided (~250 damage) while the *far* pair did not - which pins the cause to collider
  size, not gravity or spawn-at-origin. That is what made the fix (spacing) obviously correct
  rather than a shot in the dark.
- Reused the established example shape (harness wiring, `after(SpaceshipInputSystems)` aim,
  throttled readout, examples_smoke registration) so the new example is consistent with 06/07/08
  and needed no new infrastructure.

## What went wrong

- Picked target positions from intuition (~13 units apart looked "spread out") without accounting
  for the asteroid collider being much larger than its `radius`. One wasted run. Root cause:
  assumed the nominal radius was the collider radius; the low-health targets then made the
  mismatch fatal instead of invisible.

## What to improve next time

- When placing physics objects that must not touch, do not eyeball the gap from the nominal
  size - check the actual collider extent (here the asteroid mesh/collider is several times its
  `radius`), or copy spacings from a working example (08 uses 30-60 units) rather than inventing
  tighter ones.
- Low-health test targets are a good stress test in themselves: they expose spawn-time physics
  jitter that high-health objects silently absorb. Worth using deliberately when checking a
  spawn setup is clean.

## Action items

- [ ] Possible follow-up bug: an asteroid's collider is a good deal larger than its
      `AsteroidConfig.radius`, so `radius` is misleading for placement/spacing. Worth aligning the
      collider extent with `radius` (or documenting the real factor) - a separate nova_scenario
      task.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
