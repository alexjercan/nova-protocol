# Retro

## Bug and fix

- The arena relied on one-time shared NOVA OS transition hooks. Make the arena's
  interactive-screen invariant explicit and continuous: clocks paused, pointer
  free.

## Next time

- Player-path examples with custom pause and result flows need a direct test for
  simulation freeze, not only shared menu-state tests.
