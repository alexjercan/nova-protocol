# Asteroid look: kinds of rock, and a surface that stops repeating

Round entry for the ASTEROID lane. Baseline is master `f05298fe`.

The owner's read was that the asteroids look repetitive and flat. Both halves of
that are true, they have different causes, and they are fixed by different
layers. This records what was wrong, what the technique options were and what
they cost in licence terms, the `Kind` seam that mining and ores can hang off
later, and what the captures actually show.

Nothing here is a mining feature. A kind is a LOOK plus a name, and the name is
the hook.

---

## 1. What was wrong

### 1a. Repetitive: one projection, one scale, one tile, on every rock

The surface was a triplanar sample of a single texture and nothing else
(`assets/shaders/asteroid_surface.wgsl` at `f05298fe`, lines 73-116). It read the
body-local position, scaled it by `material.tiling`, took three
`textureSampleGrad` fetches, blended them by the face normal, and multiplied the
result into `base_color`. There was no second scale, no per-body offset, and no
noise.

Three consequences, in order of how loudly they read:

1. **The tile lands several times per body.** `ROCK_TEXTURE_TILING = 0.35`
   (`crates/nova_scenario/src/objects/asteroid_surface.rs:59` at `f05298fe`), so
   the texture repeats every `1 / 0.35 = 2.86` body-local units. A rock's
   surface stands `ASTEROID_GEOMETRIC_FACTOR_MIN = 3.5` to
   `ASTEROID_GEOMETRIC_FACTOR_MAX = 6.0` units out
   (`crates/nova_scenario/src/objects/asteroid.rs:395,397`), so the period fits
   into the radius between 1.2 and 2.1 times, i.e. roughly five to eight repeats
   across a whole body. That is well inside the range where a viewer reads a
   grid rather than a rock.
2. **Every rock lands in the same place in the tile.** The projection frame was
   the body's own axes with no per-body rotation and no per-body offset. Two
   rocks of the same seed are identical, and two rocks of different seeds still
   show the same texture features at the same distance from their centre.
3. **The tile is square and the wrap is by hand.** `sample_tile` wraps with
   `fract`, so the texture's own edges meet at right angles along body-local
   planes. Any feature that crosses a wrap draws a straight line at 90 degrees
   to another one. That is what the control capture shows most clearly.

The one thing that DID vary per body was the silhouette: `seed_stretch`
(`crates/nova_scenario/src/objects/asteroid_surface.rs:446`) feeds the mesh's
height field. The shape changed, the surface did not.

### 1b. Flat: a default material over a low-contrast texture

`insert_asteroid_render` built every rock's material as
`StandardMaterial::default()` with the extension bolted on
(`crates/nova_scenario/src/objects/asteroid.rs:433` at `f05298fe`). So every
asteroid in the game had:

- `base_color` white, so the only colour in the frame is whatever is in the PNG;
- `perceptual_roughness` 0.5, the same everywhere on every rock, so no part of a
  rock scatters differently from any other part;
- `metallic` 0.0, so nothing can ever read as metal.

And the PNG is narrow. `assets/base/textures/asteroid.png` is 736x736; measured
over all 541,696 texels its LINEAR luminance sits at p05 0.038, median 0.095,
p95 0.171. That median is recorded in the code as
`ROCK_TEXTURE_LINEAR_MID` (`crates/nova_scenario/src/objects/asteroid_kind.rs:154`)
because the shader needs it as a divisor. A texture occupying 0.04 to 0.17 with
a flat 0.5 roughness and no tint is, by construction, one dark grey-brown thing.

Note the texture is not the problem: a narrow, near-neutral albedo is exactly
what real rock has (see the albedo table in section 2e). The problem is that it
was the ONLY input.

---

## 2. Technique survey

### 2a. What the `noise` crate gives, and where it cannot help

`noise` 0.9 is already a dependency and the rock MESH already uses it:
`RockHeight` wraps `Fbm<Perlin>` to displace the silhouette
(`crates/nova_scenario/src/objects/asteroid_surface.rs`). Licence
`Apache-2.0 OR MIT` (verified in the published 0.9.0 `Cargo.toml` on docs.rs) -
compatible, no action needed.

It gives, directly and usefully: `Perlin`, `Simplex`, `OpenSimplex`,
`SuperSimplex`, `Value`, `Worley`, `Checkerboard`; the fractals `Fbm`,
`BasicMulti`, `Billow`, `HybridMulti`, `RidgedMulti`, all generic over a source
so `Fbm<Perlin>` and `Fbm<Fbm<Perlin>>` both work; the transformers `Displace`,
`RotatePoint`, `ScalePoint`, `TranslatePoint`, `Turbulence`; and the modifiers
`Abs`, `Clamp`, `Curve`, `Exponent`, `Negate`, `ScaleBias`, `Terrace`. Domain
warping is `Displace`/`Turbulence`, and cellular noise is `Worley`. So on paper
every technique below is already in the crate.

It cannot help HERE, for two reasons that are both fatal:

- **It has no GPU path at all.** Verified three ways on the published 0.9.0
  artefact: the dependency list is `num-traits`, `rand`, `rand_xorshift` and an
  optional `image` (no wgpu, naga, ash, cust or ocl); the source tree contains
  no shader files of any kind; and `NoiseFn::get` returns `f64` while `NoiseMap`
  is a `Vec<f64>`. It is a single-threaded CPU heightfield generator. There is
  no rayon either. (`Worley` also holds an `Rc<dyn Fn>`, so it is not
  `Send`/`Sync` - it cannot even go on a task pool without a wrapper.)
- **Its output would have to become a texture, and a texture is the fault.**
  The only way to get `noise` onto a rock is to bake a map and sample it. That
  map then tiles, at whatever scale it is applied, and re-introduces exactly the
  repeat this work exists to remove.

`PlaneMapBuilder::set_is_seamless` is the obvious escape hatch and it does not
work either. It has no doc comment; reading the source, it is a bilinear blend
of four shifted copies of the same non-periodic field:

```
V = (1-u)(1-v)*f(x+X,y+Y) + u(1-v)*f(x,y+Y) + (1-u)v*f(x+X,y) + uv*f(x,y)
```

Opposite edges converge, so the tile wraps. But it is not periodic noise - no
modulo, no lattice period, no wrapped permutation table - and the output is not
the source field, because every pixel is a convex mix of four different points.
Two consequences follow from that formula and neither is documented: each source
feature appears at up to FOUR output positions (structural ghosting), and
`Var(V) = sigma^2 * sum(w_i^2)` collapses from full variance at the corners to a
quarter of it at the tile centre - amplitude halved, so the middle of every tile
reads washed out. It also costs 4x the noise evaluations. It is C0 across the
seam only, so a crease survives under lighting.

**Decision: the per-pixel field is written in WGSL.** `noise` keeps the mesh job
it already does well. Nothing was added to or removed from the CPU side.

### 2b. Anti-tiling

| Technique | Source | Licence | Verdict |
| --- | --- | --- | --- |
| Per-tile hash offset + mirror; smooth-Voronoi texture bombing; N virtual offset copies indexed by low-frequency noise (Technique 3) | Inigo Quilez, "Texture repetition", 2015, <https://iquilezles.org/articles/texturerepetition/> | Site states technical code snippets are MIT (the shader ART is not) | **USE** - Technique 3 is the shape of what was built |
| Histogram-preserving blending | Heitz & Neyret, PACMCGIT 1(2) Art. 31, 2018, DOI 10.1145/3233304, <https://inria.hal.science/hal-01824773v1> | Paper CC BY 4.0 (HAL deposit licence field); no OSI-licensed reference code exists | **LEARN** - reimplement or skip |
| Same, with per-channel 1D LUTs and a soft-clipping contrast operator | Burley, JCGT 8(4) 31-53, 2019, <http://jcgt.org/published/0008/04/02/> | Paper CC BY-ND 3.0 | **LEARN** |
| Hex-tiling without histogram preservation | Mikkelsen, JCGT 11(2) 77-94, 2022, <http://jcgt.org/published/0011/03/05/> | Paper CC BY-ND 3.0; **reference code MIT**, <https://github.com/mmikk/hextile-demo> | **USE** if this is revisited |
| Texture bombing | Glanville, GPU Gems ch. 20 | (c) 2004 NVIDIA, all rights reserved | **LINK only** |

Two notes worth carrying forward. First, Shadertoy's default per-shader licence
is CC BY-NC-SA 3.0, which is unusable in this repo: IQ's article pages are the
MIT source, his Shadertoy mirrors are not. Copy from the article, never from the
shader. Second, "two incommensurate scales blended by a low-frequency mask" is
folklore with no clean primary; the closest rigorous citation is IQ's Technique
3, which is the same idea done properly with a float index and two fetches.

**Built:** the two-scale variant, plus a per-body frame. It is the cheapest
member of the family (one extra triplanar, so 6 fetches instead of 3) and it
needs no precomputation, no LUT and no engine integration. `hextile-demo` is the
upgrade path if the two-scale blend is ever judged not enough.

The one non-obvious finding, and it cost a capture iteration: **do not `mix` the
two scales linearly.** Two decorrelated samples averaged at even weight have
half the variance of either, and that variance IS the crevice detail the texture
is kept for. The first pass came back visibly SOFTER than the control it was
meant to beat. The fix is a near-binary choice with a narrow crossfade -
`smoothstep(0.42, 0.58, mottle)` - which keeps full contrast almost everywhere
and still leaves neither scale's period intact.

### 2c. Domain warping

Inigo Quilez, "Domain warping", 2002, <https://iquilezles.org/articles/warp/>.
MIT under the site's code term. `f(p + h(p))`, stackable as
`fbm(p + fbm(p + fbm(p)))`.

**Built,** at one level: `warped = local + warp * vec3(noise, noise, noise)`
evaluated at `local * 0.7`. This is the single highest-value line in the shader.
Plain fBm reads as soap bubbles; one warp turns it into stretched, tangled
strata that read as bedding planes. Three extra noise reads for most of the
character.

### 2d. Cellular / Worley, triplanar, and value noise

- Worley, "A Cellular Texture Basis Function", SIGGRAPH 96, pp. 291-294, DOI
  10.1145/237170.237267. **LINK.** Implementations: `stegu/webgl-noise` is MIT
  (commonly and wrongly called public domain); `ZRNOF/wgsl-noise` is the same
  code ported to WGSL, MIT; `stegu/psrdnoise` is MIT and is genuinely PERIODIC
  simplex noise with analytic gradients in native WGSL. `BrianSharpe/Wombat` has
  no licence file and only a "use it however you wish" header - **avoid**.
- Triplanar: GPU Gems 3 ch. 1 (Geiss) is the canonical write-up, **LINK**;
  Ben Golus's "Normal Mapping for a Triplanar Shader" is the practical
  reference, and its companion repo
  <https://github.com/bgolus/Normal-Mapping-for-a-Triplanar-Shader> is under
  **The Unlicense** - the most copy-safe artefact found. Not needed: the repo
  already had triplanar, and no normal map is involved.

**Built, all hand-written, no third-party code copied into this repo.** The
shader contains an integer avalanche hash, quintic-smoothed 3D value noise,
4-octave fBm at lacunarity 2.03 (deliberately not 2, so the octaves' lattices
never realign into a visible grid), and a 27-cell Worley F2-F1. F2-F1 rather
than F1 because it is near zero ON a cell wall and rises away from it, which
makes the wall a LINE that can be painted rather than a blob that can be tinted.
`psrdnoise` or `wgsl-noise` would have been legitimate to vendor; they were not,
because the field needed here is small, and a dependency-free shader is easier
to tune than one whose noise is someone else's contract.

The second finding that cost an iteration: **Worley must be fed the WARPED
coordinate.** On an unwarped lattice it draws cells of one size in rows, which
is a grid however jittered the seeds are. The first metal capture came back
wearing a visible honeycomb. Warping first turns the network into fracture.
Narrowing `VEIN_WIDTH` from 0.14 to 0.09, squaring the edge falloff, and
dropping metal's `vein_strength` from 0.45 to 0.16 finished the job.

### 2e. Real asteroid albedo, for the palette

Median visible geometric albedo by taxonomic class, from Mainzer et al. 2011,
"NEOWISE Studies of Spectrophotometrically Classified Asteroids", ApJ 741, 90,
DOI 10.1088/0004-637X/741/2/90, preprint <https://arxiv.org/abs/1109.6407>
(Table 1, Tholen classes):

| Class | N | Median pV |
| --- | --- | --- |
| E | 9 | 0.430 |
| V | 12 | 0.309 |
| S | 502 | 0.210 |
| M | 33 | 0.125 |
| X complex | 77 | 0.099 |
| B | 52 | 0.082 |
| C | 323 | 0.055 |
| D | 90 | 0.053 |
| P | 35 | 0.044 |

The paper's own less-biased recommendation, for bodies over 30 km, is S = 0.166
and C = 0.053, with an explicit endorsement of using spectral type as a proxy
for albedo. Masiero et al. 2011 (ApJ 741, 68) adds that the belt is strongly
BIMODAL - a bright complex near 0.25 and a dark complex near 0.06 - which is a
useful licence to make the kinds far apart rather than evenly spaced.

Two corrections that changed the palette:

- **C-types are not blue.** From the Bus-DeMeo mean spectra they are essentially
  neutral with a faint RED tilt. The genuinely blue class is B. So `carbon` is
  a neutral-to-warm near-black, not a cool one.
- **D-types are as dark as C-types but far redder.** That is a free, defensible
  way to get colour variety into a dark palette without cheating on albedo, and
  it is why `carbon`'s tint is warmer than its shade rather than greyer.

Not copied literally. Game rocks are lit by one key light against black and read
several stops darker than a lab measurement; the shipped values are chosen so
the ORDER and the RATIOS match the table while the frame stays legible. The
honest statement is "informed by", not "matched to".

---

## 3. The `Kind` design, and where mining hangs off it

**A kind is an open string id, not an enum.** The repo already prefers this: it
is the `SurfaceMaterial` / `MATERIAL_ROCK` convention, and `AsteroidConfig` had
the field already.

The key is the EXISTING `AsteroidConfig::material` field. Its doc at `f05298fe`
already anticipated exactly this ("the field exists so a mod can field an ice
body"). No new RON field, no migration, no touched content, and no change to any
of the ~30 existing constructors:

```ron
kind: Asteroid(( radius: Meters(90.0), texture: "base/textures/asteroid.png",
                 material: Some("ice"), seed: Some(4711), ... ))
```

Five lines of design:

1. `AsteroidConfig::material` IS the kind id. `None` resolves to `KIND_ROCK`,
   which is literally `MATERIAL_ROCK`, so every existing rock is unchanged.
2. The spawned entity carries `AsteroidKind(String)` beside the `SurfaceMaterial`
   the audio observers already read - one authored id, now with three consumers
   (impact sound, destruction sound, look).
3. `asteroid_kind_look(&str) -> AsteroidKindLook` is the lookup: fifteen scalars
   and three colours. An unknown id falls back to rock, so a mod can never spawn
   an unrendered asteroid.
4. The look plus the body's own seed becomes one `AsteroidSurfaceUniform`. Seed
   drives only the projection frame, so a kind stays a kind and a rock stays its
   own rock.
5. Ore hangs off the id, not off the look. `AsteroidKind` is a queryable
   component with a stable string, so a future mining table is
   `kind -> yield` and nothing about it needs the renderer.

Five kinds ship. Four are content; the fifth is a test instrument:

| id | reads as | roughness | metallic | veins |
| --- | --- | --- | --- | --- |
| `rock` | warm tan stone banded with cool slate | 0.70 - 0.96 | 0.00 | faint |
| `metal` | cold grey-blue with bright cut lines | 0.28 - 0.70 | 0.85 | strong |
| `ice` | pale blue-white, glossy, bright fracture network | 0.08 - 0.55 | 0.00 | strongest |
| `carbon` | near-black, faintly warm, matte | 0.88 - 1.00 | 0.00 | faint |
| `plain` | **the control**: every knob off, the surface exactly as it was | 0.50 flat | 0.00 | none |

`plain` exists so the before picture can stand in the after picture's own
lighting, in the same frame. It is not a fifth rock and no scenario should use
it.

### Authorable and moddable

Adding a kind today is one arm in `asteroid_kind_look` plus a constant. That is
deliberately the SIMPLE version the brief asked for, and it is the one thing
here that should not survive contact with real content: the table is a Rust
`match`, so a mod cannot add a kind without recompiling. Section 5 says what the
next step is.

---

## 4. What was built

Four layers in `assets/shaders/asteroid_surface.wgsl`, in the order the fragment
computes them:

1. **A per-body frame.** The seed is hashed to one 0..1 `jitter`, which becomes
   a rotation about Y, a tilt about X at 0.618 of that angle, and a translation.
   Two rocks now sit in different places in the tile AND in different places in
   the noise, from one number.
2. **The macro field.** One domain warp, then 4-octave fBm, then a contrast
   push. This is the layer that decides what the rock IS.
3. **Cell walls.** Worley F2-F1 on the warped coordinate, painted with the
   kind's vein colour, behind a uniform branch so a kind with no seams pays
   nothing.
4. **The texture, twice.** Triplanar at `tiling` and again at `0.41 * tiling`
   with an offset, chosen between by the macro field. The texture is used as
   RELIEF - a ratio against `ROCK_TEXTURE_LINEAR_MID` - rather than as colour,
   so it multiplies the palette instead of dragging it down to 0.095.

Colour and specular read the same field (two thirds macro, one third grain), so
where a rock looks worn it also scatters like it. That is the whole of the
"flat" fix: nothing on a rock is at a constant roughness any more.

The art-direction finding worth keeping: **a palette that ramps in LUMINANCE
reads as lighting; a palette that ramps in COLOUR TEMPERATURE at comparable
luminance reads as material.** The first two `rock` attempts ramped dark-tan to
light-tan and came back as one uniform blob with a shadow on it. Ramping cool
slate to warm tan at similar brightness is what made it read as strata.

Rust side:

- `crates/nova_scenario/src/objects/asteroid_kind.rs` - new. The kind ids, the
  `AsteroidKind` component, `AsteroidKindLook`, the five tables, and five unit
  tests.
- `crates/nova_scenario/src/objects/asteroid_surface.rs` - the four scalar
  `#[uniform(100)]` fields become one `AsteroidSurfaceUniform` deriving
  `ShaderType`, plus `seed_jitter`.
- `crates/nova_scenario/src/objects/asteroid.rs` - resolve the id once at spawn,
  insert `AsteroidKind` beside `SurfaceMaterial`, and pass the look and the seed
  into the material.
- `examples/playable/asteroid_kinds.rs` - new, both harness modes.

### wasm32

The two hand-written `_webgl2_padding_16b*` fields were REMOVED, not extended.
`encase`'s `ShaderType` gives `LinearRgba` alignment 16, so a struct led by
three colours has alignment 16 and its size rounds up to a multiple of 16
automatically. The `#ifdef SIXTEEN_BYTE_ALIGNMENT` block went with them. The web
build targets WebGPU rather than WebGL2 (`tasks/20260812-100256/DESIGN-round3.md:11-12,197`), so
the stricter WebGL2 uniform rule was not in play regardless. WGSL portability
was kept in mind: `bitcast<u32>` rather than `u32()` for the hash, and
`clamp(x, 0, 1)` rather than `saturate`.

**Not verified: no wasm build was run.** See section 7.

---

## 5. Deliberately left out

- **Mining, ore, yield, and any economy.** The brief said not to build it. The
  hook is `AsteroidKind`, a component with a stable string id; a yield table
  reads it and the renderer never learns about it.
- **A RON kind registry.** The look table is a Rust `match`, so a mod can add a
  kind id but gets rock's look for it. The right next step is an asset-loaded
  `kind.ron` keyed by the same string, with the shipped five as the default set.
  That is a real piece of work (asset lifetime, hot reload, the fallback path)
  and it is not "a feeling for it now".
- **Normal and roughness MAPS.** Roughness is driven from the same scalar field
  as colour, which is free and reads correctly at the distances a rock is seen
  at. A real normal map would need a triplanar normal blend (Whiteout or RNM,
  per Golus), which doubles the fetch count again.
- **Histogram-preserving blending and hex-tiling.** Both would give a stronger
  anti-tiling result. Both are more code, and `hextile-demo` (MIT) is sitting
  there if the two-scale blend is ever judged insufficient.
- **Per-kind mesh character.** All five kinds wear the same silhouette
  distribution. Ice arguably wants sharper facets and carbon rounder ones. That
  is `RockHeight`'s business, not the shader's, and it is a separate change.
- **Parallax, self-shadowing, emissive veins.** Not attempted.
- **Any timing.** See below.

---

## 6. Captures

All six are 1920x1080, shot through `examples/playable/asteroid_kinds.rs` under
`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`, on the real scenario path
(`AsteroidConfig` -> `asteroid_scenario_object` -> `insert_asteroid_render`).
Every rock in every frame was spawned the way a scenario spawns one.

| file | what it shows |
| --- | --- |
| `asteroid-look/asteroid-kinds-grid.png` | the whole lineup: five kinds across, three seeds down |
| `asteroid-look/asteroid-kinds-control-vs-rock.png` | `plain` beside `rock`, same lighting, same distance |
| `asteroid-look/asteroid-kinds-metal-vs-ice.png` | two kinds that must not read as two tints |
| `asteroid-look/asteroid-kinds-control-closeup.png` | **the before picture**, one body filling the frame |
| `asteroid-look/asteroid-kinds-rock-closeup.png` | **the after picture**, same distance |
| `asteroid-look/asteroid-kinds-rock-carved.png` | the same body as above, after five craters |

Four more were shot for the content and editor passes, live, on the shipped
paths rather than on the bench example:

| file | what it shows |
| --- | --- |
| `asteroid-look/menu-weave-belt.png` | the `menu_weave` backdrop, scattered from its authored mix |
| `asteroid-look/salvage-belt.png` | the campaign salvage plate, from `stage::belt()` |
| `asteroid-look/editor-rock-kind-rock.png` | a rock placed from the editor palette, kind `rock` |
| `asteroid-look/editor-rock-kind-ice.png` | the same rock one click later, kind `ice` |

`tasks/20260815-231945/asteroid-look/asteroid-look.html` lays the same six
frames out as a page, with the before and after side by side. Open it in a
browser; it is self-contained apart from one webfont.

What they actually show, read off the pixels:

- **The control closeup has visible rectangular tile repeats.** Straight
  horizontal and vertical edges cross the body where the `fract` wrap lands, and
  the same texture feature appears more than once at the same size. Colour is
  one flat pale tan over the whole rock with nothing but the key light varying
  it. This is the fault, in one frame, with no argument attached.
- **The rock closeup has no tile grid at all.** No straight wrap edges survive
  the two-scale choice and the per-body frame. Warm tan and cool slate strata
  run across the body at angles the silhouette did not cause, and the crevice
  grain from the texture is still there at full contrast - it is not a blurred
  version of the control.
- **The grid reads as five materials, not five tints.** Metal is cold and
  bright-specular, ice is pale blue and glossy, carbon is near-black and matte,
  rock is warm and dry, and the control sits among them looking conspicuously
  like an untextured white object. Across a row, three seeds of one kind are
  three different rocks: the frames differ in stratum layout and in where the
  bright and dark regions fall, not only in outline.
- **`metal` vs `ice` are clearly different materials.** Both are bright, and
  they separate on specular behaviour and on vein character rather than on hue
  alone, which is the harder test.
- **The carved rock wears the same surface.** This is the one that matters for
  the carve path: five craters, cut through the real damage path, and the fresh
  interior walls carry the same warm/cool strata continuously into the crater.
  No seam at the cut, no stretching, no reversion to the default white material.

---

## 7. Limits, cost, and what was NOT verified

### No timing was measured, and none is claimed

This lane shared a GPU with a second lane running concurrently. Any millisecond
figure taken here would be a fact about contention, not about the shader. **No
benchmark was run and no performance claim is made in this document.** What can
be stated is the static cost, which is countable from the source:

| | before (`f05298fe`) | after |
| --- | --- | --- |
| `textureSampleGrad` per fragment | 3 | 6 |
| `hash_cell` per fragment | 0 | 56, or 83 with veins |
| uniform bytes | 16 | 112 |
| branches | 0 | 1, uniform across the draw |

The 56 hashes are 24 for the domain warp (3 value-noise reads at 8 lattice
corners each) and 32 for the 4-octave fBm. The Worley walls add 27 and sit
behind a uniform branch, so `rock` and `carbon` - the kinds a scenario actually
scatters by the hundred - pay for them at a low `vein_strength` and `plain` does
not pay at all. This is a materially more expensive fragment shader than the one
it replaces. Whether that matters on a field of two hundred rocks is an open
question and it needs a quiet box to answer.

### Honest limits

- **The two-scale blend is the weakest member of its family.** At extreme
  closeup - closer than a ship ever gets - the near scale's period is still
  findable if you look for it. Hex-tiling would fix it properly.
- **Fifteen scalars is a lot of knobs for four kinds.** They were tuned against
  captures, by eye, over four iterations. They are not derived from anything and
  a fifth kind will want a sixteenth knob.
- **The palette is informed by real albedos, not matched to them.** See 2e.
- **`grain_mid` is a constant measured off one texture.** A mod shipping its own
  asteroid texture with a different mean will get relief that is biased bright
  or dark. The honest fix is to measure it at load; the cheap fix is to expose
  it per kind. Neither was done.
- **All five kinds share one silhouette distribution.**

### Verified

- `cargo fmt`, clean.
- `cargo check --features debug --example asteroid_kinds`, clean, which pulls in
  every crate that changed.
- `cargo test -p nova_scenario --lib asteroid_kind`: 5 passed, 0 failed.
- The example RUN on Xvfb, both harness modes exercised, `autopilot: cycle
  complete, no panic`, all six shots written.
- All six PNGs opened and judged. Four capture iterations were driven by what
  they showed; three of the findings above (the linear-mix variance collapse,
  the Worley honeycomb, the luminance-vs-temperature palette) came out of
  looking at frames that were worse than the control.
- The carve path, end to end, in this example: `apply_damage` through the real
  `DamageMarks` -> `AsteroidField` -> remesh route, and the resulting frame.

### NOT verified

- **No wasm32 build was run.** The alignment argument is read off `encase`'s
  `ShaderType` derive and the WebGPU target statement in `DESIGN-round3.md`. It
  is reasoning, not a green build.
- **No timing, on anything.** By instruction.
- **No workspace-wide clippy and no full test suite.** By instruction.
- **`examples/playable/carve_asteroids.rs` was not used.** It stalls
  deterministically at its `hold actual PDC fire on one point` step after 15 s,
  twice, before producing any capture, on this worktree. **I did not establish
  whether that stall pre-exists this change**, and it is not obviously related -
  the step never gets as far as damage. The carve claim in this document rests
  on the `bug_carve_apply` systems range (passes clean) and on the carved
  capture above, both of which go through the same `apply_damage` route. Someone
  should check `carve_asteroids` against master.
- **Every existing capture in the repo that contains an asteroid is stale.**
  Every shipped asteroid now looks different from how it looked at `f05298fe`.
  That is the point. The repo-wide re-shoot is the release task's, not this
  one's.

Added by the content and editor passes:

- **`web && npm run ci` was NOT run.** The worktree has no `node_modules` and
  installing them needs the network. The four web pages touched
  (`web/src/create/objects.md`, `web/src/create/actions.md`) are Markdown
  content inside existing tables and code fences. `mdbook build` for `docs/`
  is clean.
- **`first_shift_map` and `second_shift_map` were not shot.** They build their
  layout from `examples/playable/shared/first_shift_stage.rs`, a
  candidate-layout module, NOT from the authored content - so they would not
  show the mixes. `first_shift_03_salvage` was shot instead: it runs
  `first_shift_scene` from the real `nova_authoring` builders, which is the
  path the mixes actually live on.
- **`salvage-belt.png` is an X root-window grab, not a harness capture.**
  `first_shift_03_salvage` uses the bare `nova_autopilot()` script, which has
  no shoot step, so the scene was run live and grabbed with `import`.
- **No mod was loaded from the portal.** The migration was proved on
  `assets/mods/example` and on `web/mods/**` by lint, not by installing a
  published mod and watching it fail.

---

## 8. The content pass: kinds in the shipped fiction

### The scatter mix

`ScatterObjectsConfig` gained one field, modelled on `asteroid_radius` at
`spawn.rs:298` and then made REQUIRED under section 9:

```rust
pub asteroid_kinds: Vec<(String, u32)>,
```

**Weighted, not uniform.** A `Vec<(String, u32)>` lets an author state a
proportion once - `[("rock", 12), ("metal", 1)]` is one metal body in
thirteen - where a repeated uniform list makes them count out the copies by
hand and re-count every time the mix changes. The weights are relative COUNTS,
not percentages, so a mix never has to add up to anything.

**Its own RNG stream.** The scatter already ran two: positions off `seed`, and
per-rock silhouette seeds off `seed ^ SILHOUETTE_SALT`. Kinds are a third,
`seed ^ KIND_SALT`. Decorrelating them is what makes the field safe to change:
adding kinds to a scatter that already ships moves no rock and changes no
radius, because neither of the other two streams is advanced.

**A hashed index for hand-authored lists.** `asteroid_kind_at(&MIX, index)`
serves the belt, which is a written-out list rather than a scatter. It hashes
the index instead of cycling the buckets, because an authored list is usually
in rough spatial order and a cycle would stripe the map: rock, carbon, ice,
rock, carbon, ice, along the belt, in a straight visible line.

### The mixes, one line of reasoning each

| where | mix | why |
| --- | --- | --- |
| `menu_weave` (40) | rock 12, carbon 4, ice 3, metal 1 | the widest mix in the game, because this backdrop's whole job is to prove a belt is not one rock |
| `menu_duel` (12) | rock 7, carbon 3 | dressing behind a fight: carbon reads as depth without competing with a muzzle flash |
| `menu_gauntlet` (26) | rock 8, carbon 3, ice 1 | mostly stone under the stand, with one thing down there that catches a highlight |
| `menu_waystation` (18) | rock 12, ice 5, metal 1 | a freight lane: ice is the propellant and water a waystation exists to move, and exactly one metal body |
| campaign salvage plate (40) | rock 12, carbon 5, ice 2, metal 1 | the belt the player works in, and the one a future ore system reads: rich enough to be worth looking at, still overwhelmingly stone |
| campaign ambient rocks (20) | rock 6, carbon 3, ice 1 | background, so no metal at all - the rare kind is spent where the player is close enough to see it |

**Metal stays rare everywhere.** It is the kind a future ore system makes
valuable, and a valuable thing that is everywhere is not valuable. One in
twenty is the most any field gets, and two fields have none.

`second_shift.rs` needed no mix of its own: both shifts spawn their rocks
through `stage::belt()` (`first_shift/mod.rs:573`, `second_shift.rs:486`), so
the salvage and ambient mixes are already what the second shift flies through.
Its own references to `stage::SALVAGE_ROCKS` are clearance tests and a wreck
scatter offset, not spawns.

### Keeping `texture:`

The kind and the texture were reviewed as possible duplicate answers to one
question. They are not, and `texture` stays:

- They answer different questions. The kind supplies the palette, the
  large-scale character and the specular; the texture supplies the fine
  crevice grain the kind modulates - `AsteroidKindLook::grain` is measured
  against the texture's own mean luminance precisely so the two compose rather
  than fight.
- Removing it would take a mod's only per-rock art hook away with nothing to
  replace it. `assets/mods/example/textures/rock.png` exists and is authored.
- The right time to remove it is when the kind registry moves into RON. The
  texture then belongs to the KIND, is named once per kind instead of once per
  rock, and leaves `AsteroidConfig` for a reason rather than as a tidy-up.

---

## 9. The no-backwards-compatibility audit

The rule applied to the whole branch: no implicit default, no `None` that
quietly means a value, no unknown id that resolves to a house answer. Unknown
is an error at the earliest point that can see it - lint first, load path
second.

| where | was | is |
| --- | --- | --- |
| `AsteroidConfig::material` | `Option<String>`, `#[serde(default, skip_serializing_if)]`, absent meant `rock` | plain `String`, required; a file without it fails to deserialize |
| `asteroid_kind_look` | `_ =>` fell through to `rock()`, documented as "so a mod cannot spawn an unrendered asteroid" | returns `Option`; `None` is a REFUSAL, and the reasoning in the doc is inverted to say so |
| `insert_asteroid_render` | took the fallback look | `error!`s with the offending id and the shipped list, and renders nothing |
| `AsteroidSurfaceUniform` | had a `Default` impl that was silently the rock look | the impl is deleted, and `Default` is off the `AsteroidSurfaceMaterialExt` derive |
| `ScatterObjectsConfig::asteroid_kinds` | (new field, first drafted as `Option`) | required `Vec`; an asteroid template with no weight `error!`s and spawns NOTHING |
| `lint/scenario.rs` | no check | `check_asteroid_kind` errors on an unknown id; `check_scatter_kind_mix` errors on an empty mix, a mix on a non-asteroid template, an unknown id in a mix, and a zero-weight entry |
| `assets/mods/example/example.content.ron` | three asteroids with no `material` | hand-migrated, with the teaching comment that is now the migration guide |
| `web/mods/**` | 37 asteroid blocks, 4 scatters | every one names its kind and every scatter its mix |
| editor `node.rs` stock template | would have inherited the default | names `KIND_ROCK` explicitly |
| editor inspector | - | the kind is a PICK LIST, which is the only control that cannot author a kind the game does not ship |

Two `Option`s were reviewed and KEPT, because absence is a documented override
with runtime-derived behavior behind it and the field's own doc says what it
means: `AsteroidConfig::mass` (absent = the global rule about which radii are
wells) and `AsteroidConfig::lock_signature` (absent = the radius).
`AsteroidConfig::seed` is the same shape: absent derives a seed from the
object's own id, which is a stated rule, not a house answer.

### How the break was proved

Both proofs were run against `content lint`, on files restored afterwards by
copy from a backup - never by `git checkout`.

- **Missing field.** Deleting `material:` from one asteroid in
  `assets/mods/example/example.content.ron`:
  `parse .../example.content.ron: 433:25: Unexpected missing field named 'material' in 'AsteroidConfig'`.
- **Unknown kind.** Setting `material: "obsidian"`:
  `ERROR [example] example.content.ron example_arena: asteroid 'example_target_2': 'obsidian' is not a kind - author one of ["rock", "metal", "ice", "carbon", "plain"]`,
  and, for a zero-weight mix entry,
  `ERROR [gauntlet] ... 'rock' is weighted 0, so it never appears`.

---

## 10. The editor pass: placing a rock by kind

### The inspector CAN edit a `String`, and a text box is still the wrong control

`BeaconConfig::label` is a `String` and is editable today as a `RowValue::Text`,
so the reflection walk needed nothing new to WRITE the kind. It needed
something new to TEACH it: nobody guesses `carbon` from an empty box, and a box
is also a control that can author a kind the game does not ship. So the kind is
a `RowValue::Choice`.

What that took:

- `ASTEROID_KIND_SUMMARIES` beside `ASTEROID_KINDS` in `asteroid_kind.rs`: the
  same ids, in the same order, with one line each. A test pins the two together
  so a sixth kind cannot be added to one and not the other.
- `offer_object_vocabularies` in `inspect.rs`, keyed on the OBJECT's kind rather
  than declared in the `DECLARED` field table. The table matches by field name
  at any depth, and a ship section has a `material` too - it names a paint id
  rather than a rock.
- `choose_field` learned a `String` target. Every choice before this one
  switched an enum by variant name; a vocabulary field takes the option text as
  its value.
- `MATERIAL` in `ASTEROID_PICKS`, so the curated panel shows it. A panel that
  showed the radius and hid the kind would be hiding the thing a builder came
  to pick.

An id the game does NOT ship stays a text row showing exactly what the file
says. A picker that snapped it to `rock` would hide a broken document behind a
control that looks like it works.

### The stage shows the kind

Two changes, both in `preview.rs`:

- `asteroid_preview_material(kind, texture)` paints the preview sphere in the
  kind's palette, blended by the kind's own `kind_mix` so `plain` stays the
  texture untouched, at the mean of its two roughnesses and its metallic. It is
  NOT the shipped shader - the stage draws a smooth sphere, the flown rock is a
  triplanar surface over a carved mesh - but the palette and the specular are
  what tell an ice rock from a carbon one at a glance, and that is the question
  the picker asks. An unknown kind paints magenta, unlit, with no emissive (the
  round 5 audit's `emissive` + `unlit: true` pattern is deliberately not
  copied).
- `drawn_fields` for an asteroid gained `"material"`. Without it the body is
  never rebuilt and the picker looks like it does nothing - the exact bug this
  feature would have shipped with.

No reconciler was touched. The kind row changes its VALUE, not its shape: the
existing `Choice` reconciliation is keyed by slot and re-marks the selected
segment on its own.

### Verified live

`system_ship_editor` on `:95`, through the real pointer pipeline, gained four
beats after the rock's pose edits: frame the placed rock, shoot it, assert
every shipped kind is offered as a widget, click `ice`, assert both the panel
AND the material on the stage, shoot it again. `named_widget_exists` is the
guard that matters - `click_named` warns and continues when a widget is
missing, so a walk that only pressed the option would pass with nothing
offered and nothing written.

`editor-rock-kind-rock.png` and `editor-rock-kind-ice.png` are the two frames.
Read off the pixels: the Material row draws as a segmented bar of all five ids
with `rock` lit; one click later `ice` is lit, and the SAME rock in the middle
of the stage has gone from dry warm grey-brown to a brighter cool grey with a
hard specular highlight where there was none. The rock changed, not just the
panel.

### The content pass, verified live

- `menu-weave-belt.png`: `NOVA_MENU_BACKDROP=menu_weave`, 37 of 40 rocks
  placed. The frame carries warm tan stone throughout, several pale blue-white
  ice bodies, dark near-black carbon bodies, and one metallic body with bright
  glints on the left. It reads as a belt with character rather than as noise,
  and the rare kind is in frame rather than theoretically present.
- `salvage-belt.png`: `first_shift_03_salvage`, the production scene built by
  `first_shift_scene` from the real content builders. Same reading: a stone
  majority, several distinctly dark carbon bodies, one bright ice body, and one
  metallic body.

Reproducibility was checked at the unit level rather than by eye:
`scatter_draws_kinds_from_a_weighted_mix` scatters the same seed twice and
asserts the two kind lists are identical, that every id in the mix appears, and
that the majority kind is a majority. An empty mix and an all-zero mix are
asserted to spawn nothing at all.
