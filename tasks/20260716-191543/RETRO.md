# Retro: content lint core + bin + CI gate

- TASK: 20260716-191543
- BRANCH: feature/content-lint (landed 00698783)
- REVIEW ROUNDS: 2

## What went well

- Running the linter on the REAL tree during development surfaced the
  over-strict duplicate check immediately (the ch4 choice fork) and
  drove the Error/Warn split - a rule calibrated on real corpus before
  it gates anything.
- The gate A/B literally replayed the incident that motivated the task
  (unknown prototype) and CI failed with ship/section/prototype named.

## What went wrong

- The first cut only checked prototypes on DIRECT spawns; the review
  pass asking "where else can a ship config hide?" found the
  ScatterObjects template hole - the same bug one wrapper deeper. Root
  cause: checks were written per action VARIANT instead of per embedded
  TYPE (ScenarioObjectConfig appears in two places).

## What to improve next time

- When linting a config tree, enumerate every PATH to the target type
  (grep the type name across the config structs) before writing checks,
  not the action variants you remember.

## Action items

- [x] Ledger: new lint-covers-types-not-variants (x1).
