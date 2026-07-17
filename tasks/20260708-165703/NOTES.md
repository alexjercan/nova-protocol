# Lock-on acquisition dwell + radial ring - arc notes

Two tasks, one feature: the mechanic (20260708-165703) and its visible cue
(20260717-004302). This doc is the end-to-end record; per-task specifics are in
each TASK.md.

## The mechanic (input/targeting.rs)

Radar locks used to commit the instant a candidate settled under the aim ray
(the deliberate-radar model: hold CTRL, the picker live-writes the slot every
held frame past the 0.25 s threshold). Now every slot write is gated behind a
per-target ACQUISITION DWELL:

- While the radar is held and a candidate is settled, `RadarState.dwell_secs`
  charges on `RadarState.dwell_target`. The slot (`TravelLock`/`CombatLock`,
  whichever the stance latched) is written ONLY when the dwell completes.
- Sweeping to a different candidate, or off into empty space, resets the dwell
  (`dwell_target`/`dwell_secs`/`dwell_needed` cleared) - the cancel-by-moving-off
  beat. Re-designating to a new target earns a FRESH dwell; the previously
  committed lock keeps-last while the new one charges.
- The once-per-gesture acquire cue (`RadarLockAcquired` -> `NovaSfx::LockOn`)
  and the re-designation tick (`RadarRetargeted`) moved to the commit, so the
  audio snap now lands exactly on the ring completing - no new audio work.

Both slots are gated (user decision 2026-07-17: combat AND travel), and this is
a SEPARATE, earlier stage than the existing 1.5 s component focus dwell
(`LockFocus`, untouched): the ring fills to get the ship lock, then the focus
bar fills to unlock the component fine-lock layer.

### Dwell duration

`lock_dwell_secs(distance, modifier, settings)` (pure, unit-tested):

    raw = base * (1 + range_factor * clamp(distance / reference_range, 0, 1)) * modifier
    dwell = clamp(raw, min, min.max(max))

Tunables live on `TargetingSettings` (reflected, inspector-editable). Shipped
values:

- `lock_dwell_base` = 0.6 s (point-blank dwell)
- `lock_dwell_range_factor` = 1.5 (a lock at the reference range costs 2.5x base)
- `lock_dwell_reference_range` = 2000 u (distance term saturates here)
- `lock_dwell_min` = 0.25 s, `lock_dwell_max` = 2.5 s

Distance is normalized by a fixed `reference_range`, NOT the target's own
effective lock range: dividing by effective range coupled the dwell to object
class oddly (a ship lockable at 20 km would dwell ~instantly). The `min.max(max)`
in the clamp guards a misordered-knob panic.

### The stealth seam

`modifier` is passed 1.0 today. It is the extension point for a future
stealth/aspect mechanic ("harder to lock at a bad aspect / partially invisible
at a certain degree"): such a mechanic multiplies the dwell up via `modifier`
(read from an optional per-target component), with no change to the gate or the
HUD. Not built here.

## The visible cue (hud/lock_dwell_ring.rs + assets/shaders/lock_dwell_ring.wgsl)

nova's first `UiMaterial`: a WGSL fragment draws a thin annulus that fills
clockwise from the top as a `progress` uniform goes 0 -> 1. A thin consumer of
the `screen_indicator` widget - one ring node whose anchor the driver points at
`RadarState.dwell_target` (the PENDING candidate, which during a re-designation
differs from the still-committed lock) while `RadarState.is_dwelling()`, and
whose material `progress` tracks `RadarState.dwell_fill()`. The widget shows /
hides / sizes / projects the node for free; the ring vanishes the instant the
dwell completes (the LockOn SFX is the audible half of the same snap). The layer
spawns / despawns with the player ship via the hud/mod.rs observers, and the
`RadarState` read surface (`dwell_needed`, `dwell_fill`, `is_dwelling`) was added
to the mechanic so the HUD renders the fill without recomputing the distance
curve.

Palette: a near-white `LinearRgba(1.0, 1.0, 1.0, 0.9)` acquiring ring (playtest
tweak 2026-07-17, from the initial spring-green), sized 39.2 px. A feel knob.

Rendering choice: a real `UiMaterial` arc, not the `ammo_readout` trig-pip
segment ring. The segmented ring was the wasm escape hatch; the shader is
trivial maths (no textures, no derivatives), safe on the WebGL2 target, and the
11_hud_range example run exercises it under a real render app.

## Deferred

- The stealth/aspect mechanic itself (the `modifier` seam is ready).
- AI-side dwell (the dwell is player-only; AI still commits its mirror
  instantly).
- Any per-target dwell-difficulty component (would feed `modifier`).
