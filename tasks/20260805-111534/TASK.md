# Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- STATUS: CLOSED
- PRIORITY: 66
- TAGS: v0.10.0, render, scenario, modding

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

- [x] `crates/nova_scenario/src/objects/light.rs` (new), per the `NOTES.md`
      sketch: `LightConfig::{Directional, Point}`, `LightMarker`,
      `ScenarioLightConfig`, `light_scenario_object`, `LightPlugin { render }`,
      the `Add<LightMarker>` observer, `LIGHT_TYPE_NAME = "light"`, and the
      `aimed_light_base` authoring helper. Module `prelude`, rustdoc on every
      public item. Field name is `shadow_maps_enabled` on both Bevy lights
      (confirmed against `kit.rs:84`).
- [x] Wire it: `objects/mod.rs` gets `pub mod light`, the prelude re-export, and
      `LightPlugin { render: self.render }` in `ScenarioObjectsPlugin`;
      `actions/spawn.rs` gets the `ScenarioObjectKind::Light(LightConfig)`
      variant and its dispatch arm.
- [x] Tests FIRST for the three DoD test proofs: a `Directional` object inserts
      `DirectionalLight` with the authored illuminance/color/shadows and applies
      `aim` over the base rotation; a `Point` object inserts `PointLight`;
      `render: false` inserts NEITHER. Plus a RON round-trip over both variants,
      since hand-authored mod RON is the point.
- [x] DELETE the hardcoded `DirectionalLight` spawn at `loader/lifecycle.rs:203`.
      From here until the sweep lands, scenes render black - this is why the
      delete and the relight are one commit.
- [x] Relight the 6 shipped builders (9 scenarios). At least one menu backdrop
      uses a `Point` light so both methods ship exercised. Then
      `cargo run -p nova_assets --bin content -- gen` and commit both sides.
      Never hand-edit `assets/base/scenarios/*.content.ron`.
- [x] Relight the 8 hand-authored mod RON files directly, using `aim` rather
      than hand-written quaternions. Spelling must match what serde emits for
      the regenerated shipped files (`kind: Light(Directional(...))`,
      `Srgba((red: ...))`) - copy from a generated file, do not guess.
- [x] Relight the 17 examples that build a `ScenarioConfig`, and
      `nova_editor/src/scenario.rs`.
- [x] Migrate the three photo-rig consumers (`screenshot_scene`,
      `screenshot_combat`, `screenshot_flight`) to author `kit.rs`'s EXACT
      numbers in their `ScenarioConfig`, then delete `photo_rig`,
      `PhotoRigLight` and `replace_key_light` from
      `examples/screenshots/shared/kit.rs`.
- [x] Decide: extract a shared three-point-rig helper into
      `nova_probe::fixtures` (its stated purpose - scenario shapes shared by
      examples, already 3+ callers) once the sweep shows the same rig repeated,
      or defer with the reason recorded. Do NOT build it speculatively before
      the sweep shows the shape.
- [x] Docs: `CHANGELOG.md` marked `(breaking)`;
      `web/src/wiki/dev/modding-ron.md` gains the light object with a copyable
      RON block; the scenario-authoring guides (`guide-author-scenario.md`,
      `guide-extend-scenarios.md`, `guide-make-a-mod.md`) gain a lighting
      section stating that a scene with no authored light renders black.
- [x] Verify by RUNNING under Xvfb `:99`, not `cargo check`: the three migrated
      screenshot examples (frames should be visually unchanged - same rig
      numbers), one relit example per category, the menu backdrops, and the
      editor play-test. Batch the frames for the owner's visual pass.

## Definition of Done

- [x] A scenario can author a directional light: the object spawns with
      `DirectionalLight` carrying the authored illuminance, color and shadow
      flag, and `aim` overrides the base rotation.
      (test: `directional_light_object_inserts_authored_light`)
- [x] A scenario can author a point light, and `render: false` inserts no light
      component at all. (test: `point_light_object_and_headless_render_flag`)
- [x] Both `LightConfig` variants survive a RON round-trip, so hand-authored mod
      files are a supported input. (test: `light_config_ron_round_trip`)
- [x] The engine no longer lights any scene.
      (cmd: `! grep -n 'DirectionalLight' crates/nova_scenario/src/loader/lifecycle.rs`)
- [x] Every shipped and mod scenario that spawns objects authors its own
      lighting; the two redirect scenarios stay unlit.
      (cmd: `bash -c 'test $(grep -l "kind: Light" assets/base/scenarios/*.content.ron webmods/*/*.content.ron assets/mods/example/*.content.ron | wc -l) -eq 17'`)
- [x] Every example that builds a scenario authors its own lighting.
      (cmd: `bash -c 'test $(grep -rlE "ScenarioObjectKind::Light|ThreePointRig" examples --include="*.rs" | grep -v "/shared/" | wc -l) -eq 17'`)
- [x] The screenshot examples' code-side photo rig is gone, replaced by authored
      lights. (cmd: `! grep -rnE 'photo_rig|PhotoRigLight|replace_key_light' examples`)
- [x] The generated content is in sync with the relit builders and lints clean.
      (cmd: `nix develop --command cargo run -p nova_assets --bin content -- gen && git diff --exit-code assets/base && nix develop --command cargo run -p nova_assets --bin content -- lint`)
- [x] The breaking mod-format change is documented where a mod author reads it.
      (cmd: `grep -q 'kind: Light' web/src/wiki/dev/modding-ron.md && grep -qi 'light' CHANGELOG.md`)
- [x] The workspace builds and documents warning-free.
      (cmd: `nix develop --command env RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug && nix develop --command cargo doc --workspace --no-deps`)
- [x] The relit scenes look GOOD, not merely not-black - judged against the
      `kit.rs` photo rig as the quality bar - and the three migrated screenshot
      examples capture visually unchanged frames.
      (manual: owner inspects the batched Xvfb frames - APPROVED 2026-08-05)

## Notes

Proof corrections made during work, both widening nothing:

- The example proof grepped for a literal `ScenarioObjectKind::Light` in each of
  17 files. The sweep authors its lights through `ThreePointRig`, the shared
  helper Step 8 asked for, which does not emit that literal - so the pattern now
  matches either form. Same strength (all 17 files must still author lighting),
  and `--include='*.rs' | grep -v /shared/` scopes it to the cataloged examples
  so `shared/kit.rs`'s doc mention cannot pad the count.
- The `RUSTFLAGS=-Dwarnings` proof was recorded as green on master. It is NOT:
  master fails it with 4 `ambiguous import visibility` errors in
  `nova_gameplay/src/hud/nova_os_{map,ship}/mod.rs`, unrelated to this task and
  predating it. Fixed here (disambiguated to `contacts::MapContactCode` /
  `sections::SectionCode`) because the proof cannot otherwise go green.

Step 8 decision - DEFER the `nova_probe::fixtures` extraction. The rig landed as
`ThreePointRig` in `nova_scenario::objects::light` instead, next to
`aimed_light_base`. That reaches BOTH consumer groups (the `nova_assets`
builders and the examples, which see it through `nova_protocol::prelude`); a
`nova_probe` copy would be a wrapper over a wrapper, reachable by only one of
them.

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

## Close-out

What and why. Lighting moved from the engine into content: `ScenarioObjectKind::Light`
(`Directional` + `Point`) on the existing `SpawnScenarioObject` path, and the loader's
hardcoded `DirectionalLight` deleted with no fallback. The relight is the proof, not the
tax - 9 shipped scenarios, 9 hand-authored mod scenarios (17 RON files), 17 examples and
the editor sandbox each spawn their own rig.

Alternatives taken differently from the plan:

- Step 8 asked whether to extract a rig helper into `nova_probe::fixtures`. Landed as
  `ThreePointRig` in `nova_scenario::objects::light` instead - one helper reaching BOTH
  the `nova_assets` builders and the examples (which see it via `nova_protocol::prelude`).
  A `nova_probe` copy would have served only one of the two.
- `ThreePointRig::scale` exists because a directional light reads only its DIRECTION, so
  the reel's numbers light a 6-unit hero shot and a 200-unit backdrop identically. That is
  what makes the screenshot migration exact: `around(prefix, ZERO, 1.0)` IS `kit.rs`.

Difficulties and diagnosis:

- Spawning `base_scenario_object` and the light bundle in ONE `world.spawn` panics on a
  duplicate `RigidBody` - the light bundle OVERRIDES the base's `Dynamic`, which only
  works as spawn-then-insert. The first test rig got this wrong; production was always
  right. Test harness now mirrors the action path exactly.
- `final_tally` needed rig scale 20, not 10: at 10 the key light sits inside the claim
  planetoid's worst-case geometric body and `final_tally_claim`'s clearance invariant
  fails. Caught by the existing test, not by eye.
- `assets/mods/example/example.content.ron` holds TWO scenarios (`example_arena` and
  `example_menu`); the first sweep lit only the first. Both are lit now.
- The three migrated screenshot examples do NOT capture byte-identical frames, and cannot:
  asteroid meshes seed from `GlobalRng` (entropy-seeded per run), so rock silhouettes
  differ every run regardless of this change. Measured rather than assumed - master-vs-
  master RMSE 0.100/0.109/0.119 vs master-vs-branch 0.106/0.114/0.096, i.e. the migration
  sits inside the pre-existing noise floor. The ship's lighting is visibly identical.

Evidence. All `cmd:` proofs green (engine light gone; 17 RON files; 17 examples; photo rig
gone; `content -- gen` + `git diff --exit-code assets/base`; `content -- lint` 0/0/0;
`RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --features debug`;
`cargo doc --workspace --no-deps`). All three `test:` proofs green, plus `nova_scenario`
148/148 and the full `nova_assets` suite. RUN under Xvfb `:99`, not just checked:
`menu_newgame`, `menu_scenarios`, `hull_section`, `many_bodies`, `scenario_grammar`,
`hud_range`, `scene_baseline` all exit 0 with `reached Playing` + `cycle complete, no
panic`; the three screenshot producers captured all 23 reel frames.

Frames for the owner's visual pass (the one pending `manual:` proof) are in the worktree
at `target/reel/*.png` (23 screenshot-producer frames) and `target/shots/menu_t32.png`
(the lit main-menu backdrop). Nothing renders black.

Reflection. The plan's claim that `RUSTFLAGS=-Dwarnings` was green on master was wrong -
master fails it with 4 pre-existing `ambiguous import visibility` errors in
`nova_gameplay`. Verified on master before touching them rather than assuming they were
mine. Worth checking a "green by design" guard proof actually is green at plan time; two
of this task's ten proofs were mis-recorded, and both only surfaced under a real run.

### Follow-up: the portal republish (owner catch)

The first pass relit the webmod `.content.ron` files but left their bundle
versions alone, which would have shipped the relight to nobody. The Mods screen
tags an installed mod as updatable on an EXACT version-string mismatch against
the catalog (`nova_menu/src/tests/portal.rs`), and `gen-portal.py` publishes each
release under `<id>/<version>/` - so republishing under an unchanged version
serves the relit bytes at the old path and every installed copy keeps its unlit
content and renders black. Per `guide-make-a-mod.md`'s own rule (content rework
-> bump the MINOR):

- `webmods/gauntlet` `1.3.0 -> 1.4.0`, `webmods/the-ledger` `1.14.0 -> 1.15.0`;
- both exact-version test pins updated (`gauntlet_course.rs`,
  `ledger_ch5_raid.rs`) - the repo's guard against exactly this omission, which
  would have caught it in CI;
- `assets/mods/example` is NOT bumped: it ships inside the game's assets rather
  than through the portal, so it updates with the game and has no install path
  to strand.

Two doc corrections in the same section: the version-history line was stale, and
the guide claimed there is "no update detection in code today", which is wrong -
nothing compares versions for ORDER, but the Mods screen compares them for
EQUALITY and that is exactly what makes a republish reachable.

Verified: `gen-portal.py --source webmods --shipped assets/mods.catalog.ron`
publishes `gauntlet 1.4.0` (4 files) and `the-ledger 1.15.0` (10 files), and the
relit content is present under the NEW version directories (3 lights in
gauntlet, 6 ledger files carrying lights). Full `nova_assets` suite green.

### Follow-up: review round 1

The shadow-map cost the relight adds, measured rather than argued (R1.2). Master
lit every scene with ONE non-shadowing `DirectionalLight`, so before this branch
gameplay rendered no directional shadow map at all; the rig's key light casts
one. `scene_baseline`, release, `asteroid_field`, Xvfb `:95`, 1280x720, RTX 3060
Ti, 900 frames per run:

| Run | session | mean ms | p50 ms | 1% low fps |
| --- | --- | --- | --- | --- |
| shadows on (as shipped) | 1 | 21.840 | 19.264 | 25.89 |
| shadows off | 1 | 21.590 | 19.182 | 25.73 |
| shadows off, repeat | 1 | 21.466 | 19.278 | 25.60 |
| shadows on (as shipped) | 2 | 21.625 | 19.165 | - |
| shadows on (as shipped) | 2 | 21.818 | 19.497 | - |
| shadows off | 2 | 20.794 | 19.134 | - |

Restated after review round 2 measured it again in a second session. Within one
session the mean delta reads 0.25 ms (~1.1%); across sessions it ranges 0.25 to
0.9 ms, and the between-session spread on mean (~0.8 ms) is itself larger than
the within-session noise (0.124 ms), so the MEAN delta is not resolvable at 900
frames per run. What both sessions agree on: the p50 delta stays under 0.4 ms
and the tail is unchanged (session 1's 1% low is marginally BETTER with shadows
on, which is noise). Kept on: it is the approved look, and a cost that will not
separate from run-to-run variance is not a reason to ship the flat version.

The `aim` path's runtime coverage (R1.1). `aim` is the only lighting path the 8
hand-authored mod RON files use, and it was exercised only under
`MinimalPlugins`. The suspected failure - the base bundle's physics body seeding
`Position`/`Rotation` from the spawn transform and interpolating the observer's
later `Transform` insert away - was tested directly and DOES NOT happen: the
aimed rotation survives 8 ticks of real `PhysicsPlugins` with
`angle_between == 0`. So the production spawn path is unchanged and the gap was
coverage, not a bug; `spawn_light` now builds its app with the real physics
stack (the `spawn` action's own harness shape) and ticks 8 times, which pins the
aimed pose where it can actually be falsified.

`broadside_gunship`'s rig prefix was `"tally"`, reading as `final_tally`'s scene;
renamed to `"gunship"` and regenerated (R1.3).
