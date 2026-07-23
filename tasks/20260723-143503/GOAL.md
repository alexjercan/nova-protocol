# Goal: ch3 speed-provocation - wake the Magpies when the player burns too hot

- DATE: 20260723
- UMBRELLA TASK: 20260723-143503
- LANDING SCOPE: squash-merge each task to master (local default branch), no push. Default flow landing.

## Goal

In The Ledger chapter 3 (THE QUIET CHANNEL), add a fifth stealth provocation:
burning too hot wakes the two NEUTRAL Magpie pickets. The chapter's fantasy is
"run dark and slow" (player speed cap 25 u/s); the pickets currently only wake
on a picket-watch zone entry or a combat-lock paint, so speed should be noise
they hear.

Delivered in two halves: a reusable ENGINE capability - expose the player's
live speed to scenario content as a reserved `player_speed` variable, mirroring
`scenario_elapsed` - and the ch3 CONTENT that consumes it as a warn-then-trip
overspeed handler. Threshold 8 u/s; the first overspeed while sneaking warns
(Vesh), a fresh breach after slowing back down trips (both Magpies -> Enemy),
with a hysteresis band so one continuous burn cannot warn-and-trip in
consecutive frames.

## Done means

1. A reserved scenario variable `player_speed` tracks the player ship's live
   speed (avian3d `LinearVelocity.length()`) each live-unpaused frame, reads
   0.0 with no player, and freezes under pause. (test: `cargo test -p nova_scenario`)
2. Content lint accepts `player_speed` in Expression filters - no
   undefined-variable flag. (cmd: `cargo run -p nova_assets --bin content -- lint webmods/the-ledger`)
3. In ch3, while sneaking (`spotted == 0 && act == 1`): the first `player_speed > 8`
   fires a Vesh warning and leaves both Magpies Neutral; a fresh breach after
   slowing below the rearm band (7 u/s) wakes BOTH Magpies (SetAllegiance ->
   Enemy) and stamps `spotted = 1`; a single continuous burn does not
   warn-and-trip; once any provocation sets `spotted`, the speed handlers are
   inert. (test: `cargo test -p nova_assets --test ledger_ch3_channel`)
4. Ch3 still loads through the real modding loader. (cmd: `cargo test -p nova_assets --test webmods_validation`)

Overall: the full check suite passes (CI), and a ch3 playtest shows creep =
unseen, overspeed = warned-then-caught.

## Tasks

- [ ] 20260723-143530 (p60, nova_scenario) expose player_speed as a reserved scenario variable (engine)
- [ ] 20260723-143603 (p58, the-ledger) warn-then-trip picket provocation on player overspeed (content) [depends on 20260723-143530]

## Decisions (load-bearing, architectural)

- Reserved variable over new filter/event type: reuse the existing Expression
  filter machinery (like `scenario_elapsed`) rather than add a bespoke speed
  filter or `OnSpeedThreshold` event. Cheapest, content-general, mirrors a
  proven pattern. (No separate DECISION.md - recorded here.)

## Manual acceptance (batched for the user at Finish)

- (pending) 20260723-143603: playtest ch3 - creep under 8 u/s and slip past
  unseen; gun it to hear Vesh's warning; gun it again after slowing and confirm
  both Magpies go hot.
