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

## Stacking controllers

A big hull can mount more than one, and steers better for it - with sharply diminishing returns. The extras do not each add their own steering; they share the work, and the ship's total steering budget grows on a curve that flattens out:

| Controllers | Steering budget | Peak turn rate |
| --- | --- | --- |
| 1 | 1.00x | 1.00x |
| 2 | 1.50x | ~1.22x |
| 4 | 1.75x | ~1.32x |
| 10 | 1.90x | ~1.38x |

The budget can never pass **twice** what one controller is worth, no matter how many you bolt on. Controller authority is authored as angular acceleration, so hull inertia does not weaken it: a capital hull and a light craft carrying the same computer get the same baseline response. Steering lag describes how far the hull trails a moving command; a mixed stack uses its fastest live computer.

Stacking adds authority and _precision_. A stacked hull starts arresting its turn earlier, so it stops on the heading you pointed at instead of sailing past and swinging back. The second controller gives most of the available gain; the tenth is nearly dead weight.

<div class="widget" data-widget="controller-stacking">
<p>The budget follows one curve toward its x2.00 ceiling: two controllers reach x1.50, four x1.75, ten x1.90 - the tenth adds about a hundredth. Peak turn rate grows with the square root of the budget.</p>
</div>

The other half of stacking is **redundancy**. Lose one of two and the ship does not go brain-dead - it drops to single-controller handling and keeps fighting. Only the last one is the ship's brain.

<figure class="figure">
    <!-- Capture: assets/loop-section-controller.webm (short gameplay loop) -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loop-section-controller.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: a ship flipping to a new heading
            and settling on it without overshoot.</span
        >
    </div>
</figure>

## Variants

Every shipped flight computer steers with the same numbers - authority is authored as angular acceleration, so hull inertia does not weaken it. What separates the fuselage computers is how much structure they add to the ship that carries them.

<div class="catalog">
<!-- Stats verified against crates/nova_authoring/src/base_content/sections/standard.rs (basic_controller_section mass :344, health :346, steering_lag 0.5 :354, max_angular_acceleration 0.5 :359) and crates/nova_authoring/src/base_content/ships/ (semantic-part mass shared.rs:204, controller kind steering_lag/authority shared.rs:246-247; racer.rs:81 fuselage 240; cargo_b.rs:77 fuselage 300; cargo_a.rs:84 fuselage 350). -->
<div class="catalog__head"><span class="catalog__kindicon"><span class="figure__placeholder"><span class="figure__placeholder-name">assets/icon-controller.png</span></span></span><span class="catalog__title">Controller - shipped prototypes</span></div>
<table>
<thead>
<tr><th></th><th>Variant</th><th>Turn authority</th><th>Steering lag</th><th>Health</th><th>Mass</th></tr>
</thead>
<tbody>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-basic-controller-section.png</span></span></span></td><td><span class="catalog__name">Basic Controller Section</span><span class="catalog__id">basic_controller_section</span></td><td class="catalog__num">0.5 rad/s^2</td><td class="catalog__num">0.5 s</td><td class="catalog__num">100</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-racer-fuselage.png</span></span></span></td><td><span class="catalog__name">Racer // Fuselage</span><span class="catalog__id">racer_fuselage</span></td><td class="catalog__num">0.5 rad/s^2</td><td class="catalog__num">0.5 s</td><td class="catalog__num">240</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargob-fuselage.png</span></span></span></td><td><span class="catalog__name">CargoB // Fuselage</span><span class="catalog__id">cargob_fuselage</span></td><td class="catalog__num">0.5 rad/s^2</td><td class="catalog__num">0.5 s</td><td class="catalog__num">300</td><td class="catalog__num">1.0</td></tr>
<tr><td><span class="catalog__thumb"><span class="figure__placeholder"><span class="figure__placeholder-tag">capture</span><span class="figure__placeholder-name">assets/catalog-cargoa-fuselage.png</span></span></span></td><td><span class="catalog__name">CargoA // Fuselage</span><span class="catalog__id">cargoa_fuselage</span></td><td class="catalog__num">0.5 rad/s^2</td><td class="catalog__num">0.5 s</td><td class="catalog__num">350</td><td class="catalog__num">1.0</td></tr>
</tbody>
</table>
</div>
