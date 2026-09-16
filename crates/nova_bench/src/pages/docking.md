# Docking: putting two ports together

A docking port is a sealed hatch on one cell of hull. `flight.dock` clamps
your port to a port on the ship under `me.travel_lock`. The clamp holds the
pose you met in and no more: two ships stay two ships. It is MODAL - while
it holds, your drive, RCS and helm are dead, and a second tap of
`flight.dock` is the only thing that gives them back.

## What has to be true

`me.docking.pair` is the nearest pair of free ports between you and your
travel lock, measured the way the game measures it. It appears with the
lock and reads `null` without one. Three gates, all at once:

- `gap_m` at or under `capture_m` (the shipped port: 10 m), measured FACE TO
  FACE between the two ports - not between the two hulls, and not between
  their centres. Your port face stands ahead of your hull's origin.
- `facing_deg` at or under `capture_deg` (15 degrees): the two port axes
  looking at each other. Roll does not count; any twist docks.
- `relative_mps` under `max_relative_mps` (5 m/s) and `relative_spin_dps`
  under `max_relative_spin_dps`: how the two hulls move relative to EACH
  OTHER. Absolute speed is nothing here.

`gap_ok`, `facing_ok` and `motion_ok` are those gates, and `eligible` is all
three. Tap `flight.dock` only when `eligible` is true. A tap before that
does nothing and reports nothing.

## Flying it

1. Lock the other ship: hold `targeting.radar_hold` with it near bearing
   `[0, 0]` until `me.travel_lock` names it, then release.
2. Point the nose down the other port's axis. Turn with
   `camera.camera_rotate` until `align_bearing_deg` reads near `[0, 0]`;
   that is the facing gate, for a port on your bow. Release
   `flight.rcs_modifier` first: the helm is frozen while it is held.
3. Put your port face on that axis. `their_face_offset_m` is
   `[starboard, up, ahead]` from your port face to theirs. Hold
   `flight.rcs_modifier` and aim `flight.rcs_aim` to push the hull sideways
   and fore-aft without turning it: positive `delta[0]` starboard, positive
   `delta[1]` aft, negative `delta[1]` ahead. The RCS has no up or down: take
   a vertical error out by pitching the nose a few degrees, pushing ahead,
   and squaring up again.
4. Close. Push ahead until `gap_m` runs down, and take the speed off before
   it reaches `capture_m`: a full push for 3 ticks is about 2.5 m/s, and at
   a 70 m gap that is a 30 second coast. The main drive is too coarse for
   this range; if you use it, tap it for a tick or two, never hold it.
5. With `eligible` true, tap `flight.dock`. `me.docking.docked` turns true
   and `connection` names the ship and the two ports. Your controls are now
   inert until you tap `flight.dock` again.

Small acts. Push, run 30 to 60 ticks, read `gap_m`, `their_face_offset_m`,
`facing_deg` and `relative_mps`, push again. Every push keeps going after
the aim stops, so cancel it with an equal push the other way once the
offset reads near zero, and arrive at the face with almost nothing on.
