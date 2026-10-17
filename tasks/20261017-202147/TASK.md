# Economy: refuse a save whose clock, frozen stamps or trader preference are invalid at open

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.16.0,economy,bug

## Evidence
- `read_state` (`crates/nova_world_base/src/save/mod.rs:452-476`) validates posting, neutralized and goal stamps against `state.clock.play_seconds`. It does not validate the clock itself (finite, >= 0), a ledger body's `frozen_at` (`crates/nova_world/src/frozen.rs:382`), or a frozen `TraderMarket.role_preference`.
- `WorldClock::elapsed_since` (`crates/nova_gameplay/src/clock.rs:43`) asserts `frozen_at <= play_seconds`. `price` (`crates/nova_gameplay/src/market.rs:385`) asserts the preference sum is above zero.
- A hand-edited save with a NaN clock, a stamp ahead of the clock or a zero preference opens, then panics at the first thaw or trade.
- Low severity: the game's own writer never produces such a save.

## Fix direction
- Refuse at open as `WorldRefusal::Unreadable`, as the rest of `read_state` does.

## Done when
- A save with a non-finite or negative clock, a `frozen_at` ahead of the clock, or a zero preference sum is refused at open. No panic at thaw or trade.
- Permanent proof: unit tests in `crates/nova_world_base/src/save/tests.rs`, one per invalid field, each asserting `WorldRefusal::Unreadable`.

Found by the 2026-10-17 review of epic 20261012-100025.

## Fix 2026-10-17
- Claim: a save whose clock is not finite or negative, whose ledger body froze at a reading outside `0..=clock`, or whose frozen trader (player, escort, ledger ship, station hull or parked hull) has a role preference that is not finite, below zero or sums to nothing is refused at open as `WorldRefusal::Unreadable`. Save format stays 20.
- Evidence: `read_state` validated only the posting, stance and goal stamps (`crates/nova_world_base/src/save/mod.rs:415-477`). The three new tests opened each such save as `Ok(())` before the fix.
- Change: `check_saved_clock` (`crates/nova_world_base/src/save/mod.rs:513`), called from `read_state` right after the id check and before the stamp checks that read the clock (`mod.rs:444`); `FrozenShip::market()` accessor (`crates/nova_scenario/src/objects/spaceship.rs:611`) so the check can read the frozen preference. Open-side only: the writer stamps from the live clock and freezes the market the generator drew.
- Verification: `cargo test -p nova_world_base --lib save::tests`: 41 passed, among them `a_world_whose_clock_is_not_finite_or_negative_is_refused_on_open ... ok` (NaN, inf, -1), `a_world_whose_body_froze_ahead_of_the_clock_is_refused_on_open ... ok` (5 s against a 2 s clock, NaN, -1; a stamp at the clock opens), `a_world_whose_trader_has_no_role_preference_is_refused_on_open ... ok` (all zero, a NaN share, a negative share; a drawn preference opens).
