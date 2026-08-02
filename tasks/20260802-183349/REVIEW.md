# Review: Move the screenshot reel driver into nova_autopilot behind caller hooks

- DATE: 20260802-192000
- TASK: 20260802-183349
- BRANCH: feature/autopilot-reel-port
- WORKTREE: /home/alex/.cache/sprouts/nova-protocol/feature/autopilot-reel-port
- BASE: master (2380c98c)

## Round 1

- REVIEWER: fresh `/flow 20260802-183349` session entering at REVIEWING, no
  implementation context (review skill rule 2 default; no extra subagent
  spawned).
- VERDICT: APPROVE

Checked the diff (`master...HEAD`) against Story, Steps, DoD and the source it
ports (`crates/nova_debug/src/harness.rs:216-561`), and against the sibling
`screenshot.rs` for shape consistency.

### R1.1 MINOR - the `ready` doc says "first beat", the code checks it every frame

`crates/nova_autopilot/src/reel.rs:295` evaluates the predicate on every
`reel_drive` call, before the `resource_scope`. The docs
(`reel.rs:29`, `reel.rs:184`) both say it gates *the first beat*. A caller
whose predicate can go false again mid-reel (camera despawned on a scenario
transition, an asset unloaded) gets a silently stalled reel that only surfaces
at the completion deadline, and nothing in the docs warns them.

The behaviour is a faithful port of the old `has_camera` probe, so this is a
docs defect, not a regression. Change: say the predicate is re-evaluated every
frame and must stay true for the whole reel (or latch it on first `true` and
say so). Whichever is chosen, the two doc sites must agree with the code.

### R1.2 NIT - the path unit test can pass vacuously

`crates/nova_autopilot/src/reel.rs:361` skips the relative-path assertion
entirely when `NOVA_SHOT_DIR` happens to be set in the ambient environment.
The comment explains why it will not *set* the env (correct - the binary is
shared), but skipping means a developer with `NOVA_SHOT_DIR` exported runs half
a test and sees green. Change: derive the expected value from the observed env
instead of branching around the assertion, so the case is always checked.

### Verified claims

- Port fidelity: constants, the private `ReelWindowSize` / `ReelState` /
  `ReelCaptureDone` resources, `reel_resize_window`, the `reel_drive` cadence
  and the empty-beats warn-and-return all match the `nova_debug` source. The
  only deliberate divergences are the ones the task asked for: `AppExit::Success`
  -> `completion.done(completion::REEL)`, `ReelCamera` -> the `apply` hook, the
  camera probe -> the `ready` predicate, `SCREENSHOT_REEL_ENV`/`BCS_REEL` ->
  `REEL_ENV`/`NOVA_REEL`, and the extracted `create_capture_dir` helper that
  de-duplicates a block the source had twice.
- Hook shape matches `ScreenshotPlugin::hide_overlay` (`Option<Arc<...>>`,
  cloned into a `Startup`/`Update` closure at `build`), as the Steps required.
- The outer `///` above `pub mod reel;` is gone and the lib.rs comment now
  covers all modules (`20260802-183340` R1.3 honoured).
- Boundary: no Nova or physics dependency reaches the crate or the module.
- Test claims re-derived independently, not taken on trust: deleting the
  `ReelCaptureDone` wait in `reel_drive` makes
  `reel_beats_are_serialized_on_capture` fail; restoring it makes it pass and
  leaves the tree clean. The serialization test is load-bearing, not a
  tautology.

### Checks run

```
nix develop --command cargo test -p nova_autopilot
  15 lib + 3 reel + 4 screenshot + 1 stand-down + 3 doc tests, all green
nix develop --command cargo clippy -p nova_autopilot --all-targets   clean
nix develop --command cargo fmt --check                              clean
test -f crates/nova_autopilot/Cargo.toml && ! rg -n '^(nova_|bevy_common_systems|avian3d)' crates/nova_autopilot/Cargo.toml   exit 0
! rg -n 'nova_scenario|nova_gameplay|avian3d|bevy_common_systems|ScenarioCamera|HudVisibility|RigidBody' crates/nova_autopilot/src/reel.rs   exit 0
```

### Verdict

VERDICT: APPROVE

No BLOCKER or MAJOR. R1.1 and R1.2 are docs/test-hygiene and do not block;
fold them into the caller migration (`20260802-183403`) or a follow-up.

Pending `manual:` items: none.
