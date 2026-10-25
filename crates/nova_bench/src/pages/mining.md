# Mining: cut rock, then recover the ore

Mining uses a per-section held input. Find a working mining emitter in
`me.sections[]` by `class: "Mining"`, its `id`, and its `alive` and `disabled`
status. Hold `section.<id>` with that ID (for example, `section.mining_beam`
on the Line Warship), then release the same wire. These dynamic section wires
are not registry actions: they do not appear in `inputs.live` or the `bindings`
command. `inputs.live` reports registry actions only.

1. Approach an asteroid and point the ship so the working mining emitter
   faces it. No travel lock or lowered combat stance is needed. The ray takes
   the nearest rock it intersects within the emitter's reach; a barren rock
   in front of an ore rock refuses the pulse.
2. Hold `section.<id>` for a bounded attempt, then release that same wire.
   The emitter must deploy before it pulses; reach and aim are checked from
   its face, not the ship's centre. A short ship-centre distance or centred
   `bodies.near` bearing does not prove a pulse reached rock.
3. A successful carve can yield ore canisters only after the rock's field is
   updated. Carving does not put ore directly in the hold. A working cargo
   intake takes a whole canister only when it enters the intake trigger and
   the hold has enough free mass. See the `cargo` page before attempting
   collection.

The agent view has no carve result or intake position. `canisters` lists
all live canisters, their runtime IDs, contents, and distance and bearing
from the player's ship. A newly visible canister containing ore shows ore
in the world, not why it spawned or whether the ship recovered it. The view
does not show the intake's trigger position, so a centred canister bearing
cannot prove contact. Make bounded attempts; compare `me.cargo.items` before and
after. Only an increase in the relevant item count proves ore was picked up.
A canister's disappearance alone does not prove pickup: it may have been
destroyed. The first pulse on untouched rock can prepare its material field
without taking ore; a cut already empty yields nothing.
