# Systems ranges: flight legs, gravity wells and the AI patrol

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.14.0, testing, examples, flight, ai

## Goal

Live proof for the traversal verbs and the AI: the autopilot legs, gravity
wells, and a patrol that survives its target dying. From the 2026-09-09
coverage map: the autopilot is proved only by `flight/tests/` unit rigs and
unasserted screenshot loops; gravity has 19 unit tests and two screenshots
that assert nothing; `input/ai/passive.rs`, `behavior.rs`, `threat.rs` and
`maneuver.rs` have zero live coverage.

Rules: `examples/systems/README.md`. Each range gets its `[[example]]`
block and its roster slugs in `crates/nova_probe_cli/tests/catalog_drift.rs`,
and lands in the `world` shard of `20260909-213100`. A range that finds a
defect fixes it in the same lane and records it here. Run every range on
`block_skiff` and `block_carrier`; the AI figures task
(`20260909-213708`) needs the carrier numbers.

## Ranges

- [ ] `system_flight_legs`: one ship flies STOP, GOTO, GotoPos and ORBIT end
      to end around a well, with a second order pre-empting the first.
      Invariants: the GOTO leg arrives inside its band; the flip happens
      exactly once; STOP zeroes the velocity; ORBIT holds its radius for a
      full lap; a second order pre-empts the first; the handback does not
      lurch the camera.
- [ ] `system_gravity_wells`: a ship and a stream of rounds crossing one
      planet's sphere of influence and into a second well. Invariants: a
      body inside the sphere falls toward it; the dominant well is the
      strongest one; leaving the sphere releases the body; a round's path
      bends and the straight lead pip misses low; a held orbit keeps its
      radius for a lap.
- [ ] `system_ai_patrol`: an AI ship walking patrol legs, a hostile that
      appears, is fought, and dies. Invariants: the patrol walks its legs;
      the detour steers around a sized body with the mover's own radius
      clear; acquisition takes the hostile; the target's death returns the
      ship to its patrol; the leash pulls it back inside its territory; the
      threat memory decays. Stage Evade on the carrier once
      `20260909-213708` lands the leg budget.
- [ ] `system_engine_groups`: a burn that has to pick and blend thruster
      groups on a hull with mains, retros and laterals. Invariants: the
      cheapest group is chosen; torque is nulled across the blend; a
      severed drive re-balances the next tick. The AI thrust path item in
      `20260909-213708` reuses this rig.
- [ ] `system_helm_orders`: a scenario order taken by the AI, pre-empted by
      a threat, and given back. Invariants: the order is flown; the threat
      pre-empts it; the order resumes when the threat clears; the mission
      handover logs once. Replaces the assert-free `loop_helm_orders` as
      the proof; the loop stays a capture.

## Done when

Five ranges green three times in a row in the `world` shard, roster count
updated, every defect they found fixed and listed here.
