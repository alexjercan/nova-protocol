# Enable particle effects on wasm: WebGPU web build vs shader-particle fallback

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,wasm,polish,spike

Supersedes 20260706-162908 (backlog "re-enable particle effects on wasm").

Goal: get particle effects (thruster plume, turret muzzle flash, torpedo
launch/detonation) working in the web build. They are currently cfg'd off on wasm:
`crates/nova_gameplay/src/plugin.rs:51` (`#[cfg(not(target_family = "wasm"))]` on
`HanabiPlugin`), plus `sections/turret_section.rs:279-284` and
`sections/torpedo_section/mod.rs:294-303`.

The catch (researched 20260714): `bevy_hanabi` needs compute shaders, which on wasm
means the **WebGPU** backend only - it does NOT work under WebGL2. The web build
currently ships WebGL2 (v0.5.1 fixed a WebGL2 view-format crash). So this is a real
decision, hence a spike:

- Option A: switch the web build to `bevy/webgpu`. Pros: real hanabi particles on
  web. Cons: drops browsers without WebGPU (Firefox default-off, older
  Chrome/Safari), i.e. a reachability regression for the landing page's "Play"
  link. Verify current browser support before committing.
- Option B: keep WebGL2 and ship a lightweight wasm-only particle fallback (billboard
  quads / the existing shader effects, no compute), so every browser keeps working
  and native still uses hanabi.
- Option C: WebGPU with a WebGL2 fallback path (two builds / feature detection).

Decide A/B/C, then wire it. Also note: hanabi's `serde` feature is wasm-incompatible
(typetag), but nova does not serialize effects, so the RON modding work is unaffected.
