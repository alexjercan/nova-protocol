# Move the planetoids onto the planet surface

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.13.0,art,content,scenario

## Goal

The prototype proved a planetoid can read as a planet. This task ships it:
register the plugin, decide what an authored planet IS, move the real bodies
onto it, and check it somewhere other than an example.

Owner framing (2026-09-04): "the BIG planetoids look like a gray repeated ugly
texture. Make them read as planets - mountains, valleys, different biomes,
planet TYPES so not everything is Earth-like." And, on scope: "Start SIMPLE.
Depth comes later."

## The problem, measured

Two numbers from the prototype round. Both are still true in the shipping
game, because nothing in it changed.

- `ROCK_TEXTURE_TILING` is 0.35 repeats per world unit
  (`asteroid_surface.rs:59`), tuned by its own doc comment for "a field rock a
  few units across". One repeat is 28.6 m, and the menu planetoid's surface
  stands 740 m to 1130 m off its centre - so that tile lies about **100 times
  across the visible disc**, at roughly 20 pixels each.
- The rock material is `StandardMaterial::default()` times one greyscale image
  (`asteroid.rs:432-435`). Nothing on any body varies with elevation, latitude
  or seed. Every planetoid is the same grey.

## The prototype is done - read it, do not redo it

The research, the design and the working surface all live with
`20260815-231945`. Nothing there needs repeating here.

- `tasks/20260815-231945/PLANETOID-LOOK.md` - the round record: why it reads
  as a repeat, the technique survey, the type and biome design, the limits.
- `tasks/20260815-231945/planetoid-look.html` - the same round with the
  captures side by side. Read this one first.
- `tasks/20260815-231945/planetoid-look/` - seven captures, including today's
  planetoid and a generated planet at the identical menu pose.
- Branch `planetoid-look`, commit `0656ad50`.

The code, all of it new and none of it wired in:

- `crates/nova_scenario/src/objects/planet_type.rs` - the CONTENT: six planet
  types, the biome palettes, `PlanetConfig`, the seeded draw.
- `crates/nova_scenario/src/objects/planet_surface.rs` - the SHAPE: height
  field, `planet_mesh`, the uniform, `PlanetSurfaceMaterial`.
- `assets/shaders/planet_surface.wgsl` - the fragment. Reads no textures.
- `examples/playable/planet_types.rs` - the lineup, a focus pass and a close
  pass, plus today's planetoid at the same framing for contrast.

## What is left

Nothing. The four items below are done; they are kept as the record of what
shipped and of the decisions made along the way.

1. **`PlanetSurfacePlugin` is registered**, through a new `PlanetPlugin`
   (`objects/planet.rs`) added by `ScenarioObjectsPlugin`. The material
   pipeline stays separately addable so an example can draw a surface without
   spawning scenario objects.

2. **An authored planet is its own object**: `ScenarioObjectKind::Planet`,
   appended at the end of the enum. Decided against extending the asteroid
   because the two kinds derive `BodyRadius` completely differently (below),
   and one config that meant two different things by `radius` would be a trap
   rather than a saving.

   The type is a CLOSED enum, not the open string id the plan floated. The
   palettes, relief, sea level and detail are all exhaustive `const fn`
   matches on it; opening the id means moving that into a last-wins content
   table, which is a content-pipeline change, not a look change. The closed
   enum also buys the fail-loud rule for free - an unknown `planet_type` fails
   RON parsing, at the only point that can still name the file - and it edits
   in the inspector as a chip row. Opening it later is mechanical behind the
   same field name.

   No silent defaults: `planet_type`, `radius`, `seed` and `invulnerable` are
   all required. `mass`, `relief`, `sea_level` and `lock_signature` stay
   `Option` because each is a genuine override with documented derived
   behavior behind it, the way `AsteroidConfig::mass` is.

3. **The menu and both mainline bodies are planets.** Types and seeds:
   `menu_planetoid` Temperate seed 7, `inspection_planetoid` DustWorld seed 7,
   `concealment_planetoid` BarrenRock seed 3.

4. **Checked in the real scenes**, not the example stage - the shipped menu
   and the First Shift orbit chapter, before and after, in
   `tasks/20260815-231945/planetoid-look/`.

### The geometry contract, and what it actually said

The plan said to keep radius, mass and `invulnerable` at their current values
so the well, the SOI and the ORBIT ring would be unchanged. That premise was
wrong, and finding out is the substance of this task.

`BodyRadius` is DERIVED: `radius * unit_extent`. A rock's noise mesh puts
`unit_extent` in `[3.5, 6.0]`; a planet's is `1 + relief`, about 1.05. Keeping
the radius LITERAL would have shrunk every body about fivefold and collapsed
the SOI and the orbit ring with it.

Measured through the shipped `asteroid_scenario_object` before the swap, then
re-authored to reproduce each figure:

| body | old body radius | new mean radius | new body radius | drift |
|---|---|---|---|---|
| `menu_planetoid` | 940.6 m | 900 m | 945.0 m | +0.5% |
| `inspection_planetoid` | 1 000.1 m | 950 m | 997.5 m | -0.3% |
| `concealment_planetoid` | 2 377.0 m | 2 250 m | 2 373.8 m | -0.1% |

Pinned by `the_belt_planets_keep_the_body_radius_their_rocks_published` and
`the_menu_world_keeps_the_body_radius_its_rock_published`.

### The rest of the contract

- Collider is `Collider::sphere(1 + relief)` on the unit-scaled child, not a
  hull off the mesh. A planet is a sphere to within a few percent, and a
  primitive needs no vertex data and no hull build. The asteroid's own note
  records why this matters: 21.9 ms a step as trimeshes against 0.10 ms as
  hulls.
- `BodyRadius`, `LockSignature`, `SurfaceMaterial`, `InsetZoomable` and the
  HUD contact all publish as before. `PLANET_TYPE_NAME` had to be added to
  `nova_events` and to both tactical-map contact filters, or a planet would
  have been invisible on the map.
- The scenario lint gained `check_planet`: a non-positive radius, a relief at
  or past the radius, a sea level outside 0-1, and a non-positive mass or lock
  signature are all errors.

## Non-goals

Named so they are not re-litigated. Each is a later round, not an omission.

- Biome blending. Band edges are hard on purpose - the owner asked for no
  blending in the first pass.
- Humidity and temperature fields. Only elevation and latitude select a biome.
- Craters. A barren rock world wants them and has none.
- Clouds and atmosphere. The greenhouse type fakes weather with a soft banded
  palette; there is no atmosphere shell.
- Mesh level of detail. One subdivision per body, fixed at build time. The
  procedural grain fades with distance; the geometry does not.

## Verification debt

Still open. Each was carried in from the prototype round and is not paid.

- No WASM or WebGL2 build. The uniform is laid out entirely in `vec4`s
  specifically so its 16-byte alignment holds there with no padding fields,
  but that is reasoning, not a build.
- No frame-cost measurement of any kind. Another lane shared the GPU
  throughout both rounds, so any number taken would have been measuring that
  lane too. The editor now meshes a planet on every edit of a body field, at
  `PLANET_EDITOR_SUBDIVISIONS`, and that cost is unmeasured.
- No clippy run, and no workspace test run.
- Greenhouse and volcanic are still judged at lineup and focus range in the
  example only; neither is used by any shipped scenario.
- `web/` was not built: its node modules are not installed in the worktree.
  The changes there are Markdown only.

Paid this round: barren rock and the dust world now have real-scene captures
at gameplay framing, and the menu world has one at the shipped menu pose.

## Watch for

- **A planet has no destruction sound and cannot be carved.** Both shipped
  bodies are `invulnerable`, so nothing exercises it, but a destructible
  planet would break silently and leave no debris. Settle it before one is
  authored.
- **Mesh build cost is on the main thread.** A 4096-direction range sweep runs
  before a single vertex is placed, then the field is evaluated at every
  vertex plus two more per vertex for the finite-difference normals. It is a
  build-time cost, not a frame cost, but it is not small.
- **Six bands is the uniform's hard ceiling** and the temperate type already
  uses all six. A seventh needs the uniform widened.
- `asteroid.rs:443-751` holds an older `PlanetHeight` noise graph that nothing
  renders through. Decide whether it goes when the real surface lands.

## Done when

All four are met; what remains is under Verification debt.

- [x] `menu_planetoid` reads as a planet in the running menu, not in an
  example - `planetoid-look/scene-menu-after.png`.
- [x] A scenario author picks a planet type and a seed in RON, and the same
  values reproduce the same planet across loads. Both are required fields, and
  the editor places and re-types one from the palette
  (`planetoid-look/editor-planet-*.png`).
- [x] The wells and SOI the campaign depends on are unchanged, and checked -
  the body-radius table above, pinned by two tests.
- [x] The look is confirmed in a real scenario at gameplay framing -
  `planetoid-look/scene-orbit-after.png` and `scene-concealment-after.png`.
