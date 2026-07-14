# Review: Switch web build to bevy/webgpu and un-gate hanabi on wasm

- TASK: 20260714-233438
- BRANCH: feat/wasm-webgpu-particles

## Round 1

- VERDICT: APPROVE

Independently verified the load-bearing claim rather than trusting the diff:
`cargo tree --target wasm32-unknown-unknown` shows bevy with both `webgl2` and
`webgpu` on wasm (webgpu overrides, per bevy docs), and the native target shows
`webgl2` but NOT `webgpu` - so the target-scoped feature is correctly wasm-only and
native is untouched. `trunk build` compiled `bevy_hanabi` and the un-gated
`nova_gameplay` observers for wasm32 (`✅ success`). Swept examples/ and tests/:
nothing references the old gate or asserts observer-absence on wasm; CI is
native-only, so the trunk build was the right (and only) wasm compile gate. The
diff delivers the Goal; the FIXME gates are gone and the stale "wasm-blocked" prose
was swept (juice.rs, torpedo render.rs, docs/architecture.md). Clean, small change.

- [ ] R1.1 (MINOR) tasks/20260714-233438/NOTES.md - the runtime visual (particles
  actually rendering in a live WebGPU browser) was not eyeballed here (headless
  env), only compile + backend-wiring were. This is honestly disclosed and deferred
  to the paired gate task's `scripts/preview-web.sh` run. To make sure the deferral
  is not silently dropped, strengthen 20260714-233443's verify step to explicitly
  require confirming muzzle-flash and torpedo particles render in the WebGPU-browser
  preview, not just that "Play works".
  - Response: Acknowledged, left open (MINOR). Action lives on task 20260714-233443,
    not this branch; its verify step will be strengthened to require confirming
    particles render in the WebGPU-browser preview when that task is worked next in
    this flow. Recorded here so the deferral is not lost.
- [x] R1.2 (NIT) crates/nova_gameplay/src/juice.rs:20 - the comment reflow left this
  line at 101 chars, inconsistent with the file's ~80-col doc wrapping (rustfmt does
  not wrap `//!`). Re-wrap.
  - Response: Fixed - re-wrapped juice.rs:19-23 to ~80 cols. Verified: longest line
    now 78 chars.
