# Review: Vendor bevy-common-systems

- TASK: 20260806-180450
- BRANCH: master (no sprout - owner's instruction; diff is `f3cf3150..HEAD`)

## Round 1

- REVIEWER: out-of-context (three lanes: behavior/proofs, correctness/silent-
  behavior-change, design/standards/docs)
- VERDICT: APPROVE

No BLOCKER and no MAJOR. The ten-step lift delivers the Story: BCS is gone from
code, manifests and `Cargo.lock`, the workspace compiles and lints at both
feature settings, and every copied module was byte-diffed against BCS
`6f09461` with zero logic, constant, ordering or guard-comment deltas. The
findings below are one test-coverage gap, six wrong DoD criteria that the
`## Progress` notes already disclose, one knowing deviation awaiting the
owner's call, and a run of doc and convention nits.

- [ ] R1.1 (MINOR) crates/nova_gameplay/src/physics/rigid_body.rs:34 - the
  three `rigid_body_point_velocity` unit tests from BCS
  `src/physics/rigid_body.rs:65-101` did not come across
  (`a_point_on_a_purely_translating_body_moves_with_the_body`,
  `a_point_at_the_center_of_mass_ignores_rotation`,
  `rotation_adds_tangential_velocity_at_an_offset`, the last carrying a
  hand-computed cross-product oracle). This is the only test loss in the whole
  vendor - all other copied files match BCS test counts exactly.
  `prototypes/00-conventions.md` says to copy the tests with the code; Step 7b
  said "the function + its doctest only", so the two rules disagree and the
  tests fell through. Not higher than MINOR because the doctest at `:19-26`
  carries the same oracle as the third test. Re-add the `#[cfg(test)] mod
  tests` block verbatim, dropping nothing (none of the three touches
  `destructible_body` or `Health`).
  - Response:
- [ ] R1.2 (MINOR) crates/nova_gameplay/src/lib.rs:79 - the reworded guard
  comment reads "Re-export BY NAME, never by glob" and sits on top of a block
  that globs sixteen submodule preludes (`camera::prelude::*`,
  `mesh::prelude::*`, `transform::prelude::*`, `objectives::prelude::*`, ...).
  Two lanes read it as a broken rule. The rule the harness-twins incident
  actually taught is about foreign preludes, and the own-submodule globs
  predate this task. Scope the comment to what it means: never glob an outside
  crate's prelude; nova's own submodule preludes are curated in their own
  modules.
  - Response:
- [ ] R1.3 (MINOR) tasks/20260806-180450/TASK.md:621 - the Step 10 DoD line
  "`Cargo.lock` lost exactly two packages" is false: the diff removes six -
  `bevy_common_systems`, `bevy_common_systems_macros`, `rand 0.9.2`,
  `rand_chacha 0.9.0`, `rand_core 0.9.3`, `ppv-lite86 0.2.21`. Verified
  independently. The extra four are the rand-0.9 line only BCS pulled, so the
  outcome is better than the criterion, and the Progress note discloses it -
  but the criterion as written is not met. Rewrite it to "two BCS packages plus
  the four rand-0.9 transitives nothing else in the workspace pulls".
  - Response:
- [ ] R1.4 (MINOR) tasks/20260806-180450/TASK.md:546 - "The ten copied mesh
  tests pass" is wrong: `slice.rs` carries 3, not 4. Verified -
  `cargo test -p nova_gameplay --lib mesh::` reports 9 passed. Change "ten" to
  "nine".
  - Response:
- [ ] R1.5 (MINOR) tasks/20260806-180450/TASK.md:514 - the proof
  `cargo clippy -p nova_ui --all-targets --features debug` cannot run:
  `nova_ui` declares only a `serde` feature (`crates/nova_ui/Cargo.toml:19-20`),
  so cargo hard-errors. The Step 2 Progress note says as much but the DoD line
  was never amended. Drop `--features debug` from it.
  - Response:
- [ ] R1.6 (MINOR) tasks/20260806-180450/TASK.md:532 - the four example-run
  proofs (`:532`, `:543`, `:580`, `:592`) read
  `xvfb-run -a --server-num=99 cargo run --example X` with neither
  `NOVA_AUTOPILOT=1` nor `--features debug`, so the harness is inert and the run
  never terminates; the wrapper also always exits 1. The Step 2 note records
  both traps and the runs were done in the working form. Rewrite all four to
  `NOVA_AUTOPILOT=1 xvfb-run -a --server-num=99 cargo run --example X --features debug`
  and state that the verdict is the `autopilot: cycle complete, no panic` log
  line, not `$?`.
  - Response:
- [ ] R1.7 (MINOR) tasks/20260806-180450/TASK.md:579 - "Nothing outside
  `plugin.rs`, `input/ai/passive.rs` and `flight/tests/control.rs` adds it" is
  false: `PDControllerPlugin` is also added at `physics/pd_controller.rs:354`,
  `flight/tests/support.rs:43` and `input/ai/maneuver.rs:452,690`. Verified all
  five are inside `#[cfg(test)]` app builders, so the behavior is right and only
  the criterion is wrong. Reword to "exactly one non-test registration, in
  `plugin.rs`".
  - Response:
- [ ] R1.8 (MINOR) crates/nova_assets/src/persist.rs:16 - Step 10g says to keep
  the "Modelled on `bevy_common_systems::persist`" citation; the diff drops the
  crate name so the two absence greps pass, and the Progress note flags this as
  a knowing deviation for the reviewer. Accepting it: the greps are the shipped
  contract and a citation to a crate no longer reachable from this repo is a
  dead pointer. Two follow-ups: amend 10g's text so the record and the code
  agree, and fix the reworded sentence, which lost its referent and reads
  "Modelled as the deliberate counterpart to the load-on-build /
  save-on-`resource_changed` plugin shape" - state what it is NOT modelled on.
  - Response:
- [ ] R1.9 (MINOR) web/src/wiki/dev/architecture.md:20 - the `nova_events` row
  still reads "Game event kinds and entity identity components", but the crate
  now owns the event engine (`GameEventsPlugin`, `EventWorld`, `EventKind`) that
  `:61` says moved there. Extend the row, add a `nova_events_macros` row (the
  wiki did not get the one `AGENTS.md` gained), and add the
  `events --> events_macros` edge to the mermaid graph at `:40`.
  - Response:
- [ ] R1.10 (MINOR) AGENTS.md:28 - same stale row: `nova_events` = "Game event
  kinds and entity identity components." Extend it to name the event engine the
  crate now owns.
  - Response:
- [ ] R1.11 (NIT) crates/nova_ui/src/status_bar.rs:20 and
  crates/nova_ui/src/tween.rs:52 - both copied files kept BCS's `pub mod
  prelude`, but nothing references `nova_ui::status_bar::prelude` or
  `nova_ui::tween::prelude` (verified), no other `nova_ui` module has one, and
  the same sub-preludes were deliberately dropped in `audio/registry.rs`,
  `audio/sfx.rs` and the two `nova_debug` files. Drop them for one convention
  per crate.
  - Response:
- [ ] R1.12 (NIT) crates/nova_debug/src/inspector.rs:12 - three public
  `DEBUG_TOGGLE_KEYCODE = F11` consts now coexist (here, `wireframe.rs:23`,
  `lib.rs:73`) and can silently drift, which is what `lib.rs`'s shared const
  exists to prevent. 9d ruled only on re-export. Point both copies at
  `crate::DEBUG_TOGGLE_KEYCODE` and delete them, or add a one-line note pinning
  each to the crate const.
  - Response:
- [ ] R1.13 (NIT) crates/nova_events/Cargo.toml:14 - the `[features]` block was
  deleted outright rather than reduced to `debug = ["bevy/track_location"]`;
  same for `nova_scenario`. Nothing selects either today and
  `bevy/track_location` unifies workspace-wide, so nothing observable changes -
  but `cargo check -p nova_events --features debug` now errors and these are the
  only two nova libs without the feature. Either restore
  `debug = ["bevy/track_location"]` in both, or record the deliberate drop.
  - Response:
- [ ] R1.14 (NIT) crates/nova_gameplay/src/plugin.rs:79 - the re-pointed
  registrations mix three addressing styles in one block
  (`crate::camera::wasd::WASDCameraPlugin`,
  `crate::transform::prelude::PointRotationPlugin`,
  `crate::lifetime::TempEntityPlugin`, `crate::mesh::prelude::ExplodeMeshPlugin`)
  while the file already does `use crate::prelude::*;`. Pick one form for the
  whole block.
  - Response:
- [ ] R1.15 (NIT) crates/nova_gameplay/src/camera/mod.rs:20 - the six vendored
  rigs are `pub mod` while nova's own five (`authority`, `framing`, `handback`,
  `mode`, `rig`) are private with an explicit `pub use`. If the rigs only need
  to be reachable through `camera::prelude`, make them private and `pub use` the
  names.
  - Response:
- [ ] R1.16 (NIT) crates/nova_gameplay/src/lib.rs:3 - the rewritten crate
  docstring module list adds `physics`, `objectives`, `lifetime` and `cooldown`
  but never names `math`, `mesh` or `transform`. Add the three so the list
  matches the `pub mod` block.
  - Response:
- [ ] R1.17 (NIT) web/src/wiki/dev/automation-harness.md:21 - "the
  shared-helpers crate" (also `:58`) names a crate that no longer exists
  anywhere in the repo, so a cold reader cannot resolve the referent. Either
  name the retired crate explicitly as history, the way CHANGELOG does, or state
  the current contract without the ghost.
  - Response:
- [ ] R1.18 (NIT) tasks/20260806-180450/TASK.md:541 - the Step 4 proof
  `git diff --exit-code crates/nova_gameplay/Cargo.toml Cargo.lock` (and `:512`)
  compares the worktree to HEAD, so it passes vacuously at any commit and cannot
  prove the step added no dep. Pin it to the step's own range, e.g.
  `git diff --exit-code <step3-sha>..HEAD -- ...`.
  - Response:

- Process signal: ten Steps, ~8k lines and roughly 6.5k LOC of vendored code in
  one TASK.md, run directly on master with no worktree per the owner's
  instruction. The record held up and every Step was commit-sized and
  independently provable, but this was an epic living in a single task record.
- Process signal: one DoD line was rewritten mid-flight - the `objective_marker`
  grep at `TASK.md:561` was tightened from `! grep -rn objective_marker crates/`
  to `objective_marker(::|;|\.rs)`. Disclosed with a correct rationale (the loose
  form matches the unrelated pre-existing `hud/objective_markers.rs`) and the
  intent is preserved. Noted because DoD edits during implementation are
  otherwise invisible at review time.
- Process signal: six of the eighteen findings are wrong DoD criteria, all six
  caught and disclosed by the implementer in the Progress notes rather than
  waved through. That is the record working; the gap is that a disclosed-wrong
  criterion still ships as the task's stated proof. Amending the DoD line at the
  moment of disclosure would close it.
- The `nova_ui` addition to `nova_core`'s `log_filter_str` (`lib.rs:231,235`) is
  NOT a scope finding: moving a module out of BCS silently drops its logs
  because the filter names crates explicitly, and Step 2 caught and fixed it.
  The half-completeness of that crate list is pre-existing and Step 10's note
  correctly flags it for its own task.
- Out of scope: the dead surface copied verbatim (`CameraShakeOutput`,
  `WASDCamera`, `EventHandlerIndex`, `RandomSphereOrbit`'s components) -
  `00-conventions.md` defers the sweep to a follow-up.
- Out of scope: `nova_assets`'s `debug` feature is declared but selected by
  nobody. Pre-existing.

### What the primary ran

`check --workspace --all-targets --features debug` zero errors; `fmt --check`
clean; `clippy --workspace --all-targets --features debug` (the exact CI pass,
which does not use `-D warnings`) zero errors and no warning in any file this
task created - the two `mesh/builder.rs` `chunks_exact` warnings came across
verbatim from BCS. Tests: `camera::` 28/28, `physics::` 13/13, `mesh::` 9/9,
`nova_ui tween::tests` 11/11, `objectives::` 1/1, `nova_debug inspector::` 4/4,
`nova_events --lib` 4/4. All twenty-five read-only `cmd:` proofs pass on their
stated criteria, except the three recorded above as R1.3, R1.5 and R1.7.

Load-bearing claims re-derived independently of the lanes: the `cd1bff21`
camera ordering (diffed `camera/authority.rs` against `cd1bff21`'s
`camera_controller/authority.rs` - every delta is prose, an import or a test
name; the `(Restore, Solve, Additive, Override).chain().before(Propagate)` and
both set-in-set folds are byte-identical), the `Cargo.lock` six-package delta,
and the five extra `PDControllerPlugin` registrations all being `#[cfg(test)]`.

### Pending user checks

These `manual:` proofs are the owner's, not the reviewer's, and do not block the
verdict. The Progress notes record a result for each; what is pending is the
owner's judgement of it.

- Step 5: debris geometry unchanged against the `20260805-185103` probe baseline
- Step 7: no attitude-control drift against the same baseline
- Step 8: sound plays once, not twice, in a running example
- Step 9: F11 still raises inspector, avian gizmos and wireframe as one layer,
  starting OFF
- Step 10: the crate graph gained exactly the two intended edges
- Step 10: the `cargo-about` third-party manifest - note the Progress record
  corrects 10h's premise twice (the manifest DID change, and it is gitignored
  and regenerated at release time, so nothing is committed here)
- Step 10: every example RUNS under Xvfb `:99` with no double-registration panic
  (recorded 23/23 PASS)
- Step 10: probe verdicts match the baseline (recorded sections 5, systems 3,
  stress 4, ui 5, all OK)

### Inspection commands

```
git diff f3cf3150..HEAD
diff <(git show cd1bff21:crates/nova_gameplay/src/camera_controller/authority.rs) crates/nova_gameplay/src/camera/authority.rs
git diff f3cf3150..HEAD -- Cargo.lock | grep -E '^[-+]name = '
grep -rniE 'bevy.common.systems' . --exclude-dir=tasks --exclude-dir=.git --exclude-dir=target --exclude-dir=news --exclude=CHANGELOG.md
nix develop --command cargo clippy --workspace --all-targets --features debug
```
