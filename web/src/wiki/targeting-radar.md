# Targeting & radar

Locking is deliberate: there is no passive auto-targeting. You sweep the radar yourself, your stance decides what kind of lock you get, and a lock sticks until you clear it or the target is gone.

<figure class="figure">
    <!-- Capture: assets/wiki-radar.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-radar.png</span
        >
        <span class="figure__placeholder-note"
            >Mid-sweep with CTRL held: the hollow radar box
            and a lock landing, ideally showing both a white
            nav lock and a red combat lock.</span
        >
    </div>
</figure>

## Holding to sweep

Hold <kbd>Ctrl</kbd> to run the radar: it tracks the best body within an 18-degree cone around your aim, re-targeting as you sweep. A lock is **earned by holding steady** through a short lock-on dwell, not granted the instant you point.

<div class="widget" data-widget="lock-sweep">
<p>The sweep in numbers: a quarter-second hold threshold separates a sweep from a clearing tap; past it the slot latches from your stance (lowered = white travel lock, raised = red combat lock), and a dwell scaled by range - 0.6 s point-blank stretching to 1.5 s at 20 km - has to fill while you keep the target under your aim. Sweeping off resets the dwell; a committed lock sticks until you clear it in stages with taps.</p>
</div>

<details class="explain">
<summary>Show explanation</summary>

The clocks, exactly: the tap/hold threshold is 0.25 s - at or past it the press is a sweep, under it a clearing tap. The slot latches from your stance *at the threshold*, not at the press, so raising weapons a tenth of a second into the hold still lands a combat lock. The dwell then grows linearly with range, from 0.6 s point-blank to 1.5 s at 20 km and beyond; the white ring on the HUD is that dwell filling. Sweeping onto empty space resets the charge; re-designating to a new target starts a fresh dwell while the old lock keeps holding underneath - so you never trade a lock for a maybe. Once a lock commits it sticks; releasing just ends the sweep.

</details>

<figure class="figure">
    <!-- Capture: assets/loops/lock-dwell.webm -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Loop capture needed</span
        >
        <span class="figure__placeholder-name"
            >assets/loops/lock-dwell.webm</span
        >
        <span class="figure__placeholder-note"
            >A short loop: CTRL held on a distant ship, the
            white dwell ring filling clockwise, and the lock
            snapping with its cue.</span
        >
    </div>
    <figcaption class="figure__caption">The dwell ring fills while you hold steady; the snap is the commit.</figcaption>
</figure>

## Stances and slots

Your stance picks the slot the lock lands in:

- Weapons **lowered** writes the **travel lock** (white crosshair), which feeds [GOTO](../flight-autopilot/).
- Weapons **raised** (combat stance) writes the **combat lock** (red reticle), which feeds guns, torpedoes and fine-lock. Your weapons are hot while raised or while a combat lock exists.

## Per-section fine-lock

Hold a combat lock focused for about a second and a half and you can drill into a specific [section](../sections/) of the enemy hull. The fine-lock either **snaps** to whatever your crosshair is on (with hysteresis so it does not flicker) or is **pinned** by manually cycling sections nose-to-tail with the brackets, scroll wheel or D-Pad; a manual pin holds for a couple of seconds. Turrets and the viewfinder both follow the fine-locked section.

## Clearing locks

Tap <kbd>Ctrl</kbd> (a press under the hold threshold) to clear in stages: with weapons lowered it drops the combat lock first, then the travel lock (which also disengages GOTO); with weapons raised it only ever drops the combat lock. Locks also fall on their own when the target dies, leaves range, or turns non-hostile.

An **idle** combat lock times out after about thirty seconds - "idle" meaning you are neither holding weapons raised nor firing, so a running fight never loses its lock no matter how long it lasts. You do not have to count: over the last few seconds of the window the red reticle dims and pulses faster and faster, and the moment anything you do counts as combat the reticle snaps back to full strength and the clock restarts.

## Lock ranges

How far you can lock depends on the target. Ships and gravity wells lock out to roughly **200 km**; a committed torpedo can be locked (to shoot it down) out to about **25 km**; smaller bodies carry a radar _signature_ that scales their range, and unsigned debris is point-blank only. An existing lock holds a little past its acquisition gate (hysteresis) so it does not chatter at the edge.
