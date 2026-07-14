# Add navigator.gpu WebGPU-detection gate at the Play boundary

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.6.0,wasm

## Outcome

Two-layer WebGPU gate. Game page (authoritative): `build/web/webgpu-check.js`
inlined into `index.html`, runs synchronously before trunk's deferred wasm module
(verified in `dist/index.html`), and shows a "WebGPU required" panel instead of a
crashed canvas. Landing page (courtesy): `warnIfNoWebGpu` adds a note under the
Play CTA. A mid-task playtest (Firefox/Linux hit the raw surface-creation panic)
proved the need AND exposed that presence-only detection is insufficient, so the
gate also probes `requestAdapter()`. Verified: 5-case node test on the shipped gate
file (incl. the exact playtest case), `dist/index.html` ordering, `web` npm ci
green. Live in-browser eyeball deferred to a `preview-web.sh` pass (headless env).

## Playtest verdict (20260715)

User ran the current master build (233438 landed, this gate NOT yet deployed) in
Firefox on Linux and hit the raw panic
`Failed to create wgpu surface: FailedToCreateSurfaceForAnyBackend` - i.e. the
exact non-WebGPU-browser crash this task removes (Firefox on Linux does not ship
WebGPU in 2026). Confirms the reachability analysis in the spike and the need for
the gate. It also exposed that a presence-only `navigator.gpu` check is
insufficient: the crash is at surface creation, so a browser could expose the API
object yet still fail to get an adapter. Gate strengthened to also probe
`requestAdapter()` (see Steps 1-2 and the test's "present but no adapter" case).

## Goal

Once the web build moves to WebGPU (20260714-233438), browsers without WebGPU
(Firefox on Linux/Android/Intel-Mac, older OS/browser - ~15% as of 2026) must get
a friendly "needs WebGPU" message instead of a dead canvas: a bevy `webgpu` build
fails to initialize the renderer entirely on a non-WebGPU browser. Scoped from
`tasks/20260714-085955/SPIKE.md` (Option A).

## Steps

- [x] Add `build/web/webgpu-check.js`: inject a `.webgpu-fallback` panel into
  `.game-container` (replacing the spinner + `#bevy` canvas) - heading "WebGPU
  required", body naming supported browsers, and a `../` back link.
  CHANGED per the 20260715 playtest: presence-only was insufficient (the crash is
  at surface creation), so it now (1) checks `navigator.gpu` synchronously, then
  (2) probes `requestAdapter()` async and falls back if no adapter / it rejects.
- [x] Inline it into the game `index.html` via
  `<link data-trunk rel="inline" href="build/web/webgpu-check.js"/>`, placed AFTER
  `.game-container`. Verify-first RESOLVED: trunk's auto-init is a deferred
  `<script type="module">`; a plain inline `<script>` placed after the container
  runs synchronously first, so it rewrites the container before bevy boots (no
  black flash), and bevy's `#bevy` lookup then fails quietly. Confirmed in the
  generated `dist/index.html`: gate `<script>` ~line 170, trunk module ~line 238.
- [x] Add `.webgpu-fallback` styling to `build/web/styles.css`, centered like the
  `.lds-dual-ring` spinner.
- [x] Landing layer: `web/src/webgpu.ts` (`warnIfNoWebGpu`, called from
  `web/src/index.ts`) adds a `.hero__cta-note` under the Play CTA when WebGPU is
  absent. DEVIATION from "disabled state": the link stays clickable because the
  destination (game page) now explains the requirement itself; a hard-disable would
  be worse UX. Styled in `web/src/style.css`. No `index.html` change needed (the
  note targets the existing `.hero__cta`).
- [~] Verify at the real `/play/` subpath. DONE: `webgpu-check.test.mjs` runs the
  shipped gate file in a vm against the exact playtest case (present-but-no-adapter)
  + 4 others; `dist/index.html` ordering inspected; `web` npm ci (prettier+eslint+
  webpack) green; back link is `../` (resolves `/nova-protocol/play/` ->
  `/nova-protocol/`). NOT DONE (headless env): the live in-browser eyeball of the
  message (Firefox/Linux) and particles (Chrome) - left for a `scripts/preview-web.sh`
  pass. The playtester already confirmed the pre-gate crash this replaces.
- [x] Docs: `CHANGELOG.md` entry (gate line under [Unreleased]); `NOTES.md` with the
  two-layer approach, the sync-before-deferred ordering, and the adapter-probe
  rationale.

## Notes

- Relevant files: `build/web/webgpu-check.js` (new), `index.html` (game, repo
  root), `build/web/styles.css`, `web/src/index.html` + `web/src/index.ts`
  (landing CTA).
- `sound.js` and `styles.css` are already pulled into the game index via
  `<link data-trunk rel="inline">`; use the same mechanism for the check script.
- This gate is also the hook a future Option C (auto-serve a WebGL2 fallback
  build) would plug into; out of scope here.
- Depends on / pairs with 20260714-233438; ship together. In this flow the switch
  lands first, then this gate, so master ends consistent.
