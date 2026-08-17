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

The other half of stacking is **redundancy**. Lose one of two and the ship does not go brain-dead - it drops to single-controller handling and keeps fighting. Only the last one is the ship's brain.
