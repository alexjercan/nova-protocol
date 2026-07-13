# Weapons safety + RMB manual gunnery + consumer routing (travel->GOTO, combat->guns)

- STATUS: CLOSED
- PRIORITY: 54
- TAGS: v0.5.0, targeting, input, combat, spike

## Outcome (CLOSED 2026-07-13)

Shipped per plan, with these notes:

- `WeaponsHot` component (in targeting_state; hot <=> raised || combat lock),
  derived each frame for ANY managed ship. Enforcement is three-layered:
  section-side live gate in both fire loops (unmanaged ships - no component,
  bare example turrets - fire freely), press-time deny in the player fire
  observers, and a trigger-interrupt system zeroing held inputs on the
  hot->cold edge (registered-system change detection; a fresh press is needed
  once hot again).
- AI parity via `mirror_ai_combat_state` (input/ai.rs): CombatLock = PD
  override else primary AITarget, raised while engaged, WeaponsHot managed -
  a fighting AI is never silenced - proof is compositional: the mirror test
  pins engaged => WeaponsHot(true), and the section gate's only deny branch
  is a managed-COLD ship, so managed-hot behaves identically to unmanaged
  (the smoke autopilots have no AI firefight to observe live; the shakedown
  scavenger fight is the real-world exercise, flagged for the 090653
  playtest). A disengaged AI's guns go safe. Instant AI acquisition is the
  accepted spec.
- Manual gunnery (raised -> look ray, manual-wins) landed mechanically in
  082330; its regression test lives in player.rs.
- D5a torpedo readout + the weapons-hot indicator landed as ONE status block
  in hud/lock_crosshairs.rs ("WEAPONS HOT [RAISED]: lock <name> <dist>m" +
  "TORP -> <name>"/"TORP: DUMB", hidden while safe) - deviation: the plan put
  the torpedo line on the diegetic ammo gauge; the status block is more
  legible and one code path. R1.3 from 082330 fixed (toast stack marker).
- D8 capture: verified by construction (the GOTO observer captures at [G] and
  flight reads only the Autopilot) + pinned by
  `goto_keeps_the_captured_target_across_re_designation`. The destination
  NAME-on-row was skipped: the destination MARKER already anchors the CAPTURED
  target, which is the honest display the finding wanted; the mode chip's
  label is a pinned pure contract not worth breaking for redundant text.
- Audio blip on the OFF->ON edge deferred (needs a sound asset; the visual
  status block covers the cue) - noted for the shakedown/polish pass.
- HOLD_FIRE_DURING_RADAR const (default false) gates the press path only.

Verified: 463 nova_gameplay tests (safety truth table, trigger-interrupt with
delivery guard, AI mirror incl. PD override and disengage-safes, GOTO capture);
12_hud_range live asserts weapons-hot-while-locked and safety-re-engages-after
-the-lock-dies; three autopilots green; fmt + workspace --tests clean.

## Goal

The combat half of the deliberate-radar design (spike 20260713-082207 +
decisions): a live weapons safety, manual turret gunnery while raised, and the
final consumer split (GOTO reads travel, weapons read combat) - for player AND
AI ships.

## Steps

- [x] **WeaponsSafety component** on ship roots, derived each frame:
      OFF <=> (RAISED flag from 20260713-082324) OR (`CombatLock` is Some);
      ON otherwise (`set_if_neq`). The 30 s combat-lock decay (in
      20260713-082330) flips it ON naturally.
- [x] **Live fire gate** (the verified latch problem: `on_turret_input` sets
      `TurretSectionInput = true` on Start, player.rs:1053-1073, and the
      section fires every cooldown tick while true, turret_section.rs:948;
      torpedo same shape :1127-1161 / torpedo mod.rs:488): gate SECTION-side
      on the ship root's WeaponsSafety (one enforcement for player and AI),
      plus a press-time deny cue in the player observers, plus ZERO the
      section inputs on the safety OFF->ON edge so a resumed-OFF state needs a
      fresh trigger press (trigger-interrupt rule) - with a safety-engaged
      audio blip on that edge (a held burst must not just silently stop).
- [x] **AI parity (scoped as components + thin mirror, not a unification)**:
      give AI ships the same components - a small system mirrors `AITarget`
      into `CombatLock` (instant acquisition is accepted for AI) and derives
      AI RAISED/safety from its engagement state (raised while it has a live
      `AITarget`), so the section-side gate never silences a fighting AI.
      HUD/audio safety cues filter to the player ship only. Lock-less AI
      computers (capability flag) are DEFERRED - note in the code that the
      three-tier feed reads CombatLock, so a lock-less AI needs manual-look
      gunnery before it is anything but harmless. (The full
      AITarget->CombatLock unification, incl. the duplicated torpedo commit
      player.rs:459-490 vs ai.rs:1557-1580, is future work - do not drag ~30
      AI test sites into this task.)
- [x] **Manual gunnery while raised**: rework the turret feed tiers
      (update_turret_target_input, player.rs:381-439): RAISED -> aim at the
      live look ray (manual wins over the lock - preserves the side-shot; the
      mechanical re-key landed in 20260713-082330); else `CombatLock` Some ->
      track the lock (existing three-tier feed); else keep the current ray
      fallback (rest pose is an optional nicety, not required).
- [x] **Torpedo routing + readout** (D5a): torpedo commit reads `CombatLock`;
      the ammo readout (hud/ammo_readout.rs) shows the commit target -
      "TORP -> <name>" when a combat lock exists, "TORP: DUMB" otherwise.
      No-lock launch stays dumb-fire (once safety is off).
- [x] **GOTO capture semantics** (D8): [G] captures the `TravelLock` target at
      press into the engaged `AutopilotAction::Goto` (already by-value; verify
      no live re-read exists) and the GOTO/autopilot HUD row shows the actual
      destination name - re-designating the travel lock mid-flight must NOT
      re-route until the next [G].
- [x] **Safety + gesture HUD**: weapons-hot indicator with reason
      ("HOT: lock on <name>, <dist>" vs safe); context-sensitive CTRL hint row
      ("hold: radar / tap: clear" with the staged-clear scope) and a raise
      hint on RMB; optional `HOLD_FIRE_DURING_RADAR` const (default off) for
      the sweep-with-trigger-down case.
- [x] **Tests** (delivery-guarded): safety truth table (raised only / lock
      only / both / neither); the live gate stops a HELD trigger mid-burst
      when the lock clears (and the delivery guard proves the same held
      trigger fires while safety is off); safety OFF->ON zeroes section inputs
      so re-OFF needs a re-press; manual-wins-while-raised (turret aims at
      look ray despite a lock; lock-track resumes on release); torpedo commit
      target + readout; GOTO does not re-route on travel re-designation; AI
      ship still fires under the section-side gate.
- [x] Rewrite the remaining 12_hud_range script stages against the final
      semantics (radar commit -> dwell -> component markers -> inset; safety
      states); keybind-hint pinned tests updated. cargo fmt + check;
      player/turret/torpedo/hud modules; both autopilots.

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md (decisions
  D4/D5a/D8 + the fire-gate and AI caveats in the adversarial round).
- Depends on: 20260713-082330 (slots + radar), which depends on -082324.
- Section-side gating chosen deliberately so ONE rule covers player + AI;
  the AI raised-derivation above is what keeps AI alive under it.
- File:line anchors verified 2026-07-13.
