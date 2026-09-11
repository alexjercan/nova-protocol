# Flight, AI, weapon and destruction figures derive from the hull and the round

- STATUS: OPEN
- PRIORITY: 83
- TAGS: v0.14.0, bug, balance, ai, combat, review

## Goal

Every flight, AI, weapon and destruction figure that is fixed in world
units, seconds or counts but really depends on the hull, the target, or
the round derives from it. Sibling of `20260909-213350` (HUD and camera)
and `20260909-213559` (scenario and editor). From the 2026-09-09 sweep of
`nova_ship` and `nova_gameplay`.

The model to copy is already in the crate: `HullRadius` and
`AttitudeEnvelope` are derived per tick from live sections, the autopilot's
arrival adds the mover's `HullRadius` to the target's size, and
`resolved_arrival_standoff` in `passive.rs:232` does the face-to-face sum.
The findings below are the places that did not follow it. Reference hulls:
`block_skiff` (21 sections, `HullRadius` 4.0 u) and `block_carrier` (2081
sections, 18.3 u).

Each item is a balance change as well as a fix: measure on both hulls
before and after, one commit per item, and record the numbers on the
gameplay feedback ledger (`20260909-213118`).

## Implementation decisions

### Targeting and signatures

- Keep separate `TravelLock` and `CombatLock` slots, but collect contacts once
  per observing ship per frame and resolve acquisition, hold range, relation
  and visibility through one shared policy used by the player and AI.
- Remove the combat lock's 30 s idle decay and its HUD wind-down. A committed
  lock drops only on explicit clear, target loss, range loss, allegiance change
  or occlusion. It never re-locks automatically. A radar sweep held through
  occlusion may acquire again after a fresh dwell.
- Occlusion drops BOTH lock slots. An already engaged GOTO keeps flying: the
  accepted `AutopilotAction::Goto` owns its target independently of the travel
  designation. Explicitly clearing the travel lock still cancels GOTO.
- Compute acquisition range as `min(observer sensor cap, target signature *
  sensitivity)` and held range from the acquisition-time range floor plus the
  existing hysteresis. Damage may shrink fresh acquisition range but cannot
  silently drop a visible committed lock acquired at the stronger signature.
- Keep sensitivity 30 and hold hysteresis 1.15. The player sensor cap is 200 km.
  AI uses the same policy with a 20 km default cap and an authorable
  `AIControllerConfig::sensor_range`; `Some(0 m)` and no live controller mean
  blind.
- Publish every target's computed `LockSignature`: asteroid `100 m + 0.5 *
  BodyRadius`; planet `10 * BodyRadius`; beacon authored as today; torpedo
  `500 m + authored max speed`; unsigned debris keeps its point-blank fallback.
- A ship's installed signature in meters is `280 + 5.5 * HullRadius + 100 *
  ln(1 + drive units) + 100 * ln(1 + controller units) + 100 * ln(1 + live
  weapon count)`. Drive and controller units normalize surviving installed
  authority against their standard prototypes. This targets about 20 km for
  the skiff, 58 km for the carrier, and about 9 km for a bare one-cell hull.
  Throttle and weapon activity do not change it. Reserve a separate transient
  emission term for a future visible stealth mechanic.
- Radar signature is not hit size. Publish a separate target hit radius. Whole
  ship aim uses live `HullRadius`, asteroids use `BodyRadius`, torpedoes use
  their collider envelope, and a component fine-lock uses that section's own
  collider radius. Turret and railgun gates use `atan(hit radius / distance)`;
  point aim with no entity retains a small fixed precision gate.

### Weapons and AI movement

- Pierce rounds carry no layer counter. Remaining power alone bounds their
  travel, as the v0.13.0 release contract already states. A malformed free
  layer must still make positive progress or stop the round.
- Torpedo thrust tapers over the final 15 percent of authored maximum along-nose
  speed.
- Torpedo arming requires BOTH the existing authored condition (`arm_time OR
  arm_distance`) and launcher safety. Snapshot launch `HullRadius`; until the
  launcher dies, require current projectile-to-launcher-COM separation of at
  least `launch HullRadius + blast radius`.
- AI torpedo launch range keeps its three-blast-radius tactical margin as a
  FACE gap: minimum center distance is launcher `HullRadius` plus target
  `HullRadius` plus `3 * blast_radius`.
- AI standoff is an authored face-to-face clearance. Default 1,000 m with a
  250 m face-distance band; `Some(0 m)` permits contact. Preferred center
  distance adds mover and target live `HullRadius` values.
- AI chooses goals and desired motion but never writes controller or thruster
  section inputs. Patrol uses GOTO, idle STOP and gravity orbit ORBIT as today.
  Move the passive sphere-detour planner into generic flight navigation, with
  explicit policy; only AI patrol opts in during this task. Clearance is body
  radius plus mover hull radius plus authored margin.
- Add a generic continuous `MatchVelocity` autopilot action carrying world-space
  desired velocity and optional facing direction. It holds facing while RCS can
  correct velocity, rotates a selected main-drive cluster for larger errors,
  then returns to facing. It holds until replaced or removed and does not
  self-complete. Engage and Evade drive this action through the same cluster,
  balance and spool path as player autopilot.
- Remove fixed AI orbit/chase speed caps. Tactical combat asks the shared player
  stopping solver for radial speed to the standoff band from live braking and
  turn authority. Orbit speed is the reserved lower of the centripetal and
  attitude-turn limits. All decisions use velocity relative to the target.
- Evade runs three COMPLETED legs. A leg advances on achieved velocity/facing,
  with a liveness deadline derived as turn angle over live turn rate plus
  delta-v over live acceleration plus a short burst hold. No authority exits
  Evade rather than sticking there. Each leg targets lateral clearance of
  `HullRadius + 10 m`, with speed and duration derived from that displacement.

### Controller and destruction

- Retune the standard controller from live measurements so the intact
  ten-controller carrier has about 10 percent torque headroom over its
  structural ceiling. Small hulls remain structure-bound; losing enough carrier
  controllers makes it torque-bound, so both regimes occur on shipped content.
- Collision damage has a universal 5 m/s safe contact speed. Above it, use
  Avian's solved contact-pair impulse rather than repeating whole-body effective
  mass per collider. Scale impulse by excess impact speed. Derive dissipated
  energy from that impulse, excess speed and restitution; subtract per-section
  absorption derived from maximum health before the energy damage term. Direct
  impulse still deals damage.
- A severed fragment's raw separation speed is `max(10 m/s, fragment extent /
  2 s)`. Subtract the mass-weighted mean kick so fragments clear their combined
  extents in about two seconds without adding net momentum.
- For detached pieces, compute required clearance as burial depth in the dying
  body plus piece collider radius plus 10 m. Scale both the 20-50 m/s kick and
  0.5 s chunk grace by `sqrt(required clearance / 10 m)`, so minimum kick times
  grace equals required clearance.
- Scale hulk-pyre spatial properties linearly from live `HullRadius` against the
  55.2 m gunship reference and lumens quadratically. Keep timing and root-pyre
  particle count fixed; use one per-instance Hanabi scale property.
- Queue section-pyre requests until the frame's destruction batch is known.
  Budget `clamp(ceil(6 * sqrt(condemned / 53)), 6, 48)`, always keep root pyres,
  and deterministically spread selected section pyres across the wreck instead
  of accepting the first events.
- Group body-bound impact and destruction audio/juice throttles by physical
  structure root, not a hull-sized grid. Resolve a severed fragment, asteroid
  or torpedo to its own body. Use spatial cells only when no root exists. Audio
  and juice share the resolver; same-frame destruction emits from the COM or
  the queued-position centroid.

## Execution plan

1. Create the feedback ledger and land the required combat/destruction range
   skeletons from `20260909-213623`, so every change has a before measurement.
2. Consolidate target contacts and lock lifecycle, then land idle-decay removal,
   signatures/sensor caps and angular hit radii as separate behavior commits.
3. Land Pierce, arming, taper and AI torpedo envelope changes one at a time.
4. Add and prove generic `MatchVelocity`, migrate combat actuation, then land
   standoff, physical speed, evade and generic opt-in avoidance changes.
5. Retune controller torque from the live skiff/carrier crossover table.
6. Land collision, sever, detached-piece, pyre and cue-grouping changes one at
   a time. Record both hulls before and after every item.
7. Regenerate and lint content for authored AI fields; update player, creator
   and developer documentation with each behavior or format commit; finish with
   the affected unit tests and each changed systems range green three times.

## Targeting and weapons

- [x] `nova_ship/src/sections/turret_section/aim.rs:29,34,57`
      `HULL_HIT_RADIUS` 1.6 u over `CLOSE_ENGAGEMENT_RANGE` 100 u makes the
      one fire gate `TURRET_ON_TARGET_RAD` 0.016 rad for every gun: a PDC
      holds fire until inside 0.92 deg of a 366 m target it would hit from
      11 deg off, and passes rounds with 2.4 u of miss against a torpedo.
      Gate on `atan(target radius / distance)`.
- [x] `nova_ship/src/input/targeting/contacts.rs:18` `TARGETING_MAX_RANGE`
      20,000 u for any ship root, while every body's lock range is
      `LockSignature * signature_range_per_unit`: a skiff is designatable
      from 200 km like a carrier. Publish a ship `LockSignature` from
      `HullRadius`.
- [x] `nova_gameplay/src/damage.rs:223` `MAX_PIERCE_LAYERS` 6, "past any
      shipped craft's depth": the carrier is 12 u wide and 35 u long, so
      the backstop is the binding limit. Bound by the power budget alone.
- [x] `nova_ship/src/sections/torpedo_section/mod.rs:261-262`
      `arm_distance` 50 m from the MUZZLE, or-ed with `arm_time` 0.5 s: an
      amidships bay arms with 180 m of its own hull alongside and the
      carrier eats its own warhead. Measure the safety distance against
      the launching hull's `HullRadius`.
- [x] `torpedo_section/projectile.rs:541` `THRUST_TAPER_BAND` 5 u/s below
      `max_speed`: a 60 m/s loitering warhead tapers over 83 percent of its
      envelope. Make it a fraction of the authored speed.

## AI

- [x] `nova_ship/src/input/ai/maneuver.rs:39,44` `AI_STANDOFF_RANGE` 100 u
      and its band, anchor to anchor: carrier vs carrier settles at 63 u
      face to face, a modded hull over 50 u is inside the other ship. Sum
      both `HullRadius` plus an authored standoff; add the override to
      `AIControllerConfig` beside `engage_range`.
- [ ] `maneuver.rs:28-29` `AI_ORBIT_SPEED` 8 u/s and `AI_MAX_CHASE_SPEED`
      20 u/s regardless of drive-to-mass: the carrier alternates thrust and
      brake all fight. Derive from live thruster authority over mass, the
      figures `flight/thrusters.rs` computes.
- [x] `maneuver.rs:306,318` the combat thrust path writes one scalar to
      EVERY thruster: the carrier lights retros and laterals with the mains
      and torques off its command. Route through `cluster_thrusters` and
      `balance_throttles` as the autopilot does.
- [ ] `ai/passive.rs:47,78,269` `AI_AVOID_MARGIN` 20 u past `BodyRadius`
      judged against the centre line: the carrier's flank scrapes the rock
      with 17 m of daylight. Add the mover's `HullRadius`, as `:232` does.
- [x] `ai/railgun.rs:36,43,68` `AI_RAILGUN_ALIGNMENT_COS` 0.99, an 8.1 deg
      cone, commits out to 1080 u where that is 1.5 km of miss. Gate on
      `atan(target radius / distance)`; the test comment at `:276` already
      says so.
- [x] `ai/torpedo.rs:32,68` the launch floor is `3.0 * blast_radius` anchor
      to anchor while the fuze fires at the target's skin: the margin is
      halved between two carriers, and the floor sits inside a 60 u hull.
      Add both `HullRadius`.
- [ ] `ai/threat.rs:39,48` `AI_JINK_INTERVAL_SECS` 1.2 s per evade leg
      with a 41 deg thrust gate: the carrier turns 17 of the 90 deg before
      the next leg and never thrusts, so Evade on a capital is a wallow.
      Budget the leg from the hull's `AttitudeEnvelope` rate.
- [x] `ai/acquisition.rs:91` `AI_TARGET_MAX_RANGE` 2000 u with no authored
      surface: a picket cannot see 40 km, a blinded hulk cannot see 500 m.
      Author it on the controller like `AIEngageRange`.

## Flight

- [ ] `nova_ship/src/sections/controller_section.rs:312`
      `DEFAULT_MAX_TORQUE` 1501 per flight computer, doc: "the largest hull
      reaches only 29.3 m", "all four reference hulls are structure-bound".
      The carrier's arm is 182.9 m and it is torque-bound 4.7x inside the
      regime the constant declared unreachable. Re-derive or author per
      hull class, and fix the doc.

## Destruction

- [ ] `nova_gameplay/src/integrity/core.rs:51,234,278`
      `MIN_IMPACT_SPEED_SQUARED` 0.1 is a 3.16 m/s floor while damage is
      linear in mass: two carriers docking at 0.32 u/s trade about 49 hp
      per contact at hundreds of contacts a frame and shred each other.
      Make it an energy floor, or scale the speed floor by mass.
- [ ] `nova_ship/src/sections/integrity.rs:30,508`
      `SEVER_SEPARATION_SPEED` 1 u/s: two 18 u carrier halves grind for
      18 s before they are apart. Scale with the fragment's extent.
- [ ] `nova_gameplay/src/integrity/explode.rs:86,365` `PIECE_KICK` 2-5 u/s
      with `chunk.rs:85` `CHUNK_GRACE_SECS` 0.5: a section 6 u deep in the
      carrier goes rigid while still buried in 2081 others. Scale both with
      the dying body's `HullRadius`.
- [ ] `integrity/pyre.rs:252` `HULK_PYRE` sized "against a shipped gunship,
      85 m": the carrier dies with a firecracker covering 7 percent of the
      wreck, the skiff is swallowed by a burst four times its size. Scale
      off the root's `HullRadius`.
- [ ] `integrity/pyre.rs:94` `PYRE_FRAME_CAP` 6 per frame "enough for the
      largest shipped hull": a carrier collapse condemns 2081 sections and
      lights six. Scale with the sections the frame condemned.
- [ ] `nova_gameplay/src/juice.rs:56` `JUICE_AREA_CELL` 6 u and
      `audio/mixing.rs:41` `SFX_AREA_CELL` 6 u: a carrier collapse spans
      two dozen cells and fires two dozen trauma kicks in one frame, the
      failure the throttle exists to stop. Size the cell off the dying
      ship's `HullRadius`.

## Judged correct, leave alone

`CONTACT_FUZE` (to the skin), the autopilot arrival and park envelope,
`AI_WAYPOINT_SLACK` (authored and hull-aware), `AI_POINT_DEFENSE_RANGE` and
`AI_ENGAGE_RANGE` (authored), `KINETIC_SHARDS` (derived), the per-frame
budgets (`SHARDS_PER_FRAME`, `CHUNK_ACTIVATIONS_PER_FRAME`,
`MAX_SHOTS_PER_TICK`), `MUZZLE_SPREAD_RAD`, the 8 G limit and the RCS
budget.

## Proof

- The combat ranges in `20260909-213623` run on both hulls: the fire gate,
  torpedo arming, and railgun items each get an invariant there.
- `system_collision_damage` asserts the docking-speed case is free on the
  carrier pair.
- A `bug_` or `system_` range per AI item that stages the carrier: standoff
  face to face, orbit speed held, no retro lit on engage, evade legs
  completing.
- `stress_hull_collapse` extended with the pyre count and the trauma kick
  count per frame on the carrier.
