# Add navigator.gpu WebGPU-detection gate at the Play boundary

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,wasm

Spike: tasks/20260714-085955/SPIKE.md (Option A chosen)

Goal: when the web build moves to WebGPU (20260714-233438), browsers without
WebGPU (Firefox on Linux/Android/Intel-Mac, older OS/browser - ~15% as of
2026) must get a friendly "this build needs WebGPU" message instead of a dead
canvas. A bevy `webgpu` build does not degrade gracefully - it fails to
initialize the renderer entirely on a non-WebGPU browser, so without a gate those
users see a black screen / panic.

Direction (leave the Steps for /plan):
- Feature-detect `navigator.gpu` (WebGPU availability) before the game canvas
  boots.
- On absence, show a clear fallback: "Nova Protocol's web build needs WebGPU - try
  Chrome/Edge, or Firefox on Windows" (and keep the rest of the landing site
  reachable), instead of loading the wasm.
- Wire it at both entry points as needed: the landing `web/` Play loader and the
  game's own `index.html` (`link data-trunk` / canvas boot).

Pairs with 20260714-233438 - ship together. This gate is also the natural hook for
a future Option C (auto-serve a WebGL2 fallback build to non-WebGPU browsers) if
analytics later show meaningful bounce; out of scope here.
