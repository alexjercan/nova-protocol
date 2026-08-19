# Promote the ablation and census instruments into probe capabilities

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.11.0,harness,performance

Epic: `20260818-220812`. Promote the instruments that found `20260820-003333`
from one lane's throwaway branch into `nova_probe` capabilities, before the
branch is deleted and they have to be reinvented.

## Why

**Nothing in the tree counted materials until 2026-08-19.** That is the whole
reason a 2x regression sat undetected for two days: the ablation that found it
needed a count instrument, a way to hold the scene still, and a way to stop the
fixed-step loop amplifying every arm - and all three were built by hand, on a
branch that is designed to be thrown away.

Owner: add these as probe capabilities, gated by env.

## Census - default on, cheap

Per capture, once: total entities, `Mesh3d` entities, **DISTINCT** mesh handles,
material entities, **DISTINCT** material handles, vertex sum over distinct
meshes, colliders, rigid bodies.

**Instances AND distinct, always, side by side.** The distinction is what
decided this: 9,866 instances over 600 meshes told a completely different story
from the 9,866 alone, and a census reporting only the total would have kept the
wrong hypothesis alive.

## Ablation - three constraints, all load-bearing

1. **Feature-gated, not only env-gated.** Behind `dev`/`debug` the code path is
   ABSENT from a release build. Env-only means a shipped binary carries a switch
   that disables collision, reachable from a stray environment variable.
2. **A typo MUST fail loudly**, naming the valid switches. `NOVA_ABLATE=claddding`
   silently ablating nothing hands back a confident wrong answer, which is worse
   than a crash. This exact failure mode appeared twice in one day - the NOVA OS
   map's bare `continue` (`20260819-131004`) and a capture averaging in a paused
   clock (`e6055b3e`).
3. **Ablate by `SystemSet` or plugin, from the PROBE side.** `CONVENTIONS.md`
   Bevy rule 2 already requires named `<Subsystem>Systems` sets, so most handles
   exist. Keeping the switch in the probe stops this rotting into
   `if env::var(...)` smeared through `nova_ship`.

## Promote from `arena-ablation`, verbatim in intent

- **`ABL_NOGATE`** - capture from a fixed scene state rather than a scoreboard
  predicate, so a window is a fixed number of frames from a fixed state.
- **`ABL_PEACE`** - `engage_range = 0`, `engage_delay = 1e9`: the roster patrols
  and nobody shoots, so no ship dies, no projectile is a body, and the result
  screen that pauses `Time<Virtual>` can never open.
- **The count instrument** - as the census above.
- **`NOVA_PERF_MAX_DELTA`** - and its derivation, which belongs in the harness
  docs rather than one lane's notes:

  ```
  F = B + s * F / 15.625      ->      F = B / (1 - s / 15.625)
  ```

  A frame is not slow because the fixed loop ran; the loop ran because the frame
  was slow, and then charged it again. Pinned to one step a measurement reads
  `B`, the per-frame base cost, and arms become comparable.

- **The paired/interleaved protocol** - a fresh reference capture immediately
  before every arm, each arm divided by the reference interpolated between its
  neighbours, reporting the median of per-pass ratios with min-max beside it.
  **A ratio whose spread straddles 1.00 has measured nothing.** This is what
  caught a `noclad` arm - a strict SUBTRACTION of 11,660 entities, which cannot
  be slower - reading 56% slower on a contended box.

## A caution that belongs in the capability's own docs

Ablation makes it easy to measure "what if physics were cheaper" and then reach
for the number. **The switches are a measurement tool, not a menu of shipping
options.** A physics ablation reports the size of a prize that the epic's trade
rule may not allow taking. Say so where the next person will read it.

## Done when

- A probe run reports the census without being asked, instances and distinct
  side by side.
- An ablation switch is unavailable in a release build, and a misspelled one
  fails naming the valid set.
- The `arena-ablation` branch can be deleted without losing anything.
