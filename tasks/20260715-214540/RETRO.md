# Retro: WASM portal cross-origin/CORS dev fix

- TASK: 20260715-214540
- BRANCH: fix/portal-cors-dev (landed on master as 4f6e1e2c)
- REVIEW ROUNDS: 1 (APPROVE, no findings above NIT)

## What went well

- Diagnostic-first + verify-at-deploy dissolved the "bug" before any code was
  written for the wrong reason. Reading `deploy-page.yaml` (the game and portal
  land as siblings on ONE Pages origin) and the EXISTING
  `portal_base_derives_from_the_page_location` pin proved production is already
  same-origin. A suspected client fetch bug turned into a docs+tooling fix that
  cannot regress production.
- The cycle ended honestly in a falsification-plus-fix, not a forced code change
  to the failing symptom. The residual real problem (local dev friction) got the
  fix; the imagined problem (production CORS) got a documented "does not exist"
  with the pin that proves it.
- Re-verified the one load-bearing NEW claim by construction rather than
  assertion: "a Trunk `[[proxy]]` cannot affect the deploy" because the deploy
  runs `trunk build` (no serving phase), which a serve-only proxy cannot touch.
  That is airtight in a way "I think proxies are serve-only" is not.
- Chose the reliable lever over the tempting-but-wrong one: a CORS failure is an
  opaque `TypeError` in JS, so a clearer POST-failure error would have to guess.
  Warning on the cross-origin CONFIG (two known origins compared up front) is
  deterministic - and testable natively via a pure `url_origin` helper.

## What went wrong

- The failure was self-inflicted by our own docs: mod-portal.md's "Local
  development" told web testers to use a cross-origin `?portal=` override, which
  the browser blocks. Root cause: the dev instructions crossed a browser
  security boundary (origin/CORS) without flagging it, so they manufactured the
  exact failure the user reported. A doc that steers a reader across a security
  boundary is a latent bug report generator.
- Toolchain friction: this session's shell had no `cargo` on PATH (the nix
  devshell that the prior session ran inside was not active). Cost a couple of
  empty-output runs before diagnosing it and wrapping every cargo call in
  `nix develop /path --command bash -c '...'`. Not a code problem, but it slowed
  verification.

## What to improve next time

- When writing dev-setup docs (ports, hosts, origins, auth), name any
  browser/security boundary the setup crosses and give the same-origin/safe path
  as the default - or the doc will generate the failure it forgot to mention.
- At session start in this repo, confirm `cargo` is on PATH; if not, prefix
  cargo/rustc with `nix develop <repo> --command ...` instead of assuming the
  devshell is active.

## Action items

- [x] LESSONS.md: new `dev-doc-steers-across-boundary`; bumped
  `verify-at-deploy-base-path` (x2), `diagnostic-first` (x10),
  `null-result-becomes-a-pin` (x2); new `nix-devshell-for-cargo` (toolchain).
- [ ] Manual browser session (unchanged wasm-runtime gap): `trunk serve` with
  the new proxy + the portal server, open Explore with no `?portal=`, confirm
  same-origin fetch works and the cross-origin warning fires on a `?portal=` to
  another port. Belongs to the same manual pass the mod-portal family already
  owes; not a code follow-up.
