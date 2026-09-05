# Asteroids block radar lock-on

- STATUS: CLOSED
- PRIORITY: 67
- TAGS: v0.13.0,gameplay,targeting

## Goal

Radar lock is a radio link, and rock stops radio. Today the picker sees
through an asteroid: a hostile parked behind a two-hundred-metre rock is as
lockable as one in open space, and a held lock rides through a rock that
drifts across the line. Give the lock a line of sight, so cover is cover.

## What blocks

An asteroid blocks. The occluder is a MARKER, not a type test: `nova_ship`
sits below `nova_scenario` and cannot name `AsteroidMarker`, and "what stops
radar" is a property of a body rather than of the crate that spawns it. So a
`RadarOccluder` component in `nova_gameplay`, inserted by the asteroid
spawner beside `AsteroidMarker`.

Planetoids and ship hulls are NOT in scope here. A planetoid is the obvious
next occluder and should follow once this is proved; a hull is a different
question (a picket screening its wingman) and is not part of this task.

## Where

`collect_lockable` (`crates/nova_ship/src/input/targeting/contacts.rs`) is
the single collection pass every consumer inherits - the radar pick, lock
validity, and the threat set. The occlusion test belongs there, beside the
range gate, so all three answers agree.

The test is a ray from the scanner origin (`live_structure_anchor`) to the
candidate, against occluder colliders only. The candidate itself must not
occlude itself, and neither may the player's own ship.

## Held locks

A lock the rock slides in front of drops, and the drop names itself like
every other one: a `CombatLockDrop::Occluded` branch, its `debug!` line, and
the `CombatLockDropped` message. Without a named branch "why did my lock let
go?" becomes a guess, which is what the drop-branch model exists to stop.

## Cost

One ray per candidate per frame, twice a frame while the radar is held
(`update_radar_search` and `update_contacts_and_locks` both collect). Filter
the spatial query to occluder colliders and only ray-test candidates that
already passed the range gate.

## Proof

- Unit: a candidate behind an occluder is not collected; the same candidate
  is collected once the occluder moves off the line; an occluder BEHIND the
  candidate does not block; the candidate's own collider does not block it.
- A held combat lock occluded mid-flight drops with `Occluded` and writes
  the message.
- A range example that flies a rock across a held lock and reads the drop.

## Notes

Check whether the AI's own targeting reads the same collection pass. If it
does, hostiles lose sight of the player behind a rock too - which is the
right answer, and worth saying out loud in the changelog.

## What shipped

`RadarOccluder` is a marker, as planned, but it lives in `nova_ship`
(`input/targeting/state.rs`) rather than `nova_gameplay`. `nova_scenario`
already depends on `nova_ship` - it inserts `LockSignature` on every asteroid
from the same spawner - so the marker sits beside the rest of the lock's
vocabulary and no crate gained a dependency for it.

It rides the COLLIDER, not the body root. The test is a ray, a ray hits
colliders, and an asteroid's root carries the rigid body while its child
carries the hull. So the asteroid spawner inserts it on the child node.

`RadarScan` (`input/targeting/occlusion.rs`) is the spatial half of the
scanner: a `SystemParam` over Avian's `SpatialQuery` plus the occluder query,
so no caller has to know a ray cast is how the question gets answered.
`collect_lockable` takes it and casts LAST, after the cheap component and
range rejects, and the lock-validity branch asks the same question - so the
radar pick, the held lock and the threat set cannot disagree.

Two rules the ray needs and the tests pin: the target's own hull never hides
it (the ray to a rock's centre goes through the rock), and `solid: true`, so a
scanner standing inside a rock is under cover rather than looking out of a
hollow shell.

Planetoids occlude too - follow-up on the same request, and the reason the
marker is a marker. A world is the same class of body as a rock, so
`planet_scenario_object` wears `RadarOccluder` on its sphere collider exactly
as the asteroid spawner does. Both spawners are pinned by a test that the
marker is on the COLLIDER node and not on the root a lock names.

The AI sees the same way, by the owner's call: the FLAT rule, no authored
switch. `update_ai_target` takes the same `RadarScan`, and `pick_ai_target`
gained an `in_sight` predicate asked LAST, after the range gate - the ray is
the expensive question. `RadarScan` moved from `pub(super)` to `pub(crate)`
with it: it is the ship's scanner, not the player's.

The consequence is deliberate and has teeth. An AI ship that loses its pick
behind a rock has `AITarget` `None`, which `update_behavior_state` turns into
its passive routine - a patrolling raider goes back to patrolling until the
rock clears, and re-acquires the moment it can see again. There is no
occlusion grace on either side.

Point defense is NOT gated. `update_point_defense_target` picks the inbound
torpedo to swat, and a torpedo is a thing to survive rather than a thing to
see; a ship that stopped defending itself because a rock crossed the line
would simply eat the torpedo.

The guns already knew about cover: `ai_line_of_fire_blocked` (`ai/guns.rs`)
has held the trigger behind ANY tangible collider since before this task.
What was missing was the sensor half - the AI held fire but kept the pick,
chased, and waited. Now it loses the target as well as the shot. That gate
maps a collider to its body through avian's `ColliderOf`, so `RadarScan` was
moved onto the same mapping instead of walking `ChildOf`: it is the
authoritative one, it survives nesting, and the two gates now answer alike.

## Proof

Six unit tests in `occlusion.rs` (a rock on the line, off the line, beyond the
candidate, a rock lockable through its own hull, a held lock dropping with
`Occluded`, and a despawned rock stopping nothing - the scan reads the live
tree, not a snapshot). `flight_log.rs` covers the new drop line.

`examples/systems/system_lock_line_of_sight.rs` is the range: it locks a ship
1.5 km dead ahead over an empty sky, flies a rock onto the line and reads the
drop reason, holds the radar again from behind the same rock and finds the
target is not even offered, then pulls the rock away and takes the lock back.
Four invariants, on the roster. Probe: OK, four markers in the timeline.

The AI half is pinned by two tests in `ai/acquisition.rs`: the pure scorer
skips a candidate the `in_sight` predicate refuses and falls to the next one
in the tier, and a live physics world proves a rock between an AI ship and the
player empties `AITarget` and that despawning the rock fills it again. No
range beat: staging it needs a hostile AI ship in the range's scenario, and
one would chase and shoot the player through every other beat.

Re-run against the change, all OK: `system_hud_indicators`,
`bug_neutralized_quiet`, `system_outcomes`, `system_player_path`,
`planet_types`, `compare_planets`, and the `first_shift` orbit and attack
scenes. `nova_ship --lib` is 860 green.

## Not done

**No hysteresis.** A rock grazing the line drops a held lock immediately and
nothing re-takes it; the player holds the radar again. That is the simple
behaviour on purpose. If a rock edge turns out to strobe a lock in real play,
the fix is a short grace on the occluded branch, not a wider ray.

It cuts both ways now. An AI ship skimming a rock edge can drop its pick,
fall to its passive routine for a frame and take it back, and the FSM has no
damping of its own for that. Watch a fight staged inside a belt before
deciding whether a grace is needed; if it is, it belongs on the occluded
branch for both sides rather than in the behaviour machine.
