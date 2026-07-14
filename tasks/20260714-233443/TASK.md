# Add navigator.gpu WebGPU-detection gate at the Play boundary

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,wasm

## Goal

Once the web build moves to WebGPU (20260714-233438), browsers without WebGPU
(Firefox on Linux/Android/Intel-Mac, older OS/browser - ~15% as of 2026) must get
a friendly "needs WebGPU" message instead of a dead canvas: a bevy `webgpu` build
fails to initialize the renderer entirely on a non-WebGPU browser. Scoped from
`tasks/20260714-085955/SPIKE.md` (Option A).

## Steps

- [ ] Add `build/web/webgpu-check.js`: on load, if `!navigator.gpu`, hide the
  loading spinner and the `#bevy` canvas and inject a `.webgpu-fallback` panel
  into `.game-container` - heading "WebGPU required", body naming supported
  browsers (Chrome/Edge, Safari on macOS/iOS 26, Firefox on Windows), and a link
  back to the landing site. (`navigator.gpu` presence is the standard, cheap
  WebGPU check; requesting an adapter is async and unnecessary to gate.)
- [ ] Inline it into the game `index.html` (repo root) via
  `<link data-trunk rel="inline" href="build/web/webgpu-check.js"/>`, mirroring the
  existing `sound.js` inline. Verify-first: confirm whether trunk's auto-init of
  the wasm can be cleanly prevented before it runs; if not, ensure that removing
  the `#bevy` canvas + spinner makes the failed bevy init invisible (bevy resolves
  its target by the `#bevy` selector, so a missing canvas fails early and quietly)
  - the user must see the fallback, not a black flash then an error.
- [ ] Add `.webgpu-fallback` styling to `build/web/styles.css`, centered like the
  existing `.lds-dual-ring` spinner.
- [ ] Landing layer: gate the "Play in browser" CTA so users are warned before
  navigating. In the landing entry (`web/src/index.ts` + the hero CTA in
  `web/src/index.html`), when `navigator.gpu` is absent render Play in a
  disabled/explained state ("Needs WebGPU") rather than linking straight into the
  game. The game-index gate (steps 1-2) stays the authoritative safety net for
  direct `/play/` navigation (bookmarks, shared links) that skips the landing.
- [ ] Verify at the real `/play/` subpath via `scripts/preview-web.sh` (Pages
  serves the game at `/nova-protocol/play/`, landing at `/nova-protocol/` -
  verify-at-deploy-base-path, and confirm the back-link target resolves there):
  (a) in a WebGPU browser, Play works and no fallback appears; (b) with WebGPU
  absent (a browser/flag without `navigator.gpu`, or temporarily stub it), the
  game index shows the fallback panel, not a black canvas. This is client-rendered
  JS, so it needs an actual DOM/eyeball check - a green build proves nothing
  (ci-skips-client-render).
- [ ] Docs: `CHANGELOG.md` entry; `tasks/20260714-233443/NOTES.md` recording the
  two-layer approach (landing warning + game-index safety net) and why the
  direct-navigation case forces the game-index gate.

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
