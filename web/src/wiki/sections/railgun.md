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
            >A lance mounted on a ship's spine, charge bolt
            partway up the bore, bore sight drawn ahead of
            the muzzle onto a target.</span
        >
    </div>
</figure>

A railgun is a **spinal lance**: three cells of rails and capacitor bank with no traverse of its own. It does not aim. The **ship** aims, you commit the shot, and what leaves rakes through everything standing in that line.

It is the opposite weapon to a turret in every way that matters. A turret is a mount you assign and forget; a lance is a shot you set up. It fires once every thirteen seconds or so, the trigger cannot call it back once it is pulled, and it shoves the ship that fired it.

## The hull is the aim

<!-- Behavior verified against crates/nova_ship/src/sections/railgun_section/mod.rs (RailgunSectionConfig :61, RailgunSectionInput :185) and firing.rs (the slug is born at the muzzle along the section's own -Z, :195-210; recoil applied at that point, :225). Sockets: crates/nova_authoring/src/base_content/sections/standard.rs lance_link_points :1069 - thirteen, none on the muzzle face, held by `no_lance_sockets_the_face_it_fires_through`. -->

A lance fires down its own axis, so where it points is decided when you **bolt it on**, not when you shoot. Mounted on the spine it points where the nose points. Mounted on a flank it points wherever that flank faces, which is not where you are looking - and there is no HUD crosshair that will tell you otherwise, which is why the [bore sight](#the-bore-sight) exists.

The part carries thirteen mounting sockets: the breech plate and three per flank. The **muzzle face carries none**. A lance cannot traverse off its own line, so a socket in front of the bore would only ever be an invitation to bolt a plate into your own line of fire.

## Committing the shot

Tapping the trigger starts a **1.5 second charge**, and that is the whole of the decision. Letting go does nothing, holding does nothing, and the gun does not re-check the nose when the charge ends. The slug leaves down whatever line the hull is holding at the end of those seconds, so a target that slid off in the meantime is simply missed - and a target you led onto the line during the charge is hit.

There is exactly one way out, and it is not the trigger: dropping your weapons back to **safe** dumps the charge and keeps the shell. A lance will not fire itself into a friendly you have just safed for.

The shell also carries your ship's motion with it, spin included: a lance fired while the hull is rolling throws its slug slightly off the tangent as well as down the bore. At 1500 units a second that is a small correction, but it is there, and it is one more reason to be flying straight when the charge ends.

The charge is loud and visible, to you and to anyone watching. A bolt walks the length of the bore, so how far it has travelled is how much charge is left to run: an enemy across the gap can read a lance about to fire off its hull. The capacitor loop rises in pitch as it fills.

That tell is the balance of the weapon. A lance in flight is unanswerable - the slug crosses 1800 units in just over a second - so the window in which it can be answered is the charge, by breaking the line before it ends.

## The bore sight

<!-- Behavior verified against crates/nova_hud/src/bore_sight.rs (module docs; the trace walks crossed sections through `pierce_remainder`, the same function that resolves the round; gated on WeaponsHot; drawn dimmed on an empty magazine; the line thickens with the charge). -->

Because nothing else on the HUD says where a hull is pointing, the flight computer draws the line for you: a thin blue **sight line** out of the muzzle, ending exactly where the slug would end, with a **ring on every section that shot would destroy**.

The rings are the point. "Am I on it" is a question a target bracket already answers; the question a lance actually poses is "does this angle gut it or clip a corner", and the sight answers that by walking the sections in the line through the same penetration math the round itself is resolved with. Aiming down a ship's long axis looks visibly different from catching its shoulder.

The line thickens as the charge runs, so the seconds you are committed to holding a heading are readable without looking away. The sight is up whenever your weapons are hot - raised, or holding a combat lock - and it stays up, **dimmed**, while the magazine is empty. Twelve seconds of reload is exactly when you want to be lining the next shot up.

## What one shot takes out

<!-- Values from crates/nova_authoring/src/base_content/sections/standard.rs (health RAILGUN_BASE_HEALTH 180 :902,:39; three cells LANCE_CELLS :908,:328; charge_seconds 1.5 :927; slug_speed 1500 :928; slug_damage 300 :942; slug_power 1800 :948; slug_lifetime 1.2 :951 giving 1800 u of reach; recoil_impulse 45 :955; ammo_capacity 1 :962; reload delay 12 s :967). Pierce rule: crates/nova_gameplay/src/damage.rs hit_bite :399 (flat, not speed-scaled) and pierce_remainder :427 - power spent per layer is max health / pierce_power_multiplier (:258), which clamps at 3.0, and a 1500 u/s slug is always at that ceiling. No layer cap: firing.rs:206. Drive healths standard.rs:674,:691. -->

The slug deals **300 Pierce damage to every layer it crosses**, flat. It is not scaled by how fast the two ships are closing and it does not decay with depth, so the tenth section in the line takes exactly what the first did. Three hundred clears every hull block, controller, mount and torpedo bay in the catalog outright, so an aligned shot takes a **column out of a ship** rather than damaging one.

Depth is a separate budget: **1800 power**, spent in the toughest each crossed section could ever be, and a slug this fast always crosses at the cheapest rate the rule allows. There is no layer cap under that budget.

| what the line crosses | health | crossings in one shot |
|---|---|---|
| Light Hull Section | 60 | 90 |
| Basic Controller, Torpedo Bay | 100 | 54 |
| PDC Turret | 130 | 41 |
| Reinforced / Cargo / Tank Hull | 200 | 27 |
| Vector Thruster Section | 480 | 11 |
| Capital Thruster Section | 1250 | 4 |

Twenty-seven reinforced hull blocks is past the depth of anything that flies. **Depth is not what this weapon costs you** - the commit, the recoil and the reload are.

The two large drives are the one place a lance does not simply delete what it touches: 300 does not clear 480 or 1250, so a lance cripples a capital drive over several passes instead of taking it off.

Reach is 1800 units, which outranges every mount on the ship carrying it. That is what makes lining up worth doing.

## The recoil

The shot lands an impulse backwards along the bore, applied **at the muzzle** rather than at the ship's balance point. On a spinal mount that is a straight shove. Off-axis it is a shove **and a yaw**, every single time, and the further out you hung the gun the harder it kicks the nose around.

That is the price of a wing mount, and it is deliberate: the recoil arrives at the end of the shot, so the ship you have to re-aim is a ship that has just been spun. The lance is also three cells long and carries 180 health, so it is a large and obvious thing to be carrying: it takes as much room on the hull as three plates, and it is that size to anything shooting back.

## The tempo

One shell, ever. The magazine holds a single round and takes **twelve quiet seconds** to return it, so a lance fires roughly every thirteen and a half seconds counting the charge. There is no way to queue a second shot; a lance that could would just be a turret with a slow fire rate.

The section's ammo gauge is the countdown, and the bore sight stays up dimmed through it. In practice the reload is the aiming phase: you spend it getting the hull onto the line you want, and the shell arrives about when you are ready to use it.

## Facing one

<!-- Behavior verified against crates/nova_ship/src/input/ai/railgun.rs: commit gate ~8 degrees of bore alignment (AI_RAILGUN_ALIGNMENT_COS 0.99, :36), inside 60% of the slug's reach (AI_RAILGUN_REACH_FACTOR, :43), plus a 14 s per-gun pilot cadence over the gun's own reload (AI_RAILGUN_COOLDOWN_SECS, :28). The module's own header records it as deliberately crude: the AI does not fly the shot. -->

An enemy lance does not stalk you with it. A raider carrying one commits when its orbit happens to sweep the bore across something it is already fighting, inside about eight degrees and well within the slug's reach, and it spaces its shots further apart than the gun itself needs. So it lands the occasional lance hit and never sets one up.

Your warning is the same one you give: the charge. A ship whose nose swings dead onto you and then holds still is a ship that has committed, and the second and a half before the shot is the only part of it you can do anything about.

## Variants

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs :891-971 and assets/base/sections/base.content.ron:2258-2483. No shipped ship prototype mounts one; the editor sandbox's `picket_lance` does (crates/nova_editor/src/scenario.rs:824). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-railgun.png</span></span></span><span class="catalog__title">Railgun - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Damage</th><th>Type</th><th>Depth</th><th>Charge</th><th>Magazine</th><th>Recharge</th><th>Muzzle</th><th>Reach</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-railgun-lance-section.png</span></span></span></td><td><span class="catalog__name">Railgun Lance</span><span class="catalog__id">railgun_lance_section</span></td><td class="catalog__num">300</td><td>Pierce</td><td class="catalog__num">1800 power</td><td class="catalog__num">1.5 s</td><td class="catalog__num">1</td><td class="catalog__num">1 / 12 s</td><td class="catalog__num">1500 u/s</td><td class="catalog__num">1800 u</td><td class="catalog__num">180</td></tr>
</tbody>
</table>
</div>

One lance ships, and no mainline hull carries it. It is a part you bolt on yourself in the ship editor, or one a scenario builds onto a hull it spawns - see [Ship sections for mods](../../../create/sections/#railgun) for the numbers a mod can change.
