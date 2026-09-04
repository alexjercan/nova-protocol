# Railgun

<figure class="figure">
    <!-- Capture: assets/wiki-section-railgun.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-railgun.png</span
        >
        <span class="figure__placeholder-note"
            >A railgun mounted on a ship's spine, charge bolt
            partway up the bore, bore sight drawn ahead of
            the muzzle onto a target.</span
        >
    </div>
</figure>

A railgun is a **spinal gun**: three cells of rails and capacitor bank with no traverse of its own. It does not aim. The **ship** aims, you commit the shot, and what leaves cuts a corridor through everything standing in that line.

It is the opposite weapon to a turret in every way that matters. A turret is a mount you assign and forget; a railgun is a shot you set up. It fires once every thirteen seconds or so, the trigger cannot call it back once it is pulled, and it shoves the ship that fired it.

<!-- Values from crates/nova_authoring/src/base_content/sections/standard.rs: charge_seconds 1.5 :928, slug_speed 15,000 m/s :929, slug_damage 300 :943, slug_power 1800 :949, rake_radius 10 m :968, slug_lifetime 1.2 :971 (18 km of reach), recoil_impulse 45 :975, ammo_capacity 1 :982, reload delay 12 :987. -->

| The railgun at a glance | |
|---|---|
| Commit | a **1.5 s** charge; nothing but the weapons safety can stop it |
| Slug | **15,000 m/s**, **18 km** of reach, arrives in just over a second |
| Damage | **300 Pierce** to every section in the corridor, flat, no falloff |
| Corridor | about **three cells wide**, priced from one **1800**-point power budget |
| Cycle | one shell, a **12 s** reload: a shot every 13.5 seconds |
| Recoil | **45** at the muzzle: a shove on the spine, a shove and a yaw off it |

## The hull is the aim

<!-- Behavior verified against crates/nova_ship/src/sections/railgun_section/mod.rs (RailgunSectionConfig :69, RailgunSectionInput :210) and firing.rs (the slug is born at the muzzle along the section's own -Z; recoil applied at that point). Sockets: crates/nova_authoring/src/base_content/sections/standard.rs lance_link_points :1088 - thirteen, none on the muzzle face, held by `no_lance_sockets_the_face_it_fires_through`. -->

A railgun fires down its own axis, so where it points is decided when you **bolt it on**, not when you shoot. Mounted on the spine it points where the nose points. Mounted on a flank it points wherever that flank faces, which is not where you are looking - and there is no HUD crosshair that will tell you otherwise, which is why the [bore sight](#the-bore-sight) exists.

The part carries thirteen mounting sockets: the breech plate and three per flank. The **muzzle face carries none**. A railgun cannot traverse off its own line, so a socket in front of the bore would only ever be an invitation to bolt a plate into your own line of fire.

## Committing the shot

Tapping the trigger starts a **1.5 second charge**, and that is the whole of the decision. Letting go does nothing, holding does nothing, and the gun does not re-check the nose when the charge ends. The slug leaves down whatever line the hull is holding at the end of those seconds, so a target that slid off in the meantime is simply missed - and a target you led onto the line during the charge is hit.

There is exactly one way out, and it is not the trigger: dropping your weapons back to **safe** dumps the charge and keeps the shell. A railgun will not fire itself into a friendly you have just safed for.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-railgun.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-railgun.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: the bolt walks the bore, the shot
            leaves, and a corridor opens through a hull block
            and out the far side.</span
        >
    </div>
</figure>

<details class="explain">
<summary>Show explanation</summary>

The shell also carries your ship's motion with it, spin included: a railgun fired while the hull is rolling throws its slug slightly off the tangent as well as down the bore. At fifteen kilometers a second that is a small correction, but it is there, and it is one more reason to be flying straight when the charge ends.

The charge is loud and visible, to you and to anyone watching. A bolt walks the length of the bore, so how far it has travelled is how much charge is left to run: an enemy across the gap can read a railgun about to fire off its hull. The capacitor loop rises in pitch as it fills.

That tell is the balance of the weapon. A slug in flight is unanswerable - it crosses 18 km in just over a second - so the window in which it can be answered is the charge, by breaking the line before it ends.

The shot is readable after it leaves, too. The slug carries its own light with it and drags an **ionized wake** down the line it flew: a haze with filaments running through it, thinning from the muzzle end first, which stays visible for about half a second after the slug is gone. It is the one trace of a railgun anyone gets to read, and it points both ways - at what was fired at, and back at whoever fired. On the **Low** graphics preset neither is drawn: the wake is a particle effect and the slug's light is a transient light, and Low takes neither.

</details>

## The bore sight

<!-- Behavior verified against crates/nova_hud/src/bore_sight.rs (module docs; the trace walks crossed sections through `pierce_remainder`, the same function that resolves the round; gated on WeaponsHot; drawn dimmed on an empty magazine, EMPTY_ALPHA_SCALE :81; the line thickens with the charge, CHARGE_THICKEN :65; a ring per section it would destroy, MARK_RADIUS :69). -->

<figure class="figure">
    <!-- Capture: assets/wiki-section-railgun-sight.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-railgun-sight.png</span
        >
        <span class="figure__placeholder-note"
            >The sight line out of the muzzle across the gap
            onto a corvette's long axis, a ring on each of the
            sections the shot would gut, the line fat with a
            charge nearly run.</span
        >
    </div>
</figure>

Because nothing else on the HUD says where a hull is pointing, the flight computer draws the line for you: a thin blue **sight line** out of the muzzle, ending exactly where the slug would end, with a **ring on every section that shot would destroy**.

The rings are the point. "Am I on it" is a question a target bracket already answers; the question a railgun actually poses is "does this angle gut it or clip a corner", and the sight answers that by walking the sections in the line through the same penetration math the round itself is resolved with. Aiming down a ship's long axis looks visibly different from catching its shoulder.

The line thickens as the charge runs, so the seconds you are committed to holding a heading are readable without looking away. The sight is up whenever your weapons are hot - raised, or holding a combat lock - and it stays up, **dimmed**, while the magazine is empty. Twelve seconds of reload is exactly when you want to be lining the next shot up.

## What one shot takes out

<!-- Pierce rule: crates/nova_gameplay/src/damage.rs hit_bite (flat, not speed-scaled) and pierce_remainder :427 - power spent per layer is max health / pierce_power_multiplier :258, which clamps at 3.0 (PIERCE_POWER_CEILING :197), and a slug at 15,000 m/s is always at that ceiling. No layer cap: railgun_section/firing.rs:206 (`layers: u32::MAX`). The rake: rake_radius 10 m standard.rs:957, swept in crates/nova_gameplay/src/rounds.rs sweep_raking :708 (the sphere trails the tip by its radius; only a body the tip hit directly is armed; contacts charged by depth then from the axis outward). The stand table and the three-times figure are examples/systems/system_railgun_lance.rs's stand bank (200 hp cells, 5 x 5 x 4 wall and 3 x 1 x 4 line), reproduced by the scope's model in web/tests/widgets.test.ts. Section healths standard.rs:596 (200), :675 (480), :692 (1250). -->

<div class="widget" data-widget="lance-corridor">
<p>The scope shoots a block of 200 hp reinforced hull cells, five across, five tall and four deep, with the shipped railgun. The slug's tip cuts the centre column, and a 10 m sphere trailing the tip widens that cut to the eight cells around it - the face neighbours and the diagonals, never the second ring. Every cell in the corridor takes 300 and pays a third of its max health out of the one 1800-point budget, so the shot takes 28 cells as nine, nine, nine and one, removes 5600 hp, and stops with the exit hole as wide as the entry. Set the radius to zero and the same shot takes four cells in a line; set it to 40 m and it takes the same 28 as the whole entry face plus three, and stops one layer in.</p>
</div>

The slug deals **300 Pierce damage to every section it takes**, flat. It is not scaled by how fast the two ships are closing and it does not decay with depth, so the tenth section in the line takes exactly what the first did. Three hundred clears every hull block, controller, mount and torpedo bay in the catalog outright, so an aligned shot takes a **corridor out of a ship** rather than damaging one.

| what the corridor takes | health | sections in one shot |
|---|---|---|
| Light Hull Section | 60 | 90 |
| Basic Controller, Torpedo Bay | 100 | 54 |
| PDC Turret | 130 | 41 |
| Reinforced / Cargo / Tank Hull | 200 | 27 |
| Vector Thruster Section | 480 | 11 |
| Capital Thruster Section | 1250 | 4 |

The corridor is wider than the bore. The slug drags a 10 m sphere behind its tip, so it takes the sections immediately beside the ones it actually crossed as well - about three cells across. It only ever widens what the tip has already reached: a ship the slug merely passes near loses nothing, and nothing ahead of the tip is touched by the shot that is on its way. The sphere keeps going after the tip is out the far side, so the exit hole is as wide as the corridor.

How much it takes is one budget: **1800 power**, spent in the toughest each taken section could ever be, and a slug this fast always spends at the cheapest rate the rule allows. Width and depth come out of the same number. There is no layer cap under it. Measured on the range the game tests the gun with, that is what the width is worth:

| the same shot into | needle (no rake) | shipped corridor | a 40 m sphere |
|---|---|---|---|
| a corvette line, 3 x 1 x 4 cells | 4 cells, 800 hp | **12 cells, 2400 hp** | - |
| a wall, 5 x 5 x 4 cells | 4 cells, 800 hp | 28 cells as 9 / 9 / 9 / 1 | 28 cells as 25 / 3 / 0 / 0 |
| per 13.5 s cycle, corvette line | 59 hp/s | **178 hp/s** | - |

<figure class="figure">
    <!-- Capture: assets/wiki-section-railgun-corridor.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-railgun-corridor.png</span
        >
        <span class="figure__placeholder-note"
            >The wall after the shot, entry face and exit face
            side by side: a square hole three cells wide clean
            through, the dead cells cold and cracked, the
            second ring untouched.</span
        >
    </div>
</figure>

<details class="explain">
<summary>Show explanation</summary>

Twenty-seven reinforced hull blocks is past the depth of anything that flies, which is exactly why the shot is wide: nothing in the game is thick enough to spend a slug's budget in one column, so the surplus goes sideways instead of out the far side. Against a four-cell corvette line the corridor removes three times what the bare needle did. **Depth is not what this weapon costs you** - the commit, the recoil and the reload are.

Wider is not more. Against a wall dense enough to bind the budget, a sphere four times the shipped radius removes exactly the same total, because both spend the same 1800 - it just spends it across the entry face and stops one layer in, where the shipped corridor bores three cells wide through all four layers and out the back. The radius chooses the shape of the hole; the power decides how much of it you get.

The two large drives are the one place a slug does not simply delete what it touches: 300 does not clear 480 or 1250, so a railgun cripples a capital drive over several passes instead of taking it off.

Reach is 18 km, which outranges every mount on the ship carrying it. That is what makes lining up worth doing - the [combat page's engagement ladder](../../combat-weapons/#three-reaches) puts the three weapons on one range axis.

</details>

## The recoil

The shot lands an impulse backwards along the bore, applied **at the muzzle** rather than at the ship's balance point. On a spinal mount that is a straight shove. Off-axis it is a shove **and a yaw**, every single time, and the further out you hung the gun the harder it kicks the nose around.

That is the price of a wing mount, and it is deliberate: the recoil arrives at the end of the shot, so the ship you have to re-aim is a ship that has just been spun. The railgun is also three cells long and carries 180 health, so it is a large and obvious thing to be carrying: it takes as much room on the hull as three plates, and it is that size to anything shooting back.

## The tempo

One shell, ever. The magazine holds a single round and takes **twelve quiet seconds** to return it, so a railgun fires roughly every thirteen and a half seconds counting the charge. There is no way to queue a second shot; a railgun that could would just be a turret with a slow fire rate.

The section's ammo gauge is the countdown, and the bore sight stays up dimmed through it. In practice the reload is the aiming phase: you spend it getting the hull onto the line you want, and the shell arrives about when you are ready to use it.

## Facing one

<!-- Behavior verified against crates/nova_ship/src/input/ai/railgun.rs: commit gate ~8 degrees of bore alignment (AI_RAILGUN_ALIGNMENT_COS 0.99, :36), inside 60% of the slug's reach (AI_RAILGUN_REACH_FACTOR, :43), plus a 14 s per-gun pilot cadence over the gun's own reload (AI_RAILGUN_COOLDOWN_SECS, :28). The module's own header records it as deliberately crude: the AI does not fly the shot. -->

An enemy railgun does not stalk you. A raider carrying one commits when its orbit happens to sweep the bore across something it is already fighting, inside about eight degrees and well within the slug's reach, and it spaces its shots further apart than the gun itself needs. So it lands the occasional slug and never sets one up.

Your warning is the same one you give: the charge. A ship whose nose swings dead onto you and then holds still is a ship that has committed, and the second and a half before the shot is the only part of it you can do anything about.

## Variants

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (railgun_lance_prototype and its two call sites in standard_section_prototypes) and assets/base/sections/base.content.ron. Only the campaign's stolen warship mounts one - the siege grade, both spinal guns (ships/block.rs stolen_warship); the editor sandbox's `picket_lance` carries the standard one (crates/nova_editor/src/scenario.rs). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-railgun.png</span></span></span><span class="catalog__title">Railgun - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Depth</th><th>Corridor</th><th>Charge</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-railgun-lance-section.png</span></span></span></td><td><span class="catalog__name">Railgun Lance</span><span class="catalog__id">railgun_lance_section</span></td><td class="catalog__num">300</td><td>Pierce</td><td class="catalog__num">1800 power</td><td class="catalog__num">10 m rake</td><td class="catalog__num">1.5 s</td><td class="catalog__num">1</td><td class="catalog__num">1 / 12 s</td><td class="catalog__num">15,000 m/s</td><td class="catalog__num">18 km</td><td class="catalog__num">180</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-siege-railgun-lance-section.png</span></span></span></td><td><span class="catalog__name">Siege Railgun Lance<span class="catalog__flag">experimental</span></span><span class="catalog__id">siege_railgun_lance_section</span></td><td class="catalog__num">500</td><td>Pierce</td><td class="catalog__num">360,000 power</td><td class="catalog__num">30 m rake</td><td class="catalog__num">1.5 s</td><td class="catalog__num">1</td><td class="catalog__num">1 / 12 s</td><td class="catalog__num">15,000 m/s</td><td class="catalog__num">18 km</td><td class="catalog__num">180</td></tr>
</tbody>
</table>
</div>

Two railguns ship, and they are the same gun at two grades. The standard lance is the one you bolt on yourself in the ship editor; no buildable hull carries either. The siege lance is the campaign's - the stolen warship's two spinal guns, priced to cross a carrier rather than bore a corridor through a corvette, and it is deliberately overpowered for that one scene. See [Ship sections for mods](../../../create/sections/#railgun) for the numbers a mod can change, the rake radius among them.
