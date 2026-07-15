# Assets + licenses shipping hygiene: exclude source assets (.blend) from the build, consolidate credits/licenses

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: backlog, chore, build, assets, licenses

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

## Plan (2026-07-15) + decisions

Investigation findings:
- Both shipped builds bundle the WHOLE `assets/` tree: web via Trunk
  `copy-dir assets` (index.html), native via `release.yaml` (`cp -r assets/`
  for macOS, `tar ... assets` for linux). So `assets/blender/` (2.7M of
  `.blend` sources, never loaded at runtime) ships to BOTH - relocating it out
  of `assets/` fixes both with no build-config change.
- Credits are ALREADY single-source: `credits/` is canonical (copied to
  `dist/credits/` by Trunk and into the native bundle by release.yaml);
  `dist/` is gitignored, so `dist/credits/` is an untracked build artifact, not
  a hand-maintained copy. No top-level `CREDITS.md` remains (already reduced).
- `assets/textures/cubemap.png.meta` is a REAL Bevy ImageLoader settings file
  (cubemap array layout) - keep it, it is loader config not a stray.
- User decision (2026-07-15): all bundled image assets (icons, textures,
  banner) are ORIGINAL to the project; the only third-party shipped asset is the
  Bevy icon, already credited. So the asset license audit is: document that
  everything else is original.
- User decision (2026-07-15): relocate blender sources to a new top-level
  `art/blender/` (out of the runtime `assets/` tree).

Steps:
1. `git mv assets/blender/*.blend art/blender/`; move the untracked `.blend1`
   backups too; remove the empty `assets/blender/`. Verify `assets/` has no
   `*.blend*`.
2. Update `docs/architecture.md` (the `assets/ holds blender/ sources ...`
   line) to reflect the move.
3. Expand `credits/CREDITS.md` into an explicit audit: all game assets
   (sounds, textures, icons, 3D models, shaders) are original to the project;
   the Bevy icon is the one third-party asset (MIT, text in `licenses/`). Note
   Rust crate licenses (Bevy et al.) as a separate follow-up (cargo-about),
   out of scope here.
4. DOCS REFRESH (folded in per user request): rewrite
   `assets/sounds/README.md`'s "Required files" table to list all 16 cues (it
   listed only the 5 originals; the objective/lock/UI/pickup cues were missing).
5. Verify: `assets/` is runtime-only; the web `dist/` and native bundle carry
   no `.blend`.
