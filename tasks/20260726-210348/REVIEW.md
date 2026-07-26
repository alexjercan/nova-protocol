# Review: Hidden NOVA OS terminal easter egg in the web app

- TASK: 20260726-210348
- BRANCH: feature/nova-os-easter-egg

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

The change delivers the goal cleanly. All four DoD proofs were run by the
reviewer and pass:

1. `git ls-files web | grep -i nova_os_terminal_poc` -> empty. Source truth
   stays in `examples/`.
2. `npm run build` then `diff -q dist/nova-os/index.html
   ../examples/ui/nova_os_terminal_poc.html` -> identical.
3. `npm run ci` (format:check + lint + build) -> exit 0.
4. `npm test` fake-DOM wiring harness -> all assertions pass; it faithfully
   exercises the real `initEasterEgg`/`registerHit` (armed 5-click navigate,
   every click swallowed, sub-threshold no-op, basePath-aware route, off-page
   no listener, null-brand no-op). Each test would fail if threshold,
   rolling-window aging, basePath, or the arming guard were broken.

The CopyPlugin pattern and `historyApiFallback` rewrite match the existing
`tutorial`/`wiki` conventions; `.test-out/` is gitignored; DECISION.md records
the load-bearing trigger/route choice; the easter egg is intentionally
unlinked/undocumented, so the absence of a doc surface for `/nova-os/` is
correct.

- [x] R1.1 (MINOR) web/src/site.ts:112 - `initEasterEgg(brand, brand ?
  pathOf(brand) : "", current, root)` passed `brandPath` as the identical
  expression that already produced `root`, so `brandPath` and `root` were
  provably always equal and `pathOf(brand)` was computed twice. Drop the
  separate `brandPath` param and use `root` for the arming guard.
  - Response: Fixed. `initEasterEgg` is now `(brand, current, root)` with the
    guard `root !== current`, and the call site is `initEasterEgg(brand,
    current, root)`. This also exposed a latent flaw: the round-1 armed test
    passed an impossible `root=""`/`current="/home"` combination (unreachable
    now that the guard keys off `root`), so it was corrected to the real
    landing-page case (root === current === "" at local dev). Verified: `npm
    test` and `npm run ci` green.
- [x] R1.2 (NIT) web/src/site.ts:83 - `const base = root === "" ? "" : root;`
  is a no-op ternary. Use `root` directly.
  - Response: Fixed. The navigation is now
    `` window.location.href = `${root}/${EGG_ROUTE}/`; ``.
- [x] R1.3 (NIT) web/package.json - `web/tests/site.test.ts` was outside the
  `format:check`/`lint` globs, so CI never held it to house style. Widen the
  globs to include `tests/`.
  - Response: Fixed. Added `"tests/**/*.ts"` to the `format`, `format:check`,
    `lint`, and `lint:fix` globs and `"tests/**/*"` to `tsconfig.json` `include`
    so the type-checked lint resolves the file. Reworked `makeBrand` in the test
    to close over `brand` instead of `this` (a bare `this` is `any` under the
    type-checked rules). Verified: `npm run lint` exit 0.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (round-1 findings were all MINOR/NIT cleanups; this round
  only re-verifies the fixes against the new diff)

Re-verified after the round-1 fixes:

- `initEasterEgg` is now a 3-arg `(brand, current, root)` with guard
  `root !== current`; the redundant `pathOf(brand)` recompute and the no-op
  `base` ternary are gone.
- `npm test` -> all assertions pass (armed local-dev + basePath navigate,
  sub-threshold no-op, stale/slow-drip aging, off-page no-listener, null-brand
  no-op).
- `npm run ci` -> exit 0 (prettier check now covers `tests/`, eslint covers
  `tests/` with no errors, webpack build succeeds).
- DoD 1 and 2 re-confirmed: no second source copy tracked under `web/`;
  `dist/nova-os/index.html` byte-identical to the examples source.

No new findings. Pending user checks (manual DoD): clicking the brand 5x on the
served landing page opens `/nova-os/` and the brand still navigates home from
other pages - covered deterministically by the `npm test` wiring harness and
the headless screenshot of the rendered `/nova-os/` route, to confirm live at
your discretion.
