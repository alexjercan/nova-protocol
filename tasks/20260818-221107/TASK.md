# Preload the next scenario behind the current one

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.11.0,performance,scenario,menu

Epic: `20260818-220812`. Owner: "batch loading a scenario: so maybe we can load
them in background: e.g we load in background the next scenario beforehand so
it's faster to actually load -> I think this one might be more useful for the
main menu carousel".

The carousel is the right first target: it KNOWS what is next. While the player
reads the current entry, the next one can be built behind it, so selecting it
costs nothing.

## Scope

- Main menu carousel: preload the neighbouring entries.
- Campaign chain: the next scenario in a sequence is knowable at the moment the
  current one starts.
- Do NOT build a general speculative prefetch. Preload only what something
  actually declares as next - the failure mode of this feature is loading five
  scenarios nobody opens and spending the memory and the cores on it.

## Constraints

- A preloaded scenario must be discardable. The player can always pick a
  different entry, and the work has to be cancellable without leaking entities
  or assets.
- Preload must not steal from the running frame. This rides on
  `PERF-OFFLOAD` - it is the same worker budget, and a menu that stutters while
  preloading is worse than a menu that loads on select.
- Scenario teardown already has history here (`20260525-132939`,
  `20260706-212910`). Verify a preloaded-then-discarded scenario leaves nothing
  behind, with a test, not by eye.

## Done when

- Selecting a preloaded carousel entry is measurably faster than a cold one.
- The menu holds frame rate while preloading, measured.
- Discarding a preload leaks nothing.

## Depends on

`PERF-OFFLOAD`, and on `PERF-BAKE` having settled what load-time work is.
