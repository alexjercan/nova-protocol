# Retro: Hidden NOVA OS terminal easter egg in the web app

- TASK: 20260726-210348
- BRANCH: feature/nova-os-easter-egg
- REVIEW ROUNDS: 2 (round 1 out-of-context APPROVE, round 2 in-session)

## What went well

- The one genuinely underspecified point - "open it from a secret location
  somehow" - was taken to the user before building (AskUserQuestion), and the
  chosen trigger/route recorded in DECISION.md. No guessing a mechanism and
  building the wrong shape.
- Source truth stayed in `examples/` by copying at build time (CopyPlugin),
  matching the existing `tutorial`/`wiki` route conventions rather than
  inventing a new pattern. The PoC being fully self-contained was verified
  first (no external/relative refs), so it needed zero changes to survive the
  `/nova-protocol/` deploy subpath.
- Applied the `ci-skips-client-render` ledger lesson proactively: the click
  logic was factored into a pure exported `registerHit` plus an exported
  `initEasterEgg` driven by a fake-DOM node harness, so the DOM wiring got a
  real runtime check even though CI is build-only.

## What went wrong

- R1.1: `initEasterEgg` was given a separate `brandPath` param that was the
  identical expression already producing `root`, so the two were provably
  always equal. Root cause: I threaded the arming path in as its own argument
  instead of recognizing that the brand path IS the site root that `initSite`
  already computed. It seemed right because "the path the brand points at" and
  "the site root" felt like distinct concepts, but in this codebase they are
  the same value.
- That redundancy was not just cosmetic: it let the wiring test assert an
  UNREACHABLE state - the armed case passed `root=""` with `current="/home"`,
  a combination that cannot occur once the guard keys off `root`. Collapsing
  the param to `root` broke that test (correctly), and it had to be rewritten
  to the real landing-page case (root === current). A redundant param had let a
  green test encode an impossible input.

## What to improve next time

- When a new function needs a value another already-computed local provides,
  pass that local - do not re-derive or re-parameterize it. Redundant params
  are not free: they widen the input space to include unreachable combinations
  that tests can then "pass" against, hiding the real contract.

## Action items

- [x] Ledger: `redundant-param-enables-impossible-test` (new).
- [x] Ledger: `web-tests-need-node-from-flake` (new, reference/tooling).
- No follow-up code tasks; the easter egg is complete and the ledger captures
  the process lessons.
