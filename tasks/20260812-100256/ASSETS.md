# Asset dossier: texture candidates (art round 2)

Per-asset record for the comparison examples. Every entry below was viewed
(image opened) on 2026-08-12; licenses verified on the source pages the same
day (details in SPIKE.md round 2 - not re-verified here).

Import status legend:

- `art/` - in-repo at `art/texture-candidates/<source>/`; loaded by the
  compare examples straight off disk, ships nothing.
- `scratchpad` - downloaded and judged, not imported
  (`scratchpad/textures/...` in the session scratchpad; re-download is
  scripted, nothing manual).

## Viewing them

```bash
cargo run --example compare_asteroids --features debug
cargo run --example compare_planets --features debug
# keys 1-N dress the big center subject; arrows cycle; WASD+mouse free-fly
```

Both examples stage the game's real renderer: scenario camera, skybox, the
standard three-point light rig, `StandardMaterial` with only
`base_color_texture` set (the asteroid material shape). Candidates load with
a REPEAT sampler; the shipped default is ClampToEdge, which smears stretched
per-triangle UVs - the baseline keeps it so the trap stays visible.

## Asteroid candidates (compare_asteroids)

All on the same fixed-seed rock from the game's real mesh pipeline
(`TriangleMeshBuilder` + `PlanetHeight`, planar per-triangle UVs).

### baseline asteroid.png - key 1

- Creator: project-authored (`credits/CREDITS.md`)
- File: `assets/base/textures/asteroid.png`, PNG
- Style: photoreal grey slab rock, mid contrast, brownish cast
- Import: shipped (the texture to beat); clamp sampler on purpose
- Verdict: keeper only until a candidate wins; shows the clamp smear

### Rock030 (ambientCG) - key 2 - RECOMMENDED default

- Creator: ambientCG (Lennart Demes)
- Page: https://ambientcg.com/view?id=Rock030
- Download: https://ambientcg.com/get?file=Rock030_1K-JPG.zip (scripted)
- License: CC0 1.0, verified at https://docs.ambientcg.com/license/ 2026-08-12
- Format: 1024x1024 JPG color map -> PNG in repo (shipped bevy has no JPEG
  decoder); zip also carries AO/displacement/normal/roughness (unused)
- Style: mid grey-brown, homogeneous fine grain, low contrast, thin white
  veining only - hides the per-triangle seams best, sits with flat shading
- Import: `art/texture-candidates/ambientcg/Rock030.png`
- In frame: `compare-asteroids-grid.png`, `compare-asteroids-rock030.png`

### Rock035 (ambientCG) - key 3 - RECOMMENDED dark variant

- Same creator/license/format story as Rock030
- Page: https://ambientcg.com/view?id=Rock035
- Download: https://ambientcg.com/get?file=Rock035_1K-JPG.zip
- Style: near-black blue-grey, fine grain, low contrast; reads as a coal
  rock under the rig, silhouette risk in dim scenes - check in a scenario
- Import: `art/texture-candidates/ambientcg/Rock035.png`
- In frame: `compare-asteroids-grid.png`, `compare-asteroids-rock035.png`

### dark_rock_02 (Poly Haven) - key 4 - backup

- Creator: Rob Tuytel (Poly Haven)
- Page: https://polyhaven.com/a/dark_rock_02
- Download: dl.polyhaven.org via https://api.polyhaven.com/files/dark_rock_02
  (scripted), `dark_rock_02_diff_1k.jpg`
- License: CC0 1.0, verified at https://polyhaven.com/license 2026-08-12
- Format: 1024x1024 JPG diffuse -> PNG in repo
- Style: dark brown slate with blocky macro cracks - the cracks seam at
  triangle edges; backup only
- Import: `art/texture-candidates/polyhaven/dark_rock_02.png`

### Rock062 (ambientCG) - key 5 - rejected

- Page: https://ambientcg.com/view?id=Rock062; same license story
- Style: rounded organic lobes, orange/copper crack highlights - veins seam
  per triangle, reads muddy and terrestrial
- Import: `art/texture-candidates/ambientcg/Rock062.png` (kept in the lineup
  so the rejection is visible in-engine)

### Rock048 (ambientCG) - key 6 - rejected

- Page: https://ambientcg.com/view?id=Rock048; same license story
- Style: light grey with green moss flecks and beige patches - moss is a
  terrestrial giveaway, also the brightest rock in the row
- Import: `art/texture-candidates/ambientcg/Rock048.png` (same reason)

### gray_rocks, rock_boulder_dry (Poly Haven) - not in the lineup

- Pages: https://polyhaven.com/a/gray_rocks, /a/rock_boulder_dry; CC0 1.0
  (same verification)
- Style: gravel pile with dead leaves / beige travertine ("bathroom marble") -
  rejected on sight in round 2, not worth a row slot
- Import: scratchpad only

## Planet candidates (compare_planets)

Screaming Brain Studios "Planet Surface Textures": 75 equirect maps across 12
environment families, CC0 1.0 verified on
https://opengameart.org/content/planet-surface-textures 2026-08-12; scripted
download of the OGA attachment `sbs_planets_1024x512.zip`. All maps 1024x512
RGBA PNG, painterly flat style that sits well with the game's flat shading.
Wrapped on bevy UV spheres (`Sphere.mesh().uv()`, poles rotated onto Y - the
builder puts them on Z). The asteroid mesh CANNOT wrap these (planar
per-triangle UVs); the planned planet scenario object needs a UV-sphere mesh.

Six imported to `art/texture-candidates/sbs-planets/` (names de-suffixed,
content byte-identical):

- **Barren_01 - key 1**: ochre-brown, subtle craters, polar caps; the
  "planetoid" workhorse. RECOMMENDED.
- **Gaseous_01 - key 2**: teal-green banded gas giant, clean horizontal
  bands; the best distant-sphere read of the set. RECOMMENDED.
- **Gaseous_08 - key 3**: blue-green swirled bands with storm eddies;
  livelier second gas giant. RECOMMENDED.
- **Martian_01 - key 4**: red painterly speckle with white caps; fine.
  RECOMMENDED.
- **Snowy_01 - key 5**: pale ice blue with cloud mottling; low contrast up
  close, good far.
- **Tundra_01 - key 6**: green-blue continents with white caps; the
  "habitable" look, most saturated of the six - check against the game
  palette before shipping.

Family notes from round 2: Methane family rejected (saturated yellow noise,
busy); remaining 69 maps stay in the zip, import is copy-plus-credits when a
scenario wants one.

## Shortlisted but not in an example

### Quaternius Ultimate Space Kit planets (route b, not taken)

- Creator: Quaternius, https://quaternius.com/packs/ultimatespacekit.html
- License: CC0 1.0, verified on the pack page + pack License.txt 2026-08-12
- 11 low-poly planets (Planet_1..11), 512px flat-swatch atlas; the .gltf
  files embed geometry but carry an EMPTY image URI - unusable in bevy
  without a repack to .glb with the atlas embedded
- Style (Preview.jpg): candy-cartoon palette, cuter and more saturated than
  the game
- Import: scratchpad only (`scratchpad/quaternius-space-kit/`); revisit only
  if the SBS route fails
