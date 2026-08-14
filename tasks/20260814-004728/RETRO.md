# Retro

## What shipped

- Engine: mesh-fragment debris lifetime (30 s), deterministic asteroid
  silhouette seeds, passive-flight obstacle avoidance (with corner-leg
  re-validation and in-bubble waypoint handling), authorable
  `engage_range`, authorable torpedo `projectile_health`, a
  `heavy_torpedo_section` prototype, a shot-down debug log, and the
  `NOVA_MENU_BACKDROP` capture knob.
- Content: menu_gauntlet, menu_weave, menu_duel backdrops; all three
  verified LIVE under Xvfb with log counters and screenshots.

## What went well

- Writing the contract as lib tests first (avoidance geometry, bay
  durability, engage-range transition) made the live-run failures purely
  about staging, not mechanics.
- The live runs found four real design errors review missed: passive
  re-engage vs battery parking, corner legs flown blind, the duel line
  crossing the planetoid, and SOI drag on the arena.

## Pain / next time

- The app's debug log filter (nova_core log_filter_str) does not include
  nova_ship, so AI/weapon debug lines are invisible in default debug
  runs; an hour went to a false "battery never fires" conclusion drawn
  from that silence. Next time: check the filter list BEFORE trusting
  log absence; consider adding nova_ship/nova_menu to the list (left
  untouched here - out of lane).
- Killing `nix develop --command cargo run` needs the process GROUP or
  the game grandchild leaks past `kill`/`pkill -P`.
- Torpedo/turret systems log almost nothing; the one debug line added to
  shot-down despawn was enough to make PD outcomes greppable. More
  outcome-level logs (launch, detonation) would make future scene
  verification cheaper.

## Open threads (owner's call, not filed)

- The duel phase averages ~3 min before resolving; tightening rival
  durability or gun cadence would show more of the cycle per menu visit.
- Torpedoes that die to collisions vanish silently; a contact fuze
  (detonate on tangible impact once armed) would read better when a
  stray crosses the rock ring.
