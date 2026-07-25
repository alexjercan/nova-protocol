# Retrospective

## What went well

- The user correction to use `icon: Option<AssetRef<Image>>` simplified the
  schema: the final design uses the same asset-reference path model mods
  already understand, and omitted icons remain backwards compatible.
- Fail-first tests covered the important boundaries before implementation:
  HUD stack behavior, scenario RON, sync into `StoryFeed`, and mod resource
  rewriting/validation.
- The out-of-context review caught a real player-facing consistency miss in
  the in-game keybind reference after the website docs were updated.

## What went wrong

- I initially used a broad stale-doc grep that matched valid authoring prose
  like "one line per beat." That made the DoD command noisy until it was
  narrowed to stale behavior claims.
- The first scenario verification command tried to pass two Cargo test filters
  in one invocation. Cargo accepts one positional filter, so those checks had
  to be rerun separately.
- The first mod-ref fixture shape was not production-shaped enough: it treated
  `Scenario(...)` as a bare `ScenarioConfig` instead of parsing the outer
  `Content` enum.
- I marked the task `CLOSED` before `REVIEW.md` and `RETRO.md` existed, and
  `tatr check` correctly rejected that state.

## Improve next time

- Keep DoD grep commands focused on stale claims, not generic words that are
  still legitimate in surrounding guidance.
- For schema/resource tests, start fixtures at the same enum boundary the
  production loader sees, even when the assertion only needs an inner config.
- Before closing a tatr task in `/flow`, write `REVIEW.md` and `RETRO.md`
  first, then run `tatr check --ledger LESSONS.md`.

## Candidate lessons

- A control documented on the website may also need the static in-game
  reference table. Search both docs and settings/reference surfaces when adding
  new keypresses.
