# Bug: WASM portal catalog fetch fails cross-origin (CORS) - blocked when game and /mods served on different origins

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: bug,wasm,portal

## Report (user, 20260715)

Tested the WASM build locally with the game served on one origin and the mod
portal on another:

- Game (trunk serve): `http://localhost:8090/play/?portal=http://localhost:8000/mods`
- Portal (static file server): `http://localhost:8000/mods/catalog.json`

The Explore tab's catalog fetch fails. Browser console:

```
Access to fetch at 'http://localhost:8000/mods/catalog.json' from origin
'http://localhost:8090' has been blocked by CORS policy: No
'Access-Control-Allow-Origin' header is present on the requested resource.
GET http://localhost:8000/mods/catalog.json net::ERR_FAILED 200 (OK)
TypeError: Failed to fetch (ehttp::web::fetch)
WARN crates/nova_assets/src/portal.rs:1114 portal: portal catalog fetch failed:
Failed to fetch, check the developer console for details
```

The NATIVE build works: it downloads the mod into
`~/.local/share/nova-protocol` fine (ureq/rustls does not enforce CORS).

(The `AudioContext was not allowed to start` warnings in the same log are
benign - Chrome autoplay policy, resolves after a user gesture. Not this bug.)

## Diagnosis (initial, verify in /work)

- Root cause is a browser same-origin/CORS boundary: the WASM `ehttp` fetch is
  a cross-origin GET (`:8090` -> `:8000`) and the static file server serving
  `/mods` sends no `Access-Control-Allow-Origin`, so the browser blocks the
  response (200 on the wire, ERR_FAILED to JS). ureq on native has no such
  rule, which is why native works.
- In the INTENDED production topology the portal is served by the web app under
  the SAME origin as the game (`/mods/*` on the app's own host - see the spike
  tasks/20260714-202515/SPIKE.md), so a real deploy is same-origin and CORS
  never applies. The failure is specific to the split-port LOCAL test setup.
- So this is likely a local-dev / documentation gap rather than a client bug,
  but confirm: (1) does the deployed web app actually serve `/mods` same-origin
  (check the web/ build + hosting config), and (2) should the portal client
  degrade more gracefully / surface a clearer "cross-origin, check CORS" hint
  than the generic "Failed to fetch"?

## Goal

Make the WASM Explore flow work in local testing and be correct in production.
Decide and implement the right fix among:

- Document + support a same-origin local dev path (serve the game and `/mods`
  from one origin, e.g. copy the generated portal into the trunk `dist/mods`
  or reverse-proxy `/mods`), so the `?portal=` default is same-origin.
- And/or add permissive CORS headers to the local portal server helper (dev
  only) with a note that production is same-origin.
- Confirm the production deploy serves `/mods` same-origin (verify-at-deploy
  path lesson) so this never bites a real user.
- Optionally: clearer client-side error copy distinguishing a CORS/cross-origin
  failure from a genuine network error.

## Findings (verified in code, 20260715)

This is NOT a production client bug. Two confirmations resolve the fork:

1. **Production is same-origin, already.** `deploy-page.yaml` assembles one
   GitHub Pages origin: the trunk game under `/play/` (`cp -r dist/. site/play/`)
   and the `nova_portal_gen` output under `/mods/` (`--out site/mods`) as
   SIBLINGS. On wasm, `PortalConfig::from_environment` derives the portal base
   from `window.location` (`portal_base_from_href`), stepping out of `/play` and
   appending `/mods` -> same origin. This is ALREADY pinned by the unit test
   `portal_base_derives_from_the_page_location` (asserts `/play/` -> sibling
   `/mods`, and a trunk-root `localhost:8080/` -> `localhost:8080/mods`). So a
   real deploy never hits CORS, and neither does a default local `trunk serve`.
2. **The reported failure was the doc steering the user cross-origin.**
   mod-portal.md's "Local development" tells web testers to open the game with
   `?portal=http://localhost:8000/mods` - an EXPLICIT cross-origin override (page
   at :8090/:8080, portal at :8000). The browser blocks that cross-origin GET
   because the static file server sends no `Access-Control-Allow-Origin`. Native
   (ureq) ignores CORS, which is why native worked.

CORS itself is undetectable from JS (the browser hands `ehttp` an opaque
`TypeError: Failed to fetch`, indistinguishable from DNS/refused), so the client
cannot say "this is CORS" after the fact. But it CAN detect a cross-origin
CONFIG up front (compare the resolved base_url origin to the page origin) and
warn before the opaque failure.

## Plan / Steps

- [x] **Same-origin local dev path (Trunk.toml `[[proxy]]`).** Add a dev-only
  proxy so `trunk serve` serves `/mods` same-origin by forwarding to the local
  portal server (`backend = "http://localhost:8000/mods"`). Trunk proxies apply
  to `trunk serve` only, never `trunk build --release`, so zero production/deploy
  impact. The web build then fetches `localhost:8080/mods/catalog.json`
  same-origin with NO `?portal=` override and no CORS.
- [x] **Proactive cross-origin warning (wasm client).** In
  `PortalConfig::from_environment` (wasm branch), after resolving `base_url`,
  `warn!` once if its origin differs from `window.location.origin` - naming both
  origins and that the portal must send CORS headers or be served same-origin.
  Extract a pure `url_origin(&str) -> Option<String>` helper (scheme://host:port)
  and unit-test it natively (the cfg-independent test-pin pattern the other
  derivation fns use). Turns the opaque "Failed to fetch" into an actionable
  heads-up whenever someone points `?portal=` cross-origin.
- [x] **Rewrite mod-portal.md "Local development".** Teach the same-origin path
  for the WEB build (run portal gen + local server, `trunk serve` with the
  proxy, open the game with NO `?portal=`), keep the native `NOVA_PORTAL_URL`
  path (native has no CORS), and explain the same-origin/CORS reason so nobody
  gets steered cross-origin again. Note `?portal=` cross-origin only works if the
  portal server sends CORS headers.
- [x] **CHANGELOG** (Fixed/Changed): the local-dev web portal path is now
  same-origin via the trunk proxy; a cross-origin portal config warns clearly.
- [x] **Close-out + regression pin.** The production-same-origin invariant is
  already pinned by `portal_base_derives_from_the_page_location`; the new
  `url_origin` test pins the cross-origin detector. Evidence rig in the close-out.

## Close-out

Outcome: this was largely a FALSIFICATION plus a real dev-tooling/docs fix - the
reported failure is not a production client bug. No production code path was
broken; the deploy already serves the portal same-origin.

- What changed:
  - `Trunk.toml`: a dev-only `[[proxy]]` forwarding `/mods` to the local portal
    server, so `trunk serve` serves the portal SAME-ORIGIN (mirroring the
    production `/play/`+`/mods/` sibling layout). Proxies apply to `trunk serve`
    only, never `trunk build`, so zero deploy impact.
  - `crates/nova_assets/src/portal.rs`: on wasm, `PortalConfig::from_environment`
    now `warn!`s (naming both origins) when the resolved portal base is
    cross-origin to the page - the reliable up-front signal the opaque
    post-failure `TypeError: Failed to fetch` cannot give. Backed by a pure
    `url_origin` helper with a native unit test.
  - `web/src/wiki/dev/mod-portal.md`: the "Local development" section now teaches
    the same-origin web path (proxy, no `?portal=`) and explains the CORS reason,
    instead of steering testers to the cross-origin `?portal=` that caused the
    report. Native `NOVA_PORTAL_URL` path kept.
  - CHANGELOG Fixed entry.
- Why not a client fetch/error-copy change: a CORS failure is an opaque
  `TypeError` in JS, indistinguishable from DNS/refused, so the post-failure
  message cannot honestly name CORS. The proactive cross-origin-config warning is
  the reliable lever; the existing "check the developer console" pointer (where
  the browser prints the real CORS error) stays.
- Evidence rig (production is same-origin, so no user ever hits this):
  - `.github/workflows/deploy-page.yaml` assembles ONE Pages origin - trunk game
    `cp -r dist/. site/play/` and `nova_portal_gen --out site/mods` as siblings.
  - `portal_base_from_href` steps out of `/play` and appends `/mods` -> same
    origin. PINNED by `portal_base_derives_from_the_page_location` (asserts
    `/play/` -> sibling `/mods` AND trunk-root `localhost:8080/` -> same-origin
    `/mods`).
  - New `url_origin_extracts_scheme_host_port` pins the cross-origin detector
    (same host + different port IS cross-origin - exactly the reported :8090 vs
    :8000 case; a relative base is never flagged).
- Verification: `cargo test -p nova_assets --lib portal` 12/12 (incl. the new
  pin); fmt + `cargo check --workspace` clean. The wasm branch is cfg-gated and
  NOT compiled by regular CI (only the deploy `trunk build`), so the
  `from_environment` edit is guarded by static review + the native `url_origin`
  pin; `web-sys`'s `Location` feature (already enabled) provides `origin()`.
- Remaining manual gap (unchanged by this task, honest): actually running the
  wasm build in a browser to SEE the same-origin proxy serve Explore. The
  trunk proxy is standard/documented but browser-verified only by a manual
  session - the same wasm-runtime gap the mod-portal family close-outs flagged.
- Reflection: the diagnostic-first + verify-at-deploy discipline paid off - the
  "bug" dissolved once the deploy topology and the existing derivation pin were
  read, turning a suspected client fix into a docs+tooling fix that can't regress
  production. The trap was self-inflicted: the doc's own dev instructions steered
  cross-origin.

## Notes

- Relevant files: crates/nova_assets/src/portal.rs (PortalConfig::from_environment
  wasm branch ~97, portal_base_from_href ~136, tests ~1285), Trunk.toml,
  web/src/wiki/dev/mod-portal.md ("Local development" ~106), .github/workflows/
  deploy-page.yaml (topology evidence, read-only).
- No client fetch/error-copy change: the post-failure error stays "check the
  developer console" (which shows the real CORS message) because JS cannot
  distinguish CORS from other network failures. The proactive config warning is
  the reliable lever instead.
- Discovered while implementing the scenario picker; native portal path is
  known-good, this is the wasm-only cross-origin gap the family close-outs
  flagged as the "manual browser session" verification.
