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
- `spaceship-blocks/` - Fertile Soil Productions "Spaceship Blocks Collection"
  (95 OBJ+MTL modular ship pieces, flat Kd colours, no textures)
    - URL: <https://fertile-soil-productions.itch.io/spaceship-blocks-collection>
    - License: CC0 (verified on the itch.io page 2026-08-12; the zip ships no
      license file, this entry is the record)
- `quaternius-ultimate-spaceships/` - Quaternius "Ultimate Spaceships" pack
  (11 ships), imported as BAKED flat-Kd OBJ+MTL conversions under `baked/`
  (one colour variant per ship; each `.mtl` header records its source ship +
  atlas variant)
    - URL: <https://quaternius.com/packs/ultimatespaceships.html>
    - License: CC0 1.0 Universal (the pack's own `License.txt`; copy at
      `credits/licenses/Quaternius_Ultimate_Spaceships_License.txt`, verified
      2026-08-12)
    - Obtained: 2026-08-12, `Ultimate Spaceships - May 2021` zip. The source
      OBJs + 2048px palette-atlas textures (~260 MB, 5 variants per ship) are
      NOT committed - the pack is a palette-atlas pack (one grey `Kd`, colours
      in the UV texture), so the repo carries the flat-Kd bakes produced by
      `scripts/bake-atlas-to-kd.py` instead; re-baking (e.g. another colour
      variant) needs the zip.
- `part-candidates/` - GENERATED part `.glb` candidates for the parts-based
  ship building spike (task 20260812-100246), browsed by
  `examples/playable/parts_viewer.rs`. Regenerate with
  `scripts/cut-obj-into-parts.py` (recipes in `scripts/part-recipes/`,
  blocks/craft conversions recorded in the task SPIKE). `shells/` holds the
  recipe-generated thruster shell candidates - original work, regenerated with
  `scripts/gen-thruster-shells.py` (recipes in
  `scripts/thruster-shell-recipes/`), judged in
  `examples/screenshots/thruster_gallery.rs` (task 20260817-013639). Not
  shipped; anything promoted into the game moves to `assets/` through the
  content builders.
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
