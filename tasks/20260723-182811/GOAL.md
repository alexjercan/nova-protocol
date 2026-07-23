# Goal: ch3 overspeed reaction window + a torpedo-ship reward raid finale

- DATE: 20260723
- UMBRELLA TASK: 20260723-182811
- LANDING SCOPE: squash-merge each task to master (local default branch), no
  push. Default flow landing.

## Goal

Two player-facing improvements to The Ledger campaign, both pure content (RON)
- no engine changes needed (the engine already exposes `player_speed` and
  `scenario_elapsed`, and the ally/enemy/torpedo/station primitives all exist).

1. TUNE ch3 overspeed. Today the "second strike" (the trip that wakes both
   Magpie pickets) fires the instant the player breaches 8 u/s again after a
   warning - effectively zero reaction time. Give the second strike a real
   ~3.5s sustained-overspeed grace: the first breach still warns instantly
   (harmless), but after re-arming, a fresh breach starts a 3.5s countdown and
   the pickets wake only if the player HOLDS above the limit for the full
   window; easing back under the rearm band cancels it and the run stays dark.

2. ADD a reward finale. When the player chooses to FIGHT the last chapter (ch4
   SELL / the Auditor gunship) and wins, chain to a NEW victory-lap scenario
   (ledger_ch5): the player takes the helm of a big torpedo-armed ship and,
   with two AI wingmen, raids the Magpie base - a real multi-section station
   (Hull + Turret sections, built as an actual structure, not an asteroid) -
   defended by 4-5 small enemy fighters, among asteroids and a couple of
   planetoids. Torpedoes get their hero moment: cracking the station is part of
   the win. The BURN (no-fight) ending stays terminal and unchanged. This is
   the one time the campaign hands the player a capital ship - make it worth it.

## Done means

1. In ch3, the second strike requires sustained overspeed: first `player_speed
   > 8` while sneaking (`spotted == 0 && act == 1`) warns instantly and leaves
   both Magpies Neutral; after re-arming (slowing under 7), a fresh breach
   starts a ~3.5s countdown; holding above 8 for the full 3.5s wakes BOTH
   Magpies (SetAllegiance -> Enemy, `spotted = 1`); dropping under 7 during the
   countdown cancels it and leaves the run dark; a single continuous burn still
   only ever warns; once any provocation sets `spotted`, the speed handlers are
   inert. (test: `cargo test -p nova_assets --test ledger_ch3_channel`)
2. ch3 content lint + real-loader load stay clean. (cmd: `cargo run -p
   nova_assets --bin content -- lint webmods/the-ledger`; cmd: `cargo test -p
   nova_assets --test webmods_validation`)
3. A new scenario `ledger_ch5` exists: the player controls a big torpedo-armed
   ship; two AI wingmen fly on the Player side; 4-5 AI enemy fighters defend a
   multi-section Magpie base station (destructible, a torpedo target); asteroids
   and 2-3 planetoids fill the arena. It is reachable ONLY from the ch4 SELL
   (fight) victory via `NextScenario`; the BURN path stays terminal (no chain).
   (cmd: content lint clean; cmd: webmods_validation loads ch5 through the real
   modding loader)
4. ch5 resolves: destroying the base and its defenders fires Victory; player
   death fires Defeat and retries ch5. ch4's Auditor-death (SELL) handler chains
   to ch5 instead of ending the campaign. (test: `cargo test -p nova_assets`
   ch5 rig - base+defenders down => Victory, player death => Defeat+retry)
5. The bundle lists ch5, its version is bumped, and every doc surface is synced
   (CHANGELOG, README, wiki version-history, news draft). (cmd: content lint;
   manual: docs read true against the shipped RON)

Overall: the full check suite passes (CI), and a playtest confirms both the
softer overspeed window and the finale playing HAM.

## Tasks

- [ ] 20260723-182811 (p0, umbrella) this goal
- [ ] 20260723-182850 (p60, the-ledger) ch3 overspeed: sustained ~3.5s window on the second strike (content)
- [ ] 20260723-182855 (p58, the-ledger) ledger_ch5: torpedo-ship reward raid finale (content)

## Decisions (load-bearing, architectural)

- Overspeed grace via the `scenario_elapsed`-deadline idiom (like `beat_gate`),
  not a new engine timer: stamp `overspeed_deadline = scenario_elapsed + 3.5`
  on the armed breach and gate the trip on `scenario_elapsed > overspeed_deadline
  && player_speed > 8`, with a cancel handler when the player slows under 7.
  Reuses proven content machinery; no Rust.
- Reward finale as a NEW scenario chained on ch4 SELL victory (mirrors the
  ch1->ch2->ch3->ch4 `NextScenario` chain), NOT a branch inside ch4: variables
  do not survive scenario changes, and a fresh scenario is the clean way to hand
  the player a different (big) ship. BURN stays terminal.
- Allies are `controller: AI` + `allegiance: Some(Player)` (the lifeline
  "relief wing" pattern); enemies are AI with the default Enemy allegiance; the
  base is a static multi-section `Spaceship` (Hull + Turret, Enemy) like
  final_tally's anchorage - there is no dedicated station primitive and no
  `Ally` allegiance in the engine.

## Manual acceptance (batched for the user at Finish)

- (pending) ch3: playtest - warn, then gun it again and confirm you get ~3.5s
  to ease off before both Magpies wake; hold it and confirm they wake.
- (pending) ch5: playtest the finale - win the ch4 Auditor fight, confirm you
  drop into the big torpedo ship with two wingmen, and that raiding the Magpie
  station with 4-5 defenders among the rocks feels like a worthwhile reward.
