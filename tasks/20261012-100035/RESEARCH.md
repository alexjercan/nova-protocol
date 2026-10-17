# Stance: research note

Task `20261012-100035`, epic `20261012-100034`, decision 12 of
`tasks/20261012-095526/DESIGN.html` (D13). Written 2026-10-17 against
`9b507e6e5` on `master`. No code, no test, no type was added for it.

Every claim carries one of three labels:

- **Measured**: read from the tree at `9b507e6e5` (`path:line`) or printed
  by a disposable census spike that was run once and deleted (section 9).
- **Proposed**: a rule this note recommends. Nothing proposed is approved.
- **Test design**: the proof task `20261012-100036` would make permanent.
  None of it exists.

The owner accepts or amends the decisions in section 11 before any code.

## 1. What the owner asked for

Measured, from the task bodies and the design page:

- Decision 12 (`DESIGN.html:926`): a per-civilization stance graph, Ally /
  Neutral / Enemy, never a float; edges only between neighbouring
  civilizations; one-hop propagation; a way back; a research note first with
  the cluster radius, the event and threshold table, the propagation rule,
  the redemption path and a seed sweep. Build only after acceptance and after
  Stage 2.
- D13 (`DESIGN.html:682-688`): destroy or loot its ships and a civilization
  turns Enemy, its allied neighbours follow, its rival neighbours warm, one
  hop only; thresholds and a grace count; rewards hang on Ally (roster,
  escorts, station purchase); Enemy refuses docking and patrols engage; a
  fine at an Ally of theirs or time is the way back.
- Epic done-when (`tasks/20261012-100034/TASK.md:17`): looting three ships of
  one civilization makes its station refuse the player and its rival's
  station sell an escort; a flip never reaches a civilization beyond the
  cluster radius.
- Owner feedback 2026-10-16 (`tasks/20261012-100034/TASK.md:23-24`): some
  neutral ships do not retaliate (the escorts for sale). Attacking a neutral
  ship must turn its whole group Enemy until the stance clears after some
  time or by some other rule.
- Owner direction 2026-10-17 (`tasks/20261012-100033/TASK.md:39`): the game
  is an endless sandbox. A charter is a finite optional milestone, never an
  ending; a met charter stamps the save and changes nothing else.
- A role-dependent fleet cap or a leadership stat that grows is an owner
  idea that was mentioned, not an approved rule. This note does not build
  on it. The fleet stays two slots (`FLEET_SIDES`,
  `crates/nova_ship/src/sections/escort_hire.rs:26`;
  `tasks/20261012-100016/TASK.md:27`). The graph decides who sells an
  escort, not how many the player may hold.

## 2. What exists today (Measured)

### 2.1 Civilizations

- One civilization per node of a 240 km lattice
  (`CIVILIZATION_LATTICE`, `crates/nova_world_base/src/civilizations.rs:33`),
  jittered up to 40 km per axis (`CIVILIZATION_JITTER`, `:45`). Influence
  reaches zero at 320 km (`INFLUENCE_REACH`, `:51`). A compile-time assert
  keeps every place within reach of some centroid (`:94-98`).
- Every trait is its own seeded stream off the id (`CivilizationId::stream`,
  `crates/nova_gameplay/src/civilization.rs:87-98`): centroid, status,
  advancement, roles, allegiance (`civilizations.rs:205-240`). Living with
  chance 0.7 (`:70`).
- `Civilization.allegiance` is one side per civilization, Enemy / Player /
  Neutral with one chance in three each (`:140-142`, `:238-240`). It is the
  only relation a civilization has today, and it is toward the player's
  side, not toward another civilization.
- `in_reach(position)` scans the nodes whose jittered centroid could lie
  within the reach (`:265-317`). There is no neighbour query between
  civilizations.

### 2.2 Sides, retaliation, provocation

- `Allegiance { Player, Enemy, Neutral }` (`crates/nova_gameplay/src/relations.rs:37-44`).
  `relation` reads Player and Enemy as hostile to each other and Neutral as
  hostile to nobody (`:64-72`). There is no Ally variant: allied crews ARE
  `Allegiance::Player`.
- An intact generated ship spawns with its civilization's side and an AI
  crew; only an Enemy crew gets an engage delay
  (`crates/nova_world_base/src/sector_ships.rs:283-296`). A derelict is
  `Neutral` with no controller (`:296`).
- A Neutral ship answers one attacker at a time through
  `RetaliationTarget` (`relations.rs:74-83`, `ship_relation` `:100-110`),
  after that attacker's damage tally crosses 30 hit points
  (`NEUTRAL_RETALIATION_THRESHOLD`,
  `crates/nova_ship/src/input/ai/threat.rs:106-112`) net of a 10 hp/s drain
  (`:114-120`). The tally names attackers by `Entity` and is not frozen, so
  a thawed ship holds no grudge (`:121-128`). An unarmed Neutral never
  answers (`:376-382`). A provoked Neutral station lets go of the player's
  dock (`:386-397`).
- The retaliation is per ship and per pair: a Neutral answering the player
  stays Neutral to every third ship (`relations.rs:169-195`). Nothing on a
  civilization reads it. This is the gap the 2026-10-16 feedback names.
- A parked hull (a hull for sale, a ship the player left, or an escort
  for hire) is a ship root of its own and inert: nobody drives it, it is
  Neutral, every capability is off
  (`crates/nova_scenario/src/objects/parked_hulls.rs:5-9`). It has no AI
  crew, and `on_damage_track_threat` runs only on `AISpaceshipMarker`
  (`threat.rs:336-346`), so the escorts for sale cannot answer fire by
  construction. This is the 2026-10-16 observation, explained.

### 2.3 Admission and the robbery window

- `station_admission` refuses in this order: every port held, then (owned or
  dead admits), Enemy side, the player within the robbery window of the
  station's civilization, hostile by `ship_relation`
  (`crates/nova_ship/src/sections/docking_section/admission.rs:63-87`,
  `DockRefusalType` `:30-46`).
- `NeutralizedCivilizations` is the player's record of civilizations whose
  hull the player neutralized, stamped with the play clock
  (`crates/nova_gameplay/src/civilization.rs:116-182`). The world layer
  stamps it on the neutralize (`crates/nova_world_base/src/economy.rs:104`)
  and forgets stamps older than `PriceTable::robbery.refusal_seconds`
  (`:116-118`; `crates/nova_gameplay/src/market.rs:234`). The live content value is 900 s
  (`ROBBERY_REFUSAL_SECONDS`, `crates/nova_authoring/src/base_content/market.rs:44`,
  `:91`); the 600 s at `market.rs:588` is the table default outside the base
  content. The stamp counts a neutralize whose `LastAttacker` is the player
  ship or a fleet escort (`economy.rs:77-88`).
- Decision 10 says this window is subsumed by the stance graph
  (`DESIGN.html:922`): the robbery event becomes an edge flip.

### 2.4 Stations, escorts and the station buy

- A living generated station takes its civilization's side; a dead one is
  Neutral (`crates/nova_world_base/src/clusters.rs:1861-1863`). Every intact
  generated ship and living station carries a `TraderMarket` naming its
  civilization (`crates/nova_gameplay/src/market.rs:289-291`;
  `economy.rs:77-81`).
- A dead station spawns Neutral and loses its AI marker and retaliation
  target (`crates/nova_scenario/src/objects/station.rs:283-286`, `:353-357`).
- A bought station takes `OwnedStationMarker`, `Allegiance::Player` and the
  owned name (`crates/nova_ship/src/sections/station_buy.rs:46-52`), at twice
  its worth (`:54-62`). The buy refuses only not docked, not a
  living generated station, already owned, or cannot pay
  (`StationBuyRefusalType`, `station_buy.rs:96-108`): any living station
  that admits the player sells, whatever its side.
- Escorts for hire ride the station's hull catalogue as `Hire { slot }`
  offers, gated by the `Refit` service, at the hull price times 3/2
  (`tasks/20261012-100016/TASK.md:20-27`). The hire refuses only not docked,
  no Refit service, a full fleet, cannot pay, or overflow
  (`EscortHireRefusalType`, `escort_hire.rs:89-107`); no side is read.
- The fleet has two slots (`FLEET_SIDES`, `escort_hire.rs:26`) and lives in
  `SavedPlayer.fleet` (`crates/nova_world_base/src/save/mod.rs:175-178`;
  `crates/nova_scenario/src/objects/fleet.rs:39-56`).

### 2.5 Events, jobs and the save

- `OnNeutralizedEvent` and `OnDestroyedEvent` carry the ship's scenario id
  and type name only (`crates/nova_events/src/lib.rs:284-305`), no
  civilization. The world layer reads the victim's civilization off its
  `TraderMarket` on `Add<NeutralizedMarker>` (`economy.rs:77-104`), not off
  the event.
- A completed job keeps its kind and its poster, the station id
  (`crates/nova_gameplay/src/jobs.rs:418-424`, `:599-600`); the poster's
  civilization is read off the station, not stored on the job. The job
  generator resolves a poster's civilization through `CivilizationField` at
  post time (`crates/nova_world_base/src/jobs.rs:399-412`); a hand-in can
  resolve it the same way.
- Save format 19, unreleased (`save/mod.rs:84`; `v0.15.0` has no save
  module, so no shipped world needs a migration). `SavedPlayer` holds the
  ship, the view and the fleet (`:164-179`). `WorldSaveState` carries
  the clock, `neutralized` and `goal` (`:149-158`); the clock is
  `WorldClock.play_seconds` (`crates/nova_gameplay/src/clock.rs:30-33`).
- The role goals are `StationNetwork`, `KillList`, `Flagship`, `OwnYard`
  and `MiningEmpire`
  (`crates/nova_scenario/src/objects/start_role.rs:186-213`); the pirate role
  has `goal: None` pending this graph (`tasks/20261012-100033/TASK.md:22`).
  The base content carries the `TODO(20261012-100033)`
  for it (`crates/nova_authoring/src/base_content/start_roles.rs:19`, `:124`).
  The station-network charter counts stations that are not derelict and
  not Enemy (`crates/nova_world_base/src/charter.rs:59-60`), so a flip to
  Enemy already changes what a charter counts.
- Station sections are invulnerable in v1 (decision 3, `DESIGN.html:885`).
  No station can die today.

### 2.6 Where the side is read (blast radius of a new owner)

`Allegiance` appears in 71 files. The heavy readers are the
HUD marker colours (`crates/nova_hud/src/allegiance_markers.rs`), the AI
threat and acquisition (`crates/nova_ship/src/input/ai/threat.rs`,
`acquisition.rs`), the docking admission, the Map contacts
(`crates/nova_interface/src/map/contacts.rs`) and the scenario actions that
author a side (`crates/nova_scenario/src/actions/ship.rs`). The graph must
not change what the enum means to them: it must only decide which value a
generated body carries, and when it changes. Story scenarios author the
side per ship and have no civilization; they stay outside the graph.

## 3. Cluster radius and the neighbour rule

### 3.1 Bounds from the constants (Measured, derived)

With a 240 km lattice and a 40 km jitter per axis, every pairwise centroid
distance is bounded by the lattice offset alone:

| Lattice offset | Nominal km | Minimum km | Maximum km |
| --- | ---: | ---: | ---: |
| face `(1,0,0)` | 240 | 160.0 | 339.4 |
| edge `(1,1,0)` | 339 | 226.3 | 459.6 |
| corner `(1,1,1)` | 416 | 277.1 | 554.3 |
| two apart `(2,0,0)` | 480 | 400.0 | 571.3 |
| `(2,1,0)` | 537 | 430.8 | 649.9 |

Minimum: each axis offset shrinks by twice the jitter; maximum: it grows by
twice the jitter and the zero axes gain 80 km each. Consequences:

- No single radius separates face neighbours from diagonal ones: the face
  maximum (339.4 km) is above the edge minimum (226.3 km).
- Any radius at or below 400 km never admits a pair two nodes apart. Any
  radius at or above 339.4 km always admits all six face neighbours.
- So a radius between 339.4 and 400 km gives a neighbourhood that always
  holds the six face neighbours, sometimes holds edge and corner diagonals,
  and never leaves the 3x3x3 block round the node: degree 6 to 26 before
  status. 330 km, the first round number under the face bound, held every
  face pair in the 16 measured seeds (maximum 328.4 km) but the constants
  allow a face pair up to 339.4 km, so it guarantees nothing.

The census spike measured the actual distribution (section 9).

### 3.2 Options (Proposed)

- **(A) Metric radius, 340 km.** The owner's framing: one parameter, a
  circle the Map can draw, every face neighbour always in by the constants.
  Consequence: the degree varies by seed between 6 and 26 (measured 7 to
  17) and a player cannot predict from the lattice which diagonal is a
  neighbour; the sweep must assert the 3x3x3 bound rather than a fixed
  count.
- **(B) Lattice face adjacency.** Neighbour means the six nodes one step
  away; no parameter. Consequence: fixed degree 6 and a trivial locality
  proof, but a diagonal civilization 230 km away is "not a neighbour" while
  a face one 339 km away is, which reads wrong on a distance Map.
- **(C) Metric radius inside the 3x3x3 block, 400 km.** Consequence: most
  diagonals become neighbours (degree near 26) and a bloc covers the whole
  block; one flip touches up to 26 civilizations, which weakens "the far
  side stays untouched".

Recommendation: **(A), `STANCE_NEIGHBOUR_RADIUS = 340 km`**, strict
`distance < radius`. It is the smallest round radius above the face bound
(339.4 km) and it stays 60 km under the two-apart floor, so both "every
face neighbour is a neighbour" and "no neighbour outside the 3x3x3 block"
hold for every seed by the constants, not by luck. Edges exist only between living civilizations: an
extinct one has no crews and its dead station admits anybody
(`admission.rs:74-76`), so a stance toward it would read nothing.

## 4. Stances and the player's side

### 4.1 The values (Proposed)

A closed enum `StanceType { Ally, Neutral, Enemy }` (naming per the
`<Name>Type` rule in `AGENTS.md`). Two tables:

- The **edge table**: one symmetric stance per unordered pair of living
  neighbours, a pure function of the seed (section 5). Never saved.
- The **player table**: one stance per living civilization toward the
  player, seeded from the civilization and moved by events (section 6).
  Saved sparsely: only entries that differ from the seed.

A body's `Allegiance` is derived from the player stance of its
civilization: Ally -> `Player`, Neutral -> `Neutral`, Enemy -> `Enemy`. The
enum and every reader in section 2.6 keep their meaning; only the writer
changes. No float, no counter is shown anywhere; the Map shows the colour
it shows today.

### 4.2 The seed of the player table (Proposed, decision for the owner)

- **(a) From today's draw.** `Civilization.allegiance` becomes the seeded
  player stance (Player -> Ally). Consequence: a third of the map starts
  Ally, a third Enemy, as every encounter proof and the even-draw test
  assume today (`civilizations.rs:579-609`); Enemy patrols stay the combat
  content of a fresh world.
- **(b) All Neutral.** Only events make a stance. Consequence: a fresh world
  has no hostile patrol until the player makes one and no Ally yard until
  the player earns one; the industrial charter (two escorts) needs three
  jobs first; the even-draw test and the Enemy engage delay die.

Recommendation: **(a)**. "The far side of the map stays Neutral to you
until you fly there" (`DESIGN.html:684`) reads as "unchanged by your
events", which (a) satisfies: propagation never leaves the neighbourhood.

## 5. Seeding the edges (Proposed) and the rival guarantee

Rule, for living civilizations `a` and `b` with `distance(a, b) < radius`:

1. Draw `u` off a pair stream keyed by the world seed, the aspect
   `b"stance"` and the two nodes in lexicographic order, so `edge(a, b) ==
   edge(b, a)` from either side and from any query order (the keying pattern
   of `CivilizationId::stream`, `civilization.rs:87-98`).
2. `u < ALLY_SHARE` -> Ally; `u >= 1 - ENEMY_SHARE` -> Enemy; else Neutral.
   Proposed shares 0.25 / 0.25, Neutral 0.5.
3. **Designated rival.** If `a` has at least one living neighbour and none
   of its drawn edges is Enemy, `a` designates the living neighbour with
   the lowest second draw of their pair stream as its rival. The final edge
   is Enemy if the draw says Enemy or if either endpoint designates the
   other; otherwise the draw stands.

Why this guarantees a rival: a designation only turns an edge Enemy and
never turns one away, so after the rule every living civilization with a
living neighbour has at least one Enemy edge, by construction. The rule is
local (it reads at most the 26 pairs round `a`) and order-free (each
designation is a function of `a`'s own drawn edges, which no designation
changes). A loner (no living neighbour) has no edge and no rival; the
census counts how many there are (section 9).

What a mod civilization gets: the same rule, since it is keyed by the id.
The shares are content on the price-table-like rule set, not on a
civilization (D13 "bloc templates" would be a second step, not this one).

## 6. Events, thresholds and the one-hop rule

### 6.1 Event table (Proposed)

Per living civilization `X`, two saved tallies toward the player: offence
stamps (play clock, each forgotten after `OFFENCE_WINDOW`) and a merit
count. Values are proposals; the owner picks them in section 11.

| Event | Source today | Effect on X | Propagates |
| --- | --- | --- | --- |
| The player neutralizes or destroys an intact hull of X | the neutralize that stamps `NeutralizedCivilizations` (`economy.rs:104`) (`economy.rs:77-104`: `Add<NeutralizedMarker>` with the player or a fleet escort as `LastAttacker`) | offence +1; at `OFFENCE_STEP` (3) within the window: one step down, tally cleared | yes (6.3) |
| The player loots a hull of X it neutralized | dock + Take on a lootable hull | nothing more: the neutralize was the offence | no |
| The player's damage to a Neutral or Ally body of X crosses the 30 hp provocation threshold | `threat.rs:373-376` (`provoke` returns true, shooter is the player) | provoked stamp (section 7); no offence | no |
| The player hands in a job posted by a station of X | `Jobs::completed` poster -> station -> civilization | merit +1; at `MERIT_STEP` (3): one step up, count cleared | no (6.3 option) |
| The player pays the fine for X at a neighbour's station | new service, section 8 | Enemy -> Neutral at once; offences and provoked stamp cleared | no |
| No offence against X for `ENEMY_DECAY` while X is Enemy by events | play clock, catch-up at thaw like the robbery window | Enemy -> Neutral | no |
| The player buys a station of X or hires an escort from X | existing | nothing | no |

Grace: the first two offences inside the window move nothing. A stray shot
never counts: a shot is a provocation (temporary, section 7), and an
offence is a neutralize, which is never stray. Rams count toward
provocation as they do today (`threat.rs:313-317`).

### 6.2 Steps (Proposed)

A step down: Ally -> Neutral -> Enemy; a step up: Enemy -> Neutral -> Ally;
both saturate. A saturated step is not a flip and propagates nothing.

The epic's proof (three ships -> refused) holds from Neutral. From Ally,
three offences reach Neutral and the station still admits; six reach
Enemy. Options for the owner: **(a)** one step per threshold (symmetric
with merits, recommended; the epic proof starts from a Neutral
civilization), **(b)** an offence threshold always lands on Enemy
(consequence: an Ally is lost as fast as a stranger, and the merit ladder
is not the mirror of the offence ladder).

### 6.3 One-hop propagation (Proposed)

When an **event-driven** step changes X's player stance by `d` (-1 down,
+1 up), then for each living neighbour `N` of X (`distance < radius`):

- `edge(X, N) == Ally`: N steps by `d` (follows).
- `edge(X, N) == Enemy`: N steps by `-d` (a rival warms when X cools).
- `edge(X, N) == Neutral`: nothing.

A propagated step never propagates. A step on N saturates like any step.

Direction options: **(a)** both directions propagate (a job for X cools
X's rivals one step; consequence: trade alone can make a rival Enemy, which
the pitfall list warns against); **(b)** only steps down propagate, merits
stay local (consequence: a bloc is never won as a bloc, each Ally is earned
at its own board). Recommendation: **(b)**, "violence spreads one hop,
trade stays local".

Why no flip reaches a non-neighbour: the rule enumerates `N` from the
neighbour query of X alone and changes nothing else, and the propagated
steps are not events. The affected set is `{X} + neighbours(X)`, every
member within the radius of X, and within the 3x3x3 block by section 3.1.
This is a property of the rule, not of the seed; the sweep (section 9)
exists to catch an implementation that breaks it.

## 7. Group retaliation (owner feedback 2026-10-16)

Today a provoked Neutral answers one shooter per ship; its sister ships,
its station and its parked hulls do nothing (`relations.rs:74-83`,
`threat.rs:373-382`). The escorts for sale are parked hulls: inert, Neutral, no AI crew
(`parked_hulls.rs:5-9`), so no retaliation path runs on them at all.

Proposed: a **provoked stamp** per civilization in the player record, the
shape of `NeutralizedCivilizations` (`civilization.rs:124-129`). It is
written the moment `provoke` returns true with the player as the shooter on
any body that carries a civilization. While `clock - stamp < PROVOKED_WINDOW`
the civilization's **effective** stance toward the player is Enemy
whatever its table stance, so:

- every live body of X in the window has its `Allegiance` rewritten to
  `Enemy` and reads as today's Enemy crews do (engage on sight, the station
  refuses with `EnemyStation`); a body thawed while the stamp holds
  materializes Enemy;
- when the stamp expires or is cleared, the bodies return to the table
  stance's side.

Rules: a provocation is not an offence (no tally, no propagation; the
group answer is the cost, and a neutralize during it is still an offence).
The stamp restarts on every provocation. It clears by time
(`PROVOKED_WINDOW`, proposal: the current robbery window, so decision 10's
rule dies instead of living beside this one) or by the fine (section 8).
Only the player provokes a group: a third ship's fire keeps today's pair
answer.

Group options: **(a)** the group is the civilization (recommended; it is
what the Map names and what the save already keys on); **(b)** the group is
the ships of one station or cluster (consequence: a second grouping key
that no record carries today and that a ship in flight leaves behind).

Parked hulls (the escorts for sale): **(a)** they stay merchandise and
never fire, the station's guns and crews answer for them (recommended,
no AI on a parked hull); **(b)** every parked hull gets an AI crew and a
retaliation target (consequence: a hull for sale that can fly off and die
before it is bought, and a hire of a hull that is mid-fight).

## 8. Redemption (the way back)

Three paths, all Proposed; the ladder is one rule in both directions:

1. **Time.** Offence stamps expire after `OFFENCE_WINDOW`; an Enemy by
   events returns to Neutral after `ENEMY_DECAY` with no new offence; a
   provoked stamp expires after `PROVOKED_WINDOW`. Ally never decays. All on
   the play clock with catch-up at thaw, as `forget_before` runs today
   (`economy.rs:116-118`).
2. **The fine.** At any living station that admits the player and whose
   civilization is a neighbour of X, the Station pane offers "Settle with
   <X>": price `FINE_CR`, paid to that station; X goes Enemy -> Neutral and
   its stamps clear. Options for where: **(a)** any admitting neighbour
   (recommended; it always exists when X has a living neighbour, and the
   census counts the loners where only time works), **(b)** only a neighbour
   with an Ally edge to X (the D13 wording; consequence: the share of
   civilizations with no Ally edge, measured in section 9, has no fine at
   all). Price options: **(a)** a flat content value scaled by X's
   advancement like `ship_credits` is (recommended), **(b)** the ask of the
   hulls neutralized (consequence: a per-offence hull record in the save).
3. **Deeds.** Three jobs for X step it up one. An Enemy X cannot be served
   (its station refuses), so deeds take a civilization from Neutral to Ally
   and never out of Enemy; time or the fine does that.

A seeded Enemy (section 4.2 (a)) is redeemed by the same ladder; there is no
"born hostile" that cannot be settled.

## 9. The sweep

### 9.1 What was measured (Measured, disposable spike)

A disposable example, `crates/nova_world_base/examples/civ_census.rs`, was
built and run once on `9b507e6e5`
(`nix develop --command cargo run --quiet --example civ_census -p nova_world_base`),
its output kept at `/tmp/civ-census.md`, and the file and its directory
deleted; the tree holds nothing of it. It read `CivilizationField` through
the public API and prototyped the rules of sections 5 and 6.3 beside it.
Sixteen seeds: the three of `civilizations.rs:370` plus 1, 2, 3, 7, 10, 42,
99, 1234, 2026, 31337, 65536, 123456789 and 987654321. Centre nodes in
`[-4, 4]^3` (729 per seed), neighbours searched in `[-6, 6]^3`.

**Geometry (current code).** Every measured extreme lies inside the bounds
of section 3.1:

| Pair type | Bound km | Measured over 16 seeds km |
| --- | ---: | ---: |
| face maximum | 329.8 | 319.5 to 328.4 |
| two-apart minimum | 400.0 | 401.2 to 405.4 |
| edge-diagonal minimum | 226.3 | 235.0 to 246.4 |
| corner-diagonal minimum | 277.1 | 298.4 to 320.3 |
| any pair, minimum | 160.0 | 162.3 to 167.7 |

Degree (other centroids strictly inside the radius) and what the pairs are:

| Radius km | Degree min / mean / max | Pairs outside the 3x3x3 block | Face pairs of 4374 present |
| ---: | --- | ---: | ---: |
| 300 | 3 / 6.9 / 11 | 0 | 4153 to 4210 (some faces dropped) |
| 330 | 6 / 10.2 / 16 | 0 | 4374 (all, every seed) |
| 340 | 7 / 11.7 / 17 | 0 | 4374 |
| 360 | 10 / 14.7 / 19 | 0 | 4374 |
| 400 | 16 / 19.9 / 24 | 0 | 4374 |
| 480 | 25 / 29.5 / 35 | 43,115 | 4374 |

At 340 km the neighbour pairs are 4374 face, 3995 to 4150 edge and 29 to
58 corner per seed. 300 km drops face neighbours (degree 3 at seed 42);
480 km is the first radius in the sweep that reaches past the block.

The spike ran its censuses below at 330 km, before the face bound in 3.1
was corrected from 329.8 to 339.4 km. Every 330 km neighbour is a 340 km
neighbour, so at 340 km the loner count stays zero and both prototype
invariants (a rival for everyone, locality) hold by the same construction;
the intervention share and the Ally share at 340 km were **not measured**
and would move a little (more neighbours, fewer interventions).

**Living census (current status and side draws).** At 330 km over 8,141
living centre nodes: living neighbours 1 to 14 per node, mean 7.0 to 7.5,
**zero loners** in every seed; 92.5% of living nodes have a living
neighbour on today's Enemy side, 7.1% have none on the Player side. Under
face adjacency instead: 6 loners across the 16 seeds, 20.2% with no
Player-side neighbour. This is the measured cost of option (B) in 3.2.

**Prototype of the seeding rule (section 5, shares 0.25 / 0.5 / 0.25).**
The designated-rival rule had to intervene for 11.8% to 17.8% of living
nodes (14.9% overall); after it, **zero** living nodes with a living
neighbour and no Enemy edge, in every seed. 84.3% of living nodes keep at
least one Ally edge; 2.6% to 5.3% of drawn Ally edges were turned Enemy by
the rule. Per living node the edges average 1.7 Ally, 3.4 Neutral and
2.0 Enemy. Caveat: only centre nodes designated rivals, so a node outside
the centre cube could not designate one; the permanent test must run the
rule over the whole evaluated block.

**Prototype of one-hop propagation (section 6.3, option (b) direction,
player table seeded from today's draw).** Stepping every living centre node
down once, in every seed: **zero** changed nodes outside `{X} + neighbours(X)`,
and at most 13 nodes changed by one event (10 to 13 across seeds).

What this does and does not show: the geometry rows are facts of the
current constants and draw streams. The prototype rows show that the
proposed rules, as the spike wrote them, satisfy their two invariants on
these seeds. They prove nothing about code that does not exist yet.

### 9.2 Permanent proof (Test design, for task 100036)

Unit tests beside the rule (pure functions over `CivilizationField`), over
the three seeds `civilizations.rs:370` uses plus the 13 the spike used, on
nodes in `[-4, 4]^3`:

- `every_living_civilization_with_a_living_neighbour_has_a_rival`: after
  the designated-rival rule, zero living nodes with a living neighbour and no
  Enemy edge.
- `an_edge_reads_the_same_from_both_ends_and_in_any_order`: `edge(a, b) ==
  edge(b, a)` and equal to a fresh field's answer.
- `a_neighbourhood_never_leaves_the_block_round_the_node`: every neighbour
  pair has lattice offset components in `{-1, 0, 1}`, and every face pair is
  a neighbour.
- `a_flip_moves_only_the_civilization_and_its_neighbours`: for every living
  X, step X down from each stance; the diff against the untouched table is a
  subset of `{X} + neighbours(X)`, every member within the radius.
- The epic proof as an asserted systems example: neutralize three intact
  hulls of a Neutral civilization, its station refuses with the Enemy
  reason, a rival neighbour's station lists an escort for hire; a
  civilization two nodes away keeps its stance across a save and a Load.

Nothing above exists. The spike's example was deleted; its prototype is not
the implementation.

## 10. Save and ownership (Proposed)

- `nova_gameplay::civilization` owns `StanceType`, the player table record
  and the stamps, beside `NeutralizedCivilizations`, which it replaces
  (admission reads one record: "effective stance is Enemy"). `DockRefusalType::NeutralizedHull`
  dies; `EnemyStation` covers both reasons.
- `nova_world_base::civilizations` owns the neighbour query and the edge
  draw (pure, seeded); `nova_world_base` stamps events and rewrites the
  `Allegiance` of bodies in the window when an effective stance changes,
  as `economy.rs` stamps the robbery today.
- Bodies need a civilization to be found by: today `TraderMarket.civilization`
  on every intact generated ship and living station (`market.rs:289-291`),
  frozen with the ship (`crates/nova_scenario/src/objects/spaceship.rs:599`).
  A derelict's market is `None` (`sector_ships.rs:298-299`), a parked hull
  and a fleet escort carry none (`fleet.rs`, `parked_hulls.rs`). Crews and
  stations need no new component; a parked hull follows its station's
  civilization and a fleet escort is the player's.
- The save gains the player table (sparse), the offence stamps, the merit
  counts and the provoked stamps on `SavedPlayer`; `validate` refuses a stamp
  ahead of the clock as `NeutralizedCivilizations::validate` does
  (`civilization.rs:172-181`). Format 19 -> 20, unreleased, no migration.
  The edge table is never saved: it is a function of the seed, and changing
  its constants moves every world like changing `civilizations.rs` does.
- Scenario-authored ships keep their authored side; the graph runs only
  while an `OpenWorld` scenario is live, as the world arms today.

## 11. Decisions for the owner

Each with options, one consequence each, and a recommendation (R).

1. Neighbour rule: (A) 340 km metric radius (R), (B) lattice faces, (C)
   400 km. See 3.2.
2. Seed of the player table: (a) today's side draw (R), (b) all Neutral.
   See 4.2.
3. Edge shares: Ally 0.25 / Neutral 0.5 / Enemy 0.25 (R); or an even third
   each (consequence: more rivals, fewer Ally yards near the start).
4. Offence threshold and window: 3 offences in 3600 s of play (R); or 3 in
   the 600 s robbery window (consequence: a paced pirate never flips anyone).
5. Steps per threshold: (a) one step (R), (b) offences land on Enemy. See
   6.2.
6. Propagation direction: (b) steps down only (R), (a) both. See 6.3.
7. Provoked window: reuse the robbery window and delete decision 10's rule
   (R); or a second constant (consequence: two windows to explain).
8. Group of a provocation: (a) the civilization (R), (b) the station's
   ships. Parked hulls: (a) merchandise, never fire (R), (b) crewed.
9. Fine: where (a) any admitting neighbour (R) / (b) Ally-edge neighbour;
   price (a) flat by advancement (R) / (b) hulls' ask.
10. Enemy decay: 2 play-hours with no offence (R); or never by time
    (consequence: a loner with no neighbour station stays Enemy for good).
11. Pirate milestone: see section 12.
12. Names: `StanceType`, `STANCE_NEIGHBOUR_RADIUS`, `OFFENCE_STEP`,
    `OFFENCE_WINDOW`, `MERIT_STEP`, `PROVOKED_WINDOW`, `ENEMY_DECAY`,
    `FINE_CR`; the player record name (`CivilizationStances`) and the
    Station pane line "Settle with <X>". All proposed, none approved.

## 12. The pirate milestone (feasibility)

Decision 13 gives the pirate "turn a civilization Enemy and outlive it: its
last station dead or yours" (`DESIGN.html:653`, `tasks/20261012-100033/TASK.md:8`).
Under this note:

- "Turn Enemy" is feasible: three neutralizes of a Neutral civilization's
  intact hulls within the window. The sparse player table records that the
  flip was the player's, so a seeded Enemy does not count.
- "Its last station dead" is not feasible in v0.16: station sections are
  invulnerable (decision 3) and a living station cannot be neutralized.
- "Or yours" is not feasible either: the buy needs Ally, and an Enemy never
  sells; redeeming it to Ally to buy it undoes the first half.

Options for a feasible milestone, all Proposed, as content on the pirate
role like the other five:

- **(a) Outlaw by deeds.** Turn one civilization Enemy by offences and
  neutralize `hulls` more of its intact hulls while it is Enemy (counts like
  `KillList`). Consequence: reuses the kill counting and the stance record,
  no new world verb. (R, with `hulls: 6` as a starting count for the owner.)
- **(b) Outlaw by time.** Turn one civilization Enemy and keep it Enemy for
  N play-hours (no fine, offences renewed). Consequence: a milestone met by
  waiting, which the pitfall list calls aimless.
- **(c) Wait for station integrity.** Keep the wording and ship the pirate
  charter after stations can die. Consequence: the pirate role stays
  `goal: None` through v0.16.

Per the 2026-10-17 direction a met pirate charter stamps the save and
changes nothing else; the civilization stays Enemy until redeemed like any
other.

## 13. Ideas that are not rules

- A fleet cap that grows with the role, or a leadership stat: an owner
  idea, unapproved; the graph does not touch the cap.
- Bloc templates as content and a stance Map layer: D13 mod seam, later.
- "Becoming a civilization" when the player owns a station
  (`DESIGN.html:641`): not in this note; an owned station is Player-sided
  and admits everybody today.

## 14. Unverified

- A destroy with no prior neutralize: `OnDestroyedEvent` carries no
  civilization and the stamp observer listens to `Add<NeutralizedMarker>`
  only, so whether an outright destruction stamps the robbery window today
  was not traced. The event table treats destroy as an offence; the
  implementation must find its civilization before the despawn.
- `FrozenStation` (`crates/nova_scenario/src/objects/station.rs:388`) was not
  read field by field; the sector-station test asserts `station.civilization`
  on the planned station (`sector_stations.rs:372`), which is the planning
  record, not the frozen one.
- No existing tool prints centroid neighbour distances: `world_civilizations`
  reports reach, shares and counts within 1,250 km
  (`examples/playable/world_civilizations.rs:44-48`). The census numbers in
  section 9 come from the disposable spike only and were not reproduced by a
  second tool.
- Every rule in sections 3 to 8 and 10 to 12 is a proposal. The prototype
  in the spike is not the implementation; its numbers describe the proposal
  under the stated shares and radius, nothing in the tree.
