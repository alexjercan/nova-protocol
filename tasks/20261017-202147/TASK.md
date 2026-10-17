# Economy: refuse a save whose clock, frozen stamps or trader preference are invalid at open

- STATUS: OPEN
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
