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

## Targeting and weapons

- [ ] `nova_ship/src/sections/turret_section/aim.rs:29,34,57`
      `HULL_HIT_RADIUS` 1.6 u over `CLOSE_ENGAGEMENT_RANGE` 100 u makes the
      one fire gate `TURRET_ON_TARGET_RAD` 0.016 rad for every gun: a PDC
      holds fire until inside 0.92 deg of a 366 m target it would hit from
      11 deg off, and passes rounds with 2.4 u of miss against a torpedo.
      Gate on `atan(target radius / distance)`.
- [ ] `nova_ship/src/input/targeting/contacts.rs:18` `TARGETING_MAX_RANGE`
      20,000 u for any ship root, while every body's lock range is
      `LockSignature * signature_range_per_unit`: a skiff is designatable
      from 200 km like a carrier. Publish a ship `LockSignature` from
      `HullRadius`.
- [ ] `nova_gameplay/src/damage.rs:223` `MAX_PIERCE_LAYERS` 6, "past any
      shipped craft's depth": the carrier is 12 u wide and 35 u long, so
      the backstop is the binding limit. Bound by the power budget alone.
- [ ] `nova_ship/src/sections/torpedo_section/mod.rs:261-262`
      `arm_distance` 50 m from the MUZZLE, or-ed with `arm_time` 0.5 s: an
      amidships bay arms with 180 m of its own hull alongside and the
      carrier eats its own warhead. Measure the safety distance against
      the launching hull's `HullRadius`.
- [ ] `torpedo_section/projectile.rs:541` `THRUST_TAPER_BAND` 5 u/s below
      `max_speed`: a 60 m/s loitering warhead tapers over 83 percent of its
      envelope. Make it a fraction of the authored speed.

## AI

- [ ] `nova_ship/src/input/ai/maneuver.rs:39,44` `AI_STANDOFF_RANGE` 100 u
      and its band, anchor to anchor: carrier vs carrier settles at 63 u
      face to face, a modded hull over 50 u is inside the other ship. Sum
      both `HullRadius` plus an authored standoff; add the override to
      `AIControllerConfig` beside `engage_range`.
- [ ] `maneuver.rs:28-29` `AI_ORBIT_SPEED` 8 u/s and `AI_MAX_CHASE_SPEED`
      20 u/s regardless of drive-to-mass: the carrier alternates thrust and
      brake all fight. Derive from live thruster authority over mass, the
      figures `flight/thrusters.rs` computes.
- [ ] `maneuver.rs:306,318` the combat thrust path writes one scalar to
      EVERY thruster: the carrier lights retros and laterals with the mains
      and torques off its command. Route through `cluster_thrusters` and
      `balance_throttles` as the autopilot does.
- [ ] `ai/passive.rs:47,78,269` `AI_AVOID_MARGIN` 20 u past `BodyRadius`
      judged against the centre line: the carrier's flank scrapes the rock
      with 17 m of daylight. Add the mover's `HullRadius`, as `:232` does.
- [ ] `ai/railgun.rs:36,43,68` `AI_RAILGUN_ALIGNMENT_COS` 0.99, an 8.1 deg
      cone, commits out to 1080 u where that is 1.5 km of miss. Gate on
      `atan(target radius / distance)`; the test comment at `:276` already
      says so.
- [ ] `ai/torpedo.rs:32,68` the launch floor is `3.0 * blast_radius` anchor
      to anchor while the fuze fires at the target's skin: the margin is
      halved between two carriers, and the floor sits inside a 60 u hull.
      Add both `HullRadius`.
- [ ] `ai/threat.rs:39,48` `AI_JINK_INTERVAL_SECS` 1.2 s per evade leg
      with a 41 deg thrust gate: the carrier turns 17 of the 90 deg before
      the next leg and never thrusts, so Evade on a capital is a wallow.
      Budget the leg from the hull's `AttitudeEnvelope` rate.
- [ ] `ai/acquisition.rs:91` `AI_TARGET_MAX_RANGE` 2000 u with no authored
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
