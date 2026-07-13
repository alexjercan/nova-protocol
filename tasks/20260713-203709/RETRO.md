# Retro: smoke suite command-error gate

- TASK: 20260713-203709
- BRANCH: fix/smoke-command-error-gate (landed as d9bd053)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 1 NIT, addressed and verified)

## What went well

- Plan-to-green in one pass: the mechanism was fully understood when the
  task was filed (20260712-115902's bevy-source dig), so the work was one
  assertion, one sabotage proof, two doc corrections.
- The reviewer re-ran the sabotage independently and confirmed the
  sabotaged example EXITS 0 while the suite goes red - the cleanest
  possible proof the new grep is load-bearing rather than redundant.
- The reviewer's one MINOR (the offending line can scroll out of the 48 KB
  tail) came from measuring an actual run's stderr volume, not from
  reading the diff - measurement beats inspection even in review.

## What went wrong

- Nothing structural. The adjacent event: the drift task filed by the
  previous cycle (20260713-220512) was falsified by the USER spotting a
  keybind collision - holding Space to fire also drives the global
  FlightBurnInput. The probe measured the drift precisely but never asked
  "who else consumes the key I am holding?".

## What to improve next time

- When a scripted stimulus is a shared input (a key, a button), enumerate
  every action bound to it before attributing its effects - an input key
  is a knob with multiple readers.

## Action items

- [x] Ledger: bumped `confounded-knob-experiment` with the input-key
      variant.
