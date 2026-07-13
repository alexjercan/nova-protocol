# Travel/combat lock slots + deliberate radar: CTRL hold/release/tap, componentized locks, capability flag

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.5.0, targeting, input, hud, spike

## Outcome (CLOSED 2026-07-13)

Shipped the full slot split + deliberate radar per the plan and the D1-D9
decisions. Highlights and deviations:

- Components on the player ship root (`targeting_state()` bundle, inserted by
  an observer on PlayerSpaceshipMarker): TravelLock, CombatLock, CombatDecay,
  LockFocus, ComponentLock, ThreatContacts; RadarState exists only while the
  search is held. All four legacy resources deleted.
- Radar gestures exactly as specced: RadarHoldInput (Hold) + RadarClearInput
  (Tap) on one RADAR_TAP_SECS const, commit on Complete only, clear on
  Fire<Tap>, Cancel closes the search; slot latched at Start (D2); empty
  release no-op (D1); staged tap-clear + GOTO disengage + toast message (D3a);
  same-entity re-commit preserves focus by equality. Pause drops the commit.
- Deliberate-only: auto cone pick, signature fallback, sticky held gate, pin
  machinery, CTRL+scroll cycle, TargetCycleNext/Prev + DPadUp binding, the
  target_cycle hint row and the CTRL free-aim raw read all deleted; the wheel
  is always the component cycle. Free-aim re-keyed to WeaponsRaised
  (manual-wins semantics finalized in 082337).
- The scanner range model survives as collect_lockable (shared by the radar
  picker and the always-on upkeep); radar_pick adds the D7 angular hysteresis
  (pick_target folded into it - it had no other consumer). unsigned_lock_range
  retuned 15 -> 5.
- update_contacts_and_locks: validity clears (death/despawn/out-of-range with
  incumbent range hysteresis), allegiance-FLIP clear (Changed-gated so
  deliberate neutral locks stay legal), 30 s CombatDecay (raised resets;
  firing joins in 082337), ThreatContacts top-5 + lock-member rule (D9);
  target_candidates HUD module deleted, edge indicators re-keyed.
- Lock capability: FlightVerb::Lock + ControllerVerbs.lock (default granted,
  so catalog + shakedown ships inherit it); radar start gated on it.
- HUD: new hud/lock_crosshairs.rs - white travel crosshair (min 40 px, the
  combat reticle stays 32 px so an overlapped pair reads as two rings; the
  combat reticle KEEPS its relation tint - red on hostiles is the common case,
  and own/neutral tints are information worth keeping), hollow slot-colored
  provisional box with the candidate's name, fading tap-clear toasts.
- 12_hud_range: the script now SETS the locks (radar stand-in - the gesture
  pipeline is covered by the targeting e2e tests with real input + manual
  real-clock stepping); every downstream HUD assertion unchanged and green.

Verified: 459 nova_gameplay tests green (radar rig family, upkeep family with
a REGISTERED system so Changed<> detection is real across runs, gesture e2e
incl. the exact-boundary frame, capability denial with delivery guard, pause
drop; ~25 old-contract tests rewritten/deleted); 12_hud_range + 10_gameplay +
03_scenario autopilots green; fmt + workspace --tests clean.

Gotchas for the retro:
- run_system_once rebuilds the system each call, so Changed<T> filters see
  everything as changed - the allegiance-flip tests MUST use
  world.register_system + run_system.
- SAME-FRAME RMB+CTRL press latches the TRAVEL slot: the raised flag derives
  in Update (camera chain) while the radar Start observer fires in PreUpdate.
  Humanly the raise precedes the radar by frames, and the 12_hud_range live
  gesture staggers them; recorded as a sharp edge (a PreUpdate raised
  derivation or a TriggerState-read latch would remove it if playtest ever
  hits it).

Live proof: 12_hud_range now performs the REAL gesture through the live input
pipeline (press RMB, press CTRL, the radar finds the ship dead ahead by
itself, release commits the combat lock) and every downstream HUD assertion
passes unchanged.

## Goal

Replace passive lock acquisition with deliberate radar locking on two
coexisting sticky ship-root lock components (spike 20260713-082207 + its
adversarial round and the D1-D9 decisions). Hold CTRL = radar (live retarget),
release = commit into the slot LATCHED at press (travel when lowered, combat
when raised), tap = staged clear. All aim-assist auto-acquisition dies.

## Steps

- [x] **Componentize first (pure port, no behavior change).** Move
      `SpaceshipPlayerTargetLock` / `SpaceshipPlayerLockFocus` /
      `SpaceshipPlayerComponentLock` / `SpaceshipPlayerTargetCandidates`
      (input/targeting.rs) to ship-root components (`TravelLock`, `CombatLock`,
      `LockFocus`, `ComponentLock`, `ThreatContacts`), with the CURRENT single
      lock mapping to `CombatLock` so every consumer port is mechanical:
      turret feed + torpedo commit + GOTO (input/player.rs:381-490, :801-849),
      HUD (hud/torpedo_target.rs:246/:267/:339/:405, hud/component_lock.rs
      :104-152, hud/target_inset.rs:287-288/:370, hud/target_candidates.rs
      :108-109, hud/edge_indicators.rs:262-263), examples/12_hud_range.rs
      resource reads (:338/:376/:581/:624). Auto-acquisition still runs at this
      step; autopilots must stay green. Respawn hygiene: components die with
      the ship root (this fixes the stale-resource-across-respawn wart - do
      not reintroduce a resource).
- [x] **Radar input actions** on the flight rig (input/player.rs, replacing
      the CTRL modifier action at :616-626): `RadarHoldInput` with
      `Hold::new(RADAR_TAP_SECS)` (`one_shot: false`) and `RadarClearInput`
      with `Tap::new(RADAR_TAP_SECS)`, BOTH bound to ControlLeft/ControlRight,
      ONE shared const (~0.25 s). Event mapping per the verified
      bevy_enhanced_input 0.26.0 table: radar-active state = the Hold action's
      TriggerState (Start -> Ongoing, threshold -> Fired), COMMIT observer on
      `Complete<RadarHoldInput>` only (sub-threshold release emits Cancel and
      must not commit), CLEAR observer on `Fire<RadarClearInput>` only (ignore
      Tap's Complete and its t=threshold Cancel). Add a boundary-frame test at
      exactly RADAR_TAP_SECS.
- [x] **Slot latch + provisional candidate** (D2): on `Start<RadarHoldInput>`
      (pause-guarded), latch the destination slot from the RAISED flag
      (20260713-082324) into a `RadarState` ship-root component
      { latched_slot, candidate: Option<Entity> }; the commit observer writes
      the LATCHED slot regardless of RMB churn mid-hold. Pause rule: a release
      while paused DROPS the commit and clears RadarState (deliberate
      gestures do not survive a pause; document it).
- [x] **Radar picker system**: run while the Hold action's TriggerState is
      Ongoing/Fired (the `cycle_modifier_held` TriggerState-read pattern,
      targeting.rs:791-795 - never stacked input conditions), reading the
      active-look-ray accessor; reuse the existing candidate collection +
      LockSignature range gates (targeting.rs:415-433) + `pick_target` angle
      pick, ADD incumbent hysteresis on the provisional candidate (the
      cos-ratio band pattern, D7). Writes `RadarState.candidate` (a SEPARATE
      field - never the live slot - so focus dwell neither accrues nor resets
      during a hold). Note: the commit observer runs in PreUpdate and reads
      last frame's candidate - by design, do not recompute in the observer.
- [x] **Commit semantics**: candidate Some -> write the latched slot;
      SAME-entity re-commit is a no-op (lock, focus dwell, component fine-lock
      all survive); candidate None -> NO-OP, old lock survives (D1 - sweep-off
      release is the radar abort).
- [x] **Staged tap-clear** (D3a): lowered - first tap clears the CombatLock if
      one exists, else clears the TravelLock (and disengages an engaged
      `AutopilotAction::Goto` via `remove::<Autopilot>()` - writer-agnostic,
      player.rs:820-828 / camera handback on Remove<Autopilot>); raised - tap
      clears the CombatLock only. Emit a HUD toast event naming what cleared.
      Pause-guarded like every intent observer.
- [x] **Delete the auto machinery**: the every-frame cone auto-pick + the
      signature auto-acquire fallback (`pick_signature_target` call site) +
      the sticky `held` gate + `pinned_until` + `step_target_lock` +
      `TargetCycleNext/PrevInput` actions and DPadUp binding (player.rs
      :666-688) + `TargetCycleModifierInput` + `cycle_modifier_held` routing
      (wheel becomes always-component-cycle) + `FlightVerbHints.target_cycle`
      (player.rs:100, :267-274) and its keybind_hints row + pinned tests -
      a compile-checked cascade. Keep the pure helpers (`pick_target`, range
      gates, ranking) as the radar picker's rules.
- [x] **Natural clears** system: slots clear on target death/despawn and
      out-of-range (existing gates), on allegiance flip to non-hostile
      (combat slot; the round-3 m5 carry-over), and the COMBAT slot decays
      after `COMBAT_DECAY_SECS = 30.0` without combat activity (activity =
      raised or firing; D4) - safety then follows in 20260713-082337.
- [x] **Threat set survives, candidate list retires** (D9): keep the ranked
      tracker (`rank_combat_targets` + maintenance) always-on as
      `ThreatContacts` feeding hud/edge_indicators.rs; retire the
      hud/target_candidates.rs module + its hud/mod.rs wiring and observers.
- [x] **Lock capability**: add `lock` to `ControllerVerbs`
      (controller_section.rs:97-152; update its doc comment to
      "computer-provided capabilities" - chosen over a sibling component to
      reuse SetControllerVerb + hints plumbing; compile-checked struct-literal
      breakage at player.rs:1461/:1509 and nova_scenario/actions.rs:462-511
      tests). Radar observers no-op (deny cue later) without it; grant it on
      the catalog/default ships (nova_assets/src/sections.rs:88-91) and the
      shakedown player ship. AI-side check is 20260713-082337's scope.
- [x] **Range knob retune**: `unsigned_lock_range` 15 -> ~5 (debris at ~5 m),
      sanity-check the asteroid example (~200 m for small rocks) against
      `signature_range_per_unit` - constants only.
- [x] **HUD crosshairs**: rework the single lock reticle
      (hud/torpedo_target.rs) into the red slightly-smaller COMBAT crosshair
      on `CombatLock`; add the white TRAVEL crosshair on `TravelLock`
      (screen-indicator widget, overlappable); add the provisional HOLLOW
      crosshair in the latched slot's color with a target-name label while
      radar is active (D2/D7); wire the cleared-locks toast.
- [x] **Tests** (delivery-guarded): hold->commit travel when lowered;
      RMB+hold->commit combat; slot latch survives RMB flip mid-hold; empty
      release no-op preserves old lock + engaged GOTO; sub-threshold release
      (Cancel) commits nothing; staged tap order (combat first, travel+GOTO
      second; combat-only while raised); same-entity re-commit preserves
      focus/component lock; 30 s decay clears combat lock (and delivery guard:
      activity resets the clock); hysteresis holds the provisional candidate
      against a marginally-nearer challenger; capability-less computer cannot
      lock. Rewrite/delete the ~20 old-contract targeting tests
      (auto-acquire :1058-1597, sticky/pin :1328-1473 + :1998-2213, CTRL e2e
      :2215-2404); rewrite the 12_hud_range script lock/dwell stages to
      radar-driven (scripted press/hold/release or direct component writes).
- [x] cargo fmt + cargo check --workspace; targeting/player/hud test modules;
      12_hud_range + 10_gameplay autopilots.

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md (design,
  adversarial round, decisions D1-D9).
- Depends on: 20260713-082324 (active look ray + raised flag).
- The free-aim raw CTRL read (player.rs:434) is re-keyed to the RAISED flag
  here as a mechanical condition swap (so CTRL is fully freed); full manual-
  gunnery semantics land in 20260713-082337.
- bevy_enhanced_input timers tick on REAL time (TimeKind::Real default) -
  thresholds advance during pause; the drop-commit-on-pause rule above is the
  chosen answer.
- Port-surface inventory with file:line lives in the spike's adversarial
  round; anchors verified 2026-07-13.
