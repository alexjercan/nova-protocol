# Planetoid look: planet types, biomes and a seeded surface

Round record for the PLANETOID lane. The ask, in the owner's words: the big
planetoids read as a grey repeated texture. Make them read as planets, with
mountains, valleys, biomes and planet TYPES, so not everything is Earth-like.
Drive it from a config that reproduces a type, a seed and a few settings. Start
simple. No biome blending yet. Depth comes later.

The prototype is new code beside the existing planetoid. Nothing that ships
today changed. `menu_planetoid` still spawns exactly what it spawned before.

`planetoid-look.html` in this folder is the same round with the captures shown
side by side. Read that one first if you want to see the answer before the
reasoning.

## 1. Why today's planetoid reads as a grey repeat

Three facts, and the third is the one that does the damage.

**The body is enormous.** `backdrop_planetoid` authors `radius: Meters(200.0)`
(`crates/nova_authoring/src/base_content/scenarios/main_menu/shared.rs:24`),
and the asteroid generator then displaces its unit mesh well past that. The
seed sweep pins the geometric factor at `[3.70, 5.64]` over 256 seeds
(`crates/nova_scenario/src/objects/asteroid.rs:369-381`), so the real surface
stands 740 m to 1130 m off the centre. From the menu's reference pose of
`(0, 570, 1920)` m (documented at `.../main_menu/shared.rs:34-40`) the body
overfills the frame vertically. Everything wrong with the surface is shown at
full size.

**The texture is tuned for a rock, not a world.** `ROCK_TEXTURE_TILING = 0.35`
repeats per world unit (`crates/nova_scenario/src/objects/asteroid_surface.rs:59`)
and its own doc comment says what it was tuned against: "a field rock a few
units across" (`asteroid_surface.rs:56-58`). That is the correct number for the
thing it was tuned for. One repeat is `1 / 0.35 = 2.86` world units, which is
28.6 m of surface.

Put the two together. Take the mid-range body radius, about 90 world units. The
arc across the face you can see is `pi * r`, about 283 world units, so roughly
**one hundred repeats lie across the visible disc**, and at the menu's distance
one repeat subtends about **20 pixels** of a 1080-line frame. A twenty-pixel
tile, laid down a hundred times, on a body that overfills the screen. That is
the grid you can see in `planetoid-look/planet-types-today-planetoid.png`.

**There is no colour anywhere.** The rock material is
`StandardMaterial::default()` with the triplanar extension over it
(`crates/nova_scenario/src/objects/asteroid.rs:432-435`). Default is white. The
only colour on the whole body is the one greyscale rock image, multiplied by
one flat tint. Nothing varies with elevation, latitude, or seed. Every
planetoid in the game is the same grey, and the only thing that separates two
of them is the silhouette.

So the complaint is precise and it decomposes cleanly: **a tile that repeats**
plus **a palette of exactly one colour**. Both have to go.

An aside worth recording: `asteroid.rs:443-751` already holds a `PlanetHeight`
noise graph with a `CURRENT_SEED` and a comment inviting you to change it. It
builds a `NoiseFn<f64, 3>`, one unit test exercises it, and nothing renders
through it. That is a previous attempt at this problem, left in the tree. This
round does not extend it: it is CPU-side elevation only and carries no notion
of a planet TYPE, which is the half the owner actually asked for.

## 2. Technique survey

### The one decision that matters: no texture at all

A tile repeats. Any tile repeats. Every anti-tiling technique in the literature
attacks the SYMPTOM: stochastic and hex-grid tiling break the lattice by
blending randomly-offset copies, histogram-preserving synthesis fixes the
blend's contrast loss, and detail maps hide the low frequencies under a second
scale. All of them cost extra texture samples per pixel, and none of them give
you a coastline, a snow line, or a second planet that is a different colour.

The alternative is to sample a function of the surface direction instead of an
image. A function has no lattice, so it has nothing to repeat at any scale, and
it is the same function on a 200 m body and a 2000 m one. That is what this
prototype does: **the planet material reads no textures at all.**

### Signed height field, recovered from position

The shape is a signed height field over the unit sphere, evaluated on the CPU
and baked into a displaced icosphere. The fragment then needs to know the
elevation again, to choose a band. Two ways to give it that: add a vertex
attribute, or recover it.

Recovering is simpler and it is what the prototype does. The uniform carries
`radius_min` and `radius_span`; the fragment takes `length(local_position)`,
subtracts, divides, clamps. No extra attribute, no mesh format change, and it
works on any mesh with the same convention. The cost is that elevation is
recovered from a LINEARLY interpolated position, so its contours across one
triangle are straight lines and a band edge follows the mesh facets. That is a
real artefact and section 4 says how it is dealt with.

### Icosphere, not a UV sphere

Nothing here reads UVs, so a UV sphere buys nothing and costs the pole pinch,
where the quads collapse and displacement tears. An icosphere has near-uniform
facets everywhere. Bevy gives one directly: `Sphere::new(1.0).mesh().ico(n)`.

Watch the parameter. `n` is points added per edge, not recursion depth, so the
vertex count is `10 * (n + 1)^2 + 2` and the hard ceiling is 65,535 vertices at
`n = 79`. A first pass here read `n` as a depth and got a 492-vertex ball; the
test `subdivisions_clamp_instead_of_failing` now pins the exact count at the
ceiling so that misreading cannot come back.

### Latitude plus noise for biomes, colour ramps per type

Full biome assignment in the reference implementations uses several
independent fields. Gaia Sky generates elevation, humidity and temperature as
three separate noise fields, packs them into a biome texture and resolves the
final colour through a 3D lookup table. That is the shape of "depth comes
later" and it is explicitly out of scope for this round.

Simple version, and the one built: **elevation and latitude, nothing else.** A
band claims a height floor, and optionally a latitude floor as well. The
fragment walks the bands and the last match wins. Two comparisons per band, no
lookup table, no third field. It gives you continents by height and polar caps
by latitude, which is most of what makes a body read as a world.

### Making a threshold not look like a threshold

Thresholding raw elevation draws contour rings, and thresholding raw latitude
draws a ruled line across the body. Three cheap corrections, all reusing fields
already sampled:

- **A coarse warp added to the elevation before the threshold.** This is what
  turns a contour ring into a coastline. It costs one add, not a second
  palette lookup.
- **The same warp added to the latitude.** A cap without this is a straight
  line drawn on a sphere.
- **A fine grain added to the elevation, at a very small amplitude.** This is
  the facet fix from above: the grain is the only field fine enough to hide a
  triangle edge, and it was already being sampled for the colour variation.

### Roughness and normal from the same field

One noise field, read once, doing four jobs: colour variation inside a band,
the roughness swing, the band-edge break-up, and (with two more samples for the
gradient) the shading normal. Without the normal term a close pass reads as
painted plastic, because the mesh carries the mountains and nothing carries
anything smaller than a facet.

### Fading procedural detail by screen footprint

Procedural detail that is finer than a pixel cannot be resolved and can only
alias, and an aliasing NORMAL sparkles rather than just shimmering. The fix is
screen-space: `length(fwidth(direction)) * grain_frequency` is roughly how many
noise cells one pixel spans, and the grain fades out between 0.5 and 1.5 cells
per pixel. That single number is the whole level-of-detail scheme here. At
backdrop range the bands and the mesh carry the body; at close range the grain
comes back.

### What `noise` 0.9 gives, and what it does not

`noise = "0.9"` is already a dependency of `nova_scenario`
(`crates/nova_scenario/Cargo.toml:14`). It gives, directly and with no extra
code: `Fbm<Perlin>` (signed output), `RidgedMulti<Perlin>`, and the
`MultiFractal` setters for octaves, frequency, persistence and lacunarity, all
over `NoiseFn<f64, 3>`. That is enough for the whole CPU height field.

Two things it does not give.

It does not give a stable seed under composition. `Fbm` seeds its octaves as
`seed + n` in `u32`, so two planets whose seeds are three apart share octaves,
and adjacent seeds produce visibly related terrain. Seeds are therefore passed
through an FNV-1a hash before they reach a noise constructor, matching the
`asteroid_seed_from_id` precedent already in the tree.

It does not run on the GPU at all. Every per-pixel field in this prototype is
hand-written WGSL: a lattice hash, trilinear value noise over it, and a
three-octave fbm.

### Sources

- Gaia Sky, procedural planet generation (elevation/humidity/temperature into a
  biome LUT). MPL-2.0, per its Codeberg repository. Used as a design reference
  for where this goes next, not as code.
- `SebLague/Procedural-Planets`. MIT. Used as a design reference for the
  displaced-icosphere approach.
- Bevy 0.19's own `extended_material` example and `pbr_fragment` module, read
  locally, for the `ExtendedMaterial` fragment structure.
- `crates/nova_scenario/src/objects/asteroid_surface.rs`, read locally, for the
  repo's own `ExtendedMaterial` + `#[uniform(100)]` precedent. This prototype
  follows its structure closely on purpose.

Both external references are LEARN-grade: nothing was copied from either.

## 3. Planet types and biomes

### What a `PlanetType` is

A `PlanetType` is a named palette plus five numbers. It is a Rust enum, not
data, because a type is content the game ships rather than a knob an author
turns. There are six: `BarrenRock`, `DustWorld`, `IceWorld`, `Volcanic`,
`Greenhouse`, `Temperate`. The fiction is not the Solar System, so none of them
is named after one.

Each type carries:

- **`slots()`** - the ordered biome slots, low band first, any polar cap last.
- **`relief()`** - how far the surface stands off the mean radius, as a fraction
  of it: 0.020 (Greenhouse) to 0.060 (Volcanic). Openly exaggerated. Real
  relief is a rounding error on a planet's radius (Everest is 0.14% of Earth's)
  and a body modelled that honestly has no silhouette at all.
- **`sea_level()`** - the height fraction below which the surface flattens to a
  true sphere. `IceWorld` 0.30, `Temperate` 0.42, `None` for the other four. A
  sea painted onto a displaced surface still has hills in it, so the flattening
  is what makes an ocean read as an ocean.
- **`detail()`** - the warp, grain and bump amplitudes and frequencies. This is
  the type's texture personality: Volcanic is high-contrast and busy
  (grain 0.22 at frequency 100), Greenhouse is soft and banded (grain 0.10 at
  frequency 30, bump 0.10) because haze has no edges to break up.

### Which biomes a type may draw from

A `BiomeSlot` is a height floor (optionally also a latitude floor, which makes
it a cap) plus a small array of `Biome` the seed may choose between. A `Biome`
is a name, an sRGB colour, a roughness, and a glow.

| Type | Slots | Shape |
| --- | --- | --- |
| Barren rock | 4 | basalt/shadow plain, regolith, highland, ridge chalk |
| Dust world | 5 | dark basin, ochre plain, pale dust, oxide ridge, frost cap |
| Ice world | 5 | dark ice, blue shelf, pressure ridge, frost peak, polar glare |
| Volcanic | 5 | lava lake (glowing), cinder flat, basalt, ash slope, ash peak |
| Greenhouse | 3 | haze basin, sulphur plain, cloud deck |
| Temperate | 6 | deep sea, pale sand, grass/steppe/savanna, forest, upland, ice cap |

Six bands is the hard limit (`PLANET_BAND_LIMIT`), because six is what the
uniform holds. A test asserts no palette exceeds it, a second asserts every
palette is authored low band first, and a third asserts any cap is last.

Two biomes deserve a note. Volcanic's lowest band is `molten`: it carries a
glow, and glow multipliers here start in the tens rather than at a fraction,
because Bevy's default `Exposure::BLENDER` multiplies lit surfaces by about
0.001 while emissive bypasses exposure entirely. And no biome's roughness goes
below about 0.3, water and ice included, because the shading normal varies per
facet and again per grain cell, so a band glossy enough to hold a tight
specular lobe catches a different slice of that lobe at every pixel and reads
as sparkle rather than as a sea. That was measured, not guessed: see section 4.

### How the seed picks

One `SeedStream`, an FNV-1a hash with a shift-xor finalizer, drawn in a fixed
order so a given `(type, seed)` always produces the same planet:

1. One biome per slot, from that slot's choices.
2. A small jitter on each band's height floor (up to 0.06) and on the cap's
   latitude floor (up to 0.07).
3. A palette-wide tint, up to 7% on each channel, so two dust worlds with the
   same biome names are still different colours.
4. A separate `shape_seed` for the terrain itself.

That is the whole draw. The captures in
`planetoid-look/planet-types-lineup.png` show its range: the bottom row is one
type, `DustWorld`, at four seeds, and they are recognisably the same kind of
world and clearly four different worlds.

### The author knobs

`PlanetConfig` is deliberately shaped like `AsteroidConfig` - a radius in
meters and an optional seed that pins the body across loads - so a planet reads
as the same kind of authored thing:

```ron
(
    radius: Meters(200.0),
    planet_type: DustWorld,
    seed: Some(4242),
    relief: Some(Meters(9.0)),
    sea_level: Some(0.0),
)
```

Two required fields and three optional ones. `seed: None` is seed 0, so an
unseeded planet is still the same planet on every load. `relief: None` and
`sea_level: None` take the type's own defaults; `sea_level: Some(0.0)` drains a
sea. A test round-trips the whole thing through RON.

### No blending, stated out loud

**Band edges are hard.** A fragment picks exactly one biome and takes its
colour, roughness and glow unmodified. There is no interpolation between
adjacent bands, no transition width, and no humidity or temperature field
selecting between them. The only thing that softens an edge is a small amount
of grain added to the elevation the threshold reads, and that exists to hide
mesh facets, not to blend two biomes. Blending is the next round.

## 4. What was built, and how it looks

### The three new pieces

`crates/nova_scenario/src/objects/planet_type.rs` owns the CONTENT: the types,
the biome palettes, `PlanetConfig`, and the seeded draw that turns one into a
`PlanetSurface`. It knows nothing about meshes or shaders. Nine tests.

`crates/nova_scenario/src/objects/planet_surface.rs` owns the SHAPE and the
material: the CPU height field, `planet_mesh`, the uniform, and
`PlanetSurfaceMaterial` as an `ExtendedMaterial<StandardMaterial, _>` following
the asteroid precedent. Seven tests.

`assets/shaders/planet_surface.wgsl` is the fragment: recover elevation and
latitude, warp both, walk the bands, write colour, roughness, emissive and a
bumped normal. No textures.

The height field is a 6-octave `Fbm<Perlin>` for continents, plus a 4-octave
`RidgedMulti<Perlin>` for mountains masked to the high ground, so ridges appear
on continents and not in ocean basins. The result is normalised per planet over
a 4096-direction sweep, which is what guarantees every authored band actually
appears - a test asserts exactly that, across every type and several seeds.

`PlanetSurfacePlugin` adds only the `MaterialPlugin`. It is deliberately NOT
added to `ScenarioObjectsPlugin`: nothing in the shipping game renders through
this yet.

### At backdrop distance

`planetoid-look/planet-types-lineup.png` puts all six types at one seed on the
top row and one type at four seeds on the bottom, at a range where each body is
a few hundred pixels across. The six read as six different kinds of world at a
glance, with no label needed: grey cratered rock, orange dust with a frost cap,
blue-white ice, dark basalt with orange fissures, a smooth banded cream
greenhouse, and a blue-green ocean world.

`planetoid-look/planet-types-backdrop.png` is the one that answers the
complaint. It frames a generated planet at the menu's exact reference pose,
`(0, 570, 1920)` m. Set it beside
`planetoid-look/planet-types-today-planetoid.png`, which is today's
`menu_planetoid` at the same pose in the same scene: a faceted beige potato
with a visible fine grid across it. The new body has a frost cap, continents,
a coastline that is not a contour ring, and mountains at the limb.

The grain is invisible at this range and that is the design. The footprint fade
has already taken it out; the bands and the silhouette carry the body.

### Up close

`planetoid-look/planet-types-focus-dust.png`,
`-focus-volcanic.png` and `-focus-temperate.png` fill about three quarters of
the frame height with one body. `planetoid-look/planet-types-close.png` goes
closer still, to a horizon.

Close up the grain is doing its job: the dust world reads as a rough oxidised
surface with a real horizon, the volcanic world's lava lakes glow against dark
cinder, and the temperate world has recognisable shorelines, forest against
grass, and an ice cap. Band edges are ragged rather than polygonal.

### Three defects found by looking, and their fixes

Each of these was visible in a capture, diagnosed, fixed, and re-captured.

**Directional streaking across every surface.** The first WGSL lattice hash
folded a whole 32-bit coordinate per FNV round. FNV mixes one byte per round;
folding four at once leaves neighbouring cells differing by a near-constant
amount, so the noise had a direction. Measured over an 80x80x6 lattice, the
neighbour correlation was `[-0.10, 0.54, 0.81]` on x, y and z. Adding a
`hash ^= hash >> 13` avalanche step between components and a final multiply-
shift took it to `[0.0002, 0.002, 0.0001]`.

**Band edges following mesh facets, and caps as ruled lines.** Both predicted
by section 2 and both visible. Fixed by the elevation grain (`EDGE_GRAIN`,
0.035 of the height range) and the latitude warp (`CAP_WARP`, 0.06).

**Per-pixel sparkle on seas and ice.** The bump bends the normal along the
grain. On a rough band that reads as texture; on a glossy one every bent normal
catches a different slice of a tight specular lobe, and it reads as white
noise. Two fixes together: the bump is now scaled by the band's own roughness
(`smoothstep(0.05, 0.55, roughness)`), so a sea gets essentially none and rock
gets all of it, and the glossiest biome roughnesses were raised to a floor near
0.3. The ocean now shows a broad coherent sunglint instead of static.

### Reproducibility, and how it was checked

The example was run three times as separate processes on the same display, into
three separate output directories, and all seven PNGs were compared by SHA-256.
**All seven are byte-identical across all three runs**, including across the
`cargo fmt` pass that sat between the second and third. Same seed, same planet,
down to the pixel.

Determinism is also pinned by unit tests rather than only by the captures:
`the_same_type_and_seed_draw_the_same_planet` compares two independent draws,
`different_seeds_draw_different_planets` asserts the converse, and
`the_same_config_meshes_the_same_planet` compares two independently generated
meshes vertex by vertex.

### The example

`examples/playable/planet_types.rs`, registered in the root `Cargo.toml`
immediately after `compare_planets`, with the same two harness modes as that
example. Interactively, keys 1-8 and the arrows re-draw the focus planet from a
type and a seed and the readout names the drawn biomes. Under
`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1` it walks the lineup, today's planetoid, the
backdrop framing, three focus draws and the close pass, and exits.

The row bodies mesh at subdivision 32 (10,892 vertices) and the focus body at
64 (42,252). The library default, `PLANET_SUBDIVISIONS`, is 48 (24,012).

## 5. What was left out, and honest limits

### Deliberately not built

- **Biome blending.** Asked for explicitly. Edges are hard.
- **Humidity and temperature fields.** The Gaia Sky shape from section 2. Only
  elevation and latitude select a biome here.
- **Craters.** A barren rock world wants them and has none. They want a
  separate impact-distributed field, not another fbm octave.
- **Clouds, atmosphere, rim scattering.** The greenhouse world fakes weather
  with a soft banded palette. There is no atmosphere shell.
- **Level of detail on the MESH.** One subdivision per body, chosen at build
  time. The grain fades with distance; the geometry does not.
- **Rings, moons, city lights, terminator glow.**
- **Any scenario rewiring.** No `ScenarioObjectKind` was added, no content was
  regenerated, `menu_planetoid` is untouched, and `PlanetSurfacePlugin` is not
  in the plugin graph. This is a prototype you have to ask for.

### Limits worth knowing before the next round

**The facet problem is hidden, not solved.** Elevation is recovered from an
interpolated position, so band contours across one triangle really are straight
lines. `EDGE_GRAIN` hides that at the grain's scale. Below the grain's scale -
a much closer pass than any capture here, or a much coarser mesh - the facets
will come back. The real fix is to evaluate the height field in the fragment
rather than recover it, which costs a full fbm per pixel.

**The terminator is gritty.** Where the bump-bent normal crosses `N·L = 0` you
get hard lit/unlit speckle, visible as fine glitter along the day-night
boundary in the focus captures. Real rocky bodies do this too, so it is not
obviously wrong, but it is stronger here than it should be and `BUMP_GAIN` is
the dial.

**Six bands is a real ceiling.** The uniform is a fixed array. Temperate
already uses all six. A type wanting a seventh needs the uniform widened.

**Relief is a lie, and a documented one.** 2% to 6% of the radius, against a
real world's 0.1%. It is what gives the limb a silhouette.

**One mesh per distinct planet.** Two planets of the same type and seed but
different radii still build two meshes. Nothing caches or instances.

### Cost, without numbers

**No timing figures are given here on purpose.** Another lane was building and
capturing on this same GPU throughout, so any frame time measured today would
be measuring that lane as much as this shader. What can be said structurally:

The fragment samples the fbm four times - once for the grain, once for the
coarse warp, twice more for the normal gradient - at three octaves each. That
is twelve trilinear value-noise lookups, and eight lattice hashes per lookup,
so on the order of ninety-six integer hashes per pixel. Meaningfully more ALU
than the asteroid's three texture samples, traded against zero texture
bandwidth and zero texture memory. Which side wins depends on the hardware, and
it has not been measured.

The band walk is at most six iterations of two compares, over a fixed-size
uniform array, with an early break. It is not the cost.

The footprint fade already skips the grain's contribution at distance, but it
skips the CONTRIBUTION, not the SAMPLE: the fbm is still evaluated and then
multiplied by zero. Branching around it when `fade` reaches zero is an obvious
and unmade optimisation.

Mesh generation is the CPU cost and it is not small: the range normalisation
sweeps 4096 directions before a single vertex is placed, and a subdivision-64
body then evaluates the field at 42,252 vertices plus two more per vertex for
the finite-difference normals. It is a build-time cost, not a frame cost, but
it is on the main thread today.

## Verification performed

- `cargo fmt`, clean.
- `cargo check --features debug --example planet_types --jobs 10`, clean, no
  warnings from this code.
- `cargo test -p nova_scenario --lib --features serde planet`: 16 passed, 0
  failed.
- The example RUN, not just checked, under Xvfb, three separate times.
- All seven captures opened and judged by eye, and iterated on three times
  before being accepted.

## Not verified

- No timing or frame-cost measurement of any kind (see above for why).
- No clippy run, and no workspace test run.
- No WASM or WebGL2 build. The uniform is laid out entirely in `vec4`s
  specifically so its 16-byte alignment holds there with no padding fields, but
  that is reasoning, not a build.
- Not run on any GPU other than this box's, and not in a real scenario - only
  in the example's own stage.
- The greenhouse and barren rock types were never given a dedicated focus
  capture; they were judged only at lineup range.
