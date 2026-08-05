# Decision: Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- DATE: 20260805-161647
- STATUS: ACCEPTED
- TASK: 20260805-111534
- TAGS: render,scenario,modding,breaking

## Context

`on_load_scenario` hardcodes exactly one `DirectionalLight` (illuminance 10000,
straight down) at `crates/nova_scenario/src/loader/lifecycle.rs:203`. Nothing
authorable can add to it, move it or remove it, so every shipped scene and
every mod scene reads flat: no rim, no fill, no separation from the skybox, and
almost nothing bright for the camera's `Bloom::NATURAL`. The only escape is
Rust - `examples/screenshots/shared/kit.rs:62` despawns the loader's light from
an observer and spawns a three-point rig - which mods cannot do.

Full problem and context: `NOTES.md` in this task folder.

## Decision

**Lights become a scenario object kind, and the engine stops lighting scenes.**

1. Add `ScenarioObjectKind::Light(LightConfig)` as a fifth kind on the existing
   `SpawnScenarioObject` action path (`nova_scenario/src/actions/spawn.rs`),
   with a `nova_scenario/src/objects/light.rs` module and plugin registered in
   `objects/mod.rs`. `BaseScenarioObjectConfig` supplies id, name, position and
   rotation; the light bundle overrides `RigidBody::Static` over the shared
   base's `Dynamic`, the way `beacon_scenario_object` already does.
2. `LightConfig` covers two methods from the start:
   `Directional { illuminance, color, shadows }` and
   `Point { intensity, range, radius, color, shadows }`. Supporting two makes a
   scene's lighting method a one-line RON change later.
3. **Delete the hardcoded light at `lifecycle.rs:203` outright.** No fallback,
   no reserved entity id. A scene looks like exactly what it authored.
4. Relight every rendering scene in the repo as the feature's proof, lit to
   look good rather than merely to not-be-black, judged by owner visual
   inspection with the `kit.rs` photo rig as the quality bar:
   - 10 shipped scenarios via their 7 Rust builders, then `content -- gen`;
   - 9 hand-authored mod scenarios (`webmods/the-ledger/*`,
     `webmods/gauntlet`, `assets/mods/example`), edited directly;
   - 13 rendering examples plus the 6 screenshot examples;
   - the editor play-test scenario (`nova_editor/src/scenario.rs`).
   At least one menu backdrop uses a `Point` light, so both methods ship
   exercised.
5. The three screenshot examples carrying `kit::photo_rig()`
   (`screenshot_scene`, `screenshot_combat`, `screenshot_flight`) move the rig's
   exact numbers into their authored `ScenarioConfig` and drop the observer.
   `kit.rs` loses `photo_rig`/`PhotoRigLight`/`replace_key_light`.
6. No new content-lint rule.

Headless fixtures (`nova_menu/src/tests/*`, `nova_scenario/src/*/fixtures.rs`)
never render and get no lights.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| `EventActionConfig::SpawnLight` sibling action, next to `SetSkybox` | Honest about a light having no body, and despawn is free either way (kind-agnostic `EntityId` match). But it re-declares the id/name/transform the object envelope already standardises and sits outside `ScatterObjects`, the lint walk and `object_count`. One enum variant beat one parallel vocabulary. |
| Keep the light as a fallback when a scenario authors none | Zero breakage for 9 hand-authored mod scenes and every third-party mod. Rejected by the owner after being shown the blast radius: relighting everything IS the deliverable, and the fallback preserves the magic light the task exists to remove. |
| Delete the engine light, ship a `default_lighting()` Rust helper | Cheap for the Rust-side scenes, worthless for the hard half: a helper is unreachable from hand-authored RON, so the 9 mod scenes spell the block out and break anyway. |
| Named lighting presets ("key+rim", "backlit", "flat") | TASK.md's own YAGNI-preferred answer at filing time. A naming layer over a vocabulary that does not exist yet; if ever wanted it is a builder-side helper over authored lights, not RON vocabulary. |
| Directional-only `LightConfig` | Covers every named requirement (the photo rig is 3 directionals). Overridden by the owner: two methods from the start keeps the enum shape honest and makes swapping cheap. |
| Add a lint rule for degenerate lights (illuminance <= 0, black color) | A black scene is an authoring mistake the eye catches during the relight, not a class the lint can usefully name. Costs a rule, fixtures, and a judgement call about "too dark". |

## Consequences

- **Breaking mod-format change.** Third-party mod scenarios outside this repo
  render black until their authors add light actions. Ships marked `(breaking)`
  in `CHANGELOG.md` and in `web/src/wiki/dev/modding-ron.md`, with the
  scenario-authoring guides gaining a lighting section.
- Large diff by touch-site count (~38 scenes) but shallow: one new module, one
  new enum variant, one deletion, and per-scene authored light lists.
- Verification is visual and must RUN the examples (Xvfb `:99`), not
  `cargo check` them. Screenshot examples are the regression check: same rig
  numbers should mean visually identical frames.
- `assets/base/scenarios/*.content.ron` is generated - edit the builders, run
  `content -- gen`, commit both. `webmods/*` is hand-authored and edited
  directly.
- New public items (`LightConfig`, the light module's plugin and bundle fn)
  need prelude exports per the repo rule, and rustdoc that keeps
  `cargo doc --workspace --no-deps` warning-free.
