# Retro: Tune PDC turret damage

- TASK: 20260712-172035
- BRANCH: fix/pdc-damage-tuning (landed as 190da72)
- REVIEW ROUNDS: 1 (APPROVE, self-review with re-derivation)

Playtest-driven balance tweak. What/why: TASK.md. Process only here.

## What went well

- **Mid-flow playtest feedback handled by the book.** The "PDC one-shots
  asteroids" report arrived while the bullet-type-slot task was in review. Per the
  flow discipline I finished that cycle first, then filed the PDC issue as its own
  prioritized task and ran a full cycle on it - no widening of the in-flight
  branch, no lost feedback.
- **Diagnosed before tuning.** Traced the actual numbers - player flies
  `better_turret` (~20/hit @ 100 rounds/s ≈ 2000 DPS), field asteroids 100 HP, so
  ~5 rounds in ~50 ms - and found "one bullet" was PERCEPTUAL (a dense stream),
  not a literal single hit (20 < 100). That reframed the fix from "damage > HP" to
  "rounds-to-kill / DPS", which is the right lever.
- **Made the knob legible and guarded.** Extracted a named const with the math in
  a comment and added a falsifiable guard test (fails at the old value) that pins
  the fix intent while leaving tuning headroom - so a future creep back toward
  one-shot territory trips a test, but a deliberate re-tune within reason does not.

## What went wrong

- Nothing. One footgun avoided: `cargo test -p nova_assets <filter>` first hit the
  integration-test target (`cubemap_meta`, 0 matches) and looked like it ran
  nothing; re-ran with `--lib` to actually execute the unit guard. (Consistent
  with the one-cargo-test-filter / target-selection friction.)

## What to improve next time

- For a delegated balance change ("tune that please"), pick a defensible value,
  ship it with the math + a guard, and state the tradeoff (here: ~5x slower ship
  TTK) so the playtest loop can correct in one step rather than round-tripping on
  "what did you change it to".

## Action items

- [x] Retro written; ledger `diagnostic-first` bumped (traced the DPS/HP numbers
  before theorising the fix).
- [ ] Awaiting playtest confirmation of the new PDC feel (4.0/hit); adjust the
  const if it wants more/less punch (guard allows up to ~8.3).
