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

Hold <kbd>Ctrl</kbd> to run the radar: it tracks the best body on your look ray, re-targeting as you sweep, considering only candidates within a cone (about 18 degrees) around your aim. Cross the ~0.25 s hold threshold and the slot arms (your stance picks which - see below); then a **lock-on dwell** has to fill while you keep the target under your aim before the lock actually commits. The dwell takes longer the farther away the target is, and sweeping off the target before it fills cancels it - so a lock is earned by holding steady, not granted the instant you point. Re-designating to a new target starts a fresh dwell while the old lock holds. Once a lock commits it sticks; releasing just ends the sweep.

## Stances and slots

Your stance picks the slot the lock lands in:

- Weapons **lowered** writes the **travel lock** (white crosshair), which feeds [GOTO](../flight-autopilot/).
- Weapons **raised** (combat stance) writes the **combat lock** (red reticle), which feeds guns, torpedoes and fine-lock. Your weapons are hot while raised or while a combat lock exists.

## Per-section fine-lock

Hold a combat lock focused for about a second and a half and you can drill into a specific [section](../sections/) of the enemy hull. The fine-lock either **snaps** to whatever your crosshair is on (with hysteresis so it does not flicker) or is **pinned** by manually cycling sections nose-to-tail with the brackets, scroll wheel or D-Pad; a manual pin holds for a couple of seconds. Turrets and the viewfinder both follow the fine-locked section.

## Clearing locks

Tap <kbd>Ctrl</kbd> (a press under the hold threshold) to clear in stages: with weapons lowered it drops the combat lock first, then the travel lock (which also disengages GOTO); with weapons raised it only ever drops the combat lock. Locks also fall on their own when the target dies, leaves range, or turns non-hostile, and an idle combat lock times out after about thirty seconds.

## Lock ranges

How far you can lock depends on the target. Ships and gravity wells lock out to roughly **20000 u**; a committed torpedo can be locked (to shoot it down) out to about **2500 u**; smaller bodies carry a radar _signature_ that scales their range, and unsigned debris is point-blank only. An existing lock holds a little past its acquisition gate (hysteresis) so it does not chatter at the edge.
