# Ships & damage

A ship in Nova Protocol is not a monolithic model - it is a root entity with a handful of _section_ children. Each section is mounted to the hull, carries its own mass and health, and contributes exactly one behavior to the whole ship. Knock a section off and the ship loses that capability but keeps flying on whatever is left, which is what makes damage feel local: shoot the turret off and it stops shooting; take out the controller and it can no longer steer.

<figure class="figure">
    <!-- Capture: assets/wiki-ships-damage.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-ships-damage.png</span
        >
        <span class="figure__placeholder-note"
            >A hull coming apart under fire: outer sections
            blowing off, the core still firing, a wreck
            drifting clear.</span
        >
    </div>
    <figcaption class="figure__caption">
        A ship keeps fighting until the sections carrying
        its guns come off.
    </figcaption>
</figure>

## Taking a ship apart

You do not have to shoot every last section off a ship. A hull carrying less than a twentieth of the structure it was built with **collapses**: it tears itself apart, and you watch it go. The outermost sections blow off first, each bursting its own debris; the ones they were holding follow, and the wreck peels inward frame by frame until nothing is left and the ship is gone. A scenario can author a tougher ship to hold together further down, so a capital takes more dismantling than a fighter.

A ship coming apart is still a ship for those moments. Its guns keep firing until the sections carrying them blow off, so a kill you have already earned can shoot back on the way down.

That is a different thing from being **out of the fight**. A ship that has lost every weapon, or the flight computer that aims and flies it, stops fighting but keeps its hull - it drifts as a derelict until someone finishes it, or does not. The [viewfinder](../hud/#target-viewfinder) tags it NEUTRALIZED, and a neutralized hull stops defending itself too: nobody is left aboard to work the mounts, so a wreck lets your ordnance fly straight past even with an intact turret still bolted to it. It is still solid and it still takes damage; it just does not answer.

The health bar on your combat lock's readout measures a target against the hull it was BUILT with, so it only ever falls as you work through the sections.

## What damage looks like

A ship never changes shape from being shot. Sections keep the shape they were built in right up to the moment they die; then the whole part comes off the ship, tumbles away, and the wreckage clears after a while. Nothing on a hull is eaten away, and nothing turns red.

What you read instead is two different things at once.

**How far gone a part is.** Every section fractures as its own health falls: dark veins first, then a glow through the cracks when it is about to go, then cold and burnt when it is dead. Each part is graded on its OWN health, so a stripped skin plate reads as wrecked while the hull it was bolted to still looks untouched. Past about a third gone, the parts that carry machinery start throwing sparks, and a damaged drive's exhaust runs short and unsteady - guttering, never quite out. A hurt thruster still pushes exactly as hard as a fresh one; the plume is telling you what happened to it, not what it can do.

**Where it was hit.** Every hit is remembered where it landed, and that is what throws material off the spot you actually shot rather than off the middle of the part.

Which looks a part wears is decided by whoever built it, so a modded section can crack, spark, gutter, or show nothing at all.

Cladding is the exception to "nothing changes shape". A [clad ship](../keybinds/) wears plates over its structure, and the plate that stops a round dies and comes off, leaving a hole onto the bare hull underneath. That is a piece leaving, not a part being eroded.

Rocks are the other exception, and they carve for real - see [Shooting rock](../combat-weapons/#shooting-rock).

## The parts

Every section is its own page - what it does, how it behaves, its numbers, and what it is like to face one. [Ship sections](../sections/) opens that chapter with what every part shares and the catalog at a glance.
