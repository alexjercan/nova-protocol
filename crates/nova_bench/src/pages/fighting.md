# Fighting

The loop is: turn, raise, lock, close, fire, cease.

1. Turn until the hostile is near bearing `[0, 0]`.
2. Hold `targeting.combat_stance`. In a LATER act, hold
   `targeting.radar_hold` until `me.combat_lock` names the contact, watching
   `me.radar.dwell_fill` while you hold. Release the radar.
3. Close with `flight.main_drive`, gently. From 2.4 km, 60 ticks of drive
   (about 80 m/s) and a coast bring you inside 2000 m in a few seconds. Tap
   `flight.autopilot_stop` near 1700 m and you settle about 1500 m off. A
   longer burn overshoots and the fight becomes a chase.
4. Inside 2000 m, hold the turret `section.<id>` triggers while the lock
   stands. The turrets track the lock whatever the nose does: manage the
   RANGE, do not turn the hull to chase.
5. Release the triggers when the contact reads `defeated` or leaves
   `contacts` altogether. A hull that breaks up despawns and may never read
   `defeated`.

## Not standing still

`ordnance.inbound` is rounds in the air that are not yours. A ship that holds
station in front of a gun is hit; a ship with cross-range drift is much
harder to hit. Turn the approach into a crossing pass and keep the drift
through the fight: the turrets hold their lock while you slide, so the cost
is arc, not accuracy.

## Watching your own hull

`me.health` is the aggregate. `me.sections` says which mounts are still alive
and `me.hull_plates` counts the armour as `total`, `damaged` and `lost`.
Damage that lands is damage you did not notice coming - check the hull after
a pass, not just the guns.
