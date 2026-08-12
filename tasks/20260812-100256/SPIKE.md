# Art research spike (2026-08-12)

Goal: find free-license art that fits Nova Protocol, or identify
generate-it-ourselves routes. Exploratory; escalation into tasks comes later.
All licenses verified on the source pages 2026-08-12 (not aggregator claims).

## Current art inventory

- Ships: Kenney Space Kit (CC0), 3 of the kit's 153 craft imported
  (`art/kenney-space-kit/`), cut into cube-section libraries
  (`assets/base/gltf/cargob/`, `racer/`) by `scripts/cut-obj-into-hulls.py`.
- Own Blender models: `hull-01`, `torpedo-bay-01`, `turret-{yaw,pitch,barrel}-01`
  (`art/blender/` -> `assets/base/gltf/`).
- Skyboxes: 2 cubemaps from space-3d (Unlicense).
- Textures: 1 asteroid texture; icons and banner authored in-project.
- Sounds: generated placeholders (out of scope here; noted for completeness).
- Licensing posture: MIT game; `credits/CREDITS.md` handles attribution.
  CC0 preferred, CC-BY 4.0 workable, NC/ND/SA and store licenses out.

## Gap map (what has no real art today)

- Controller section: no model; renders the default cuboid.
- Thruster section: no model; renders the default cuboid.
- Beacon scenario object: plain `Sphere`
  (`crates/nova_scenario/src/objects/beacon.rs:188`).
- Salvage scenario object: plain `Cuboid`
  (`crates/nova_scenario/src/objects/salvage.rs:259`).
- Ship variety: 2 cube-section libraries (cargob, racer) + 1 hull mesh.
- Environment: no stations, planets, or debris props; 1 asteroid texture.
- Stations/planets have no scenario-object type - new props need code too,
  so they are escalation candidates, not drop-ins.

## Pipeline constraint check (measured, not assumed)

`scripts/cut-obj-into-hulls.py` needs LESS than we thought:

- Verified against the known-good Kenney craft: their vertices are NOT
  grid-quantised (best strict alignment: 8% at q=0.5). The cutter clips
  triangles at cube planes, so any geometry partitions cleanly.
- Real requirements: single-mesh OBJ, flat `.mtl` `Kd` colours, no texture
  maps. `--scale` is an aesthetic choice (how many cube sections the ship
  spans), not a property the model must have.
- Trap found while validating: palette-atlas packs (Quaternius, KayKit, newer
  Kenney kits) export ONE grey `Kd` and keep colours in a UV texture - the
  geometry cuts fine but the output is colourless. Needs a bake-atlas-to-Kd
  pass before such packs are usable.

New helper: `scripts/inspect-obj-pack.py` - point it at an .obj, a directory,
or a downloaded .zip; reports mesh/material inventory, bbox, cell counts at
candidate `--scale` values, and a GOOD/PARTIAL/POOR cutter-fit verdict
(detects textures, missing mtl, multi-object files, palette atlases).
Validated against `art/kenney-space-kit/` (GOOD) and the packs below.

## Findings: ship/section 3D kits (ranked)

1. **Kenney Space Kit - the kit we already ship has ~150 unused models.** CC0.
   Direct zip on kenney.nl. Measured with the inspector: 136/153 GOOD, rest
   PARTIAL (multi-object), 0 POOR. Unused and on-format: 6 more craft
   (`craft_miner`, `craft_speederA-D` all GOOD at --scale 1-2), 2 turrets
   (`turret_single` GOOD, `turret_double` 2-object), 10 modular rocket parts,
   7 hangars, 14 corridors, 17 pipes, 13 platforms, 12 monorail pieces,
   terrain tiles, rocks/meteors/crystals, satellite dishes, machines, barrels,
   astronauts. Zero new licensing work - same license file already in
   `credits/`.
2. **Fertile Soil Productions "Spaceship Blocks Collection"** (itch.io, free).
   CC0, creator confirms derivatives OK. Modular ship BLOCKS (hulls, wings,
   cockpits, engines) - conceptually the best match for the section system:
   it is already block-based, no cutter needed. Built for AssetForge, so
   OBJ expected. itch.io blocks scripted download; needs ONE manual download
   to confirm format/grid.
3. **Kenney Space Station Kit** (97 models) + **Modular Space Kit** (40,
   2026-02). CC0, direct zips. Station interiors/shells, corridors, rooms.
   Measured: all POOR as-is - newer Kenney kits use a shared 512x512
   `colormap.png` palette texture instead of per-material Kd. The texture is
   flat swatches, so a small bake-UV-to-Kd script makes them cuttable.
4. **Quaternius "Ultimate Spaceships"** (11 ships x 5 colour variants). CC0
   (quaternius.com + OGA mirror). Measured: all PARTIAL - palette-atlas
   (one grey Kd, colours in 2048px UV atlas), ~1.8k verts/ship. Visually the
   closest non-Kenney ships. Usable as monolithic ships now, or as sections
   after the same bake pass as (3).
5. **anaxarch "3D Space Ship Pack"** (OGA, 9 ships, 3 LODs each). CC0. TRUE
   flat Kd colours - but measured PARTIAL: 3 LOD objects per file (needs LOD
   extraction) and large bboxes (needs per-ship --scale). More "realistic
   placeholder" silhouette; style fit partial.
6. **Quaternius "Ultimate Space Kit"** (92 models). CC0. Only 3 ships; value
   is 9 planets, base structures, rovers, props. Palette-atlas like (4).
7. **KayKit "Space Base Bits"** (48+ models). CC0. Base buildings/vehicles,
   no ships; gradient atlas texture. Redundant with Kenney station kits.

## Findings: environment / 2D (ranked per category)

Skyboxes:
- Screaming Brain Studios "Seamless Space Backgrounds" - CC0, 64 seamless
  nebula/starfield PNGs (512/1024). Same-creator CC0 "Cubemap Splitter" tool;
  also Planet Surface Skyboxes if ever needed. Style fit good.
- StumpyStrust "Space Skyboxes" (OGA) - CC0, Spacescape-made cubemap sets,
  122 MB zip. Style fit good.
- NASA SVS "Deep Star Maps 2020" - US public domain, equirect up to 64K EXR.
  Photoreal; subtle far-background only. Ship the full credit line
  ("NASA/GSFC SVS. Gaia DR2: ESA/Gaia/DPAC") in credits.
- Ulukai skyboxes (OGA) - CC-BY 3.0 (not 4.0), 2 painterly cubemaps, 1024px.

Planets/asteroids:
- Screaming Brain Studios "2D Planet Pack 1+2" - CC0, 303 planet sprites +
  76 sphere-wrap textures. Billboards + real 3D planet spheres.
- Quaternius Ultimate Space Kit planets (9, CC0) - low-poly flat style match.
- Solar System Scope textures - CC-BY 4.0, photoreal 8K equirect solar-system
  set, if recognisable planets are wanted.
- USGS astrogeology maps / NASA Blue Marble - US public domain, photoreal
  source material; needs restyling. USGS site 403s scripted fetch.

Effects/particles:
- Kenney Particle Pack - CC0, 80 tintable 512px textures (smoke, flares,
  sparks, trails, scorch). The obvious engine-trail/explosion source; pairs
  with bevy_hanabi.
- Soluna "Explosion effects and more" (OGA) - dual CC0/CC-BY, 5 animated
  explosion spritesheets for flipbook quads.
- Kenney "Space Shooter Redux" - CC0, 295 sprites: laser bolts, shields,
  damage overlays; billboard projectiles/decals.

UI/icons:
- Kenney "UI Pack Sci-Fi" (130 elements) + "Crosshair Pack" (200, vector) +
  "Game Icons" (105) - all CC0. Crosshairs are a direct fit for target locks.
- game-icons.net - thousands of flat SVG glyphs, CC-BY 3.0, PER-AUTHOR
  attribution required in credits (e.g. "Icons by Lorc, game-icons.net").

## Findings: generate-it-ourselves routes (ranked)

1. Kitbash-by-script over CC0 cube libraries: recombine cut cube sections
   (and kit modules) into ship/station variants on the 1-unit grid. Python
   stdlib or Blender headless; CC0 in, CC0-derived out. ~4-8 h.
2. Blender SpaceshipGenerator (github.com/a1studmuffin/SpaceshipGenerator,
   MIT; maintained MIT fork ldo/blender_spaceship_generator for Blender 3+).
   Blender GPL does not reach exported artwork (blender.org FAQ). Greeble look
   is normal-map fakery on an angular low-poly base mesh - strip materials,
   assign palette colours, export .glb, feed the cutter. ~5-10 h incl. tuning.
   Silhouettes read "gritty freighter", not Kenney-clean.
3. Self-authored Blender headless planet bake: seeded bpy script, low-subdiv
   icosphere, 2-4 colour bands; 100% own output. No published CC0/MIT
   geometry-node planet setup exists (searched). ~4-8 h.
4. Spacescape (MIT, GUI-only, no CLI) - denser skyboxes than space-3d, but
   forever manual; only if space-3d quality becomes the bottleneck.
5. AI text-to-3D: not recommended. Hosted tools license outputs via paid
   plans; local MIT models (TripoSR, TRELLIS) emit dense triangle soup with
   baked textures - the opposite of flat-shaded quads; Hunyuan3D license has
   regional exclusions + attribution. Cleanup costs more than kitbashing.

## Rejected / license traps

- Maxparata voxel spaceships: CC-BY-ND - grid-cutting is a derivative, unusable
  despite perfect voxel geometry.
- Skorpio Space Ship Construction Kit (OGA): CC-BY-SA/GPL - copyleft.
- Emerald Eel spaceships (itch): custom "attribution + no resale" license,
  compliance ambiguous.
- ESA/Hubble: CC-BY 4.0 but strict verbatim-credit rider + photoreal style.
- Poly Haven: CC0 but zero space HDRIs (only ground-based night skies).
- Store licenses (CraftPix, HDRI Hub, Adobe/Freepik/Vecteezy): forbid
  redistribution.
- Sketchfab mirrors: license labels are aggregator claims; verify at source.
- NASA-hosted third-party content (often ESA) is NOT public domain; NASA use
  must not imply endorsement; insignia off-limits.

## Escalation options (if we proceed)

1. Cheapest, zero licensing: import more of the Kenney Space Kit already in
   `art/` - speeders+miner as new section libraries, `turret_single/double`
   as turret meshes, satellite dish/machines/barrels for beacon/salvage
   props. Only cutter runs + content builder wiring.
2. One manual download: Fertile Soil Spaceship Blocks - if OBJ+flat colours,
   it natively matches the section system (no cutter needed).
3. Small tool investment: bake-colormap-to-Kd script unlocks BOTH newer
   Kenney kits (stations) and all Quaternius packs (ships, planets).
4. Environment pass: SBS skyboxes/planets + Kenney Particle Pack; each is a
   drop-in CC0 download plus a credits entry.
5. Own-art pass: Blender planet bake and/or SpaceshipGenerator experiments.

## Sources

Ship kits: kenney.nl/assets/space-kit, /space-station-kit, /modular-space-kit,
/blaster-kit; fertile-soil-productions.itch.io/spaceship-blocks-collection;
quaternius.com/packs/ultimatespaceships.html, /ultimatespacekit.html;
opengameart.org/content/3d-space-ship-pack; kaylousberg.itch.io/space-base-bits;
poly.pizza (aggregator, re-verify at source).
Environment: screamingbrainstudios.itch.io (seamless-space-backgrounds,
planetpack, 2d-planet-pack-2, cubemap-splitter);
opengameart.org/content/space-skyboxes-0, /ulukais-space-skyboxes,
/explosion-effects-and-more; svs.gsfc.nasa.gov/4851;
solarsystemscope.com/textures; usgs.gov copyrights-and-credits;
kenney.nl/assets/particle-pack, /ui-pack-sci-fi, /crosshair-pack, /game-icons;
game-icons.net/about.html.
Generators: github.com/a1studmuffin/SpaceshipGenerator,
ldo/blender_spaceship_generator, petrocket/spacescape (+FrozenStormInteractive
fork), MightyBOBcnc/nixis, Deep-Fold/PixelPlanets; blender.org/support/faq
(output ownership).

## Round 2 (2026-08-12)

Acquisition + evaluation pass. Every candidate below was LOOKED AT (image
opened, model inspected), not just listed. Licenses re-verified on source
pages 2026-08-12. Scratchpad root:
`/tmp/claude-1000/-home-alex-personal-nova-protocol/e40c1ba5-3c25-401b-a96a-efdb7955e78f/scratchpad/`
(below: `scratchpad/`). Nothing binary enters the repo in this round;
imports happen on escalation with credits entries.

### Pipeline constraints (measured this round)

- Asteroid UVs are per-triangle planar (`nova_gameplay mesh/builder.rs
  uvs()`: u along edge b-a, v = normal x u, origin at vertex a). Texture
  continuity across triangles is broken BY CONSTRUCTION -> macro features
  (long cracks, veins, big colour patches) show a seam at every triangle
  edge. Fine-grain, homogeneous, low-contrast seamless textures hide them.
  Equirect maps cannot work on this mesh.
- No sampler config anywhere in the repo -> Bevy default ClampToEdge.
  Noise-stretched triangles can push UVs past 1.0 (unit edge ~0.2 pre-noise,
  radial displacement up to ~5x). The swap task must set repeat sampling
  (image meta / loader settings) or verify UV extents; smear otherwise.
- Baseline `assets/base/textures/asteroid.png` (viewed): photoreal grey
  slab rock, mid contrast, brownish cast. The reference to beat.

### Asteroid textures (downloaded + judged)

ambientCG - CC0 verified at docs.ambientcg.com/license 2026-08-12; scripted
download API works (`ambientcg.com/get?file=<id>_1K-JPG.zip`). No
moon/lunar/asteroid-specific materials exist there (API searched); rock
series only. Downloaded to `scratchpad/textures/ambientcg/<id>/`:

- **Rock030** - RECOMMENDED. Mid-grey/brown, homogeneous fine grain, low
  contrast, thin white veining only. Closest to the current texture but
  cleaner and flatter -> hides triangle seams best; least fight with flat
  shading.
- **Rock035** - dark variant. Near-black blue-grey, fine grain, low
  contrast. Good authored second texture (dark rocks); check in-game
  lighting, may read as silhouette-only.
- Rock062 - rejected: rounded organic lobes + orange crack highlights;
  veins would seam per triangle, reads muddy/terrestrial.
- Rock048 - rejected: light grey with green moss flecks and beige patches;
  moss is a terrestrial giveaway.

Poly Haven - CC0 verified at polyhaven.com/license 2026-08-12; scripted
download works (api.polyhaven.com + dl.polyhaven.org). Downloaded to
`scratchpad/textures/polyhaven/`:

- dark_rock_02 - partial fit: dark brown slate, but blocky macro cracks ->
  visible triangle seams. Backup only.
- gray_rocks - rejected: gravel pile with dead leaves and green weeds.
- rock_boulder_dry - rejected: light beige travertine blotches, reads
  bathroom marble.

Screaming Brain Studios - own site blocks scripted fetch (406) and pack
pages carry no direct links; itch.io blocks scripting. No SBS tileable-rock
candidate relevant here (their strength is planets, below, via OGA mirror).

RECOMMENDATION: swap default to ambientCG **Rock030** (Color map only, 1K),
add **Rock035** as an authored dark variant. `AsteroidConfig.texture` is
already per-asteroid content -> variety is pure RON authoring, zero code.
Optional import tweak: slight desaturate/darken to sit with flat shading.

### Planets (route decision)

(a) SBS Planet Surface Textures - CC0 verified on
opengameart.org/content/planet-surface-textures 2026-08-12; scripted
download from the OGA mirror works. Downloaded 1024x512 set (75 unique
textures, 12 environments) to `scratchpad/textures/sbs-planets/`. Viewed:

- Barren_01: ochre/brown, subtle craters, painterly flat bands, polar
  caps - good.
- Gaseous_01: clean teal-green banded gas giant - best of the viewed;
  distant-sphere ready.
- Martian_01: red painterly speckle - fine.
- Methane_01: saturated yellow noise - busy, weak; skip that family.

(b) Quaternius Ultimate Space Kit planets - CC0 verified on
quaternius.com/packs/ultimatespacekit.html 2026-08-12; pack License.txt
(from the Drive folder) is CC0 1.0 full text (header mislabeled "Ultimate
Platformer Pack", Quaternius boilerplate). Scripted per-file Drive fetch
works; downloaded Planet_1/4/8.gltf + Atlas.png + Preview.jpg to
`scratchpad/quaternius-space-kit/`. Findings: 11 planets exist
(Planet_1..11); Atlas.png is 512px flat colour swatches; the .gltf files
embed geometry as base64 but have an EMPTY image URI -> need a repack
(.glb with atlas embedded) before Bevy can load them. Preview viewed:
candy-cartoon palette (purple/pink planet, ice planet, swirl trees, animal
astronauts) - cuter and more saturated than the game's look.

(c) Own Blender bake - unchanged from round 1: 4-8 h, bespoke look,
fallback.

RECOMMENDATION: route (a), SBS textures on a UV sphere. Reasons: zero art
tooling (route b needs a gltf repack AND palette fits poorly; route c costs
hours), 75 CC0 maps across 12 environments incl. gas giants (no low-poly kit
has those), painterly style sits well with flat shading, and scenario
authors pick a texture per planet the same way asteroids do today.
ESCALATION FLAG (code, separate task): planets need a new scenario object
type. It must use a spherical-UV sphere mesh (Bevy `Sphere` primitive), NOT
`TriangleMeshBuilder` - the planar per-triangle UVs cannot wrap an equirect
map. Gravity well + invulnerable plumbing to crib from `asteroid.rs`.

### Scene dressing (wanted props -> verified sources, concrete files)

Kenney Space Kit - CC0, license already in `credits/`; unzipped at
`scratchpad/space-kit/Models/OBJ format/`. Space-relevant subset only:

- Debris/field rocks: `meteor.obj`, `meteor_detailed.obj`,
  `meteor_half.obj`; crystal flavour: `rock_largeA/B.obj`,
  `rock_crystals.obj`, `rock_crystalsLargeA/B.obj`.
- Salvage-site dressing: `machine_generator.obj`,
  `machine_generatorLarge.obj`, `machine_barrel.obj`,
  `machine_barrelLarge.obj`, `barrel.obj`, `barrels.obj`,
  `barrels_rail.obj`, `machine_wireless.obj`, `machine_wirelessCable.obj`.
- Beacon/nav dressing: `satelliteDish.obj`, `satelliteDish_detailed.obj`,
  `satelliteDish_large.obj`.
- Derelict/traffic craft: `craft_miner.obj`, `craft_speederA-D.obj` (all
  GOOD cutter fit, round 1), `turret_single.obj` (GOOD; `turret_double` is
  2-object).
- Not space-usable (owner filter): terrain_*, corridor_*, platform_*,
  monorail_*, hangar_*, stairs_*, desk_*, chimney*, gate_* (surface/
  interior).

Fertile Soil Spaceship Blocks - CC0 (round 1 source-page verification
2026-08-12); the ONE manual download happened -> 95 OBJ+MTL at
`scratchpad/spaceship-blocks/`. Measured with `scripts/inspect-obj-pack.py`:
all 95 PARTIAL purely for multi-object files - irrelevant when used WHOLE
as props (no cutting); flat Kd colours (3-4 per model), zero textures,
sane bboxes (0.2-3 u). It is a station kit in practice (all files are
`Spacestation_*`). Concrete kitbash set for one derelict station:
`Spacestation_Structure_Habitat`, `..._Fuselage_Straight_*` /
`..._Fuselage_Curved_Outer_Large_*` / `..._Fuselage_Angled_*` families,
`..._Ribbed_Round(_Band)`, `..._Runway_End/Mid/Fuselage`,
`Spacestation_Miscellaneous_Dish(_Rotating)`, `..._Bay_Door`,
`Spacestation_Propulsion_Thruster_Triple_Large`, and
`Spacestation_Weapon_Modular_Gun_Base/Mount/Barrel_*` for defense guns.

Quaternius Ultimate Spaceships - CC0 (round 1, quaternius.com); at
`scratchpad/quaternius-ships/Ultimate Spaceships - May 2021/` (11 ships,
Blend+FBX+OBJ + 5 colour textures each). Viewed `Executioner_Blue.png`
(2048px): painted panel-line hull with decals and grunge - NOT flat
swatches. Round-1 "bake-atlas-to-Kd" would destroy the paint; the right
use is WHOLE textured ships (OBJ -> .glb with texture) as distant traffic
or derelict hulls. Grittier than Kenney; distance hides the mismatch.

Quaternius Ultimate Space Kit props (rovers, domes, solar panels): planet-
surface content, skipped for the in-space game; only the planets matter.

### Downloads vs manual

Downloaded this round (scratchpad paths above): 4 ambientCG rock materials
(1K), 3 Poly Haven diffuse maps (1k), SBS planet pack 1024x512 (75
textures), Quaternius USK Planet_1/4/8.gltf + Atlas.png + License.txt +
Preview.jpg. Already present from round 1: Kenney space/station/modular
kits, Quaternius Ultimate Spaceships, anaxarch ships, Fertile Soil blocks.

Manual downloads for the owner: NONE required for the recommended routes.
Optional, only if route (b) is ever revisited: full Ultimate Space Kit from
https://drive.google.com/drive/folders/17F8HlI2zPTlo32aieW5YPPwOk78xo-2m
(per-file scripted fetch works; the one-zip folder download needs a
browser).

### In-engine comparison (after round 2)

Runnable lineups: `cargo run --example compare_asteroids --features debug`
and `compare_planets` (keys 1-N swap the focus subject). Per-asset dossier
incl. import status and keybinds: `ASSETS.md` in this folder. Candidates live
in `art/texture-candidates/` (NOT `assets/` - ships nothing).

### Escalation plan (ordered) - SUPERSEDED by round 3 (see below)

1. Asteroid texture swap (content only): import Rock030 as the new default
   + Rock035 dark variant into `assets/base/textures/`, credits entries
   (ambientCG, CC0, obtained 2026-08-12), scenario RON updates via the
   content builders, repeat-sampler check (ClampToEdge trap above).
2. Planet scenario object (code + content): new `planet` object type on a
   spherical-UV Bevy `Sphere`; import 4-6 SBS maps (Gaseous x2, Barren,
   Martian, Snowy) + credits (SBS, CC0, obtained 2026-08-12); author one
   planet into a scenario for the look check.
3. Debris/salvage/beacon props (code + content): generic gltf-prop
   scenario object (or render slots on beacon/salvage), then import Kenney
   meteors + satellite dishes + machines/barrels as .glb; zero new
   licensing.
4. Station kitbash (art): Fertile Soil blocks -> one derelict station .glb
   in Blender (concrete piece list above); spawns via the prop object
   from (3); credits entry for Fertile Soil.
5. Distant traffic (code + art): Quaternius ships as textured .glb movers;
   lowest priority, biggest style risk.

## Round 3 (2026-08-12): planets in-game + destructible asteroids

Design spike after the owner reviewed the compare examples. Full analysis
with numbers: `DESIGN-round3.md` (this folder). Summary:

- Planets: HYBRID route - one `planet` scenario object with a `backdrop`
  flag. Body mode: stylized miniature (radius 200-500 u, UV sphere +
  sphere collider, SBS equirect map, gravity well + ORBIT beat; 300 u at
  the mu cap gives a 1.9 km SOI). Backdrop mode: sky-anchored impostor
  (re-anchored to `camera_pos + offset` each frame, no physics/lock/radar;
  f32 + reversed infinite-Z make a second camera unnecessary). Skybox
  baking parked. Lighting agrees by authoring convention: aim the key
  light along the skybox sun.
- Asteroid UVs: TRIPLANAR projection in a `MaterialExtension` (local-space,
  no UVs consumed; repeat sampler required). Rejected: spherical UVs
  (poles/seams, dies on carve remeshes), baked unwrap (runtime-generated
  mesh). 0.5-1 day; also the prerequisite for carving (remeshed geometry
  has no UVs either).
- Destructible asteroids: per-rock f32 SDF grid in local unit space,
  seeded ANALYTICALLY from `PlanetHeight` (d = |p| - (1 + height));
  32^3-48^3 (131-442 KB, lazy-allocated on first hit). Remesh with naive
  SURFACE NETS + flat normals (low-res grid keeps the faceted look;
  ~2-5k tris vs 512 today). Spherical SDF carve per damage hit, coalesced
  per tick; trimesh collider rebuild (~1-5 ms, same call as spawn; VHACD
  rejected). Severed voxel islands (flood fill, <1 ms) spawn drifting
  chunks (convex hull, inherit v + omega x r; parts-object alignment) and
  shards/salvage. Async remesh native; 32^3 cap on wasm (single-threaded,
  WebGPU). Health stays the kill gate; slice.rs STAYS as the death path
  (fragments the carved mesh), for ships/props, and as the cheap fallback.
- Escalation plan v3 (in DESIGN-round3.md) supersedes the round-2 list:
  texture swap + triplanar first, then planet body object, backdrop mode,
  props, station, then the carve chain (prototype -> severing ->
  integration), traffic last.
