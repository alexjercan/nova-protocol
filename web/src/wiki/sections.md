# Ship sections

A ship in Nova Protocol is not a monolithic model - it is a root entity with a handful of _section_ children. Each section is mounted to the hull, carries its own mass and health, and contributes exactly one behavior to the whole ship. Knock a section off and the ship loses that capability but keeps flying on whatever is left, which is what makes damage feel local: shoot the turret off and it stops shooting; take out the controller and it can no longer steer.

<figure class="figure">
    <!-- Capture: assets/wiki-sections.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-sections.png</span
        >
        <span class="figure__placeholder-note"
            >A built ship with its hull, controller,
            thruster, turret and torpedo-bay sections called
            out - ideally labelled or lightly
            exploded.</span
        >
    </div>
    <figcaption class="figure__caption">
        A ship is an assembly of sections, each with one
        job.
    </figcaption>
</figure>

## Taking a ship apart

You do not have to shoot every last section off a ship. A hull carrying less than a twentieth of the structure it was built with **collapses**: it tears itself apart, and you watch it go. The outermost sections blow off first, each bursting its own debris; the ones they were holding follow, and the wreck peels inward frame by frame until nothing is left and the ship is gone. A scenario can author a tougher ship to hold together further down, so a capital takes more dismantling than a fighter.

A ship coming apart is still a ship for those moments. Its guns keep firing until the sections carrying them blow off, so a kill you have already earned can shoot back on the way down.

That is a different thing from being **out of the fight**. A ship that has lost every weapon, or the flight computer that aims and flies it, stops fighting but keeps its hull - it drifts as a derelict until someone finishes it, or does not.

The health bar on your combat lock's readout measures a target against the hull it was BUILT with, so it only ever falls as you work through the sections.

## What damage looks like

A ship never changes shape from being shot. Sections keep the shape they were built in right up to the moment they die; then the whole part comes off the ship, tumbles away, and the wreckage clears after a while. Nothing on a hull is eaten away, and nothing turns red.

What you read instead is two different things at once.

**How far gone a part is.** Every section fractures as its own health falls: dark veins first, then a glow through the cracks when it is about to go, then cold and burnt when it is dead. Each part is graded on its OWN health, so a stripped skin plate reads as wrecked while the hull it was bolted to still looks untouched. Past about a third gone, the parts that carry machinery start throwing sparks, and a damaged drive's exhaust runs short and unsteady - guttering, never quite out. A hurt thruster still pushes exactly as hard as a fresh one; the plume is telling you what happened to it, not what it can do.

**Where it was hit.** Every hit is remembered where it landed, and that is what throws material off the spot you actually shot rather than off the middle of the part.

Which looks a part wears is decided by whoever built it, so a modded section can crack, spark, gutter, or show nothing at all.

Cladding is the exception to "nothing changes shape". A [clad ship](../keybinds/) wears plates over its structure, and the plate that stops a round dies and comes off, leaving a hole onto the bare hull underneath. That is a piece leaving, not a part being eroded.

Rocks are the other exception, and they carve for real - see [Shooting rock](../combat-weapons/#shooting-rock).

## The sections

Pick a section for the details - what it does, why it matters, and how it ties into the rest of the ship.

<div id="wiki-children"></div>

## Variants

The standard unit-cell catalog at a glance - every child page carries the full per-kind stats, plus the per-craft semantic parts (noses, wings, pods, fuselages). A section weighs the space it fills, so the unit cells all weigh the same and health and the kind stat are what separate them - the torpedo bays are the exception, a two-cell tube at twice the mass.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs: reinforced hull health 200 :584; basic thruster :611; basic controller :687 (max_torque 1501 :704); light hull 60 :732; cargo hull 200 :757; tank hull 200 :778; pdc_turret_prototype :414 (health 130 :429,:32) with gatling call sites :791-815 at 100/s :67 and twin call sites :816-839 at half per muzzle :75; torpedo bay builder :959 with call sites :842-861; heavy_torpedo_section :864. -->
<table>
<thead>
<tr><th>Kind</th><th>Variant</th><th>Health</th><th>Signature stat</th></tr>
</thead>
<tbody>
<tr><td>Hull</td><td><span class="catalog__name">Reinforced Hull Section</span><span class="catalog__id">reinforced_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Light Hull Section</span><span class="catalog__id">light_hull_section</span></td><td class="catalog__num">60</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Cargo Hull Section</span><span class="catalog__id">cargo_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Hull</td><td><span class="catalog__name">Tank Hull Section</span><span class="catalog__id">tank_hull_section</span></td><td class="catalog__num">200</td><td>structure only</td></tr>
<tr><td>Controller</td><td><span class="catalog__name">Basic Controller Section</span><span class="catalog__id">basic_controller_section</span></td><td class="catalog__num">100</td><td class="catalog__num">1501 torque</td></tr>
<tr><td>Thruster</td><td><span class="catalog__name">Basic Thruster Section</span><span class="catalog__id">basic_thruster_section</span></td><td class="catalog__num">70</td><td class="catalog__num">1.0 thrust</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Kinetic)</span><span class="catalog__id">pdc_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">PDC Turret (Pierce)</span><span class="catalog__id">pdc_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 100/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Kinetic)</span><span class="catalog__id">pdc_twin_kinetic_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">4.0 Kinetic at 2 x 50/s</td></tr>
<tr><td>Turret</td><td><span class="catalog__name">Twin PDC Turret (Pierce)</span><span class="catalog__id">pdc_twin_pierce_turret_section</span></td><td class="catalog__num">130</td><td class="catalog__num">2.0 Pierce at 2 x 50/s</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Serpent)</span><span class="catalog__id">torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 30 u</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Torpedo Bay (Lance)</span><span class="catalog__id">lance_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">750 blast / 30 u</td></tr>
<tr><td>Torpedo bay</td><td><span class="catalog__name">Siege Torpedo Bay Section<span class="catalog__flag">hidden</span></span><span class="catalog__id">heavy_torpedo_section</span></td><td class="catalog__num">100</td><td class="catalog__num">2000 blast / 45 u</td></tr>
</tbody>
</table>
</div>
