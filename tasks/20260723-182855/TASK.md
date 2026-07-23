# ledger_ch5: torpedo-ship reward raid finale (content)

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.8.0, content, scenario, playtest

## Story

Umbrella 20260723-182811. The Ledger currently ends at ch4 (THE BUYER): the
SELL path hands you the Auditor gunship fight and, on victory, a terminal
"PAYDAY AT A PRICE" Victory; the BURN path ends terminal with no fight. The
player never once gets to fly a big ship with torpedoes across the whole
campaign. This task adds a reward victory-lap chapter, ledger_ch5, that ONLY
the fight (SELL) path reaches: win the Auditor fight and you drop into a big
torpedo-armed ship and, with two AI wingmen, raid the Magpie base - a real
multi-section station - defended by 4-5 small enemy fighters, among asteroids
and a couple of planetoids. "Go HAM, make it worth it." The BURN ending stays
exactly as-is (terminal, no chain).

## Design (confirmed with the user)

- REACHABILITY: chain ch5 from ch4's Auditor-death (SELL) handler
  (`ledger_ch4.content.ron` ~line 1199-1227). That handler currently fires a
  terminal `Outcome(Victory)`. Keep the Victory beat (the PAYDAY overlay) and
  ADD `NextScenario((scenario_id: "ledger_ch5_...", linger: true))` alongside it
  - exactly the Victory + NextScenario shape ch3's fight-path yard-arrival uses
  (`ledger_ch3.content.ron` ~2001-2013) to chain into ch4. The BURN terminal
  overlay (~1158-1191) is untouched, so only the fight path leads to ch5.
  (Re-word the SELL Victory message so it reads as a lead-in to the raid, not
  "the end" - the payday buys the strike on the Magpies.)
- BIG SHIP: player-controlled `Spaceship` with Torpedo sections. Model the ship
  on `assets/base/scenarios/broadside_gunship.content.ron` (multi-section hull)
  and the torpedo bay section `assets/base/sections/base.content.ron`
  (`id: "torpedo_section"` / the cargo torpedo bays ~1367+); wire the torpedo
  launch into the Player `input_mapping` (a distinct key from the guns). Give it
  a bigger section footprint than the salvage tug so it reads as a capital ship;
  keep `infinite_ammo: true` or a generous torpedo count so the hero moment is
  not ammo-starved. Author it as its own ship, sized to feel big.
- WINGMEN (2): `controller: AI` + `allegiance: Some(Player)` - the lifeline
  "relief wing" pattern (`assets/base/scenarios/lifeline.content.ron` ~431,
  ~1267). Small ships; give them a `patrol`/`orbit` near the player's ingress so
  they fly in with you and engage the defenders (Player-vs-Enemy is Hostile, so
  they auto-target).
- ENEMIES (4-5): `controller: AI` with the DEFAULT allegiance (AI ships default
  to Enemy - see `crates/nova_scenario/src/actions.rs`
  `authored_allegiance_overrides_the_controller_default`). Small fighters
  orbiting/leashing the base. Consider `engage_delay` so their approach is
  readable and the fight opens with a beat, not an instant furball.
- THE MAGPIE BASE: a real multi-section static `Spaceship` (Hull + Turret
  sections, Enemy allegiance, NO thruster/no patrol so it holds station) - build
  it as an actual structure, like final_tally's `anchorage_bow`/`anchorage_stern`
  (`assets/base/scenarios/final_tally.content.ron` ~405-453), NOT an asteroid.
  It is destructible and a torpedo target: cracking it is part of the win. (User
  note: reuse `hull_sections` to give it a station silhouette; dedicated station
  art comes later - build it as a thing now.)
- ARENA: asteroids via `ScatterObjects` (Ring region, the ch3 debris idiom
  `ledger_ch3.content.ron` ~1259-1288) plus 2-3 big planetoids as cover/scenery
  (the `menu_*`/`gauntlet` planetoid spawns). Give the fight room and verticality.
- WIN / LOSE: Victory when the base AND the 4-5 defenders are destroyed (track a
  live enemy count / base-down flag with the OnDestroyed + VariableSet counter
  idiom the ledger already uses; the last kill fires `Outcome(Victory)`). Player
  death fires `Outcome(Defeat)` + `NextScenario` retry of ch5 (the ch4 finale
  retry pattern ~1232-1252). No further chain - ch5 is the campaign's new end.
- SCENARIO PLUMBING: `hidden: true` (reached only by playing, like ch2b-ch4);
  seed all gate/counter variables in OnStart before any handler reads them
  (undefined-variable rule); an OnStart briefing from Vesh framing the raid.

## Steps

- [x] Authored `webmods/the-ledger/ledger_ch5_the_raid.content.ron`: shell
      (`hidden: true`), OnStart seeding (act/raiders_left/base_down/win_said/
      base_said), Vesh briefing + recap Objective. Big section blocks (cargoB
      player ship, racer small ships) spliced in from the shipped auditor /
      broadside layouts rather than hand-transcribed.
- [x] Player big torpedo ship: the cargoB gunship (same hull-class as the ch4
      Auditor), Player controller, guns (the 2 turret cubes) on LMB /
      RightTrigger2 and torpedoes (the 2 torpedo bays) on RMB (dedicated) +
      RightTrigger2 on gamepad, infinite_ammo, speed_cap 35.
- [x] Two AI wingmen (`allegiance: Some(Player)`) on the player's flanks,
      patrol leading toward the base, leash 700.
- [x] Magpie base as a custom multi-section station (reinforced_hull +
      better_turret around a controller core, NO thrusters so it holds station),
      Enemy; four AI raiders (default Enemy) orbiting + leashed to the base with
      engage_delay 6-8s.
- [x] ScatterObjects Ring of destructible rocks + three big invulnerable
      planetoids (with gravity) for cover and scale.
- [x] Win/lose: per-raider OnDestroyed counter (raiders_left) + base_down flag,
      an OnUpdate Victory gate (base_down==1 && raiders_left==0) latching act->2
      one-shot (win_said); player-death Defeat latching act->3 + NextScenario
      ch5 retry. Both terminals gate act==1 so neither overwrites the other.
- [x] Chained from ch4: the Auditor-death (SELL) handler now adds NextScenario
      -> ch5 beside its Victory, and its message re-worded as a raid lead-in.
      BURN path left terminal. ch4 rig updated to the new contract; its stale
      "chapter 4 of 4"/"last chapter" comments fixed.
- [x] Registered ch5 in the bundle `content` list and bumped `meta.version`
      1.9.0 -> 1.10.0 (+ bundle description/header comment).
- [x] Added `crates/nova_assets/tests/ledger_ch5_raid.rs` (mirrors the ch4
      rig): 9 tests - cast/counters, the torpedo gunship (carries Torpedo
      sections, bound + infinite ammo), wing=Player / raiders=Enemy / base=Enemy,
      win-needs-base-AND-all-defenders + partial-clear-does-not-win, player-death
      Defeat+retry, victory-not-overwritten, ch4-SELL-chains-here (exactly one
      handler), bundle-ships-raid+version.
- [x] Content lint (`--target webmods/the-ledger`) 0 err/warn/finding (1
      pre-existing ack); `webmods_validation` loads ch5 through the real loader.
- [x] Docs sweep: CHANGELOG 1.10.0 entry; README (chapter count/list/hidden
      range + new chapter 5); bundle description + header comment; ch4 RON header
      + burn-overlay comment; `docs/news-0.8.0-the-ledger.md` finale bullet; the
      mod-guide version walk 1.9.0 -> 1.10.0. (news-0.7.0 left verbatim - dated
      release history.)

## Definition of Done

- cmd: `cargo run -p nova_assets --bin content -- lint --target webmods/the-ledger`
  is clean (no undefined-variable / dead-handler flags in ch5); `cargo test -p
  nova_assets --test webmods_validation` loads ch5 through the real loader.
- test: the ch5 rig passes - loads, spawns player + 2 Player-allegiance wingmen
  + base + 4-5 Enemy defenders; base+defenders down => Victory; player death =>
  Defeat + ch5 retry; ch4 SELL handler chains to ch5 and BURN does not.
- cmd: the full CI check suite is green.
- manual: playtest - win the ch4 Auditor fight, confirm you drop into the big
  torpedo ship with two wingmen, and that raiding the Magpie station (4-5
  defenders, asteroids + planetoids, torpedo the base) feels like a real reward.

## Notes

- Do NOT run the full local suite / clippy (memory: skip-local-tests-and-clippy).
  Run fmt/check + lint + the two tests above; CI runs the rest.
- No engine changes: allies are AI+Player-allegiance, enemies are AI default
  Enemy, the base is a multi-section Enemy Spaceship, torpedoes are the existing
  Torpedo section. If any of these turns out to need an engine primitive
  (e.g. a static "no-drift" flag for the base), STOP and surface it - do not
  widen this content task into engine work; file it as its own task.
- LANDS AFTER 20260723-182850 (the overspeed task): both bump the bundle
  version, so sequential landing avoids a clash. This task bumps on top of that
  one's bump.
- Big content file. Copy real section blocks from broadside_gunship / base
  sections / final_tally rather than hand-writing cube layouts; the exact cube
  ids and offsets must match a shipped, loading ship.

## Outcome (2026-07-23)

CLOSED. A new reward-finale chapter, pure content (RON) - no engine changes.

**What changed and why.** Added `ledger_ch5_the_raid.content.ron` and chained it
off ch4's SELL (fight) win so the campaign's fight ending now pays off with a
victory-lap raid: the player flies a cargoB-class gunship (the Auditor's own
hull-class) with real torpedo tubes, two Player-allegiance AI escorts, against a
custom multi-section Magpie station and four Enemy fighters, among asteroids and
planetoids. The BURN ending stays terminal, so the raid is specifically the
reward for choosing to fight (confirmed with the user).

**Key design decisions.**
- No engine work was needed or done: allies = `controller: AI` +
  `allegiance: Some(Player)` (the lifeline relief-wing pattern), enemies = AI
  default Enemy, the base = a static Enemy `Spaceship` of `reinforced_hull_section`
  + `better_turret_section` around a `basic_controller_section` core with NO
  thrusters (so it holds station), torpedoes = the shipped `cargob_cube_*` bays.
- The big ship reuses the ch4 Auditor's proven 42-section cargoB layout; small
  ships reuse the broadside racer layout. I spliced these shipped section blocks
  in via a placeholder pass rather than hand-transcribing ~2000 lines of cubes -
  the exact cube ids/offsets must match a loading ship, and copying removes
  transcription risk. Section ids are ship-local, so the same block is reused
  across all six small ships safely.
- Torpedoes get a DEDICATED trigger where the input model allows: `Mouse(Right)`
  (RMB) for the tubes, guns on `Mouse(Left)`; on gamepad both share
  `RightTrigger2` because the flight rig reserves every other gamepad button
  (the lint caught my first attempt at `LeftTrigger2`, which double-drives RCS).
- Win needs the base AND all four fighters (a base-down flag + a per-raider
  decrement counter, checked by an OnUpdate gate); both the Victory and the
  player-death Defeat latch a distinct terminal act (2 win / 3 loss) gated on
  act==1, so neither can overwrite the other (outcome-is-last-write-wins).

**Difficulties / how diagnosed.** The content lint was the workhorse here and
caught three real classes before anything shipped: (1) my base turrets floated
with empty cells below them - a turret's -Y mount base must sit against an
occupied cell, fixed by mounting the turrets on top of the arm hull cubes;
(2) the base + four raiders spawned inside their own 450u threat envelope of the
player ("spawned-dead" - under fire before first input), fixed by pushing the
base to z=-520 and the raiders to ~460-575u so the raid is a real approach;
(3) the LeftTrigger2 torpedo binding double-drove the flight RCS. All fixed and
re-linted clean. Separately, the ch4 change broke the ch4 rig's "the sell win
does not chain" assertion (a fixture pin far from the diff) - updated it to the
new contract (sell chains to ch5) and fixed the stale ch4 "chapter 4 of 4" /
"last chapter" comments.

**Verification.** `content lint --target webmods/the-ledger` 0 err/warn/finding
(6 scenarios balance-audited incl. ch5, 1 pre-existing Auditor ack);
`webmods_validation` loads ch5 through the real modding loader; `ledger_ch5_raid`
10/10 and `ledger_ch4_ending` 9/9 green; `cargo fmt --check -p nova_assets` clean.

**Self-reflection.** Leaning on the lint as a fast structural oracle (turret
mounts, spawn distances, input conflicts) before writing the rig saved a lot -
those are exactly the errors a text agent cannot eyeball in a 2900-line data
file. The splice-shipped-section-blocks approach is the right call for capital
ships and should be reused. Next time, run the sibling rig (ch4 here) as part of
the same-scenario blast radius immediately after touching a chained chapter,
rather than discovering the broken fixture pin later.

Manual playtest (fly the raid, feel the reward) stays pending for the umbrella's
Finish checkpoint.
