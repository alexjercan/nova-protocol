# Ammo: stop running dry, warn before the reserve is empty

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.19.0, combat, economy

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- "running out of ammo is rough".

## Decisions
- Owner 2026-11-16: a low-ammo warning (HUD and audio) before the reserve
  is empty is approved. Other resupply ideas below stay candidates.
- Warning rule (agent call, stated 2026-11-16): per ammo item the ship's
  weapons use, LOW when the hold reserve is below one full magazine of that
  weapon, at least 2 rounds (PDC < 500, torpedo < 6, railgun < 2); DRY when
  the reserve is 0. Edge-triggered: one HUD notice and one audio cue on each
  crossing into LOW and into DRY; it re-arms when a restock lifts the
  reserve above the line. A steady HUD mark shows LOW/DRY while it holds.
  Existing sound assets only. The threshold is a const, not content.

- Owner 2026-11-16, resupply approach from `MEASURE.md`:
  - Implement option 2 (stations keep at least 4000 PDC rounds), option 3
    (start kits carry 3000 PDC rounds) and option 6 (craft ammo from ore).
  - Option 4 (kill loot) is already in the game: a destroyed ship can be
    looted. Never drop more loot than the ship carried.
  - Options 1 (on-ship trickle) and 5 (cheaper rounds) are not chosen.
  - Order: the crafting rework (task 20261116-132140), at least the part
    that makes crafting a feature, lands first; then this task's resupply.

## Agent findings
- Magazines and reloads (`assets/base/sections/base.content.ron:1597-1601,2417-2421,3079-3083,3321-3325`):

  | Weapon | Magazine | Reload | Reserve item |
  |---|---|---|---|
  | PDC | 500 at 100 rounds/s | 200 per 3 s | PdcRound |
  | Torpedo | 6 | 1 per 10 s | Torpedo |
  | Railgun | 1 | 1 per 12 s | RailSlug |

- A full PDC magazine is about 5 s of fire. Reload pulls from
  `ShipInventory`; no stock means no progress
  (`crates/nova_ship/src/sections/ammo.rs:14-23`).
- Restock: market buy while docked only. Stations stock their
  civilization's ammo (`crates/nova_world_base/src/station_layout.rs:497-520`).
  No automatic rearm. Station PDCs recharge a private reserve
  (`ammo.rs:490-501`); the player does not.
- Start kits: scavenger, bounty_hunter, pirate and military carry 1000
  PdcRound (two refills); military adds 2 torpedoes; no role carries RailSlug
  (`assets/base/roles/base.content.ron:31-123`).
- A dry player is not neutralized (`neutralize.rs:145-152`) and is still
  hunted.
- v0.17 audit: "Job reward versus ammo cost remains unmeasured"
  (`tasks/20261025-214714/TASK.md:38`).

## Candidate ideas (agent, not approved)
- Make ammo on the ship from raw ore or parts (ties to task 20261116-132140).
- A slow reserve trickle, modelled on the station PDC recharge.
- Ammo in wreck and kill loot.
- Larger start kits or cheaper rounds.
- A low-ammo HUD and audio warning before the reserve is empty.

## Delivery
1. Measure: rounds spent per fight and ammo cost against job pay, per
   combat role. Record the table here.
2. Owner picks the approach from the measured table.
3. Implement, retune content through the builders, regenerate and lint.

## Verification
- Before/after table of rounds per fight, cost per fight and pay per job.
- Unit or ECS proof for any new resupply rule.

## Done when
- The low-ammo warning ships with a proof that it fires before the reserve
  is empty.
- A combat role can finish a typical fight and get back to full fire without
  a long trip, and the player gets a warning before the reserve runs out.

## Progress (2026-11-16)
- Landed: the low-ammo warning (`crates/nova_ship/src/sections/ammo_warning.rs`,
  HUD tag and notice, `ammo_dry` cue, `examples/playable/ammo_warning_sandbox.rs`).
  Verified: nova_ship rule and ECS tests (one LOW, one DRY, restock re-arms),
  nova_hud notice and readout tests, live walk PASS with LOW and DRY frames
  read. Not verified live: audio output, torpedo lines, re-arm by a market buy.
- Measured: `MEASURE.md`. Supply, not price, is the limit. Resupply approach
  waits on the owner (Delivery step 2).
