# Review: Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- TASK: 20260805-111534
- BRANCH: feature/authorable-lighting

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_scenario/src/objects/light.rs:248 - the `aim`
  branch is the ONLY lighting path every published mod scenario uses (8
  hand-authored RON files carry `aim: Some(..)`; `ThreePointRig` always emits
  `aim: None`), yet it is exercised only under `MinimalPlugins`, where no avian
  plugin derives `Position`/`Rotation` from the spawn-time `Transform` and no
  `TransformInterpolation` writes one back - so the observer's post-spawn
  `Transform` insert may be silently reverted in a real app and no run or frame
  would show it. Author `aim: Some(..)` in one relit example's rig and RUN it
  under Xvfb `:99`, keeping the frame as evidence.
  - Response: coverage gap real, suspected mechanism falsified. A scratch rig
    span the base bundle, inserted the aimed `Transform` post-spawn exactly as
    the observer does, and ticked 8 times under real `PhysicsPlugins`: the
    aimed rotation survived, `angle_between == 0`. So avian does not interpolate
    it away and the production path needed no change. Fixed the coverage
    instead: `spawn_light` (light.rs:292) now builds its app with
    `TransformPlugin` + `AssetPlugin` + `MeshPlugin` + `PhysicsPlugins` -
    the `spawn` action's own harness shape - and ticks 8 times, so the aim
    assert runs against the stack that could break it. Fixed in the round-1
    commit; no Xvfb frame, because the falsification is stronger evidence than
    a screenshot and nothing renders a mod scenario without new example code.
- [x] R1.2 (MAJOR) crates/nova_scenario/src/objects/light.rs:128 - master's
  engine light was ONE `DirectionalLight` with `shadow_maps_enabled` at Bevy's
  `false` default (`loader/lifecycle.rs:202` on master); every relit scene now
  ships three directional lights, one of them shadow-casting, so all 9 shipped
  scenarios, the menu backdrops and the mod scenes take an unmeasured
  per-frame shadow-map cost - on the WASM build too. Run `nova_probe` before
  and after on one gameplay scenario and record the frame-time delta in
  TASK.md, or set `THREE_POINT_LIGHTS`' key entry to `shadows: false` and keep
  the shadow caster only in the screenshot rigs.
  - Response: measured, kept on. `scene_baseline` release, `asteroid_field`,
    Xvfb `:95`, 1280x720, RTX 3060 Ti, 900 frames: shadows on 21.840 ms mean /
    19.264 p50, shadows off 21.590 / 19.182, and an identical shadows-off
    repeat 21.466 / 19.278. The shadow map costs 0.25 ms mean (~1.1%); the p50
    delta of 0.082 ms is below the 0.096 ms run-to-run noise and the 1% low is
    unchanged. Table recorded in TASK.md's round-1 follow-up. Not turned off:
    it is the look the owner approved, and 1% of a frame does not buy the flat
    version back.
- [x] R1.3 (NIT) crates/nova_assets/src/scenario/broadside.rs:638 - the gunship
  scene's rig prefix is `"tally"`, which reads as `final_tally`'s scene (that
  one uses `"anchorage"`), so the generated object ids are `tally_key/rim/fill`
  in the wrong scenario; rename the prefix to `"gunship"` and regenerate.
  - Response: fixed in the round-1 commit - prefix is `"gunship"`,
    `content -- gen` regenerated `broadside_gunship.content.ron` (3 ids).
- [x] R1.4 (NIT) crates/nova_scenario/src/objects/light.rs:95 -
  `aimed_light_base` is `pub` and prelude-exported (line 14) with exactly one
  caller, `ThreePointRig::objects` at line 183, and no user in `crates/`,
  `examples/` or the mods; make it private and drop it from the `prelude` list
  until a second caller appears. NIT rather than MAJOR YAGNI: Step 1 names it
  as a deliverable of the public authoring surface, so shipping it is the plan
  being followed, not invented generality.
  - Response: pushback, left public. Step 1 names `aimed_light_base` as part of
    the module's authoring surface, and it is the one helper a scene needs to
    author a SINGLE light in Rust without hand-writing a quaternion -
    `ThreePointRig` covers only the three-light rig. Making it private would
    force the next one-light scene to either reach for a quaternion or widen
    the API back. No behavior at stake either way.

Re-derived by the recording pass, independent of the round-1 reviewer:

- All ten `cmd:`/`test:` DoD proofs run verbatim in the worktree and green:
  engine light gone; 17 lit RON files; 17 lit examples; no
  `photo_rig|PhotoRigLight|replace_key_light`; `content -- gen` +
  `git diff --exit-code assets/base` clean; `content -- lint` 0/0/0;
  `RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`
  green; `cargo doc --workspace --no-deps` emits nothing for `nova_scenario`;
  `cargo test -p nova_scenario --lib light` 3/3.
- `THREE_POINT_LIGHTS` (light.rs:121-146) carries master `kit.rs`'s
  `replace_key_light` offsets, illuminances, colors and shadow flags verbatim,
  and the three migrated screenshot examples call
  `ThreePointRig::around(prefix, Vec3::ZERO, 1.0)`, so the close-out's "same
  rig numbers" claim holds at the source level.
- The three new tests assert behavior, not execution: the directional test
  seeds a base rotation of `Quat::IDENTITY` deliberately not pointing at the
  target, so the `aim` assert cannot pass by coincidence, and each would fail
  with the insert observer deleted. No existing test was weakened; the two
  webmod version pins were tightened.
- The webmod bundles are republished at bumped versions (`gauntlet 1.4.0`,
  `the-ledger 1.15.0`), so the relit content reaches installed copies.
- Doc sweep: no stale `photo_rig` / engine-light mentions anywhere outside the
  exempt `tasks/` tree.

Pending user check (does not block a verdict): the one open `manual:` proof -
the owner's visual pass over the batched Xvfb frames, recorded APPROVED
2026-08-05 in commit 7089b30f. Note the approved batch contains no mod
scenario, which is what R1.1 asks to cover.

- Process signal: the example DoD proof was rewritten mid-task (literal
  `ScenarioObjectKind::Light` -> `ScenarioObjectKind::Light|ThreePointRig`).
  The change is honestly recorded in TASK.md and the widened set is exactly
  `grep -rl 'ScenarioConfig {' examples`, so it did not weaken the count.
- Out of scope: `cargo doc` emits ~30 private-item-link warnings across
  `nova_gameplay`, `nova_debug`, `nova_assets` and `nova_ui` on master.
- Out of scope: master fails
  `RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`
  with 4 `ambiguous import visibility` errors in `nova_gameplay`; this branch
  fixes them because the proof cannot otherwise go green.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

Round 1's four findings: R1.1 CONFIRMED FIXED, R1.2 CONFIRMED FIXED, R1.3
CONFIRMED FIXED, R1.4 PUSHBACK ACCEPTED. The reviewer re-derived R1.1's
mechanism independently (the light bundle overrides the body to
`RigidBody::Static` and the `On<Add, LightMarker>` observer's `Transform`
insert flushes before the entity's first physics prepare, so `Position` and
`Rotation` are seeded from the aimed pose) and rebuilt `scene_baseline` at HEAD
to re-measure R1.2. `git diff 8fd58aab..HEAD -- objects/light.rs` shows zero
changed `assert` lines, so the harness change weakened no DoD test.

- [x] R2.1 (MINOR) tasks/20260805-111534/TASK.md:299 - the "0.25 ms mean /
  ~1.1%" cost and the "0.124 ms noise floor" are single-session artifacts: a
  second session measured 21.625 and 21.818 mean with shadows on against 20.794
  with them off, a ~0.9 ms gap, and a between-session spread on mean (~0.8 ms)
  far larger than the stated noise floor; restate as "p50 delta under 0.35 ms,
  mean delta not resolvable at this run count (0.25-0.9 ms across sessions)"
  and keep the "kept on" conclusion, which the data still supports.
  - Response: fixed - the table now carries both sessions' runs and the prose
    says the mean delta is not resolvable at 900 frames per run, with the p50
    bound and the unchanged conclusion.
- [x] R2.2 (NIT) crates/nova_scenario/src/objects/light.rs:296 - the new harness
  docstring claims it "must catch" an aimed pose that only survives under
  `MinimalPlugins`, but the light bundle overrides the body to
  `RigidBody::Static` and the aimed `Transform` lands before the entity's first
  physics prepare, so avian never writes it back and the test has no failure
  mode of that kind; reword to what it actually pins.
  - Response: fixed - the docstring now states what it pins (the authored pose
    under the production plugin stack, the `Static` body inert across ticks)
    instead of a catch it cannot make.

Both round-2 findings are record and comment text, fixed in the same round; the
round-2 reviewer verified each fix (and the session-2 numbers against its own
run JSON) before its box was ticked. Neither is blocking. The verdict line was
written by the recording pass from the reviewer's per-finding confirmations, not
by the reviewer itself; no BLOCKER or MAJOR is open in this round.

- [ ] R2.3 (NIT) tasks/20260805-111534/TASK.md:306 - the restated prose bounds
  the p50 delta at "under 0.35 ms" while its own session-2 rows give
  19.497 - 19.134 = 0.363 ms, so the bound contradicts the table one line above;
  change it to "under 0.4 ms".
  - Response: fixed - the bound reads "under 0.4 ms".

- Process signal: the R1.1 Response leans on a scratch rig that is not in the
  tree. The in-tree `spawn_light` harness is equivalent evidence and passes, but
  the recorded `angle_between == 0` is uncorroborated on its own terms - the
  in-tree assert is `< 1e-4`.
- Out of scope: the pre-existing `RigidBody::Dynamic` + `TransformInterpolation`
  on every scenario object, already deferred in TASK.md.
- Out of scope: `cargo doc`'s ~30 private-item-link warnings across
  `nova_gameplay`, `nova_debug`, `nova_assets` and `nova_ui` on master.

Pending user check (does not block APPROVE): the one open `manual:` proof, the
owner's visual pass over the batched Xvfb frames, recorded APPROVED 2026-08-05
in commit 7089b30f and unchanged by either round.
