# Travel: getting somewhere

There is no brake. `flight.main_drive` accelerates along the nose and you keep
whatever velocity you had when you let go.

## By hand

1. Turn until the destination is near bearing `[0, 0]`.
2. Hold the main drive for as long as the leg needs (60 ticks from rest is
   about 80 m/s, 240 ticks about 320 m/s), then release.
3. Coast. Read `distance_m` and `speed_mps` and work out the arrival.
4. Tap `flight.autopilot_stop` when you want to be where you are.

Burn, then coast. A held throttle across a whole leg overshoots.

`flight.autopilot_stop` turns the hull to face the drift before it burns, so
the nose swings while it works. From 80 m/s it is still within about 300 ticks
and 500 m; from 240 m/s it needs 600 ticks and a few kilometres. Turrets keep
their lock while the nose swings, so range is what you have to manage.

## By autopilot

Lock the destination with the radar while the weapons are LOWERED (that makes
it a `travel_lock`), then tap `flight.autopilot_goto`. The helm flies there and
parks a safe margin off - about 300 m from a ship, a clear margin off a body's
surface - then hands back: `me.autopilot.engaged` reads `null` again with
`speed_mps` near zero and the target still under `travel_lock`. That is the
park. A second GOTO tap from there does nothing useful.

Without a travel lock, the tap does nothing at all.

While an autopilot holds the helm the hull does not follow `camera_rotate`:
tap `flight.autopilot_off` before you steer.

## Bodies

Do not fly into a rock or a planet: the hull takes the impact. A body that the
current drift runs into is carried in `bodies.near` with its `closing_mps`
however far off it is - that is the view telling you to do something about it.
