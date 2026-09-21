# Review 02: Gameplay damage and projectiles

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor finding

## Finding

### MINOR - `crates/nova_gameplay/src/rounds.rs:232` - A rustdoc link names a removed field

The `MAX_BITES_PER_STEP` comment links to `ProjectileDamage::layers`. `ProjectileDamage` has `amount`, `power`, and `kind`; repository search found no `layers` member. The sentence describes the authored power budget, so the link should name `power`.

This affects source and generated API documentation only. No new API is needed.

## Adjudication

- Dropped the complaint about the phrase "load-bearing." The same sentence states the concrete ordering requirement and failure mode.
- Dropped the unconditional `advance_rounds` velocity scan as a finding. The scan exists, but the reviewer supplied no measured frame cost and the code documents a small body-count bound. It does not meet the performance lane's requirement for a real frame cost.
- Checked and cleared suspected blast collision orientation, pierce accounting, raking charge, blast lifetime, torpedo blast ownership/lifetime, and `RunCheats` reset/consumer behavior.

## Coverage

Fully read `damage.rs`, `rounds.rs`, `rounds/tests/exact.rs`, `relations.rs`, `cheats.rs`, `projectile_hooks.rs`, and the gameplay plugin/crate roots. Targeted reads covered integrity carve/core callees, torpedo detonation, scenario area collision handling, cheat consumers, and the relevant vendored Avian collision implementation.

Verification: `nix develop --command cargo test -p nova_gameplay --lib` passed 343 tests, with 2 ignored and no failures.

Not checked: the remaining integrity implementation, full downstream ship/scenario consumers, content lint/generation, wasm compilation, rustdoc execution, or a game/probe measurement. Cross-crate consumers outside this batch were grep- or excerpt-checked only.
