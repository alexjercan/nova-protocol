# Review: Audio/SFX system

- TASK: 20260708-162011
- BRANCH: feature/audio-sfx-system

## Round 1

- VERDICT: APPROVE

The branch delivers the Goal: five cues play off real gameplay seams through the
reusable bcs `SfxPlugin`/`SoundBank`, placeholders are generated + committed, the
wav decoder is enabled correctly (on a normal dep, not the dev-only root bevy),
wasm is covered by the existing copy-dir, and the crate-tier boundary is
respected (only the event->sound map is game-specific). Independently ran
`cargo fmt --check` (clean), `cargo clippy --all-targets` (clean), and the audio
unit tests (4 pass). The three unit tests are meaningful (throttle semantics,
engine-volume mapping, key/file drift guard) and no existing test was weakened.
Deviations from the plan (observer seams over editing weapon systems;
`SoundBank::load` over the gated collection) are sound and documented.

Findings are all non-blocking and left to the implementer's discretion; given
this is a "feel & juice" release, R1.1 and R1.2 are worth a cheap follow-up.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/audio.rs:139 - the explosion cue is
  the one one-shot with no throttle, yet it is the most likely to stack: when a
  multi-section ship dies, every section gets `IntegrityDestroyMarker` in the
  same frame, so N explosions play at once (volume 0.6 each) and can clip. This
  is exactly the storm the turret/impact throttles guard against. Suggest either
  a short explosion min-interval in `SfxThrottle` (consistent with the other
  cues) or an explicit decision to accept a louder "ship died" stack, documented.
  - Response: fixed in follow-up commit. Added `last_explosion` to `SfxThrottle`
    and an `EXPLOSION_MIN_INTERVAL` of 0.06s, applied in
    `on_destroyed_play_explosion` the same way as turret/impact. A multi-section
    death now sounds one explosion; distinct kills >60ms apart still each sound.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/audio.rs:105 - `engine_volume` clamps
  the *sum* of per-thruster inputs to 1.0, but `ThrusterSectionInput` is a 0..1
  throttle per thruster (`thruster_impulse_system` does `input.clamp(0.0, 1.0)`),
  and a ship maneuvers with several thrusters at once. So the sum is >= 1
  whenever more than one fires, pinning the hum to max and making it effectively
  binary - the exponential smoothing then mostly smooths on/off transitions, not
  throttle level. Suggest averaging over the active thrusters (sum / count) so
  the hum tracks throttle proportionally.
  - Response: fixed in follow-up commit. `update_thruster_loop_volume` now
    averages the throttle over the active thrusters (sum / count) and
    `engine_volume` maps that 0..1 average, so the hum tracks burn intensity
    instead of pinning to max. Doc comments updated.
- [x] R1.3 (NIT) crates/nova_assets/src/lib.rs:register_sounds - inserts a fresh
  `SoundBank` on every `OnEnter(GameAssetsStates::Processing)`; on a scene reload
  that re-enters Processing this drops and reloads the handles. Harmless today
  (Processing is entered once), but a `resource_exists` guard or load-once would
  avoid the churn if scene reloads become common.
  - Response: left as-is (NIT). Processing is entered once in practice and the
    reload churn is harmless; a guard can be added if scene reloads become common.

## Round 2

- VERDICT: APPROVE

Verified the two fixes against the new diff:

- R1.1: `EXPLOSION_MIN_INTERVAL` (0.06s) + `last_explosion` in `SfxThrottle`,
  applied in `on_destroyed_play_explosion` - matches the turret/impact pattern.
  Confirmed. [x]
- R1.2: `update_thruster_loop_volume` folds to `(sum, count)` and feeds
  `sum / count` to `engine_volume`, whose semantics/doc are now "average
  throttle". Confirmed. [x]
- R1.3: accepted as a NIT, left for later. [x]

Re-ran `cargo fmt --check` (clean), `cargo clippy --all-targets` (clean), the
audio unit tests (4 pass, including the extended default-throttle test), and a
headless `BCS_AUTOPILOT=1 10_gameplay` run (reached Playing, cycle complete, no
panic, no sound asset errors). No new findings. Ready to merge.

## PR #53 - Audio/SFX system (PR-level review)

- PR: https://github.com/alexjercan/nova-protocol/pull/53
- BRANCH: feature/audio-sfx-system (10 commits) -> master
- REVIEWED AT: bc60cdd (branch tip)
- TASKS: 20260708-162011, -213155, -214821, -215922, -222026
- VERDICT: **APPROVE** (no blockers or majors; findings below are polish /
  robustness / coverage and can land before or after merge)

### Scope

Adds the game's first audio: five placeholder cues (explosion, impact, turret
fire, torpedo launch, and a continuous thruster hum) wired off existing gameplay
seams, plus the feel and correctness iterations that followed (distance
attenuation, a perceptual rolloff curve, a per-source throttle bugfix, and
editor-state gating). Playback reuses `bevy_common_systems` (`SfxPlugin`,
`SoundBank`); only the Nova-specific event->sound mapping lives in the new
`crates/nova_gameplay/src/audio.rs`. Design rationale is in
`tasks/20260708-162011/NOTES.md`; the per-task history is in the five
`tasks/*/TASK.md` files (task 162011 also has a `REVIEW.md`).

### Verification (re-run at branch tip)

- `cargo fmt --check` clean; `cargo clippy --all-targets` clean.
- `cargo test --workspace` green: 7 audio unit tests plus the existing suite and
  the `harnessed_examples_reach_playing_without_panic` integration test.
- Headless `BCS_AUTOPILOT` smoke on `10_gameplay` (scenario) and `09_editor`
  both reach Playing with no panic and no sound asset-load errors.
- Not verifiable headlessly (needs a GUI + audio device): audible playback,
  stereo behaviour, and "hold thrust in the editor build state = silent".

### What is good

- **Right reuse boundary.** The generic playback machinery stays in bcs; only the
  event->sound map (`NovaSfx`, `NovaAudioPlugin`, the observers) is in-repo,
  honouring the crate-tier policy.
- **Decoupled seams.** Cues ride existing markers/events via observers
  (`On<Add, IntegrityDestroyMarker>`, `On<HealthApplyDamage>`,
  `On<Add, *ProjectileMarker>`) instead of editing the weapon/integrity systems.
- **The right things are unit-tested.** `throttle`/`allow`, `engine_volume`,
  `distance_attenuation`, and `area_cell` are pure functions with meaningful
  tests - exactly the logic a headless run cannot check. One test
  (`throttle_is_independent_per_key`) directly pins the reported bug's fix.
- **Iterations were principled, not guesses.** The perceptual (geometric) rolloff
  addresses logarithmic loudness perception; the per-source throttle fixes a real
  "only one gun sounds" bug; the editor gating reuses the exact `run_if` the
  thruster physics/shader already sit behind.
- **Honest, thorough docs.** Limitations (no panning, area-cell heuristic,
  Scenario->Editor freeze edge), tuning knobs, and tradeoffs are all written down.

### Findings

Severities: BLOCKER / MAJOR / MINOR / NIT. None are blocking.

- [ ] F1 (MINOR, latent) `crates/nova_gameplay/src/audio.rs:213`
  (`listener_position`, and the four `q_camera` observers) - the listener is
  `q_camera.iter().next()`, which assumes exactly one `Camera3d`. Today that
  holds (the gameplay camera is spawned once in
  `crates/nova_scenario/src/loader.rs:167`; the editor spawns one in
  `crates/nova_editor/src/lib.rs:354`; `PostProcessingDefaultPlugin` adds a
  *component* to the existing camera, not a second one). But ECS query order is
  not guaranteed, so if a second `Camera3d` ever coexists - a minimap /
  render-to-texture / picture-in-picture, or an editor camera left alive when the
  scenario camera spawns - the listener would flip between cameras frame to frame
  and the attenuation would jitter. Suggest a dedicated listener marker (or a
  `Single<&GlobalTransform, With<MainCameraMarker>>`) rather than "first
  `Camera3d`". Worth a quick check that the editor->scenario transition does not
  already leave two `Camera3d` alive.

- [ ] F2 (MINOR) test coverage - every test is a pure-function unit test; nothing
  exercises the event->sound wiring end to end. `SfxPlugin` spawns an
  `AudioPlayer` entity on `PlaySfx` even without an audio device, so an
  integration test could build a minimal app with a `SoundBank<NovaSfx>` present,
  trigger `IntegrityDestroyMarker` / `HealthApplyDamage` / a projectile marker,
  and assert that a `PlaySfx` (or the resulting audio entity) was produced. That
  would guard the most important behaviour - "a gameplay event actually plays a
  sound" - against future refactors of the observers or the plugin wiring.
  Recommend adding one such test.

- [ ] F3 (MINOR, documented) `audio.rs:299` (`on_damage_play_impact`) -
  `HealthApplyDamage` auto-propagates up the hierarchy, so the observer fires for
  the damaged section and each ancestor (including the ship root, which sits at a
  different world position). The area-cell throttle collapses them only when they
  hash to the same cell, so a section more than `SFX_AREA_CELL` (6u) from its root
  can yield two impact sounds for one logical hit. Already noted in the code/docs
  as a heuristic; the exact fix (key by the entity's `IntegrityRoot`) is the
  recorded follow-up. Fine to defer.

- [ ] F4 (NIT) `audio.rs:205` (`play_positional`) - the audibility skip compares
  `base_volume * attenuation` against `SFX_AUDIBLE_THRESHOLD` *before*
  `SfxMasterVolume` is applied downstream by `SfxPlugin`. Correct at the default
  master (1.0); if an amplifying master (> 1) is ever added, near-threshold cues
  would be wrongly skipped. Minor layering inconsistency; note or fold the master
  into the decision if a master slider is added.

- [ ] F5 (NIT) `audio.rs:107-114` - `SFX_THROTTLE_PRUNE_WINDOW` and the
  `EXPLOSION_MIN_INTERVAL` doc block are crammed with no blank line, and the three
  `*_MIN_INTERVAL` constants are split by the area-cell / prune constants. A blank
  line and grouping the three intervals together would read better.

- [ ] F6 (NIT) commit history - the branch interleaves four `chore(tasks): plan
  ...` commits with the feature commits (10 total). Cosmetic; recommend
  squash-merge so master gets one clean commit, or fold the plan commits.

### Notes (not findings)

- The deliberate split - turret/torpedo read the projectile's local `Transform`
  (a freshly-spawned root entity's `GlobalTransform` is still identity that
  frame), while explosion/impact read `GlobalTransform` - is correct and
  well-commented. Future maintainers should not "unify" it to `GlobalTransform`.
- Turret fire is throttled to ~20/s (0.05s) while the PDC fires ~100/s, so ~4 of
  5 rounds are silent by design. It reads as rapid fire and avoids an audio-entity
  storm, but the audio cadence intentionally does not match the visual fire rate -
  keep in mind when tuning.
- Committing generated binary WAVs (~156 KB total, byte-deterministic from
  `scripts/gen-placeholder-sounds.py`) is a deliberate "runs out of the box"
  choice and is documented.

### Recommendation

Approve and merge (squash). F1 (camera listener robustness) and F2 (a wiring
integration test) are the two worth filing as small follow-ups; the rest are
nits or already-recorded heuristics.
