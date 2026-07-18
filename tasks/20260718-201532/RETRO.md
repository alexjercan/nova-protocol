# Retro: Controller-based RCS burn loop sound

- TASK: 20260718-201532
- BRANCH: feature/rcs-burn-sound
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Front-loading an Explore agent to map the audio architecture paid off: it
  found the thruster engine-hum loop as an exact template (loop-per-handle,
  split volume resource for headless testing, per-ship attribution, pause
  sweep) AND the controller's existing authored-sound family
  (`ControllerSectionSounds`). The whole feature became "do what the hum does,
  read `RcsIntent` instead of throttle" - almost no new concepts.
- Gating the sound on the same signal as `rcs_burn_system` (root `RcsIntent` +
  the Rcs verb) made it driver-agnostic for free: the player modal and the
  autopilot both write that one component, so one gate covers SHIFT nudges and
  ORBIT/STOP maneuvers without the audio layer knowing which drove it. The
  review's independent check confirmed there is no separate "autopilot path" to
  test.
- Followed the codebase's authored-or-silent + generate-from-code conventions
  (new config field -> snapshot -> `SectionMeshRefs` -> `content -- gen` ->
  parity) rather than inventing a global sound, so mods can reship it and the
  parity test stayed green.
- `check-all-targets-for-struct-field`: grepped every `ControllerSectionConfig`
  literal before trusting `cargo check`; the two builders needed the field, the
  other three used `..default()`. No surprise break.

## What went wrong

- Wasted one run pointing `--features serde` at `nova_assets` for the parity
  test; `nova_assets` has no such feature and the parity check is an
  integration test (`--test content_ron_parity`) that needs no feature flag.
  Root cause: pattern-matched the `crate-solo-tests-miss-unified-features`
  lesson (which is about nova_scenario/nova_gameplay) onto the wrong crate
  instead of first checking where the test lived.
- The `content -- gen` cold compile blew the 2-minute Bash timeout on the first
  (foreground) try; re-running it backgrounded was the fix. Predictable for any
  nova_assets binary run - should background those from the start.

## What to improve next time

- Before running a crate-scoped test with a feature flag, confirm the test's
  actual location and whether it is a lib unit test or an integration test -
  do not assume the serde-feature lesson applies to every content check.
- Background `cargo run -p nova_assets --bin content` (and other cold
  nova_assets builds) up front; they routinely exceed the foreground timeout.

## Action items

- [x] Bumped `check-all-targets-for-struct-field` and `generate-data-from-code`
  in LESSONS.md.
- [x] Filed the orbit-crash bug the user reported (2 ships in the menu scene
  crash the asteroid and cannot hold orbit) as a new tatr task - likely a
  regression from the error-relative RCS ORBIT trim (20260718-151102), the
  exact feel/looks-right risk that task's review flagged as needing a playtest.
