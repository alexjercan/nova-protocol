# Bug related to twitching

- STATUS: OPEN
- PRIORITY: 99
- TAGS: v0.5.0, bug, physics

`/home/alex/personal/nova-protocol/tasks/20260710-231931/TASK.md: [PRIORITY:
90, TAGS: v0.5.0, rendering, physics, bug] Spaceship rendering is twitchy at
high velocity`,
`/home/alex/personal/nova-protocol/tasks/20260710-231930/TASK.md: [PRIORITY:
85, TAGS: v0.5.0, rendering, physics, bug] Bullets twitch badly at high
spaceship velocity`,
`/home/alex/personal/nova-protocol/tasks/20260710-231928/TASK.md: [PRIORITY:
82, TAGS: v0.5.0, hud, bug] HUD text anchored to moving objects twitches (e.g.
velocity on the ship)`,
`/home/alex/personal/nova-protocol/tasks/20260710-231929/TASK.md: [PRIORITY:
80, TAGS: v0.5.0, turret, bug] Turret crosshair (orange square) twitches while
tracking` These bugs seem to be related somehow to the physics of the game; at
high velocities/distances the physics seems a bit janky: the camera following
the spaceship feels a bit unstable, at high speeds the bullets that are spawned
from the PDC turret do not have perfect position, they spew out and twitch, so
their position is not linear as expected, at high speeds the thrusters
sometimes create torque on the ship which is understandable, but annoying, so
for example if you try to stop the spaceship from high speed, and try to hold
the reverse direction, the spaceship cannot hold it's deceleration path
perrfectly, sometimes it twitches and flips.
