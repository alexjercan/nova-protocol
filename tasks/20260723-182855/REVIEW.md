# Review: ledger_ch5 - torpedo-ship reward raid finale

- TASK: 20260723-182855
- BRANCH: feature/ledger-ch5-raid

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (NIT) crates/nova_assets/tests/ledger_ch5_raid.rs:179 - the
  `outcome_message` helper was defined but unused, producing a `dead_code`
  warning in the test build. Use it in an assertion or remove it.
  - Response: fixed - now used in `the_raid_wins_only_when_the_base_and_all_defenders_are_down`
    (asserts the Victory carries the raid payoff message) and in
    `player_death_is_a_defeat_that_retries_the_raid` (asserts the Defeat
    message), which also strengthens both tests. Warning gone; rig 9/9 green.

What the out-of-context reviewer verified (re-confirmed in-session before adopting
- I independently traced the win-latch logic and the thrusterless-AI-base runtime
safety in `crates/nova_gameplay/src/input/ai.rs`):

- `content lint --target webmods/the-ledger`: 0 error/warning/finding, 6
  scenarios balance-audited (ch5 included), 1 pre-existing Auditor ack.
- `ledger_ch5_raid` 9/9, `ledger_ch4_ending` 10/10, `webmods_validation` 1/1;
  `cargo fmt --check -p nova_assets` clean.
- Win/lose logic: OnStart seeds act/raiders_left/base_down/win_said/base_said
  before any handler reads them. Four distinct per-raider OnDestroyed handlers
  each subtract 1; the base OnDestroyed sets base_down=1 (one-shot). The Victory
  OnUpdate gate requires act==1 && base_down==1 && raiders_left==0 && win_said==0
  and latches act=2; the player-death Defeat gate requires act==1 and latches
  act=3. Both terminals gate act==1, so whichever fires first closes the act and
  the other is inert - no double-win, no win-then-defeat overwrite, no
  partial-clear win. Confirmed by reading and by the rig.
- Reachability: exactly ONE ch4 handler (the Auditor-death SELL win) gained a
  NextScenario to ledger_ch5_the_raid; the BURN overlay chains nothing. The ch4
  rig pins both sides (a real new contract, not a weakened assertion).
- The reward is real: the player input_mapping binds the two Torpedo-kind cubes
  (cube_i1_j1_km2/_im1_j1_km2 -> cargob_cube_* Torpedo prototypes) to RMB and
  the two Turret cubes to LMB; infinite_ammo, speed_cap 35. All bound ids exist
  in the spliced block and resolve to the claimed kinds.
- Allegiances: wing_1/wing_2 = AI + Some(Player); raiders = AI default Enemy;
  base = Some(Enemy), AI, no thrusters (holds station). The base is a real
  multi-section station (controller core + reinforced_hull + 4 turrets on the
  arm tips), all shipped prototypes; lint validated turret mounts + spawn
  distances.
- Docs prose matches the RON (chapter count, reachability, versions); news-0.7.0
  left verbatim (dated history); TASK.md Outcome claims match the code.

Pending manual check (human-acceptance gate, batched at the flow Finish - not
resolved by this APPROVE): playtest the finale - win the ch4 Auditor fight, drop
into the big torpedo ship with two wingmen, and confirm raiding the Magpie
station (four defenders, asteroids + planetoids, torpedo the base) feels like a
real reward.
