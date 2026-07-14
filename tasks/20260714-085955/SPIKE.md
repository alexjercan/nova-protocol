# Spike: How do we get particle effects back on wasm - WebGPU web build vs shader fallback?

- DATE: 20260714-085955
- STATUS: RECOMMENDED
- TAGS: spike, wasm, v0.6.0, polish

## Question

Particle effects (thruster plume, turret muzzle flash, torpedo launch/detonation
bursts) are `bevy_hanabi` effects and are currently `#[cfg]`'d off on wasm. Can we
get them working in the web build, and if so how - switch the web build to WebGPU
(Option A), ship a non-compute shader/billboard fallback on WebGL2 (Option B), or
run both with feature detection (Option C)?

A good answer states plainly whether hanabi-on-wasm is possible today, what it
costs in browser reachability (the web "Play" link is a landing-page funnel, so
losing browsers is a real regression), and picks a direction concrete enough to
plan.

## Context

- Versions: `bevy 0.19`, `bevy_hanabi 0.19` (`default-features = false`, `2d`/`3d`).
- The web build has **no explicit render backend config** - `nova_core` adds plain
  `DefaultPlugins` with no `WgpuSettings`/`RenderPlugin` override
  (`crates/nova_core/src/lib.rs:63`). So the backend is decided purely by bevy's
  cargo features. Trunk builds with default features, and bevy's defaults include
  `webgl2` -> **the web build ships WebGL2 today.**
- Hanabi is gated off on wasm in three places, all with the same FIXME
  (`20260706-162908`):
  - `crates/nova_gameplay/src/plugin.rs:51` - `HanabiPlugin` itself.
  - `crates/nova_gameplay/src/sections/turret_section.rs:323,327` - muzzle + projectile effects.
  - `crates/nova_gameplay/src/sections/torpedo_section/mod.rs:320-328` - launch + detonation bursts.
- Partial fallbacks already exist and render on every target: the torpedo blast is
  also drawn as an expanding/fading `BlastRadiusVisual` sphere
  (`torpedo_section/render.rs:113`), and projectiles/torpedoes have plain mesh
  bodies. So the game is playable on web now - it just has no GPU particles.
- Existing WebGL2-specific workarounds in the tree: std140 padding fields
  (`hud/velocity.rs`, `sections/thruster_section.rs`, all `#[cfg(target_arch = "wasm32")]`)
  and the v0.5.1 `view_formats` crash fix (`hud/target_inset.rs:234`). These exist
  *because* we ship WebGL2; switching to WebGPU makes them unnecessary (but they
  stay harmless).

## Findings (researched 20260714)

**1. Hanabi on wasm is possible, but WebGPU-only. This is by design, not a bug.**
Hanabi makes heavy use of compute shaders. Compute shaders on wasm exist only
through the **WebGPU** backend - WebGL2 has no compute and never will. Since
hanabi v0.13 / bevy 0.14 this has been the rule, and it still holds in 0.19: enable
the `bevy/webgpu` feature or you get `"No wgpu backend feature ... enabled"`. So the
FIXME's "it's not working" really means "we ship WebGL2, and hanabi cannot run on
WebGL2." There is no outstanding hanabi wasm bug blocking us - flip the backend and
the same effects that run on native run on web.

**2. The real cost is browser reachability, and it has improved a lot since 2024.**
WebGPU global support is ~**85%** (March 2026), up from ~70% in 2024:
- Chrome / Edge: default since 113 (2023).
- Safari: default since macOS Tahoe 26 / iOS 26 / iPadOS 26.
- Firefox: default on Windows (141+) and macOS ARM (145+); **still behind a flag on
  Linux, Android, and Intel Macs.**

So a WebGPU-only web build drops roughly the bottom ~15%: Firefox-on-Linux,
Firefox-on-Android, Firefox-on-Intel-Mac, and anyone on an older OS/browser. For an
indie space-shooter that skew (Linux + Firefox) is likely over-represented in the
audience, so treat 15% as a floor, not a typical case.

**3. The regression is a hard fail, not graceful.** A bevy `webgpu` build does not
silently lose particles on a non-WebGPU browser - it **fails to initialize the
renderer at all** (dead canvas / panic). So "Option A, naive" turns today's "game
works, no particles" into "game does not load" for that ~15%. Any WebGPU switch
must therefore ship with a `navigator.gpu` detection gate at the Play boundary.

**4. Option C is not a free bevy feature.** Bevy still treats `webgl2` and `webgpu`
as effectively mutually-exclusive build features; the "one wasm, runtime fallback"
request (bevy#13168) was never delivered as a first-class feature. So Option C means
**two separate trunk builds** plus JS feature detection choosing which `.wasm` to
load - ~2x build time, CI, and artifact size.

## Options considered

- **Option A - switch the web build to `bevy/webgpu` + a WebGPU detection gate.**
  How: add a `webgpu` feature that turns on `bevy/webgpu`, build trunk with it
  (bevy prefers webgpu when both are on), remove the three wasm `#[cfg]` gates so
  hanabi compiles+runs on wasm, and add a `navigator.gpu` check in the Play page /
  loader that shows a friendly "this build needs WebGPU (Chrome/Edge, or Firefox on
  Windows)" message instead of a dead canvas.
  Pros: **one particle codebase** (native and web share hanabi); real effects on
  web; small, mostly-deletion diff; fully reversible (a feature flag). Cons: ~15% of
  browsers get the message instead of the game; must own the detection UX.
  Unknowns: confirm hanabi 0.19 runs clean under our exact bevy 0.19 web setup (no
  serde/typetag feature - we do not serialize effects, so fine); confirm the WebGL2
  padding/view_format hacks stay harmless under WebGPU.

- **Option B - keep WebGL2, build a non-compute particle fallback.** How: leave the
  backend alone; write billboard-quad / shader-driven stand-ins for plume, muzzle
  flash and torpedo bursts (extending the `BlastRadiusVisual` pattern already in the
  tree), keeping hanabi for native only. Pros: **100% browser reachability**, no
  loader gate, no boot regression. Cons: a **second, parallel effects system** to
  build and maintain forever, and web visuals permanently diverge from native.
  Unknowns: how close billboard quads can get to the hanabi look before it is more
  work than it is worth.

- **Option C - WebGPU with a WebGL2 fallback build.** How: two trunk builds
  (webgpu + webgl2), JS picks by `navigator.gpu`; the webgpu build gets hanabi, the
  webgl2 build gets Option B's fallback (or no particles). Pros: best of both -
  real particles for the ~85%, game still loads for the rest. Cons: the most
  moving parts - 2x builds/CI/size, feature detection, and you still have to write
  Option B for the fallback build to be more than "no particles."

- **Do nothing.** Web stays particle-free. Costs: the landing-page "Play" build
  keeps looking flatter than native; the FIXME lingers. Cheap to defer - this is
  tagged `polish`, priority 30.

## Recommendation

**Option A: switch the web build to WebGPU and un-gate hanabi, shipped together with
a mandatory `navigator.gpu` detection gate at the Play boundary.**

Reasons it beats the runners-up:
- It collapses to **one particle system**. Native already runs these exact hanabi
  effects, so Option A is mostly *deleting* `#[cfg(not(wasm))]` gates plus a build
  flag - versus Option B, which asks us to author and forever maintain a second
  effects system that only exists to dodge a backend limitation that is disappearing.
- The 2024 reason this was punted (WebGPU too niche) has largely expired: ~85% and
  climbing, all major browsers now ship it by default on current OSes.
- The one thing Option A must not do is regress the game to "won't load." The
  `navigator.gpu` gate fixes that: the ~15% get an honest "needs WebGPU" message
  (and keep the rest of the landing site), not a black screen.
- Option C is the correct *upgrade path*, not the starting point. If analytics later
  show meaningful Firefox/Linux traffic bouncing off the gate, add the WebGL2
  fallback build then - the detection gate from Option A is already the hook it
  plugs into. Building two pipelines now is premature.

Net: do A now (single codebase, real particles, graceful gate), keep C on the shelf,
reach for B only if "any browser loses the game" is judged unacceptable.

Note: this is a reachability *policy* call as much as a technical one, and the task
explicitly says "verify current browser support before committing." The numbers above
are that verification; the final A-vs-C-vs-B pick is the maintainer's to confirm.

## Open questions

- Is dropping ~15% (Firefox-Linux/Android, Intel-Mac, old OS) acceptable for the
  landing-page Play link, or is any browser loss a no-go? -> A vs C/B is the
  maintainer's call; the spike recommends A + gate.
- Does hanabi 0.19 run clean on our exact bevy 0.19 web build with no extra flags
  beyond `--cfg=web_sys_unstable_apis` (already set) and `bevy/webgpu`? -> resolve
  in the implementing task with a real trunk build + browser smoke test.
- Do the WebGL2 std140 padding fields and the `target_inset` view_formats guard need
  removing under WebGPU, or do they stay as harmless dead code? -> low-risk; decide
  during implementation.

## Next steps

Direction-level tasks this spike seeded (for `/plan` to break into steps). Option A
was confirmed by the maintainer:

- tatr 20260714-233438: Switch the web build to `bevy/webgpu` and un-gate hanabi on
  wasm (add `webgpu` feature -> `bevy/webgpu`, wire trunk to build it, remove the
  three `#[cfg(not(target_family = "wasm"))]` hanabi gates, verify effects render in
  a WebGPU browser).
- tatr 20260714-233443: Add a `navigator.gpu` WebGPU-detection gate at the Play
  boundary (landing `web/` loader + the game `index.html`) so non-WebGPU browsers
  get a friendly "needs WebGPU" message instead of a dead canvas.

Ship the two together. Option C (a second WebGL2 fallback build behind the gate)
stays on the shelf as a future upgrade if analytics show meaningful bounce.

## Fix record

- 20260714: spike RECOMMENDED Option A + detection gate; seeded 20260714-233438
  (webgpu switch) and 20260714-233443 (detection gate). No code shipped yet.
