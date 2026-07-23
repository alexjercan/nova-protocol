# RETRO - expose player_speed as a reserved scenario variable

- Task: 20260723-143530 (umbrella 20260723-143503)
- Outcome: CLOSED, APPROVE round 1 (out-of-context reviewer, zero findings)
- One commit: 5a70a774

## What went well

- The `scenario_elapsed` reserved-clock was an exact precedent. Every piece had
  a template: the const + rustdoc contract, the tracker fn, the shared
  `register_clock_and_pulse` chain, BOTH lint rules, and even the two test
  shapes (`scenario_clock_freezes_while_paused` for the tracker, the clock
  read/write lint test for the lint pin). Mirroring a proven pattern instead of
  inventing one is why review came back clean in a single round.
- Factoring the two reserved variables behind one `is_reserved_engine_var`
  predicate (rather than adding `player_speed` at each lint site independently)
  means the undefined-exempt rule and the write-error rule cannot drift apart -
  the reviewer called this out as the right call.
- Honored `would-it-fail-without-it` with a real A/B: pulled `track_player_speed`
  out of the chain, watched the tracker test go red, restored it. The fail-first
  proof was not just asserted in prose.
- Built the tracker test on the REAL `register_clock_and_pulse`, not a synthetic
  hand-seeded rig (production-faithful-rigs) - so it also exercises the shared
  gate and the pause-freeze for free.

## What went wrong / friction

- One `cargo doc` warning: a `pub const`'s rustdoc used an intra-doc link
  `[`track_player_speed`]` to a PRIVATE fn, which rustdoc rejects (public ->
  private). Caught by the doc check, fixed by downgrading to a plain code span
  `` `track_player_speed` `` - which is exactly how `SCENARIO_ELAPSED_VAR`
  already refers to its private tick fn. Minor, but it cost a rebuild.

## Lesson candidate (for /lessons at Finish)

- `rustdoc-no-public-to-private-intra-doc-link` (x1): a `pub` item's rustdoc
  cannot `[link]` a private symbol without a `cargo doc` warning - refer to
  private helpers with a plain code span, reserve `[intra-doc links]` for items
  at least as public as the referrer. Keep `cargo doc --no-deps` in the verify
  loop for any task that adds rustdoc to public items. 20260723-143530.

## What to do differently next time

- When adding rustdoc to a new public item, run `cargo doc -p <crate> --no-deps`
  as part of the verify step from the start (I ran it, but only after the test
  loop) - it is the only check that catches the private-link class, and it is
  cheap once the crate is already built.
