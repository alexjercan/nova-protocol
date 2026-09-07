# Weapons: what the mounts do

Each entry of `me.sections` with a `weapon` is a mount you can fire, by
holding `section.<id>`. `me.sections[].weapon` carries `kind`, `firing`,
`on_target` and `ammo`.

## Turrets and point defence

A turret tracks the COMBAT LOCK by itself. It does not care where the nose
points: you manage the range, not the heading. Rounds leave the barrel only
while `on_target` is true.

`on_target` says the barrel bears. It does NOT say the target is in range.
Turret rounds land inside about 2000 m and `on_target` starts reporting from
about 2500 m, so a burst fired at 2400 m is ammunition thrown away.

A magazine holds a few hundred rounds and refills a batch every few seconds,
so a held trigger fires in bursts. `ammo.rounds` is what is loaded now.

A hauler-sized hull takes half a minute or more of fire from six mounts.

## Railguns

A railgun charges while the trigger is held and fires along the NOSE, not at
the lock. `charging_secs` counts the charge. Point the ship.

## Torpedoes

A torpedo bay launches at the combat lock and the round flies itself.
`reloading` is true while the tube is refilling.

## What weapons do not do

Rounds pass straight through a rock or a planet and take nothing off it. A
body is a place to fly to or around, never a target and never cover.
