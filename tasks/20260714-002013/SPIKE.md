# Spike: authoritative code-sourced content for the remaining wiki pages

- DATE: 20260714-002013
- STATUS: RECOMMENDED
- TAGS: spike, web, wiki, content

## Question

The wiki infrastructure, the Ship sections pages, the Keybinds page and the
tutorial trim are done. Eight pages remain as coming-soon stubs: HUD, Flight &
autopilot, Targeting & radar, Combat & weapons, Gravity wells, Factions,
Scenarios, and Modding. The keybinds work proved that hand-written control docs
drift from the code. So: what is the *authoritative, code-sourced* content for
each remaining page, captured well enough that `/flow` can author accurate pages
without re-deriving it - and where does the current manifest wording already
disagree with the code?

## Context

Pages are manifest-driven (`web/src/wiki-pages.ts` + `web/src/wiki.ts`), one HTML
file each under `web/src/wiki/`, registered via `wikiPage()` in
`web/webpack.config.js`. The manifest already carries a `summary`/`headings` per
page; some of those summaries were written from memory and are wrong (see
Corrections). Content was audited directly from `crates/` by four parallel code
audits; every fact below has a file reference. bevy-common-systems (gravity math,
some HUD substrate) is a git dep with a local checkout at
`~/personal/bevy-common-systems`.

## Options considered

- **Author from memory / changelog.** Fast, but this is exactly what produced
  the keybinds errors (fabricated "RT2", wrong bumper/trigger) and the flight
  manifest summary below. Rejected.
- **Code-audit each subsystem, then author (recommended).** Pull the real
  enums/constants/mechanics with file refs, capture them here, then author. More
  up-front work, but the pages come out correct and the spike doc stays a
  durable reference for later edits.
- **Do nothing / leave stubs.** The pages are already non-navigable stubs, so
  the site is not broken - but the user asked to finish them, and half a wiki
  undercuts the "real wiki" goal. Rejected.

## Recommendation

Author the eight pages from the code-sourced outlines below (Option B). Fix the
manifest summaries flagged under Corrections as part of the work. Keep the same
manifest-driven page shape; `Gravity wells`/`Factions`/`Modding` are short,
`Flight & autopilot`/`Targeting & radar`/`Combat & weapons`/`HUD`/`Scenarios`
are the meaty ones. Consider whether `Combat & weapons` warrants section
sub-pages (turrets/torpedoes/damage) like Ship sections - probably not yet; one
page with `<h2>`s is enough.

### Corrections the code forces (manifest is currently wrong)

- **Flight & autopilot**: manifest summary says "assisted and Newtonian flight,
  the RCS budget". The code has **no flight-assist toggle, no separate Z
  Newtonian mode, and no RCS budget resource**. Manual flight *is* pure
  Newtonian (momentum persists); the **autopilot verbs are the assist**. There
  is an optional soft **speed cap** (`FlightSpeedCap`, per-ship, e.g. the
  shakedown 25 u/s starter governor), not a global FA. Rewrite the summary.
- **Keybinds/other pages** already corrected in prior work; keep those.

### Page outlines (authoritative; file refs are under crates/nova_gameplay/src unless noted)

**HUD** (`hud/mod.rs`)
- Visibility tiers: `HudVisibility` = All / Minimal / None (mod.rs:44-71),
  cycled by grave or gamepad Select, one-way All->Minimal->None. Widgets carry a
  `HudTier` = Instrument (shown at All+Minimal) or Chrome (All only) (mod.rs:79-87,
  454-475). Instrument: velocity sphere, flight chips, autopilot marker, maneuver
  instruments, lead pips, lock crosshairs, target inset. Chrome: keybind hint
  cluster, verb cues, component-lock UI, edge indicators, objective markers.
- Velocity sphere (`hud/velocity.rs`): cone+sphere from linear velocity; white/blue
  manual, cyan when autopilot engaged. Gravity variant is a yellow sphere, hidden
  in flat space.
- Flight chips (`hud/flight_status.rs`): speed chip `"{v:5.1} u/s"` always on;
  mode chip `"AP {STOP|GOTO|ORBIT} - {ALIGN|BURN|HOLD}"` only while autopilot
  engaged.
- ORBIT ring + radius spoke (`hud/maneuver_instruments.rs`): world-space torus at
  the plan radius on the r x v plane; thin spoke from well centre to ship with a
  radius chip, during ORBIT Hold.
- Keybind hint cluster (`hud/keybind_hints.rs`): bottom-left, rows STOP/GOTO/ORBIT/
  CANCEL/RADAR/COMPONENT, `"[KEY] VERB"`, contextual (hidden when the verb cannot
  act), gold ~1 Hz pulse when a scenario emphasizes a row.
- Lock crosshairs (`hud/lock_crosshairs.rs`, `hud/torpedo_target.rs`): white travel
  crosshair (min 40px, 1.35x scale), red combat reticle (min 32px) with a right-edge
  readout `DST {m}` / `CLS {+u/s}` / health bar and a focus meter that fills during
  fine-lock dwell. A 48px hollow radar box shows while the radar gesture is engaged,
  slot-coloured. Tap-clear spawns a 0.7s growing "unlatch ghost"; a denied lock
  flashes a red box 0.35s.
- Target viewfinder inset (`hud/target_inset.rs`): 256px top-right panel, a live
  512px render-to-texture of the combat lock via an offscreen camera. `InsetZoomable`
  bodies only (ships/torpedoes/asteroids, not beacons) -> otherwise a NO-SIGNAL
  pulsing panel. Border hot-red while weapons hot / steel while safe; four armed
  corner ticks when hot; faction caption (name + relation, tinted). Kill cam: on the
  framed target's death the panel freezes the final pose ~2.0s (KILL_CAM_SECS) then
  closes. Fine-locked section glows (emissive shell 1.14x) in both inset and main view.
- Screen-indicator substrate (`hud/screen_indicator.rs`): all projected HUD uses one
  widget family - anchor (Entity or Point) x sizing (Fixed / ApparentSize{min,scale} /
  WorldRadius) x offscreen policy (Hide / ClampToEdge{margin} with an arrow child).
  Runs in PostUpdate after chase-camera sync, before UI layout. Turret lead pips
  (`hud/turret_lead.rs`): 8px amber square per turret at its computed intercept,
  red when weapons hot.

**Flight & autopilot** (`flight.rs`)
- Manual = pure Newtonian: rotation via the controller PD torque, W/Space/right-
  trigger = analog main-drive burn (0..1); forward-aligned thrusters (cos >= 0.9)
  sum into the main drive; inputs spool exponentially (up 6/s, down 10/s).
- Optional soft speed cap (`FlightSpeedCap`, per-ship): burn tapers to zero over the
  last 20% of the cap along the burn axis only; turning/braking never capped.
- Centre-of-mass thrust balancing: a small convex QP allocates per-engine throttles
  (0..1) to deliver commanded forward thrust while nulling net torque, recruiting
  off-axis engines for counter-torque (lateral penalty 0.05, ~5% residual torque
  the PD mops up). Thruster groups cluster by direction (cos >= 0.9).
- Mass-legible handling: hull turn rate = `sqrt(pi*max_torque/inertia)/2`, scaled by
  turn_rate_scale 0.9, clamped 10..240 deg/s. Stripped ships snap, heavy builds
  lumber.
- Autopilot writes the SAME seams as the player (`ControllerSectionRotationInput`,
  `ThrusterSectionInput`) - diegetic, real plume/impulse; disengages on any manual
  flight input (thruster key, burn, rotation, or Z). Verbs (keys in the input rig):
  - STOP (X / South): flip retrograde, brake to rest; phases Align->Burn->Done at
    speed <= 0.2 u/s; accounts for gravity along velocity.
  - GOTO (G / North): burn to the travel lock, flip at the arrival curve, park at
    standoff = 50u + target radius (kept outside the 30u torpedo blast), measured
    from the surface; tracks a drifting target; min approach 1.5 u/s.
  - ORBIT (O / DPadDown): circularize/station-keep the dominant well; sticky plan
    computed once (ring radius clamped into a stable band, plane from r x v);
    desired speed `sqrt(mu/r)`; Hold at velocity error <= 0.8 u/s; never self-
    completes, held by micro-burns until broken/Z/loss of well or engines.
- Key tunables live on the reflected `FlightSettings` (flight.rs:279-416):
  spool 6/10, decel_margin 0.85, arrival_standoff 50, align_cos 0.95,
  stop_speed_epsilon 0.2, min_approach_speed 1.5, turn_rate_scale 0.9,
  turn_rate 10..240 deg/s, orbit_hold_enter/exit 0.8/1.2.

**Targeting & radar** (`input/targeting.rs`)
- Hold CTRL (or DPadUp) sweeps: live-retargets the best body on the look ray; the
  slot latches at RADAR_TAP_SECS = 0.25s from the current stance and is written live
  while held; release past threshold commits. Cone half-angle 18 deg
  (TARGETING_CONE_HALF_ANGLE_DEG).
- Slots: TravelLock (white, feeds GOTO) when weapons lowered; CombatLock (red, feeds
  guns/torpedoes/fine-lock/inset) when raised. `WeaponsHot` = raised OR a combat lock
  exists.
- Fine-lock: after FOCUS_TIME = 1.5s focused, sections cycle nose-to-tail
  (bracket/scroll/DPad); Snap mode follows the crosshair with SNAP_HYSTERESIS 0.75;
  manual cycle pins for COMPONENT_PIN_WINDOW = 2.0s.
- Staged tap-clear (RadarClearInput): lowered clears combat then travel (disengaging
  GOTO); raised clears only combat. Combat lock idles out after COMBAT_DECAY_SECS =
  30s; locks also drop on death / out-of-range / hostile->non-hostile.
- Ranges: ships & wells up to 20,000u (TARGETING_MAX_RANGE); committed torpedoes
  2,500u; signed bodies `signature * 30 u/unit`; unsigned debris 5u; range hysteresis
  1.15x.

**Combat & weapons** (`sections/turret_section.rs`, `sections/torpedo_section/`, `damage.rs`)
- Turrets: articulated base/yaw/pitch/barrel/muzzle; true intercept lead solved in
  the shooter's frame (`lead_intercept_point`); fire_rate rounds/s; yaw/pitch speeds
  and pitch limits (e.g. -30..+90). Rounds are sensor projectiles (mass ~1e-6, no
  shove), expended on first contact, carrying `ProjectileDamage{amount, kind}`. PDC
  (`better_turret`) per-hit damage retuned to 4.0.
- Torpedoes: proportional-navigation guidance (`pn_steer_direction`, nav_constant ~3);
  arming gate (arms after min_time e.g. 0.5s OR min_distance e.g. 5u); proximity fuze
  at blast.radius*0.5; blast is an `Explosive` area (e.g. radius 30, peak 100, linear
  falloff). Point defense prioritizes inbound torpedoes.
- Typed damage (`damage.rs:32-49`): DamageType = Kinetic / ArmorPiercing / Emp /
  Explosive. SectionDamageClass = Hull / Thruster / Controller / Turret / Torpedo.
  Resistance grid (`resistance()`, Kinetic always 1.0):
  | class | Kin | AP | EMP | Exp |
  | Hull | 1.0 | 1.5 | 0.1 | 1.0 |
  | Thruster | 1.0 | 0.75 | 0.25 | 1.5 |
  | Controller | 1.0 | 1.0 | 3.0 | 1.0 |
  | Turret | 1.0 | 1.75 | 1.5 | 0.5 |
  | Torpedo | 1.0 | 1.0 | 1.25 | 1.25 |
  Each turret has a `LoadedBullet{kind,damage}` slot; ammo readout is colour-coded by
  type (`damage_type_color`). Turret/torpedo fire bindings are per-ship, rebindable in
  the editor, default LMB; both gated by `WeaponsHot`.

**Gravity wells** (`gravity.rs`)
- Large asteroids carry a `GravityWell`; ships, torpedoes and turret rounds opt in via
  `GravityAffected`; wells never pull wells. Force: `a = mu/r^2` (mu = surface_gravity
  * body_radius^2), clamped to the surface value within body_radius+surface_margin
  (1.0), smoothstep-faded to zero over the outer 15% of the SOI, zero beyond. SOI =
  soi_factor 8 x body_radius. Defaults: surface_gravity 6 (cap 10), soi_factor 8,
  fade_fraction 0.15, switch_hysteresis 1.1. Dominant well: strongest pull wins with
  10% hysteresis (`DominantWell`); ORBIT flies the dominant well at `v = sqrt(mu/r)`.

**Factions** (`relations.rs`)
- `Allegiance` = Player / Enemy / Neutral (on ship roots, copied onto projectiles at
  spawn). `relation(a,b)` -> Own (same combatant side), Hostile (Player vs Enemy),
  Neutral (any Neutral/unmarked - asteroids, debris). Drives: signature acquisition
  (lockable at all), projectile allegiance (rounds don't hit own side, persist if the
  shooter dies), reticle/inset relation tint, AI target selection & threat memory.
  Player/AI ships get their allegiance via `#[require(Allegiance = ...)]` on
  `PlayerSpaceshipMarker` / `AISpaceshipMarker`.

**Scenarios** (`crates/nova_scenario/src/`, shipped in `crates/nova_assets/src/scenario.rs`)
- A scenario places objects (asteroids, spaceships, nav beacons, salvage crates,
  invisible trigger areas) and wires objectives through Events -> Filters -> Actions
  over typed Variables (String/Number/Boolean, arithmetic + comparisons).
- Events (`events.rs`): OnStart, OnUpdate, OnDestroyed, OnEnter, OnExit, OnOrbit
  (held 5s, re-fires every 5s), OnTravelLock, OnCombatLock.
- Filters (`filters.rs`): Entity (by id / type_name "asteroid"|"beacon"|
  "salvage_crate" / other_id / other_type_name), Conditional (Not/Or/And), Expression
  (variable comparisons).
- Actions (`actions.rs`): DebugMessage, VariableSet, Objective / ObjectiveComplete,
  ObjectiveMarkerAttach / Detach, HintEmphasisSet / Clear, SpawnScenarioObject,
  DespawnScenarioObject, SetSpeedCap, SetControllerVerb (grant/withhold GOTO/STOP/
  ORBIT), CreateScenarioArea, NextScenario (optional linger).
- Shipped: Shakedown Run (starter New Game tutorial), Asteroid Field (combat/gravity
  sandbox) + Asteroid Field - Next (loop demo), Menu Ambience (main-menu backdrop, AI
  on ORBIT, no gameplay).

**Modding** (coming-soon; `crates/nova_scenario/`, `crates/nova_assets/`)
- Honest state: scenario authoring is **code-only today** - scenarios are Rust
  functions building `ScenarioConfig` with the full event/filter/action/variable
  vocabulary. There is NO shipped data format (no serde/RON/yaml dep; no deserializer).
  A data-driven format (e.g. RON) to decouple authoring from the codebase is the
  planned next step; the page says so and "documented here once it lands".

## Open questions

- Should `Combat & weapons` become an overview + turret/torpedo/damage sub-pages like
  Ship sections? Recommend one page for now; revisit if it gets long.
- How many `.figure` image placeholders per page, and which diagrams (gravity fade
  curve, resistance grid, orbit geometry, HUD annotated shot)? Settle at authoring.
- Manifest `summary`/`related`/`tags` tweaks per page as content firms up.

## Next steps

Direction-level tasks (already seeded by spike 20260713-225157; this spike supplies
their authoritative content):

- tatr 20260713-225338: gameplay-system pages - remaining are HUD, Flight &
  autopilot, Targeting & radar, Combat & weapons, Gravity wells (Sections + Keybinds
  already shipped). Fix the Flight manifest summary (no FA toggle / Z mode / RCS).
- tatr 20260713-225353: world/meta pages - Factions, Scenarios, Modding (coming-soon).

## Fix record

(Content authored per page will be recorded by the implementing tasks.)
