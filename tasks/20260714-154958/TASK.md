# Assets + licenses shipping hygiene: exclude source assets (.blend) from the build, consolidate credits/licenses

- STATUS: OPEN
- PRIORITY: 20
- TAGS: backlog,chore,build,assets,licenses

Goal: stop shipping non-runtime source assets, and tidy license/credits so the
shipped build (web `dist/` via Trunk, and any native package) carries only what it
needs plus correct attribution.

## Don't ship source assets

`assets/` currently contains Blender SOURCES that get bundled into the shipped build
but are never loaded at runtime (the game uses the exported `.glb`):
- `assets/blender/*.blend` (3) + `*.blend1` (2) - ~the biggest wasted payload.
- also review the stray `assets/*.md` / any `.meta` that isn't a real loader-settings
  file, and confirm `.wav`/`.png`/`.glb`/`.wgsl`/`.ron` are all actually used.

Options (decide when planning):
- Relocate the sources OUT of `assets/` into a non-shipped dir (e.g. `art/` or
  `design/blender/`) - cleanest; the runtime `assets/` stays runtime-only. Update any
  build/export docs that point at `assets/blender`.
- OR keep them in-tree but exclude them from the shipped bundle (Trunk `dist` copy /
  the native packaging step). Relocation is simpler and less fragile.
Verify the web build (`trunk`) and any native zip only include runtime assets after.

## Consolidate credits / licenses

There are THREE credits/license copies today - top-level `CREDITS.md` + `LICENSE`,
`credits/CREDITS.md` + `credits/licenses/`, and a built `dist/credits/`. Clarify the
single source of truth and how it ships:
- Decide the canonical location (likely `credits/` as the source; `dist/credits/` is a
  build artifact - it should be generated/copied, not hand-maintained, and probably
  gitignored).
- Ensure EVERY third-party asset (fonts, sounds `assets/sounds/*.wav`, textures, gltf,
  any shader/lib) has a license entry with source + license name; audit for gaps.
- Confirm the shipped build (web + native) actually includes the credits/licenses the
  licenses require, and that the in-game/menu attribution (if any) points at them.

Scope note: pure release/shipping hygiene, independent of the modding work. Plan it
before doing (it is a stepless chore).
