# Review - Ledger ch3 depth (task 20260722-214105)

Out-of-context adversarial review of branch `content/ledger-ch3-depth`. Reviewer
was not the implementer. Verified by reading the full diff, recomputing the pinch
geometry independently, grepping the shipped scenario vocabulary, and running the
lint + both test suites under `nix develop`.

## Round 1

### Checks run (all green)

- `content lint --target the-ledger`: **0 errors, 1 warning, 1 finding, 2 acked**.
  The single WARN cites `ledger_ch4.content.ron` (`auditor` multi-spawn), NOT ch3,
  exactly as the task predicted. ch3's beat-sheet arms pass clean.
- `cargo test -p nova_assets --test ledger_ch3_channel`: **9 passed**.
- `cargo test -p nova_assets --test ledger_ch2_encounter`: **12 passed** (no cross-break).

### Reachability / soft-lock: clean

- Opening cascade thresholds strictly ascend (2 < 11 < 20 < 30) and each handler
  sets `open_step := N+1`, falsifying its own `Equal(open_step,N)` guard. Hand-off
  guarded `nav_posted==0`, sets 1. Monotone, single-fire, reachable on the clock.
- Corridor gates are each guarded `Equal(gate,N)` and set `gate=N+1`; strictly
  sequential, terminates at YARD (`gate==4` -> Victory -> `NextScenario
  ledger_ch4_the_buyer`, linger). `obj_ch3_recap` completes at hand-off, `obj_gates`
  posts there and completes at YARD. No orphaned objective.
- `arrive3_said` final-leg breather guard (webmods/the-ledger/ledger_ch3.content.ron:1546-1548):
  gated `gate==4 && act==1`. The YARD OnEnter (:1584-1588) sets `act=2` in the same
  actions list *before* the Victory Outcome, so once the win fires the breather is
  permanently disqualified. Before the win it is a legitimate breather window
  (gate=4, act still 1). It self-disqualifies via `arrive3_said==0 -> 1`. Cannot
  re-fire, cannot strand. Confirmed.
- `beat_gate` is shared by the NAV-2 breather (`gate==3`) and NAV-3 breather
  (`gate==4`); NAV-3 re-stamps it and the two breathers are disjoint on the gate
  guard, so no cross-fire.
- Player-death Defeat (:1609-1611) gated `act<2`, retries `ledger_ch3_quiet_channel`
  (linger). Cannot overwrite an earned win (`death_after_the_win_declares_nothing`
  test pins this). Confirmed.

### Debris pinch: threadable, invulnerable, optional - clean (recomputed independently)

Recomputed from the shipped spawn positions and the real
`ASTEROID_GEOMETRIC_FACTOR_MAX = 6.0` (crates/nova_scenario/src/objects/asteroid.rs:364):
- Boulders port (2.5,7.5,-153.3) / starboard (57.5,7.5,-116.7): centre-to-centre
  **66.06u**; worst-case bodies (3.5+3.5)*6 = **42u**; clear gap **24.06u** >> the
  test's `need` of 11u (5u ship + 6u margin). Ship fits with wide margin.
- Gap centre (30,7.5,-135) sits **0.0u off** the NAV-1->NAV-2 leg at **t=0.5** - the
  pinch is dead-centre on the lane the player already flies. Not impassable.
- Both boulders `invulnerable: true, health: 1000` (webmods/the-ledger/ledger_ch3.content.ron:466,483)
  - same config shape as ch2 cover rocks. The hazard is flown, not shot; fighting
  stays optional. Confirmed.
- NARROWS trigger (45,11,-160), area_radius 22, sits 29.4u from gap centre (t=0.77,
  only 1.49u off the leg) - reachable while flying the leg, and past the gap so the
  confirm fires only after threading. Confirmed.

### Vocabulary: all shipped, none invented

`VariableSet`/`Add`, `Equal`/`GreaterThan`/`LessThan`, `Name`, `StoryMessage` w/
`dwell`, `Objective`/`ObjectiveComplete`, `Outcome`/`NextScenario`, `Entity`,
`Beacon` w/ `area_radius`, `Asteroid` w/ `invulnerable` - all resolve in
`crates/nova_scenario/`. No invented actions or filters.

### Beat sheet: compliant

- OnStart posts only `obj_ch3_recap` (holding line), no NAV objective during the
  conversation. Pinned by `on_start_posts_only_the_holding_objective_not_the_nav_goal`.
- First NAV objective lazy-posts at the `open_step==4` hand-off. Pinned by
  `the_opening_cascade_lazy_posts_the_nav_objective_after_the_clock_pump`.
- One StoryMessage per handler across all new + existing handlers; no StoryMessage
  co-fires with an Outcome (YARD Victory carries no line).
- `dwell` values 7,9,9,6,7,6 all within [3,30]. (The two short player one-liners
  carry no dwell, which is fine.)

### Test quality: production-faithful

Loads the REAL shipped `ledger_ch3.content.ron` via `include_str!`, registers the
real non-OnStart handlers the way the loader does, and drives them with real
`OnEnter`/`OnUpdate`/`OnDestroyed` event infos plus a scenario-clock pump. The
geometry assertion is COMPUTED from the loaded spawn positions and the real
`ASTEROID_GEOMETRIC_FACTOR_MAX` constant (no magic literal). It asserts the opener
lazy-posts, the pinch warn/confirm order (including the early-arrival case that must
stay silent), the YARD->ch4 chain, and the death retry. OnStart itself is exercised
structurally (documented `rig-supplies-precondition`), not driven, but two dedicated
tests pin its seeds/spawns/objective - acceptable. No assertion spotted that would
green a regressed feature.

### LOW-1 - pinch confirm line is missable by a fast pilot (polish, not a lock)

webmods/the-ledger/ledger_ch3.content.ron:743-761. The far-side confirm requires
`pinch_warn_said==1`; a pilot who reaches NARROWS within ~4s of NAV-1 (before
`pinch_gate` elapses) passes the trigger before the warning plays, so the confirm
never speaks (the OnEnter won't re-fire). This is a dropped comms line only - the
corridor advances on the NAV beacons regardless, so no soft-lock. The test explicitly
covers and accepts this ordering. Suggested (optional): drop the `pinch_warn_said==1`
gate on the confirm, or re-arm it, if the missing "clean through" line bothers replay.
Not blocking.

## Verdict
APPROVE
