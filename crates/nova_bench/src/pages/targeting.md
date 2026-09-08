# Targeting: the radar, the locks and the stance

A lock is how the ship knows what you mean. Two slots, and which one the
radar writes depends on the weapons:

- TRAVEL lock, weapons lowered. White. What `flight.autopilot_goto` flies to.
  A ship, a beacon, a rock or a planet can all take one.
- COMBAT lock, weapons raised. Red. What the turrets track. Only a ship with
  an allegiance can take one: a derelict flagged as nobody's cannot be locked.

## Acquiring

Put the target near bearing `[0, 0]`, hold `targeting.radar_hold`, and WATCH
`me.radar` while you hold it:

- `dwell_target` is the contact the acquisition is charging on.
- `dwell_secs` of `dwell_needed` is how far it has charged; `dwell_fill` is
  the same as a fraction, and is `null` when nothing is charging at all. This
  is the ring a player watches fill.
- `candidate` is what is under the ray, whether or not it is charging.

The dwell GROWS WITH RANGE: about a second inside a kilometre, about 60 ticks
at 2500 m. That is why holding for a fixed 40 ticks and reading no lock proves
nothing - read `dwell_fill` instead and hold until it reaches 1, or until
`combat_lock` / `travel_lock` names the target. A `null` `dwell_fill` means
the ray is on nothing lockable, so holding longer will not help: re-aim. Release once locked; the lock
sticks on its own.

`me.radar` is `null` whenever the gesture is not held. Nothing is charging
then, and nothing is being searched for.

## When the lock will not take

- Nothing is under the ray. `candidate` is `null`: turn until the target is
  nearer bearing `[0, 0]`.
- A rock is in the way. The radar is a ray and a body stops it. The view says
  so: a body that blocks the line of sight is listed under
  `bodies.in_the_way` with `why` naming the lock it occludes.
- The contact has no allegiance. Nothing will ever combat-lock it.
- The weapons are down and you wanted a combat lock. Raise the stance first.

## Dropping a lock

`targeting.radar_clear` drops the combat lock; a second tap drops the travel
lock. It is a TAP on the same key as `targeting.radar_hold` - a short press
clears, a long one searches - so an act that carries both is refused. Release
the hold in one act, tap the clear in the next.

## The stance

`targeting.combat_stance` is a HOLD. While it is held the weapons are hot: the
radar writes combat locks and the triggers do something. Two consequences the
view will not spell out for you:

- A trigger pressed on the same tick as the stance is dropped by the safety.
  Raise in one act, fire in a later one.
- While the stance is raised, `camera.camera_rotate` aims the TURRETS and the
  hull does not follow. Lower the weapons to steer the hull.
