# Retro: runtime content gate + FAILED TO START overlay

- TASK: 20260716-193949
- BRANCH: feature/runtime-content-gate (landed 6a4ff060)
- REVIEW ROUNDS: 1

## What went well

- The shared-core design paid immediately: the runtime half was mostly
  PLUMBING (resource, refusal, overlay) because every check already
  existed in nova_scenario::lint - one implementation, now three live
  consumers, byte-identical semantics across CLI, CI and runtime.
- Working through "what breaks when this fires in each state" up front
  (the mod-facing failure-path lesson from 155849) caught the
  menu-camera hazard at DESIGN time: a refused backdrop load would have
  bricked the menu, so the draw filters instead of refusing.
- Every link of the pin chain has its own test, and the review
  sabotaged the central one (the refusal return) to prove it can fail.

## What went wrong

- Two known lesson classes recurred in miniature, both caught in-cycle:
  the OnEnter clear panicked menu-only rigs until the consumer plugin
  also inited the resource (resource-guard class - predicted, still
  cost one red suite run), and a text-anchored test insert stole the
  neighboring test's #[test] attribute, silently deregistering it
  (anchor-scope class, new variant: the ATTRIBUTE LINE above the anchor
  travels with the anchor). The duplicate-looking filtered output was
  the tell.

## What to improve next time

- Anchoring an insert on a `fn` line is wrong by default in Rust test
  modules: anchor on the attribute/doc block START, or insert after a
  closing brace.
- When one plugin writes a resource another consumes, init it in BOTH
  plugins as the default recipe (idempotent).

## Action items

- [x] Ledger: anchor-edits bumped with the attribute variant; two-plugin
      init noted on the messagereader-resource-guard entry.
