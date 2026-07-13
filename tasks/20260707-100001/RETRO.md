# Retro: Torpedo bay test range example

- TASK: 20260707-100001
- BRANCH: feature/torpedo-range
- PR: #28 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two intentional NITs)

See `tasks/20260707-100001/TASK.md` and `examples/06_torpedo_range.rs`; this retro
is about how the working went.

## What went well

- Front-loaded the investigation. This example touches a lot of surface (scenario
  config, event/action spawning, ship assembly, section input binding, blast
  damage, gizmos, camera). Fanning out an Explore agent to map most of it in
  parallel, then doing a few targeted follow-up reads for the exact recipes (blast
  = collision between a sensor sphere and a `Health`+`Collider`+`RigidBody` body;
  input binding keyed on the section instance id; `EntityId(pub String)`), meant
  the example compiled on the first real attempt - only an unused import and a
  lint remained.
- Reused the real pipeline instead of mocking. The range drives the actual
  scenario loader, player input and section systems, so it faithfully exercises
  the torpedo - and headlessly demonstrated the full arm -> home -> hit cycle (3
  fired, 3 armed, 3 detonated).
- Verified end to end, not just compile: autopilot smoke run and a real screenshot
  under Xvfb, plus the no-debug build proving the harness cfg's out.
- Fixed the `type_complexity` clippy warning at its root cause - the root package
  did not inherit the workspace clippy allows that every crate opts into - rather
  than sprinkling `#[allow]` on the systems.
- Caught the screenshot's forced-Playing double-setup interaction and moved scene
  setup to `GameAssetsStates::Loaded` before it bit.

## What went wrong

- The spawn path is spread across three places (the asteroid *bundle* has no
  `RigidBody`; the loader *action* `base_scenario_object` adds `RigidBody`/`Name`/
  `EntityId`; observers add the collider child), so "what components does a spawned
  asteroid actually have" took several file hops to answer. I found it, but only
  after chasing it manually.
- Two extra build cycles: an unused `bevy_enhanced_input` import (the nova prelude
  already re-exports it) and the setup-on-`Loaded` move. Each is a ~3 min
  incremental build. The unused import was avoidable by checking what the prelude
  already brings in before adding a crate import.

## What to improve next time

- When a task hinges on "what components does entity X end up with", ask the
  Explore agent to trace the *whole* spawn path (bundle + loader/action +
  observers) in one question, rather than discovering the split file by file.
- Before adding a `use some_crate::prelude::*`, check whether the project prelude
  already re-exports it (nova re-exports bevy_common_systems and, transitively,
  bevy_enhanced_input) to avoid an unused-import round-trip.

## Action items

- [ ] NITs R1.1 / R1.2 left as-is by design (range/game target double-assign is
      harmless; the moving gate is driven by setting `LinearVelocity`). No change.
- [ ] The range now unblocks interactive work on target-loss (`20260707-100004`),
      PN guidance (`20260525-133021`) and blast tuning (`20260706-162913`) - already
      tracked, no new task needed.
