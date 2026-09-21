# Fix 6 - shared-key validation ignores held state from earlier acts

Finding: `REVIEW-18-bench-channel-debug.md:24`.

## Before (reproduced)

Temporary unit repro against the old
`check_shared_keys(gestures, shared)`: act one presses
`targeting.radar_hold` (held set carries it), act two taps
`targeting.radar_clear`. The guard passed.

```
thread 'gesture::tests::repro_held_counterpart_from_a_prior_act_is_refused'
panicked at crates/nova_bench/src/gesture.rs:352:54:
called `Result::unwrap_err()` on an `Ok` value: ()

test result: FAILED. 6 passed; 1 failed; 0 ignored; 66 filtered out
```

## Change

`crates/nova_bench/src/gesture.rs` -
`pub fn check_shared_keys(gestures: &[Gesture], held: &BTreeSet<String>,
shared: &[[String; 2]]) -> Result<(), String>`. Driving one reading of a pair
while the other is in `held` is refused with its own message that names the
held wire and asks for a release act. `crates/nova_bench/src/referee.rs:274`
passes `&self.held`.

## After

`cargo test -p nova_bench --lib gesture::` and `referee::`:

```
test gesture::tests::a_reading_held_from_an_earlier_act_refuses_its_counterpart_until_released ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 66 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 67 filtered out
```

Docs shipped with the code: `crates/nova_bench/src/manual.md` (the
`inputs.shared` bullet) and `docs/agent-bench.md` (the shared-key section).
