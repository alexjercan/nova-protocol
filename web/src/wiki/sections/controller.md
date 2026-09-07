# Controller

<figure class="figure">
    <!-- Capture: assets/icon-controller.png (or a full shot) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-section-controller.png</span
        >
        <span class="figure__placeholder-note"
            >A ship turning under control, or the controller
            section highlighted on a build.</span
        >
    </div>
</figure>

The controller is the ship's steering system: it rotates the ship toward a target heading, easing in and settling without overshoot rather than snapping. It is **required** for any ship a player or AI can fly.

With no live controller, the hull cannot steer itself - so destroying the **last** controller _disables_ a ship without destroying it outright, leaving a drifting, tumbling wreck. It is also what the autopilot verbs drive when they fly the ship for you.

A hurt computer cracks and, past about a third of its health gone, throws sparks. It steers exactly as well as it did new right up to the moment it dies: handling degrades when a computer is LOST, never while one is merely damaged.

## What sets how hard a ship turns

<!-- Stats verified against crates/nova_events/src/scale.rs (LOAD_LIMIT 8 * 9.81 m/s^2 :17) and crates/nova_ship/src/physics/attitude.rs (the two ceilings and the lower one winning :75-95 - the structural one is LOAD_LIMIT in m/s^2 over an arm in meters, so rad/s^2 comes out unchanged by the scale; the arm to the outer FACE of the furthest live section :157-184, engine world units crossed to Meters by its caller, reading 5.52 u (55.2 m) on the base Patrol Gunship :151, the assembled mass properties :186-197, the sustained rate :111-113, the vector load a hard turn spends and the direction rule that holds it :117-145 with crates/nova_ship/src/physics/pd_controller.rs:156-190). The hull the widgets fly is block_gunship in assets/base/ships/base.content.ron - 53 authored cells, two flight computers at 1501 of torque each (crates/nova_authoring/src/base_content/sections/standard.rs:701,:718) - and web/tests/widgets.test.ts holds the widget's cell table to that ship's section count, mass and arm. Density is 1 and not authorable, so a section's mass IS its authored box (crates/nova_ship/src/sections/base_section.rs:470-471). Severing on a disconnected graph, and the rank that picks which body keeps the ship: crates/nova_ship/src/sections/integrity.rs:231-349,:345-385. -->

The computer does not decide it on its own. A ship turns as hard as the lower of two limits allows:

- **Its computers.** Their torque against the mass they have to swing. A heavier ship needs more of it for the same turn.
- **Its structure.** Hull metal takes 8 G and no more. The further the ship's furthest section sits from its balance point, the gentler the hardest turn that metal survives.

<div class="widget" data-widget="controller-arm">
<p>The base Patrol Gunship carries its mass over a 55.2 m arm - balance point to the outer face of its furthest section, which is the tip of its bow spur. Hull metal takes 8 G, so that arm allows 1.42 rad/s^2 and no more, while its two flight computers together could push 8.74 - six times as hard. Shoot the bow off and the balance point slides aft: the arm drops to 41.6 m, the ceiling climbs to 1.89 rad/s^2, and the wreck flips 180 degrees in 2.58 s where the whole ship took 2.97 s. Cut the spine instead and only the dorsal ridge and the two computers on it are still one ship: twenty sections sever away as wreckage.</p>
</div>

Every ship in the base fleet but one is held by the second, with a wide margin on the first. The exception is the carrier: 2,081 sections and enough inertia that its ten computers run out first, so it turns at 0.07 rad/s^2 where its metal would have allowed 0.40. Nothing authors either result - both fall out of the shape you built. A short craft swings on a short arm and whips around. A long hauler swings on a long arm and handles like the freighter it is.

Two things follow that you feel in the cockpit:

- **A wreck turns sharper.** Shoot the bow off and the ship's balance point slides aft, shortening the reach to everything behind it - so what is left of the ship turns harder than the whole ship did. Mid-turn, on the same stick input, the survivor speeds up into it.
- **A hard turn spends the margin.** Holding a fast turn already loads the hull. There is less left to turn harder with, and past a point there is none at all: the ship holds that rate for as long as you hold the stick, and no amount of input tightens it further. Ease off and the margin comes back.

<div class="widget" data-widget="controller-margin">
<p>The 8 G limit is one acceleration at the hull's furthest point, and the load that holds a curve and the load that tightens it add as a vector rather than being counted separately. So the gunship still has 97% of its 1.42 rad/s2 in hand at 34 deg/s, 83% at 51 deg/s, and nothing at all at 68 deg/s - the rate at which holding the turn spends the whole budget on its own. Authority does not taper off; it holds, then falls away.</p>
</div>

## Stacking controllers

A big hull can mount more than one. They do not each steer it: they share one steering loop, and their torque adds into it.

On a ship that is already at its structural limit - which is every base hull but the carrier - that extra torque buys no turn rate at all. Metal does not care how many computers push it. Only a hull heavy enough to run its computers out first gains turn rate from a stack, and only until it reaches its own structural limit too. The carrier is the one shipped hull on that side of the line, which is why it carries ten.

<!-- Flown 170 degree flips, printed by crates/nova_ship/src/flight/tests/stacking.rs::size_decides_the_turn_and_stacking_only_helps_a_torque_bound_hull (:314). Light hull = the "fighter" rig, 3 unit cells (:284-290); heavy barge = the "barge" rig, 15 cells at 20x density (:298-304); 5 degree traverse gate :232; stack sizes :306; 170 degree turn :309. -->

| Computers | Light hull: flip | Heavy barge: flip | Heavy barge: overshoot |
| --- | --- | --- | --- |
| 1 | 2.77 s | 7.52 s | 6.7 deg |
| 2 | 3.17 s | 5.77 s | none |
| 4 | 3.37 s | 5.03 s | none |
| 10 | 3.48 s | 5.13 s | none |

A measured 170 degree flip - the time to come within 5 degrees of the new heading. The light hull gains nothing and pays a little; the barge nearly halves its flip by the fourth computer and then stops dead, because it has run into its own structure.

What stacking always buys is _precision_. A stacked hull starts arresting its turn earlier, so it stops on the heading you pointed at instead of sailing past and swinging back - which is the barge losing its 6.7 degree overshoot. The second computer gives most of that gain; the tenth is nearly dead weight, and on a light craft it is a little slower for nothing.

The other half of stacking is **redundancy**. Lose one of two and the ship does not go brain-dead - it drops to single-controller handling and keeps fighting. Only the last one is the ship's brain.

<figure class="figure">
    <!-- Capture: assets/loops/loop-section-controller.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/loop-section-controller.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: a ship flipping to a new heading
            and settling on it without overshoot.</span
        >
    </div>
</figure>

## Variants

One flight computer ships, and every hull in the game is steered by copies of it. What a ship does with them is the ship's own business: its mass and its length decide that, and a second one buys redundancy and precision rather than turn rate.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_controller_section health CONTROLLER_BASE_HEALTH 100.0 used :701, steering_lag 0.5 :709, max_torque 1501.0 :718). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-controller.png</span></span></span><span class="catalog__title">Controller - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Torque</th><th>Steering lag</th><th>Health</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-controller-section.png</span></span></span></td><td><span class="catalog__name">Basic Controller Section</span><span class="catalog__id">basic_controller_section</span></td><td class="catalog__num">1501</td><td class="catalog__num">0.5 s</td><td class="catalog__num">100</td></tr>
</tbody>
</table>
</div>

