# Review: WASM portal cross-origin/CORS dev fix

- TASK: 20260715-214540
- BRANCH: fix/portal-cors-dev

## Round 1

- VERDICT: APPROVE

A small, production-safe change: it resolves the reported failure as a
dev-tooling + docs gap (not a production client bug) and adds a proactive
cross-origin warning. Reviewed the diff against master; re-verified the two
load-bearing claims independently rather than trusting the summary.

- **Production is unaffected, verified two ways.** (1) `deploy-page.yaml` runs
  `trunk build --release` (line 67), never `trunk serve`; a Trunk `[[proxy]]`
  is a serve-only feature with no request-serving phase during a build, so the
  new proxy is inert in the deploy - "zero deploy impact" holds by construction,
  not by assertion. (2) The deploy assembles `/play/` and `/mods/` as siblings
  on one Pages origin, and the wasm derivation resolves same-origin - already
  pinned by `portal_base_derives_from_the_page_location`, unchanged here.
- **The wasm `from_environment` edit is correct** (re-read, since regular CI does
  not compile wasm): `window` stays an owned `Option` (borrowed for base_url,
  then re-borrowed for the warning - never moved); `location().origin()`
  (Result) and `url_origin` (Option) match in one tuple `if let`; the compare is
  `base_origin != page_origin`; `warn!` is in the bevy prelude; `Location::origin`
  is covered by the already-enabled `web-sys` `Location` feature.
- **The new test is non-vacuous.** `url_origin_extracts_scheme_host_port`
  asserts same-host/different-port is cross-origin (the exact reported :8090 vs
  :8000 case), same-origin/different-path is equal, and a relative base yields
  `None` (never flagged) - it would fail if `url_origin` dropped the port or
  mishandled the path split.
- **Docs + CHANGELOG** accurately teach the same-origin web path and the CORS
  reason, replacing the cross-origin `?portal=` instruction that caused the
  report; the native `NOVA_PORTAL_URL` path is retained.
- Full check suite run: `cargo test -p nova_assets --lib portal` 12/12 (incl.
  the new pin), `cargo fmt --all --check` clean, `cargo check --workspace`
  clean.

No BLOCKER/MAJOR/MINOR findings.

- NIT (accepted, not blocking): the trunk `[[proxy]]` and the wasm warning are
  browser-runtime behaviors that only a manual `trunk serve` + browser session
  can SEE end to end. The proxy is standard/documented and the warning logic is
  native-pinned, so this is the same known wasm-runtime verification gap the
  mod-portal family already carries - recorded in the close-out, not a blocker.
