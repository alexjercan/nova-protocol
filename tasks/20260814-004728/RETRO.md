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

## Rounds 3-6 addendum (2026-08-14, owner-iterated)

- Live feedback drove five more passes: per-scene camera knob ->
  scenario-posed cameras (SetCamera contract + lint), pd_range /
  waypoint_slack / arrival_standoff / SetAmmo authoring knobs, 750-damage
  torpedoes, and finally the Factorio-style backdrop carousel (4 scenes
  hand off in a ring; ambience + scrapyard retired).
- The two hardest bugs were RENDER-layer, not gameplay: (1) backdrop
  self-reloads crashed bevy_ui (the interface resolved its render target
  through the scenario camera being torn down) - fixed by a menu-owned
  UI camera; (2) that overlay camera then blanked the whole 3D view -
  CameraOutputMode's default writes UNBLENDED, replacing the target, and
  its pooled uncleared view even ghosted the boot screen. Bisection
  (disable the camera, diff the frame) found in minutes what log-reading
  could not.
- Duel stalls taught the defeat model's edge: integrity DISCONNECTION
  can kill a ship's flight computer at full health, leaving a hulk that
  is neither destroyed nor neutralized (weapons/thrusters live). Scenes
  that need resolution must either avoid cripple geometry (no LOS
  blockers near the arena, leash wide enough to finish a drifter) or
  carry a watchdog. Both are in.
- Full-ring proof: gauntlet act -> weave rotate -> duel finale -> way-
  station in one unbroken 5-minute run, 0 panics, finale centered.

## Open threads (owner's call, not filed)

- The duel phase averages ~3 min before resolving; tightening rival
  durability or gun cadence would show more of the cycle per menu visit.
- Torpedoes that die to collisions vanish silently; a contact fuze
  (detonate on tangible impact once armed) would read better when a
  stray crosses the rock ring.
- A ship whose flight computer is disabled churns the autopilot
  engage/disengage every frame (log spam + wasted work); a passive-AI
  guard for computer-less ships would quiet it.
- "Crippled but not defeated" (disconnected computer, live guns) may
  deserve first-class defeat semantics one day; today only scene design
  and watchdogs handle it.
- The menu UI camera pattern (IsDefaultUiCamera + alpha-blended output)
  could serve gameplay overlays too if scenario switches ever crash the
  HUD the same way.

## Structural blast pressure follow-up

- The arena exposed a scaling error that small-craft tests could not: radial
  damage was applied once per section, so a large compound hull multiplied one
  warhead into whole-ship deletion.
- The first fix correctly shielded structural targets but still damaged every
  outer cladding piece directly. The rendered arena made that semantic split
  visible immediately. Fix: every Explosive target follows the centre ray;
  only `SectionMarker` blockers consume penetration.
- Avian's `ColliderTransform` was the correct world-position seam. The old body
  transform assigned one distance to every collider on a compound ship.
- Next time, include both target orderings in the first range: fixture before
  section and fixture behind section. Testing only whether fixtures consume
  pressure missed whether sections shield fixtures.
