# Multi-target tracking + subtarget cycle HUD

- STATUS: OPEN
- PRIORITY: 9
- TAGS: v0.5.0, hud, spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 3. Track several lockable candidates at once (the aim-assist in
`update_spaceship_target_input` already enumerates them, but keeps only the best),
show them as a target list / bracketed subtargets, and let the player cycle the
active lock between them (key/gamepad) in addition to panning. Consumer of the
screen-projected-indicator widget (20260708-165700).

Direction: promote the transient best-pick into a maintained candidate set (a
resource or per-candidate marker), render the set, and drive an explicit cycle input
alongside the existing look-to-aim behaviour.
</content>

Narrowed (20260709) by
docs/spikes/20260709-192358-component-lock-vats-lite.md: the subtarget-cycle
half lands with the component fine-lock (tatr 20260709-192522/192523). This
task keeps the multi-target half only: maintaining and rendering the candidate
SET of lockable ships and cycling the active ship lock between them.
