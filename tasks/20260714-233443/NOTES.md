# NOTES: WebGPU-detection gate at the Play boundary

## Why

The web build now ships bevy's `webgpu` backend (task 20260714-233438). A `webgpu`
build cannot initialize its renderer on a browser without working WebGPU - it
panics at surface creation (`Failed to create wgpu surface:
FailedToCreateSurfaceForAnyBackend`) and leaves a dead black canvas. A real
playtest confirmed this: Firefox on Linux (which does not ship WebGPU in 2026) hit
exactly that panic on the pre-gate master build. This task replaces the crash with
a friendly message.

## Two layers

1. **Game page (authoritative safety net):** `build/web/webgpu-check.js`, inlined
   into the game `index.html` via `<link data-trunk rel="inline">`. This is the one
   that matters, because it also covers people who deep-link straight to `/play/`
   (bookmarks, shared links) and never see the landing page.
2. **Landing page (courtesy warning):** `web/src/webgpu.ts` (`warnIfNoWebGpu`,
   called from `web/src/index.ts`) adds a note under the "Play in browser" CTA when
   WebGPU is absent. The link stays clickable - the destination now explains the
   requirement itself, so there is no need to hard-disable it. By design this layer
   is **presence-only** (`"gpu" in navigator`): it does not run the async
   `requestAdapter()` probe, so a browser that exposes the API but cannot get an
   adapter gets no landing note but still hits the game page's authoritative gate.
   Keeping the courtesy layer synchronous and simple is the deliberate trade.

## How the game-page gate runs before bevy (the load-bearing detail)

trunk emits its wasm bootstrap as `<script type="module">`, which is **deferred**
(runs after the HTML is parsed). `webgpu-check.js` is inlined as a **plain**
`<script>` placed after `.game-container` in the body, so it runs **synchronously
during parsing, before** the deferred module. Verified in the generated
`dist/index.html`: the gate `<script>` is at line ~170, trunk's
`<script type="module">` with `await init(...)` at ~238. So when WebGPU is missing
the gate rewrites the container (removing the `#bevy` canvas that WindowPlugin
binds to) before bevy ever boots - no black flash, and bevy's own init then fails
quietly finding no canvas.

## Presence is not enough - probe the adapter

The first cut only checked `navigator.gpu` presence. The playtest crash is at
**surface creation**, which means a browser can expose `navigator.gpu` yet still
fail to get an adapter (flag half-enabled, unsupported GPU/driver, Firefox-on-Linux
with the pref flipped but no backend). So the gate does two checks:

1. Synchronous: `navigator.gpu` absent -> show the message immediately (default
   Firefox/Linux, older browsers).
2. Asynchronous: `navigator.gpu` present -> `requestAdapter()`; if it yields no
   adapter (or rejects), show the message. This races bevy's own init; whichever
   loses, the user still ends on the message rather than a crashed canvas. On a
   working WebGPU browser the adapter resolves and nothing is shown.

## Verification

- `build/web/webgpu-check.test.mjs` (node --test): 5 cases run the ACTUAL shipped
  file in a vm with a stubbed DOM - absent-gpu, working-adapter (no fallback),
  present-but-no-adapter (the Firefox/Linux case), requestAdapter-rejects, and
  no-container. The "no adapter" case fails if the async probe is removed, so it
  pins the exact bug the playtest exposed.
- `dist/index.html` inspected for the sync-before-deferred ordering (above).
- Landing: `cd web && npm run ci` (prettier + eslint + webpack build) passes with
  `webgpu.ts` wired into `index.ts`.

NOT eyeballed here (headless env): the message actually appearing in a live
non-WebGPU browser, and particles rendering in a live WebGPU browser. The gate
logic is unit-pinned against the real failure case and the wiring is verified in
the built artifact; a live eyeball via `scripts/preview-web.sh` (open `/play/` in
Chrome to see particles, in Firefox/Linux to see the message) is the remaining
manual confirmation. The playtester already confirmed the pre-gate crash.

## Limitation / future

The async probe cannot *prevent* bevy from starting in the present-but-broken case
(it races); it reliably ends on the message but a bevy panic may still be logged to
the console. Fully preventing init would need gating trunk's auto-init, which trunk
does not cleanly support. Acceptable: the user sees the message, not a crash. This
gate is also the hook a future Option C (auto-serve a WebGL2 fallback build) would
plug into.
