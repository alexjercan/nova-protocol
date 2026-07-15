# Bug: WASM portal catalog fetch fails cross-origin (CORS) - blocked when game and /mods served on different origins

- STATUS: OPEN
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

## Notes

- Relevant files: crates/nova_assets/src/portal.rs (fetch + error at ~1114),
  the `?portal=` config parse (PortalConfig), web/ trunk build + any local
  serve scripts, docs/ portal/dev docs (web/src/wiki/dev/mod-portal.md).
- Do NOT fix in the scenario-picker task (20260715-200828); this is its own
  branch. Filed mid-flow from that session.
- Discovered while implementing the scenario picker; native portal path is
  known-good, this is the wasm-only cross-origin gap the family close-outs
  flagged as the "manual browser session" verification.
