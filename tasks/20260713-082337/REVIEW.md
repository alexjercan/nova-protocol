# Review: Weapons safety + RMB manual gunnery + consumer routing

- TASK: 20260713-082337
- BRANCH: feature/weapons-safety

## Round 1

- VERDICT: APPROVE

Scope: WeaponsHot derivation + three-layer enforcement (live section gates in
both fire loops, press-time deny, hot->cold trigger-interrupt), AI combat
mirror, weapons status HUD block with the D5a torpedo readout, GOTO capture
pin, HOLD_FIRE_DURING_RADAR flag, 082330 R1.3 fix.

Independent verification (shared-session blind-spot guard):
- Re-derived the latched-trigger enforcement stack against the verified
  finding (the input bool latches on Start): the LIVE gate alone suffices for
  correctness (checked in the fire loop every tick); the interrupt adds the
  deliberate re-press rule; the press deny adds feedback. Checked the torpedo
  loop's gate sits AFTER the input check and BEFORE ammo/spawn - no shot can
  leak between safety flip and zero.
- Challenged the "AI never silenced" claim and made the task record honest:
  the smoke autopilots contain no AI firefight, so the proof is compositional
  (mirror test pins engaged => hot; the gate's only deny branch is
  managed-cold; managed-hot === unmanaged). The live exercise is the
  shakedown scavenger fight - explicitly flagged onto the 090653 playtest.
- Verified the unmanaged-ship default (no WeaponsHot => fire freely) against
  the existing test base: all 20 turret_section tests and the AI fire-cadence
  tests pass UNCHANGED because their rigs are unmanaged - exactly the
  backward-compatible default chosen.
- D8: `goto_keeps_the_captured_target_across_re_designation` pins the
  capture; grepped flight.rs for TravelLock - zero reads, the autopilot only
  ever sees its captured action. The destination MARKER (anchored on the
  captured Autopilot target) is the honest destination display; skipping the
  name-on-chip is a reasonable scope cut against a pinned pure contract.

Findings:

- [ ] R1.1 (MINOR) [recorded] The audio blip on the safety OFF->ON edge was
  deferred for want of a sound asset; the status block covers the cue
  visually. Fold into the shakedown/polish pass (20260713-090653) where sound
  content is already in scope.
  - Response: recorded in TASK.md and the 090653 scope.

- [ ] R1.2 (NIT) The weapons status block computes name+distance strings per
  frame while hot; the != guard prevents Text churn but not the format!
  allocations. Negligible (one ship), not worth caching.
  - Response: acknowledged, no action.

Checks: 463 nova_gameplay tests; 12_hud_range live-asserts hot-while-locked
and safety-re-engages-after-the-lock-dies (grep-confirmed both fired); three
autopilots green; fmt + workspace --tests clean. CI runs the full suite +
clippy per repo policy.
