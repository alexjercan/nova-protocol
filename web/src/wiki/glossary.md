# Glossary

Short definitions for the terms and units that recur across the wiki.

## Units

- **m / km** - the distance units. Ranges and radii read in metres below a kilometre (e.g. a standoff of about 500 m) and in kilometres above it (e.g. `1.24 km`). Every distance readout on the HUD and in the NOVA OS map uses them.
- **m/s** - the speed unit, metres per second. The speed chip beside the velocity sphere reads in `m/s`, and a target's closing speed reads as a signed `m/s`.

## Terms

- **Prograde / retrograde** - prograde is the direction you are moving; retrograde is the opposite. STOP flips you to retrograde and burns to kill your speed.
- **Standoff** - the resting distance GOTO stops at, just off a target (about 500 m plus the target's radius, measured from the surface) rather than ramming it.
- **RCS** - reaction-control fine translation for docking: hold <kbd>Shift</kbd> and steer with the mouse and scroll wheel to nudge the hull straight along its own axes, no rotation, capped at a gentle per-axis speed. A trim a scenario grants rather than standard flight - see [RCS](../flight-autopilot/#rcs-fine-docking-thrusters).
- **Sphere of influence** - the reach of a [gravity well](../gravity-wells/), set by the body's authored mass alone: the distance where the pull decays to a fixed cutoff. Outside it the well does not pull on you; only the dominant well inside it matters.
- **Hysteresis** - a bit of stickiness that stops a state from flickering at its edge. A lock, a fine-lock section, and the dominant well all hold a little past their switch point so they do not chatter.
- **Fine-lock** - drilling a combat lock into one specific [section](../sections/) of an enemy hull, so turrets and the viewfinder focus that part.
- **"Hot" weapons** - your weapons are hot while you hold a raised (red) combat stance or while a combat lock exists. Hot means turret lead pips and the viewfinder frame go red and you can fire.
- **Neutralized** - out of the fight for good. A ship that loses its last working gun, or the flight computer it had, stops fighting even though its hull survives: the [viewfinder](../hud/) tags the wreck NEUTRALIZED and it keeps drifting rather than despawning.
- **Point defense** - your turrets' automatic close-in fire against torpedoes committed to you. It opens up only once an inbound torpedo is close, and every intercept costs real ammunition - see [point defense](../combat-weapons/#point-defense).
- **Kinetic / Pierce** - the two bullet behaviors, told apart by tracer colour. A Kinetic round punches: it stops at the first section it fails to destroy. A Pierce round rakes: it crosses several sections and damages every one on the way through. See [damage types](../combat-weapons/#damage-types).
- **Lance / Serpent / Breaker** - the [torpedo](../combat-weapons/#torpedoes) types, each named and tinted on the viewfinder when you lock one. The Lance runs straight and fast, so point defense can answer it; the Serpent corkscrews on approach and costs far more rounds to stop; the Breaker is the huge crimson siege warhead scripted batteries fire. (The railgun's catalog entry is named Railgun Lance; these pages call it the railgun.)
- **Railgun** - the [spinal gun](../sections/railgun/): three cells of rails with no traverse, so the hull is the aim. A tap commits a 1.5 s charge that only the weapons safety can stop, and the slug that leaves rakes a corridor through everything in the line. One shell, a twelve-second reload.
- **Bore sight** - the [HUD](../hud/#bore-sight) line a railgun-carrying hull draws from its muzzle to where the slug would stop, with a ring on every section the shot would destroy. Up while weapons are hot; dimmed through the reload.
- **Corridor** - what a railgun slug leaves: the bore column the slug's tip cut, widened by a sphere trailing the tip to about three cells across, entry to exit. Every section in it pays out of the one power budget, so a wider rake is never more damage, only a differently shaped hole - see [what one shot takes out](../sections/railgun/#what-one-shot-takes-out).
- **Carve** - what a hit does to a rock: it takes real material out of the asteroid's shape, so the silhouette and the thing you can fly into both change. Only rocks carve - a ship's parts keep the shape they were built in until they die. See [Shooting rock](../combat-weapons/#shooting-rock).
- **Crater** - the hole a carve leaves. It stays, and fire landing near it deepens the same hole instead of opening a new one.
- **Diegetic** - part of the world rather than an overlay bolted on top. The HUD instruments read the ship's real state and the autopilot flies the same real thrusters you do, so what you see is what the ship is actually doing.
