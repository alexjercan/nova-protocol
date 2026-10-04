# Mining: cut rock, then recover the ore

Mining uses the held `flight.mine` registry wire. Find it in `inputs.live`.
Mining sections have no `section.<id>` mining trigger. For weapons, the
`section.<id>` selector instead takes an ID from a `me.sections[]` record
whose `weapon` is not null; it is not listed in `inputs.live`.

1. With the combat stance lowered, aim at an asteroid and acquire a travel
   lock with `targeting.radar_hold`. Check `me.travel_lock` for the rock's ID.
   The lowered stance selects the travel-lock slot during acquisition; once
   locked, mining itself does not require the stance lowered.
2. Approach the locked rock and point the ship so a working mining emitter
   may face it. Hold `flight.mine` for a bounded attempt, then release it.
   Emitters must deploy before they pulse; reach and aim are checked from
   EACH emitter's face, not the ship's centre. A short ship-centre distance
   or centred `bodies.near` bearing does not prove a pulse reached rock.
3. A successful carve can yield ore canisters only after the rock's field is
   updated. Carving does not put ore directly in the hold. A working cargo
   intake takes a whole canister only when it enters the intake trigger and
   the hold has enough free mass. See the `cargo` page before attempting
   collection.

The agent view has no carve result, canister position, or intake position.
It cannot confirm a cut or steer accurately to a canister from a snapshot.
Make bounded attempts; compare `me.cargo.items` before and after. Only an
increase in the relevant item count proves ore was picked up. Neither a
mining gesture, a beam, proximity to the rock, nor unchanged cargo proves
carving or recovery. The first pulse on untouched rock can prepare its
material field without taking ore; a cut already empty yields nothing.
