# Let sections declare sounds (like images/models), then move base sounds under assets/base/

- STATUS: OPEN
- PRIORITY: 32
- TAGS: v0.7.0,modding,audio,feature

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

## Notes

- Depends on the Option A base-migration (tasks 20260717-000416 / -002105 /
  -002133).
- Stepless direction-level task: run /plan before /work.
