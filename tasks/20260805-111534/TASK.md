# Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- PRIORITY: 66
- TAGS: v0.10.0, render, scenario, modding
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955

## Context

`on_load_scenario` spawns exactly one `DirectionalLight` (illuminance 10000,
straight down) at `crates/nova_scenario/src/loader/lifecycle.rs:203`. Nothing
authorable can add to it, move it or remove it, so every shipped scene and every
mod scene reads flat. The only escape is Rust - `examples/screenshots/shared/kit.rs:62`
despawns the loader light from an observer and spawns a three-point rig - which
mods cannot do.

`DECISION.md` (ACCEPTED) is authority: lights become a fifth
`ScenarioObjectKind`, the engine light is DELETED outright with no fallback, and
relighting every rendering scene in the repo IS the deliverable. `NOTES.md`
holds the code sketch the owner reviewed.

## Inputs

The relight surface, verified against the tree at plan time (not NOTES' counts):

| Group | Files | Notes |
| --- | --- | --- |
| Shipped scenario builders | 6 `.rs` | `nova_assets/src/scenario.rs` (`asteroid_field`), `scenario/menu.rs` (3), `scenario/broadside.rs` (2), `scenario/final_tally.rs`, `scenario/lifeline.rs`, `scenario/shakedown/mod.rs` - 9 lit scenarios; regenerate RON with `content -- gen` |
| Hand-authored mod RON | 8 | `webmods/the-ledger/*` (6 with objects), `webmods/gauntlet/gauntlet.content.ron`, `assets/mods/example/example.content.ron` |
| Examples building a `ScenarioConfig` | 17 `.rs` | `grep -rl 'ScenarioConfig {' examples` - the full set that must author lights |
| Editor play-test | 1 | `nova_editor/src/scenario.rs`; the editor VIEW's own light (`nova_editor/src/ui/mod.rs:91`) is unaffected |

Deliberately UNLIT (redirect/index scenarios, zero spawn actions):
`assets/base/scenarios/asteroid_next.content.ron` and
`webmods/the-ledger/ledger_campaign.content.ron`. So 17 of the 19 shipped +
mod RON files carry lights.

Deliberately NOT relit: `examples/stress/scene_baseline.rs` and
`examples/screenshots/render_scale_shot.rs` load a SHIPPED scenario by id
(correcting NOTES.md, which listed `render_scale_shot` as needing fresh
lighting) - the builder relight covers them. Headless fixtures
(`nova_menu/src/tests/*`, `nova_scenario/src/{lint,loader}/fixtures.rs`,
`nova_assets/tests/*`) never render and get no lights.

Verified at plan time - no consumer forces a change beyond `actions/spawn.rs`:

- every other `ScenarioObjectKind` match is `if let` or has a `_ => {}` arm
  (`lint/ship.rs:19`, `lint_walk.rs:374`, `balance.rs:423,439`), so the lint
  needs no edit and no new rule;
- `object_count` (`loader/mod.rs:252`) counts spawn ACTIONS and is only ever
  asserted `> 0` (`nova_debug/src/harness.rs:308`);
- the NOVA OS map's `terrain` query filters `type_name.0 != "asteroid"`
  (`nova_gameplay/src/hud/nova_os_map/contacts.rs:255,361`), so a light entity
  carrying `EntityTypeName("light")` cannot appear as a contact;
- targeting admits `Static` bodies only WITH a `LockSignature`, which a light
  does not carry, so lights are unlockable.

## Steps

Ordered so the vocabulary is proven by tests BEFORE the sweep: a flaw in `aim`,
in the `Point` units or in the observer split must surface on one entity, not
after 30 files are edited.

- [ ] `crates/nova_scenario/src/objects/light.rs` (new), per the `NOTES.md`
      sketch: `LightConfig::{Directional, Point}`, `LightMarker`,
      `ScenarioLightConfig`, `light_scenario_object`, `LightPlugin { render }`,
      the `Add<LightMarker>` observer, `LIGHT_TYPE_NAME = "light"`, and the
      `aimed_light_base` authoring helper. Module `prelude`, rustdoc on every
      public item. Field name is `shadow_maps_enabled` on both Bevy lights
      (confirmed against `kit.rs:84`).
- [ ] Wire it: `objects/mod.rs` gets `pub mod light`, the prelude re-export, and
      `LightPlugin { render: self.render }` in `ScenarioObjectsPlugin`;
      `actions/spawn.rs` gets the `ScenarioObjectKind::Light(LightConfig)`
      variant and its dispatch arm.
- [ ] Tests FIRST for the three DoD test proofs: a `Directional` object inserts
      `DirectionalLight` with the authored illuminance/color/shadows and applies
      `aim` over the base rotation; a `Point` object inserts `PointLight`;
      `render: false` inserts NEITHER. Plus a RON round-trip over both variants,
      since hand-authored mod RON is the point.
- [ ] DELETE the hardcoded `DirectionalLight` spawn at `loader/lifecycle.rs:203`.
      From here until the sweep lands, scenes render black - this is why the
      delete and the relight are one commit.
- [ ] Relight the 6 shipped builders (9 scenarios). At least one menu backdrop
      uses a `Point` light so both methods ship exercised. Then
      `cargo run -p nova_assets --bin content -- gen` and commit both sides.
      Never hand-edit `assets/base/scenarios/*.content.ron`.
- [ ] Relight the 8 hand-authored mod RON files directly, using `aim` rather
      than hand-written quaternions. Spelling must match what serde emits for
      the regenerated shipped files (`kind: Light(Directional(...))`,
      `Srgba((red: ...))`) - copy from a generated file, do not guess.
- [ ] Relight the 17 examples that build a `ScenarioConfig`, and
      `nova_editor/src/scenario.rs`.
- [ ] Migrate the three photo-rig consumers (`screenshot_scene`,
      `screenshot_combat`, `screenshot_flight`) to author `kit.rs`'s EXACT
      numbers in their `ScenarioConfig`, then delete `photo_rig`,
      `PhotoRigLight` and `replace_key_light` from
      `examples/screenshots/shared/kit.rs`.
- [ ] Decide: extract a shared three-point-rig helper into
      `nova_probe::fixtures` (its stated purpose - scenario shapes shared by
      examples, already 3+ callers) once the sweep shows the same rig repeated,
      or defer with the reason recorded. Do NOT build it speculatively before
      the sweep shows the shape.
- [ ] Docs: `CHANGELOG.md` marked `(breaking)`;
      `web/src/wiki/dev/modding-ron.md` gains the light object with a copyable
      RON block; the scenario-authoring guides (`guide-author-scenario.md`,
      `guide-extend-scenarios.md`, `guide-make-a-mod.md`) gain a lighting
      section stating that a scene with no authored light renders black.
- [ ] Verify by RUNNING under Xvfb `:99`, not `cargo check`: the three migrated
      screenshot examples (frames should be visually unchanged - same rig
      numbers), one relit example per category, the menu backdrops, and the
      editor play-test. Batch the frames for the owner's visual pass.

## Definition of Done

- [ ] A scenario can author a directional light: the object spawns with
      `DirectionalLight` carrying the authored illuminance, color and shadow
      flag, and `aim` overrides the base rotation.
      (test: `directional_light_object_inserts_authored_light`)
- [ ] A scenario can author a point light, and `render: false` inserts no light
      component at all. (test: `point_light_object_and_headless_render_flag`)
- [ ] Both `LightConfig` variants survive a RON round-trip, so hand-authored mod
      files are a supported input. (test: `light_config_ron_round_trip`)
- [ ] The engine no longer lights any scene.
      (cmd: `! grep -n 'DirectionalLight' crates/nova_scenario/src/loader/lifecycle.rs`)
- [ ] Every shipped and mod scenario that spawns objects authors its own
      lighting; the two redirect scenarios stay unlit.
      (cmd: `bash -c 'test $(grep -l "kind: Light" assets/base/scenarios/*.content.ron webmods/*/*.content.ron assets/mods/example/*.content.ron | wc -l) -eq 17'`)
- [ ] Every example that builds a scenario authors its own lighting.
      (cmd: `bash -c 'test $(grep -rl "ScenarioObjectKind::Light" examples | wc -l) -eq 17'`)
- [ ] The screenshot examples' code-side photo rig is gone, replaced by authored
      lights. (cmd: `! grep -rnE 'photo_rig|PhotoRigLight|replace_key_light' examples`)
- [ ] The generated content is in sync with the relit builders and lints clean.
      (cmd: `nix develop --command cargo run -p nova_assets --bin content -- gen && git diff --exit-code assets/base && nix develop --command cargo run -p nova_assets --bin content -- lint`)
- [ ] The breaking mod-format change is documented where a mod author reads it.
      (cmd: `grep -q 'kind: Light' web/src/wiki/dev/modding-ron.md && grep -qi 'light' CHANGELOG.md`)
- [ ] The workspace builds and documents warning-free.
      (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug && nix develop --command cargo doc --workspace --no-deps`)
- [ ] The relit scenes look GOOD, not merely not-black - judged against the
      `kit.rs` photo rig as the quality bar - and the three migrated screenshot
      examples capture visually unchanged frames.
      (manual: owner inspects the batched Xvfb frames)

## Notes

Assumptions, stated so review can hit them:

- The `kit.rs` rig numbers transfer to authored RON unchanged, so the three
  migrated screenshot examples capture identical frames. Verified by RUNNING,
  not by `cargo check`.
- A `RigidBody::Static` light carrying the base bundle's `TransformInterpolation`
  is inert, as it is on beacons.
- `Point` light intensity in lumens needs scene-scale tuning by eye; the
  sketch's `2_500_000.0` is a starting point, not a verified value.

Proof honesty: the five grep/count proofs above were run on master at plan time
and are all red. The `content -- gen` + `git diff --exit-code` proof and the
`RUSTFLAGS=-Dwarnings` proof are GREEN on master by design - they are sync and
regression guards that only turn red mid-task (builders relit but not
regenerated; a new public item without rustdoc), not change detectors.

Not split, deliberately. The vocabulary alone is committable but is exactly the
proof the decision refuses to ship without. The delete and the relight cannot
separate in either order: deleting first leaves every scene black, relighting
first double-lights every scene and makes the visual judgement worthless. So
this lands as one commit and the size is accepted, not designed away.

RISK, stated rather than buried: one commit touching ~32 files with a manual,
taste-based acceptance criterion over ~30 scenes. If the owner's visual pass
rejects a group of scenes, the fix cycle re-runs the sweep for that group; two
such cycles is the flow's stop condition. The step order (API proven on one
entity before the sweep) mitigates the mechanical half; nothing mitigates the
taste half beyond doing the menu backdrops first and showing them early.

Deferred, NOT this task: `base_scenario_object` hands every scenario object
`RigidBody::Dynamic` + `TransformInterpolation`, which lights make the second
kind to override after beacons. Worth rethinking once non-physical objects are
established; lights follow the beacon precedent here.
