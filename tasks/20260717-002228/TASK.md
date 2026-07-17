# Let sections declare sounds (like images/models), then move base sounds under assets/base/

- STATUS: CLOSED
- PRIORITY: 32
- TAGS: v0.7.0, modding, audio, feature

## Context (from the base-as-normal-mod work, 2026-07-16)

Option A (task 20260716-235458 spike) moved base gltf + textures + banner.png
under `assets/base/` and made everything reference art via `self://`/`dep://base`.
`sounds/` was DELIBERATELY left at the asset root because mods cannot yet declare
or ship sounds the way they ship images and models (as `AssetRef` content fields
+ `resources`). Moving base sounds under `assets/base/` only makes sense once mod
content can reference sounds through the same scheme pipeline.

## Goal

Give audio the same authorable, mod-shippable treatment images and GLB models
already have: a section (and/or scenario) can declare a sound as an `AssetRef`
content field, ship it in `resources`, and reference it with
`self://`/`dep://base`/`dep://<id>`. Then move base `sounds/` under
`assets/base/` and repoint, closing the last root-art exception.

## Direction (for /plan)

- Audit how audio is currently wired (nova_gameplay audio, base `sounds/`, any
  hardcoded GameAssets audio handles) and what an authorable sound `AssetRef`
  field would attach to (section events? scenario actions?).
- Add the `AssetRef<AudioSource>` (or equivalent) content field(s); resolve at
  spawn like other AssetRefs; they flow through the same `self://`/`dep://`
  rewrite + membership gates automatically (the generic walk already covers any
  AssetRef string field).
- Move base `sounds/` under `assets/base/`, update GameAssets audio paths +
  gen_content, add to base `resources`.
- Tests + docs.

## Plan (2026-07-17)

### Design decision: scope + mechanism (verified against the code)

The audit found the audio system is mature and heavily tested: ALL playback goes
through a global `SoundBank<NovaSfx>` keyed by an enum (`crates/nova_gameplay/src/
audio.rs`), loaded by `nova_assets::register_sounds` via `SoundBank::load(assets,
NOVA_SFX_FILES)` which applies the `sounds/<name>.wav` convention. Section configs
hold NO sound handles today. So "sections declare sounds like images/models" is a
real design decision, not a mechanical copy of `render_mesh`.

- EXEMPLAR: the turret **fire sound**. It is the cleanest section-owned one-shot
  and there is an exact template already in the tree: `muzzle_effect:
  Option<AssetRef<EffectAsset>>` on `TurretSectionConfig` (turret_section.rs:124),
  stored on a component (`TurretSectionBarrelMuzzleEffect`, :275/:450) and resolved
  when the section spawns (`insert_turret_barrel_muzzle_effect`, :1596). A
  `fire_sound: Option<AssetRef<AudioSource>>` mirrors it precisely.
- CONSUMPTION: `on_turret_fire_play_sfx` (audio.rs:463) already carries the
  `TurretSectionPartOf(turret)` back-ref on the projectile. It queries the turret
  for its resolved fire-sound handle and PREFERS it over `bank.get(NovaSfx::
  TurretFire)`, falling back when absent. All throttle/attenuation/positioning
  logic is untouched - only the HANDLE SOURCE becomes per-section-overridable.
- OUT OF SCOPE (documented, not silently dropped): damage (Impact/Explosion) and
  UI cues stay on the global bank - they are code-driven, not section-owned. The
  thruster loop is architecturally a single global entity whose volume is a max
  over ships, so it is not per-section either. Torpedo launch is a clean future
  extension (needs a section back-ref on the projectile - verify at work). These
  become a follow-up if the reviewer wants parity beyond the exemplar.
- Base playback is UNCHANGED: base content declares `self://sounds/turret_fire.wav`
  which rewrites to `base/sounds/turret_fire.wav` at merge and resolves to the SAME
  handle `register_sounds` loads for `NovaSfx::TurretFire` (asset server dedups by
  path). The override only diverges for a MOD shipping its own sound - which is the
  moddability win and what the round-trip test proves.

### Steps

- [x] Add `fire_sound: Option<AssetRef<AudioSource>>` to `TurretSectionConfig`
      (turret_section.rs:34), mirroring `muzzle_effect`: `#[reflect(ignore)]` +
      `serde(default, skip_serializing_if = "Option::is_none")`; `None` in
      `Default`. Store it on the turret entity at spawn as a component (mirror
      `TurretSectionBarrelMuzzleEffect`), keeping the `AssetRef` and resolving it
      lazily, OR resolve to a `Handle<AudioSource>` component - pick whichever
      matches how muzzle_effect/render_mesh resolve (verify at work).
- [x] Consume in playback: in `on_turret_fire_play_sfx` (audio.rs:463) query the
      firing turret (via `TurretSectionPartOf`) for its fire-sound handle; play it
      through the existing `play_positional`/throttle path when present, else fall
      back to `bank.get(NovaSfx::TurretFire)`. Do NOT change throttling/attenuation.
- [x] gen_content emits the ref: add a `turret_fire_sound: AssetRef<AudioSource>`
      to `SectionMeshRefs` (nova_assets/src/sections.rs) emitting
      `self://sounds/turret_fire.wav` from `from_paths`, wire it into the turret
      builder's `fire_sound`, and regenerate `assets/base/**/*.content.ron`
      (`cargo run -p nova_assets --bin gen_content`). `content_ron_parity` proves
      the committed files match the builders.
- [x] Move base sounds: `git mv assets/sounds assets/base/sounds` (16 wav +
      README.md); `rm -rf` the emptied `assets/sounds/` if git leaves it (LESSONS:
      git-mv-leaves-empty-parent). Verify no gitignored siblings remain in the main
      checkout after landing (LESSONS: relocation-leaves-ignored-siblings).
- [x] Repoint the base-game direct load: find where the `sounds/<name>.wav`
      convention is applied (`SoundBank::load` in bevy_common_systems and/or
      `NOVA_SFX_FILES`/`register_sounds` in nova_assets lib.rs:~1075) and change it
      to `base/sounds/<name>.wav` so the global bank loads from the new location.
- [x] Declare all 16 sounds in base `resources` (`assets/base/base.bundle.ron`) as
      relative `sounds/<name>.wav`, so mods can reference them via
      `dep://base/sounds/<name>.wav`. Emit via gen_content if `resources` is
      generated (Option A generated it, so it cannot drift from the builders).
- [x] Repo-wide sweep for other `sounds/` refs - `examples/**`, test data,
      `include_str!` embedded RON, webmods, the screenshot reel - NOT just
      `assets/` (LESSONS: sweep-content-repo-wide-not-just-assets; the reel miss in
      retro 20260717-002105). Dump the sweep in full, count matches (LESSONS:
      truncated-sweep-is-not-a-sweep). Repoint any content refs to
      `dep://base/sounds/` or `base/sounds/` per whether they load via the merge.
- [x] Tests (integration/App-rig preferred, AGENTS.md): (a) a turret whose config
      declares a `fire_sound` plays THAT handle, and a turret without one falls
      back to the bank key - with a delivery guard that the cue fires at all
      (LESSONS: delivery-guards-on-null-assertions); (b) a mod shipping its own
      sound under `self://`+`resources` round-trips through the rewrite + membership
      gate and loads (extend `webmods_validation`/`content_lint_gate` with a fixture
      turret carrying `dep://base/sounds/...` or a self:// sound); (c)
      `content_ron_parity` + `content_lint_gate` stay green. Grep test fixtures for
      old `sounds/` refs up front (LESSONS retro 20260717-002133).
- [x] Docs: update the modding guide(s) that list authorable AssetRef fields to
      include sounds; CHANGELOG (player-facing: mods can ship + reference sounds,
      base sounds moved under base/); `assets/base/sounds/README.md`; and the dev
      wiki keeping-docs-in-sync map if a sounds surface is missing. Write a
      docs/ note on the decision + difficulties (AGENTS.md).

## Notes

- Depends on the Option A base-migration (tasks 20260717-000416 / -002105 /
  -002133) - all CLOSED.
- Verified files: audio.rs (global bank + observers), turret_section.rs
  (muzzle_effect AssetRef template), nova_assets/src/sections.rs (SectionMeshRefs +
  gen_content), base.bundle.ron (resources), content_lint_gate.rs.
