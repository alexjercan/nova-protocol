# Ammo per fight vs job pay, per combat role (task Delivery step 1)

Measured 2026-11-16 at HEAD `dc38b6c9a`. No tracked file was changed for this.
Run 1 used a clean tree. Run 3 used the parallel low-ammo-warning lane's
uncommitted HUD and audio edits. Those edits do not touch fire, reload or
market code.

## Method
- Empirical: `examples/systems/system_job_ammo_economics.rs`, unchanged.
  Settings: seed 37, `military` start, start-band Escort with 1 raider,
  lavapipe, Xvfb, autopilot. The fight, every round and the Buy are live. Only
  the berthing is a fixture move (`system_job_ammo_economics.rs:36-40`).
  The autopilot holds a 1600 m stand-off and pins its lock on the raider's
  mount (`examples/systems/shared/located_jobs.rs:207-222`).
- Run 2 timed out during the cargo build, before the game started, because a
  parallel lane held the shared build lock. It has no data.
- Analytic: section HP and weapon damage from content. Hit-rate assumptions
  are stated where they are used.
- Older runs from task 20261026-003806 (seed 2, before the raider scaling
  in `329fa2aef`) are context only: 87-1057 PDC rounds per escort, and no
  raider was ever neutralized (`tasks/20261026-003806/TASK.md:15-23`).

## Measured fights (seed 37, Escort, 1 start-band raider, reward 778 cr)

| Run | PDC fired | Damage on raider | HP per round | Raider | Pool after | Buy back |
|---|---|---|---|---|---|---|
| 1 | 582 | 116 / 1540 HP | 0.20 | alive | 1418 of 2000 | 582 rds, 65 cr |
| 3 | 1971 | 726 / 1540 HP | 0.37 | alive | **29 of 2000** | 1971 rds, 217 cr |

- Sources: run logs `ammo economics: fight ledger`, `buy` and `net`, and
  `escort job: the hauler reached its mark` (run1 `run.clean.log:380,416,439,440`;
  run3 `run.clean.log:395,433,456,457`). Logs are under
  `/tmp/claude-1000/-home-alex-personal-nova-protocol/3f34df46-822e-4bc8-b585-f58cc90d779d/scratchpad/ammo-measure/run{1,3}/`.
  That directory is temporary, so the key values are copied in this file.
- 0 torpedoes fired: the walk fires turrets only. Neither run neutralized the
  raider. The job was done because the hauler reached its mark.
- Run 3 emptied the hold in one escort. Its Buy took the posting station from
  2000 to 29 PDC rounds (`run3 run.clean.log:456`). That station's stock is
  2000, which is the generation floor (`crates/nova_world_base/src/station_layout.rs:128,518-520`).
- Live ask was 11 cr per 100 rounds (reference 10, civilization factor 1.096)
  (`run1 run.clean.log:428`).
- HP per round = damage landed / rounds fired. A PDC round does 4 HP at the
  reference closing speed (`assets/base/sections/base.content.ron:1591`).
  0.20-0.37 HP per round is about a 5-9 % hit share (derived).

## Table per role

| Role (hull, guns) | Fight jobs early (mix weight) | PDC per fight | Cost per fight | Pay per job | Kit (usable PDC) | Fights per kit |
|---|---|---|---|---|---|---|
| military (patrol_picket, 2 PDC + torpedo bay) | Escort 1, Distress 3, Supply 3 /13 | **582 / 1971 measured** | 65 / 217 cr measured; 1 torpedo = 400 cr | Escort 778 measured; 350-1600 range | 2000 PDC + 8 torpedoes | **1.0-3.4 measured** |
| bounty_hunter (interceptor, 2 PDC) | Neutralize 4 /13, only when a hostile stands in the 27-cell cube; Escort 1 /13 | Escort: as military (same guns). Neutralize: derived >= 1400 at the measured hit share (2+ mounts x 130 HP), unverified | 150-290 cr derived | Neutralize 500-900; Escort 350-1600 | 2000 | about 1 Neutralize (derived) |
| pirate (raider, 2 PDC) | Outlaw: intact ships of a civilization turned Enemy | unmeasured; a civilian is neutralized when its last thruster dies (70 HP each, count varies) | unmeasured | hold loot + 95-380 cr ship credits (derived) | 2000 | unverified |
| scavenger (salvage_hauler, 1 PDC) | Escort 1, Neutralize 1 /12 (Salvage mix) | derived 290-985 (half the fire rate over the same fight), unverified | 32-108 cr derived | Escort 350-1600 | 1500 | 1.5-5 (derived) |

Rail slugs: no start hull mounts a railgun, so no role uses them (`assets/base/ships/base.content.ron`, counted
per design). RailSlug costs 40 cr (`assets/base/items/base.content.ron:23-30`).

### Sources for the table
- Kits: `assets/base/roles/base.content.ron:25-37,69-81,91-104,114-126`.
  Usable = magazines (500 per PDC, 6 per bay) + hold
  (`assets/base/sections/base.content.ron:1597,2417`).
- Job mix: `crates/nova_gameplay/src/jobs.rs:952-953` (Procure, Recover,
  Escort, Distress, Neutralize, Supply).
- Pay (derived from code):
  - Escort = cargo/2 + 150 per raider + fee 50-250, with cargo 300-2400
    (`crates/nova_gameplay/src/jobs.rs:118-124,144,2254-2265`).
  - Neutralize = 400 + 2 x fee (`:139,2168`).
  - Distress = 200 + 2 x fee (`:126,2274`).
- If the 27-cell cube holds no hostile, Neutralize falls back to Procure
  (`jobs.rs:2124-2128`). The seed-37 board saw "0 hostile(s)"
  (`run1 run.clean.log:157`).
- Start-band raider: Scavenger role, at most 40 sections and 1 turret
  (`crates/nova_gameplay/src/jobs.rs:155-163`). It is beaten when it has no
  working weapon or loses its computer
  (`crates/nova_gameplay/src/integrity/neutralize.rs:145-152`). The turret has
  130 HP (`base.content.ron:1413`), which is 33 hits.
- Neutralize target: an Advancement-size hostile. An Armored hull at
  advancement 0.1 has 2 weapon slots
  (`crates/nova_world_base/src/ship_layout.rs:1239-1245`).
- Price: PdcRound 10 cr per 100, Torpedo 400 (`assets/base/items/base.content.ron:13-40`).
  The ask can rise to x1.5 when stock is low, and to x1.25 more at an Armored
  station (`assets/base/market/base.content.ron:2-22,104-111`).
- Ship credits: `crates/nova_world_base/src/sector_ships.rs:111`, interpolated
  to advancement 0.1.

### Restock trip
- Rearming happens only by a market Buy while docked. Ammo cost is 8-28 % of
  the escort pay, so the price is not the problem.
- An escort ends at its mark 10 km from the poster
  (`crates/nova_world_base/src/jobs.rs:102`), so the player flies about 10 km
  back. A 2-PDC hull that is fully dry needs 2000 rounds. That is the whole
  stock of a floor station: run 3 left it 29 rounds.
- The stock comes back over the market's `recovery_seconds: 1800`
  (`assets/base/market/base.content.ron:23`). Unverified: whether 1800 s is
  the time to full stock.
- **The binding limits are hold capacity, station stock, and a 5-9 %
  autopilot hit share. Credits are not a limit.**

## Assumptions
- One autopilot pilot on one seed and one raider, with 2 runs. The spread is
  3.4x on the same job, so do not average the runs. Human aim is not measured.
- Derived rounds for other roles scale the measured HP per round (0.20-0.37)
  by fire rate and target HP. An analytic best case, with 25 % of rounds
  hitting the mount: 130 HP / (4 x 0.25) = 130 rounds per start-band raider.
  This is unverified.
- A torpedo (750 blast, `base.content.ron:2397`) is likely to neutralize a
  1-turret raider in one hit. This is unverified. It costs 51 % of the 778 cr
  reward.

## Resupply options, ranked
1. **Reserve trickle up to one magazine, PDC only.** It uses the same model
   as the station PDC recharge (`crates/nova_ship/src/sections/ammo.rs:490-501`).
   - Table effect: fights per kit stop going to 0. After run 3 the hull
     regains 500 rounds (one full burst) with no trip.
   - It is the only option that meets the Done-when "back to full fire
     without a long trip".
   - The rate is a content or const knob. Example: 500 rounds over 300 s
     (unverified feel).
2. **Raise the station PDC floor** from 2000 to 4000-6000 (outside the task's
   candidate list). The stock floor is in `station_layout.rs:128`.
   - Table effect: one dock refills a fully dry 2-PDC hull, and the station
     keeps enough for a second refill. Today one Buy after run 3 empties it.
3. **Larger start kit**: 1000 to 3000 PDC, +400 kg. The picket had 1316 kg
   free at run 1's hand-in (`run1 run.clean.log:428`).
   - Table effect: fights per kit go from 1.0-3.4 to 2.0-5.1. It is a
     one-time cushion only.
4. **Kill loot.** It mostly exists already: a neutralized ship can be looted
   with Take (`crates/nova_interface/src/inventory/app.rs:2131-2140`). Raider
   holds draw PdcRound at weight 2/9 (`sector_ships.rs:514-521`).
   - Table effect: none for today's escorts, because 0 of 2 runs (and 0 of 7
     older runs) killed the raider.
   - It helps only after aim or hit share improves. A guaranteed stack
     (example: 300-500 rounds per raider) would make it count.
5. **Cheaper rounds.** Table effect: cost per fight drops from 65-217 cr.
   That cost is already small next to 778 cr of pay. It does nothing about
   running dry.
6. **Make ammo on board from ore or parts.** It depends on task
   20261116-132140. It has the largest scope, and its table effect is the
   same as option 1 but gated on cargo.

## Recommendation
Ship the approved warning, then do option 1 (PDC trickle to one magazine)
and option 2 (station floor of at least 4000).
- Run 3 shows that one start-band escort can use all 2000 rounds.
- After that fight, the posting station can sell back only one load.
- The money margin is already large (611-763 cr net per escort).
- So the fix is availability, not price.
- Leave torpedoes and slugs as bought items: they are the expensive,
  deliberate choice.

Separately, the 5-9 % autopilot hit share is a likely root cause of the
large spend. An older telemetry run found the muzzle off target in 22 of 22
samples (`tasks/20261026-003806/TASK.md:21`). Measure the hit share of
human or bench play before any retune of rounds per fight.
