# ledger_ch5: torpedo-ship reward raid finale (content)

- STATUS: OPEN
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

- [ ] Author `webmods/the-ledger/ledger_ch5_the_raid.content.ron`: the scenario
      shell (id, name, description, cubemap/thumbnail, `hidden: true`), OnStart
      seeding + Vesh raid briefing + the recap Objective.
- [ ] Spawn the player big torpedo ship (Player controller, guns + torpedo
      input_mapping, generous ammo, capital-sized section layout).
- [ ] Spawn the 2 AI wingmen (allegiance Some(Player)) near the ingress.
- [ ] Spawn the Magpie base as a multi-section static Spaceship (Hull + Turret,
      Enemy) and the 4-5 AI enemy fighters defending it (default Enemy,
      engage_delay for a readable open).
- [ ] Scatter asteroids (ScatterObjects Ring) and place 2-3 planetoids for cover
      and scale.
- [ ] Wire the objectives + win/lose: an enemy/base-down counter, `Outcome
      (Victory)` on the last defender+base destroyed with a payoff message;
      `Outcome(Defeat)` + `NextScenario` ch5-retry on player death. Post/complete
      Objectives as the raid progresses (ingress -> break the fighters -> crack
      the base) matching the campaign's objective-pacing conventions.
- [ ] Chain from ch4: edit `ledger_ch4.content.ron` Auditor-death (SELL) handler
      to add `NextScenario` -> ch5 beside its Victory, and re-word the SELL
      Victory message as a lead-in to the raid. Leave the BURN path terminal.
- [ ] Register ch5 in `webmods/the-ledger/the-ledger.bundle.ron` `content` list
      (after ch4) and bump `meta.version`.
- [ ] Add a ch5 rig test (mirror `crates/nova_assets/tests/ledger_ch3_channel.rs`
      / the ch4 rig if one exists): assert the scenario loads and spawns the
      player ship, both wingmen (Player allegiance), the base + defenders (Enemy);
      drive destroy events and assert base+all-defenders-down => Victory, and a
      player-ship destroy => Defeat + ch5 requeue. Also assert the ch4 SELL
      handler now carries a NextScenario to ch5 (fight path chains) while the
      BURN path does not (no chain).
- [ ] Content lint + real-loader load: `cargo run -p nova_assets --bin content
      -- lint webmods/the-ledger` clean; `cargo test -p nova_assets --test
      webmods_validation` loads ch5 through the real modding loader.
- [ ] Docs sweep from the final diff (keep-docs-in-sync +
      ephemeral-news-draft): `webmods/the-ledger/CHANGELOG.md` (the new chapter +
      version), `README.md` (chapter count "four" -> "five" / the reward finale),
      the bundle description if it says "four chapters", the wiki version-history,
      and the `docs/news-*.md` the-ledger bullet.

## Definition of Done

- cmd: `cargo run -p nova_assets --bin content -- lint webmods/the-ledger` is
  clean (no undefined-variable / dead-handler flags in ch5); `cargo test -p
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
