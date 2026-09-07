# Orbit: flying a ring

A planet or a large rock has a gravity well. `me.gravity_well` names the body
whose well you are inside, or is `null` in open space - roughly 3 km off a
planetoid's surface.

## Taking a ring

1. Turn to the body and burn TOWARDS it at a moderate speed. 60 ticks of main
   drive from rest is about 80 m/s; 240 ticks about 320 m/s.
2. Coast until `me.gravity_well` names the body.
3. Tap `flight.autopilot_orbit` AT ONCE.

The helm can take a ring from 320 m/s. It cannot from 550: it dives at the
surface in the Burn phase and you lose the bridge and the main drive.

Do NOT stop first. Inside a well the Stop helm kills your speed while gravity
keeps pulling you onto the surface: from 240 m/s it shed 77 m/s over 3400
ticks and the ship still fell.

## Reading the helm

`me.autopilot.engaged` runs `Align`, then `Burn`, then `Hold`. `Hold` on a
steady `surface_m` is the orbit. Leave it engaged - it holds the ring.

Without a well, `flight.autopilot_orbit` does nothing: `engaged` stays `null`.

## Orbit as a brake

Orbit is the only helm that fights gravity, so it is also the way out of a
fall. If clearance is dropping and the drive cannot cancel the fall, tap
`flight.autopilot_orbit` the moment `gravity_well` names the body.
