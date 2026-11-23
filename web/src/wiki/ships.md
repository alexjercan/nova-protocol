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
            flank: plating stripped off three sections, and an
            open hole where the port wing tip tore away with
            its turret and its drive.</span
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

A destroyed ship spills its cargo. The canisters leave one at a time from where it died: everything still in its hold, plus a share of the parts each lost section was built from, packed together in canisters of up to 1,000 kg that can mix item types. A section shot off a ship that keeps flying drops its own share of parts where it died. Fly a [cargo intake](../sections/cargo-intake/) onto a canister to take it in.

That is a different thing from being **out of the fight**. An armed ship that has lost every weapon, or the flight computer that aims and flies it, stops fighting but keeps its hull, and nobody finishes it for you. An unarmed ship that ever had a thruster goes the same way once the last working one dies; an unarmed hull that never had a thruster can only be destroyed, not neutralized. The [viewfinder](../hud/#target-viewfinder) tags it NEUTRALIZED, and a neutralized hull stops defending itself too: nobody is left aboard to work the mounts, so a wreck lets your ordnance fly straight past even with an intact turret still bolted to it. It is still solid and it still takes damage; it just does not answer. A neutralized ship stays out of the fight, except yours: repair or rebuild what you lost, at a station, in the field or through an [SOS rescue](../interface/#the-ship), and your ship is back in the fight, which also means enemies shoot at it again.

A neutralized ship drops its patrol, its escort slot and any order, and brakes to a stop with its own RCS so you can fly up and dock by hand. It needs no engine for that, but it needs a working flight computer, RCS, delta-v left in its own RCS magazine, and no gravity well stronger than its RCS. A ship that is docked, or lacks one of these, drifts on at its speed, and so does one whose magazine runs dry before it stops: a magazine that refills later does not start the brake again. The brake also ends once the ship stops: a later bump, such as your hull touching it as you dock, sets it drifting again. Your own ship never brakes by itself.

A ship with no working flight computer cannot hold its heading either, so a kill can leave it tumbling. Its tumble settles on its own: the spin halves every 8 seconds, so a fast tumble falls to a dockable 5 degrees a second in about half a minute, while the drift stays. This holds for wrecks, derelicts, dead stations and your own ship once its last computer dies; a ship with a live computer holds its own turn, and a docked pair turns as one.

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
     Palette per civilization: crates/nova_world_base/src/ship_parts.rs
     `ShipPartSnapshot::palette`; base palettes and role tones:
     crates/nova_authoring/src/base_content/palettes.rs; station finish:
     station_layout.rs; greeble habit: crates/nova_ship/src/sections/
     faction_palette.rs `SkinLook::worn_style`.
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

Each role wears its own skin style, intact or wrecked, and each civilization paints its hulls in its own **palette**: one hue across its civilians, industrials, scavengers, armored ships, wrecks and stations. The role sets the tone: a civilian is light, an industrial matte, a scavenger dull, an armored ship dark. Two neighbouring civilizations never share a palette, so a hull's colour tells you whose it is before the viewfinder does. A palette also sets how busy a civilization's hulls are with greebles, and some add their own accent pieces. A wreck keeps its builder's colours, a hull you buy keeps the colours of the station that sold it, and the start hulls keep their own.

Every civilization is **living** or **extinct**. A living civilization's ships are intact. An extinct civilization leaves **derelict wrecks**: hulls at any orientation, with every section switched off except the hull and the docking ports. Dock with one and you can [strip](../interface/#the-ship) its sections into parts.

Every ship, intact or wrecked, starts with goods and credits scaled by its civilization's advancement: an industrial hold favors ore and parts, a civilian's rations, parts and the odd medkit, and a scavenger's or armored ship's ammunition, hull plates and parts. Half the intact scavengers also carry one unit of contraband; a wreck carries none. A ship carries only ammunition its own weapons reload from: no torpedoes without a torpedo bay. Its goods are drawn apart from its credit balance. A wreck's hold and balance are drawn the same way, independently, against what is left of its hull: roughly half the cargo and 10-25% of the credits an intact ship of the same standing would carry, with at least one credit kept. A wreck may hold no hull plates at all. A low-advancement ship still carries something of each kind, nonzero. Some industrial ships, living or wrecked, also mount one or two real mining beams; a wrecked beam stays mounted but inert, like every other system a derelict carries.

A civilization sides Enemy, Player or Neutral, drawn apart from its status and advancement. An intact ship carries that civilization's crew: it flies the side's AI, patrols a loop of waypoints near where it spawned (holding at each one for a while before flying on), and will not chase a fight more than about 5 km from that loop. A derelict wreck still has nobody aboard and stays Neutral. Travel-lock one and the [target viewfinder](../hud/#target-viewfinder) names it, or select it on the [Map pane](../interface/#the-map): an intact ship reads `<civilization> <role>`, and a wreck reads `<civilization> derelict, former <role>`, for example `Halurmar derelict, former scavenger`. A distant blip on the map shows only its contact code.

An armed Neutral ship you keep shooting or ramming fights back and reads HOSTILE to you until it loses you, is neutralized, changes side, or you both leave its reach - see [Factions](../factions/#retaliation).

To loot a wreck, or a ship you neutralized in combat, [dock](../sections/docking/) with one of its ports: [Take](../interface/#take-and-give) its goods, and **Take credits** moves its whole balance in one click, in the Inventory pane. A calm ally or Neutral ship lets you dock to trade instead; one that is Hostile, Enemy, or fighting refuses the dock until it calms down or you finish it.

The world streams in 32 km sectors, and the game keeps the 5 x 5 x 5 block of sectors around yours live. For the rest of the session, a sector that streams out and back in keeps its ships as you left them: a wreck you emptied stays empty, a ship you destroyed stays destroyed, goods you took or gave stay moved, credits you took stay taken, and a ship you neutralized stays neutralized. Each ship keeps its RCS magazine and a neutralized ship's brake as they stood; a magazine that was not firing refills for the time you were away, and one that was firing gets nothing for it. Only the sector's planetoids regenerate from the seed.

## Encounters

<!-- Director, travel rule, placement and end rules:
     crates/nova_world_base/src/encounters.rs (`travel_counts`,
     TRAVEL_SPEED_MIN 50 m/s, STATION_STANDOFF 10 km, PLACE_LEAD_SECONDS 30,
     PLACE_LEAD 6 km to 40 km, PLACE_ASIDE 2 km, AMBUSH_BREAK_OFF 30 km,
     DERELICT_FOUND 1 km, `end_encounters`, `mark_encounter_bodies`). Gap,
     draw and offer window: crates/nova_gameplay/src/encounter.rs
     (ENCOUNTER_GAP_S 300 to 900 s, `plan_encounter_draw`,
     HAIL_OFFER_SECONDS 300). Base set:
     crates/nova_authoring/src/base_content/encounters.rs. Raider count and
     size: crates/nova_gameplay/src/jobs.rs `escort_raider_counts`,
     `escort_raider_size`. -->

On a long flight in a **New Game** world, the world puts an **encounter** ahead of your ship: one after each 5 to 15 minutes of travel. Travel counts only while you fly undocked at more than 50 m/s, more than 10 km from every station, out of combat, with your ship not neutralized and no [SOS call](../interface/#the-ship) open. One encounter is live at a time. Travel does not count while one is live or while you hold its job.

An encounter stands on your line of flight, 30 seconds of flight ahead, not nearer than 6 km and not farther than 40 km, and 2 km to one side. Its hulls belong to the civilization that holds that space.

| encounter | what stands there | it ends when |
|---|---|---|
| Distress call | a stranded civilian hull that hails you with a job: dock with it | you take the job and dock, you decline, or 5 minutes of play pass with no answer |
| Supply call | a stranded civilian hull that hails you with a job: bring it the goods it names and dock with it | the same |
| Ambush | raiders that wait for you, as many and as large as an escort job's raiders in that space. An unarmed ship meets none | every raider is neutralized or destroyed, or you are more than 30 km from all of them |
| Derelict find | one civilian wreck with a gold `SIGNAL` chip and its distance | you come within 1 km of it |

A hail reaches you three ways: the caller's line on [comms](../hud/#comms-and-objectives), a `HAIL <NAME>` [HUD notice](../hud/#flight-readouts), and a gold `HAIL` chip with the distance on the caller. The job is one row under **Available** on the [Jobs pane](../interface/#jobs), with **Accept** and an `X` that declines it; both work with no dock. A dock with the caller while the offer stands accepts the job. The job is done and paid on the dock, in space: there is no station hand-in. An ambush sends no message: its raiders show as hostile contacts.

When an encounter ends, its hulls stay in the world. The world seed fixes which encounter comes when, and a saved world keeps the count of encounters, the travel so far, a standing offer and a live encounter: a Load brings them back.

## The parts

Every section is its own page - what it does, how it behaves, its numbers, and what it is like to face one. [Ship sections](../sections/) opens that chapter with what every part shares and the catalog at a glance.
