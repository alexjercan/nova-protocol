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

Design spike (20260711): docs/spikes/20260711-163800-multi-target-cycle.md -
candidate set resource (top-5 hostile ships, ranked), dim bracket markers via
the screen-indicator widget, CTRL+scroll cycle (plus CTRL+brackets, dpad
up/down) with a ~4 s lock pin mirroring the component pin, and new
bottom-left hint rows [SCROLL] COMPONENT / [CTRL+SCROLL] TARGET.

Narrowed (20260709) by
docs/spikes/20260709-192358-component-lock-vats-lite.md: the subtarget-cycle
half lands with the component fine-lock (tatr 20260709-192522/192523). This
task keeps the multi-target half only: maintaining and rendering the candidate
SET of lockable ships and cycling the active ship lock between them.

## Goal

The player sees the set of nearby lockable hostile ships as dim bracket
markers, and CTRL+scroll (or CTRL+brackets / dpad up/down) deliberately moves
the active lock through that set, pinned against the aim-driven picker for a
few seconds. The bottom-left keybind cluster documents both scroll gestures.

## Steps

- [ ] Candidate set resource in `crates/nova_gameplay/src/input/targeting.rs`:
      `SpaceshipPlayerTargetCandidates` (ranked `Vec<Entity>`), maintained at
      the end of `update_spaceship_target_input` from the `candidates` vec it
      already collects. Membership: hostile (`is_hostile`) ships
      (`Has<SpaceshipRootMarker>` needs to be threaded through or re-derived);
      rank by angle to the aim ray, then distance; keep top
      `TARGET_CANDIDATE_COUNT` (const, 5); the current lock is always a member
      while it remains in the collected candidate list. Export via the module
      prelude. Unit tests on the pure ranking helper.
- [ ] Ship-lock pin state in targeting.rs: a `TargetLockMode`-style value
      (Aim | Pinned { until }) alongside the lock (new resource or a field on
      the candidates resource - pick whichever reads cleaner; the lock
      resource itself stays `Option<Entity>` for its many consumers). While
      pinned, `update_spaceship_target_input` does not overwrite the lock; the
      pin clears on deadline (`TARGET_PIN_WINDOW` const, 4.0 s), or when the
      pinned entity stops being collectible as a candidate (died / out of
      range). While pinned, candidate maintenance is order-stable: drop dead
      entries, append newcomers, no re-rank (spike: frozen cycle snapshot).
      Unit tests: pin holds against the cone pick, expires, dies with target.
- [ ] Cycle input actions `TargetCycleNextInput`/`TargetCyclePrevInput` in
      targeting.rs + observers stepping the lock through the candidates vec
      from the current lock's index (wrap; no lock yet -> first/last), setting
      the pin. Cycling requires >= 2 candidates but NOT focus (unlike the
      component cycle). Unit tests via the shared step helper, mirroring
      `step_component_lock`'s tests.
- [ ] Bindings in `crates/nova_gameplay/src/input/player.rs` (flight rig,
      `spawn_flight_input`): a `TargetCycleModifierInput` action bound to
      `KeyCode::ControlLeft`/`ControlRight`; `TargetCycleNextInput` = Chord on
      the modifier + (mouse wheel up swizzle/clamp as the component bindings),
      plus `KeyCode::BracketRight` chorded, plus `GamepadButton::DPadUp`;
      Prev symmetric with wheel-down / BracketLeft / DPadDown. Add
      `BlockBy::single(modifier)` to `ComponentCycleNextInput`/`PrevInput` so
      plain scroll stops firing while CTRL is held. Chord/BlockBy need the
      modifier action's Entity, so this block moves from the `actions!` macro
      to `Actions::<FlightInputMarker>::spawn(SpawnWith(...))` (see
      bevy_enhanced_input 0.26 chord.rs/block_by.rs docs). Verify in-game or
      via an input-focused test that CTRL+scroll does NOT also cycle
      components.
- [ ] Candidate HUD in new `crates/nova_gameplay/src/hud/target_candidates.rs`,
      following the `component_lock.rs` reconcile pattern: a
      `screen_indicator_layer()` root spawned/despawned with the player HUD in
      `hud/mod.rs`; one bracket marker per candidate EXCEPT the active lock
      (the reticle already marks it);
      `ScreenIndicatorSize::ApparentSize { min_px: ~28 }`, offscreen Hide
      (edges are 165704's job), dim hostile red distinct from the
      component-marker red (see `component_lock.rs` color notes). Bracket look:
      four corner nodes like the torpedo reticle if cheap, else a thin
      `BackgroundColor`-less bordered node. Tests mirroring
      `sync_component_markers`'s.
- [ ] Hint rows in `crates/nova_gameplay/src/hud/keybind_hints.rs` +
      `input/player.rs`: extend `FlightVerbHints` with `component_cycle` and
      `target_cycle` `VerbHint`s (fixed labels "SCROLL" and "CTRL+SCROLL" -
      wheel bindings have no keyboard label, spike open question resolved as
      fixed strings). Availability: component_cycle = focus complete on the
      lock with >= 2 attached sections; target_cycle = >= 2 candidates.
      Extend `ROW_VERBS`/`row_hint` to 6 rows (COMPONENT, TARGET appended).
      Update the existing cluster tests.
- [ ] Full check suite; update the spike doc's Next steps/Fix record with what
      shipped.

## Notes

- Key files: crates/nova_gameplay/src/input/targeting.rs (lock, focus,
  component cycle - the pin/step/observer patterns to mirror),
  crates/nova_gameplay/src/input/player.rs (flight rig bindings at ~440-536,
  FlightVerbHints at ~67-180), crates/nova_gameplay/src/hud/component_lock.rs
  (reconcile + highlight pattern), crates/nova_gameplay/src/hud/keybind_hints.rs,
  crates/nova_gameplay/src/hud/screen_indicator.rs (widget API),
  crates/nova_gameplay/src/relations.rs.
- bevy_enhanced_input 0.26: `Chord::single(entity)` fires only while the
  modifier action fires; `BlockBy::single(entity)` suppresses while it fires.
  Both need entity refs -> `SpawnWith` spawning, verified in the crate source
  (~/.cargo/registry/.../bevy_enhanced_input-0.26.0/src/condition/).
- The existing component-cycle wheel bindings use
  `(Binding::mouse_wheel(), SwizzleAxis::YXZ, Clamp::pos())` and the negated
  variant - reuse the same modifier stack for the chorded wheel bindings.
- Sibling 20260708-165704 consumes `SpaceshipPlayerTargetCandidates`; keep it
  public via the prelude.
