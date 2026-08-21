# Preload the next scenario behind the current one

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.11.0,performance,scenario,menu,wontdo

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

## WONTDO 2026-08-21 - owner call, and the evidence behind it

Owner, 2026-08-21: scenario load time is not bad enough for this to be relevant.

The measured picture agrees, and it is worth recording so a future reader does not
re-file this from first principles:

- **The load path it was meant to hide is already fixed.** `state_to_world` chunks
  the spawn queue under a 3 ms budget, the scenario script is gated on
  `scenario_has_settled`, and the loading panel is held by that gate with a 0.6 s
  floor and a 50 ms settled test (`20260816-122158`, `20260816-112353`, both
  closed). There is no frozen frame left for a preload to cover.
- **Its stated dependency is answered, and the answer removes the case.** This task
  depends on `PERF-BAKE` settling what load-time work is. `20260818-221040` is now
  re-scoped to a single unmeasured item, so the answer is "almost none".
- **Its own payoff was never measured.** "Selecting a preloaded entry is measurably
  faster than a cold one" needs a cold one that is slow, and nobody has shown one.
  `wfc_arena` reaches `Playing` about 1.1 s from its first log line.
- **It asks for cancellable speculative work** whose named failure mode is loading
  scenarios nobody opens and spending memory and cores on them. That is the
  speculative machinery the epic's own rules forbid.

Reopen if a scenario transition is ever MEASURED as slow enough to notice. The
carousel argument - that it knows what is next - stays true and stays the right
first target if that day comes.
