# Scroll rebind: SCROLL cycles targets, SHIFT+SCROLL cycles components, CTRL+SCROLL retired

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0, targeting, input, spike

## Goal

Promote target cycling to the unmodified wheel and move component fine-lock
cycling to SHIFT+SCROLL, per the two-slot model (spike 20260712-222610).
CTRL loses its scroll role entirely and keeps only free-aim. Lands against
the current resource-based single lock (before the slot split), so the
immediate behavior is: wheel = today's CTRL+wheel, SHIFT+wheel = today's
plain wheel. Pure input-dispatch swap; no targeting state changes.

## Steps

- [ ] Verify how turret free-aim reads CTRL (player.rs:361 area): confirm
      whether it shares `TargetCycleModifierInput` or reads its own
      action/keys. Free-aim must be completely untouched by this task; if
      shared, split the actions first.
- [ ] Swap the dispatch, keeping the observer-dispatch pattern (NOT
      binding-level Chords - bug 20260711-173237: Chord::evaluate ignores
      the binding value): rebind the modifier action to
      ShiftLeft/ShiftRight (rename `TargetCycleModifierInput` ->
      `ComponentCycleModifierInput`, player.rs:616-626), and in the wheel
      observers (targeting.rs:798-821, :889-912) swap the branches -
      unmodified wheel -> `step_target_lock`, modifier held ->
      `step_component_lock`.
- [ ] Gamepad unchanged: DPadUp = target next, DPadLeft/Right = component
      cycle (cite: player.rs:628-692 binding rig). Confirm no pad path
      routed through the swapped branches regresses.
- [ ] Update HUD hint rows that render the cycle gestures (binding_label
      usages; grep for the CTRL+scroll hint text) to the new gestures.
- [ ] Port/extend the dispatch tests with state-per-step assertions (retro
      20260711-173237 rule): wheel-only steps the target lock; SHIFT+wheel
      steps the component lock; SHIFT alone no-ops; wheel with empty
      candidate list no-ops; releases clear cleanly.
- [ ] cargo fmt + cargo check + run the targeting/input test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md.
- No dependencies - first task of the sequence; 20260712-223035 builds on
  the swapped dispatch.
- SHIFT is unbound in gameplay input today (verified this session).
- After the slot split (20260712-223035) the wheel routes per view
  (travel/combat); this task deliberately does NOT introduce view routing.
