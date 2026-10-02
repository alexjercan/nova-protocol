# Ships & damage

A ship in Nova Protocol is not a monolithic model - it is a root entity with a handful of _section_ children. Each section is mounted to the hull, carries its own mass and health, and contributes exactly one behavior to the whole ship. Knock a section off and the ship loses that capability but keeps flying on whatever is left, which is what makes damage feel local: shoot the turret off and it stops shooting; take out the controller and it can no longer steer.

<figure class="figure">
    <!-- Capture: assets/wiki-ships-damage.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-ships-damage.png</span
        >
        <span class="figure__placeholder-note"
            >A gunship after a broadside walked up its port
            flank: plating stripped off three sections, the
            port turret dead, and an open hole where the aft
            deck tore away.</span
        >
    </div>
    <figcaption class="figure__caption">
        A ship keeps fighting until the sections carrying
        its guns come off.
    </figcaption>
</figure>

## Taking a ship apart

You do not have to shoot every last section off a ship. A hull carrying less than a twentieth of the structure it was built with **collapses**: it tears itself apart, and you watch it go. The outermost sections blow off first, each bursting its own debris; the ones they were holding follow, and the wreck peels inward frame by frame until nothing is left and the ship is gone. A scenario can author a tougher ship to hold together further down, so a capital takes more dismantling than a fighter.

A big ship dies big. The fires of a collapse are spread across the whole wreck rather than clustered where you opened it, and how far the death reaches is drawn from the ship's own size - a capital's fireball throws burning metal hundreds of metres, a needle's is over in a flash the width of the hull. It reads as one event however much of it comes apart at once: one crash, one shove of the camera, from the middle of the ship rather than from whichever plate let go first.

A ship coming apart is still a ship for those moments. Its guns keep firing until the sections carrying them blow off, so a kill you have already earned can shoot back on the way down.

That is a different thing from being **out of the fight**. An armed ship that has lost every weapon, or the flight computer that aims and flies it, stops fighting but keeps its hull - it drifts as a derelict until someone finishes it, or does not. An unarmed ship that ever had a thruster goes the same way once the last working one dies; an unarmed hull that never had a thruster can only be destroyed, not neutralized. The [viewfinder](../hud/#target-viewfinder) tags it NEUTRALIZED, and a neutralized hull stops defending itself too: nobody is left aboard to work the mounts, so a wreck lets your ordnance fly straight past even with an intact turret still bolted to it. It is still solid and it still takes damage; it just does not answer.

The health bar on your combat lock's readout measures a target against the hull it was BUILT with, so it only ever falls as you work through the sections.

## What damage looks like

A ship never changes shape from being shot. Sections keep the shape they were built in right up to the moment they die; then the part goes up in a white flash, throws burning fragments that outlive it, and comes off the ship to tumble away until the wreckage clears. A whole hull letting go burns bigger than a single compartment does. Nothing on a hull is eaten away, and nothing turns red.

What you read instead is two different things at once.

**How far gone a part is.** Every section fractures as its own health falls: dark veins first, then a glow through the cracks when it is about to go, then cold and burnt when it is dead. Each part is graded on its OWN health, so a stripped skin plate reads as wrecked while the hull it was bolted to still looks untouched. Past about a third gone, the parts that carry machinery start throwing sparks, and a damaged drive's exhaust runs short and unsteady - guttering, never quite out. A hurt thruster still pushes exactly as hard as a fresh one; the plume is telling you what happened to it, not what it can do.

**Where it was hit.** Every hit is remembered where it landed, and that is what throws material off the spot you actually shot rather than off the middle of the part.

Which looks a part wears is decided by whoever built it, so a modded section can crack, spark, gutter, or show nothing at all.

Cladding is the exception to "nothing changes shape". A [clad ship](../keybinds/) wears plates over its structure, and the plate that stops a round dies and comes away where it stood - tumbling clear on the ship's own motion, greebles and all - leaving a hole onto the bare hull underneath. That is a piece leaving, not a part being eroded.

Rocks are the other exception, and they carve for real - see [Shooting rock](../combat-weapons/#shooting-rock).

## Generated ships

<!-- Roles and labels: crates/nova_world/src/generation.rs `ShipRoleType`.
     Status: crates/nova_world_base/src/civilizations.rs
     `CivilizationStatusType`, drawn by `CivilizationField::civilization`;
     side (Enemy/Player/Neutral, one chance in three each, drawn apart from
     status): civilizations.rs `allegiance`. Intact or wreck, hold stock and
     credits: crates/nova_world_base/src/sector_ships.rs `plan_ship`,
     `ship_stock`, `ship_credits`; an intact ship's crew and patrol loop:
     sector_ships.rs `plan_patrol`, clusters.rs `plan_patrols`. Layout from
     the loaded section catalog, including an industrial ship's seeded
     mining beams: crates/nova_world_base/src/ship_layout.rs `generate_ship`
     and `generate_wreck`; role style: civilizations.rs `role_style_id`.
     Name, controller, allegiance, crew and lootable wreck:
     crates/nova_world/src/streaming.rs `spawn_sector_ship`. Window:
     crates/nova_world_base/src/lib.rs `OPEN_WORLD_SECTOR_EDGE`,
     `OPEN_WORLD_ACTIVE_RADIUS`. Take credits: crates/nova_gameplay/src/
     inventory.rs `plan_credit_take`. Dock admission by combat state:
     crates/nova_ship/src/sections/docking_section/admission.rs. Role
     outlines, weapon slots and styles in the generated-ship-roles widget:
     web/src/widgets.ts `GENERATED_SHIP_ROLES`. -->

In a **New Game** world, the ships you meet are generated, not picked from a list of designs. Each one is laid out from the sections of the content the game loaded, for a **civilization** and a **role**.

<div class="widget" data-widget="generated-ship-roles">
<p>Four roles: a civilian is unarmed traffic, an industrial is unarmed with a cargo intake and a chance of one or two mining beams, a scavenger is a rough, low-tier armed ship, and an armored is an equipped armed ship. The role and the civilization's status are drawn apart, so each role can be met intact from a living civilization or as a wreck from an extinct one. A wreck comes from an extinct civilization, not from damage.</p>
</div>

Each role wears its own skin style, intact or wrecked. Every civilization is **living** or **extinct**. A living civilization's ships are intact. An extinct civilization leaves **derelict wrecks**: hulls at any orientation, with every section switched off except the hull and the docking ports.

Every ship, intact or wrecked, starts with goods and credits scaled by its civilization's advancement: an industrial hold favors ore and parts, a civilian's rations and parts, and a scavenger's or armored ship's ammunition, hull plates and parts, drawn apart from its credit balance. A wreck's hold and balance are drawn the same way, independently, against what is left of its hull: roughly half the cargo and 10-25% of the credits an intact ship of the same standing would carry, with at least one credit kept. A wreck may hold no hull plates at all. A low-advancement ship still carries something of each kind, nonzero. Some industrial ships, living or wrecked, also mount one or two real mining beams; a wrecked beam stays mounted but inert, like every other system a derelict carries.

A civilization sides Enemy, Player or Neutral, drawn apart from its status and advancement. An intact ship carries that civilization's crew: it flies the side's AI, patrols a loop of waypoints near where it spawned (holding at each one for a while before flying on), and will not chase a fight more than about 5 km from that loop. A derelict wreck still has nobody aboard and stays Neutral. Travel-lock one and the [target viewfinder](../hud/#target-viewfinder) names it, or select it on the [Map pane](../interface/#the-map): an intact ship reads `<civilization> <role>`, and a wreck reads `<civilization> derelict, former <role>`, for example `Halurmar derelict, former scavenger`. A distant blip on the map shows only its contact code.

An armed Neutral ship you shoot fights back and reads HOSTILE to you until it loses you, is neutralized, or you both leave its reach - see [Factions](../factions/#retaliation).

To loot a wreck, or a ship you neutralized in combat, [dock](../sections/docking/) with one of its ports: [Take](../interface/#take-and-give) its goods, and **Take credits** moves its whole balance in one click, in the Inventory pane. A calm ally or Neutral ship lets you dock to trade instead; one that is Hostile, Enemy, or fighting refuses the dock until it calms down or you finish it.

The world streams in 32 km sectors, and the game keeps the 5 x 5 x 5 block of sectors around yours live. A sector that streams out and back in is generated again from the seed: nothing you did to its ships is kept. A wreck you emptied is full again, a ship you destroyed is back, anything you gave a ship is gone, and credits you took are back on the ship you took them from.

## The parts

Every section is its own page - what it does, how it behaves, its numbers, and what it is like to face one. [Ship sections](../sections/) opens that chapter with what every part shares and the catalog at a glance.
