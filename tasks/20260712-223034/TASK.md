# Scroll rebind: SCROLL cycles targets, SHIFT+SCROLL cycles components, CTRL+SCROLL retired

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.5.0, targeting, input, spike

## Goal

Promote target cycling to the unmodified wheel and move component
fine-lock cycling to SHIFT+SCROLL (spike 20260712-222610, rounds 1+3).
CTRL loses its scroll role and keeps free-aim only. Lands against the
current resource-based single lock; wheel = today's CTRL+wheel,
SHIFT+wheel = today's plain wheel. Brackets and DPad keep component
cycling unmodified.

Body rewritten after the round-3 adversarial review (feasibility B2, m1) -
the original "swap the observer branches" shape would have regressed
brackets/DPad, which share actions with the wheel.

## Steps

- [ ] Move the WHEEL bindings (with their swizzle/negate/clamp modifiers)
      off `ComponentCycleNextInput`/`ComponentCyclePrevInput`
      (player.rs:628-663) onto `TargetCycleNextInput` /
      `TargetCyclePrevInput` (player.rs:670-688; prev has no bindings
      today). BracketLeft/Right and DPadLeft/Right STAY on the component
      actions; DPadUp stays on target-next.
- [ ] Rename `TargetCycleModifierInput` -> `ComponentCycleModifierInput`
      and rebind CTRL -> ShiftLeft/ShiftRight (player.rs:616-626). Move
      the modifier dispatch from the component-cycle observers into
      `on_target_cycle_next`/`on_target_cycle_prev`
      (targeting.rs:854-912): unmodified -> `step_target_lock`, modifier
      held -> `step_component_lock` (observers gain the focus/component/
      section params). Component-cycle observers lose their dispatch -
      brackets/DPad always component-cycle. Keep the observer-dispatch
      pattern; binding-level Chords are forbidden (bug 20260711-173237).
- [ ] Free-aim is untouched by construction: it reads raw CTRL keys
      (player.rs:434), not the modifier action (verified in round 3).
- [ ] Update BOTH hint ends: the gesture strings in
      `update_flight_verb_hints` (player.rs:256 "SCROLL", :268
      "CTRL+SCROLL") AND the field-to-caption pairing in
      keybind_hints.rs:242-243 with its pinned tests
      (keybind_hints.rs:467-469) - after the swap the `component_cycle`
      field carries "SHIFT+SCROLL" and `target_cycle` carries "SCROLL";
      do not cross-wire. Caption wording must not imply brackets need
      SHIFT (they do not).
- [ ] Tests with state-per-step assertions (retro 20260711-173237):
      wheel-only steps the TARGET lock; SHIFT+wheel steps the COMPONENT
      lock; SHIFT alone no-ops; bare BracketRight and DPadRight still
      component-cycle; DPadUp still target-cycles; empty candidate list
      no-ops; pause gating holds; releases clear cleanly.
- [ ] cargo fmt + cargo check + run the targeting/input/hud test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md (round 3
  delta 3; round 4 confirms the gesture map). Bodies rewritten clean
  post-round-4 per user directive.
- No dependencies; first task of the sequence. 20260712-223035 routes the
  wheel per raised-state on top of this.
- Input tests need the `app.finish()` dance (retro 20260708-165705;
  existing examples player.rs:1519, targeting.rs:2271).
