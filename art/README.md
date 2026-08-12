# Source art

Non-runtime source art for Nova Protocol. Nothing here is loaded by the game or
shipped in a build - it is kept in the repo so the runtime assets can be
regenerated.

Keep it OUT of `assets/`: that directory is copied wholesale into every shipped
build (web via Trunk `copy-dir`, native via `release.yaml`), so anything there
adds to the download whether or not the game loads it.

## Contents

- `blender/` - the Blender sources (`.blend`) the runtime `assets/gltf/*.glb`
  models are exported from. `.blend1`/`.blend2` autosave backups are gitignored.
- `kenney-space-kit/`
    - URL: <https://kenney.nl/assets/space-kit>
    - License: CC0 1.0 Universal (public domain)
    - Version: 1.0.0 (2026-07-25)
- `texture-candidates/` - texture candidates under evaluation (task
  `20260812-100256`). Loaded from here by the `compare_asteroids` /
  `compare_planets` examples straight off disk, so nothing ships until a
  candidate is promoted into `assets/` with a credits entry. Per-asset detail:
  `tasks/20260812-100256/ASSETS.md`.
    - `ambientcg/` - Rock030, Rock035, Rock048, Rock062 color maps
        - Creator: ambientCG (Lennart Demes)
        - URL: <https://ambientcg.com/view?id=Rock030> (and Rock035/048/062)
        - License: CC0 1.0 (verified at <https://docs.ambientcg.com/license/>
          2026-08-12)
        - Obtained: 2026-08-12, 1K JPG zips via the download API; the
          `*_Color.jpg` maps converted losslessly to PNG (the shipped bevy
          build has no JPEG decoder)
    - `polyhaven/` - dark_rock_02 diffuse map
        - Creator: Rob Tuytel (Poly Haven)
        - URL: <https://polyhaven.com/a/dark_rock_02>
        - License: CC0 1.0 (verified at <https://polyhaven.com/license>
          2026-08-12)
        - Obtained: 2026-08-12, 1k JPG via api.polyhaven.com; converted to PNG
          (same JPEG-decoder reason)
    - `sbs-planets/` - Barren_01, Gaseous_01, Gaseous_08, Martian_01,
      Snowy_01, Tundra_01 equirect maps (1024x512)
        - Creator: Screaming Brain Studios
        - URL: <https://opengameart.org/content/planet-surface-textures>
        - License: CC0 1.0 (verified on the OGA page 2026-08-12)
        - Obtained: 2026-08-12, `sbs_planets_1024x512.zip` from the OGA
          mirror; PNGs copied as-is (renamed to drop the `-1024x512` suffix)
