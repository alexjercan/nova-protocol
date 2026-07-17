# Rework ledger_ch2 encounter design: loadouts, spawn ranges, real cover, aggro stagger, act-split retry

- STATUS: OPEN
- PRIORITY: 53
- TAGS: spike,v0.7.0,scenario,content,balance

Goal: ledger_ch2_claim_jumpers currently opens with two better_turret
magpies (800 dps combined, perfect lead) at ~175u in open void, chains two
reinforced replacements at ~130u the frame the kill counter flips, and the
neutral Dray Mule sits on the crossfire axis so dodged bursts kill the
escort. Rework the encounter so it still demands skill but is fair, WITHOUT
touching AI smarts, weapon stats, or player damage taken.

Direction notes (all levers verified available in shipped RON):
- Loadout discipline: light_turret_section on act-1 magpies (broadside's
  corvettes set this canon); better_turret on at most one act-2 ship.
- Spawn geometry: push spawns to 500-800u so an approach phase exists; wave
  2 never spawns inside 400u of the player.
- Real cover: an invulnerable rock field (AsteroidConfig invulnerable: true,
  asteroid.rs:45) between the spawn bearings and the Dray Mule; destructible
  rocks stay as chaff only.
- Aggro stagger: patrols + leashes (AIControllerConfig patrol/leash) instead
  of four ships converging on a 250u orbit at once.
- Escort geometry: move the Dray Mule off the crossfire axis; consider
  objective text that hints at drawing fire away (enemies never target
  neutrals, relations.rs:50 - the hauler only dies to strays).
- Act-split retry: each act its own hidden scenario chained via
  NextScenario, so defeat retries the current act, not the chapter.

Spike: tasks/20260717-111808/SPIKE.md (findings F1/F2/F3/F4/F6/F7)
