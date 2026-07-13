# Weapons safety + RMB manual gunnery + consumer routing (travel->GOTO, combat->guns)

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.5.0, targeting, input, combat, spike

## Goal

The combat half of the deliberate-radar design (spike 20260713-082207 +
decisions): a live weapons safety, manual turret gunnery while raised, and the
final consumer split (GOTO reads travel, weapons read combat) - for player AND
AI ships.

## Steps

- [ ] **WeaponsSafety component** on ship roots, derived each frame:
      OFF <=> (RAISED flag from 20260713-082324) OR (`CombatLock` is Some);
      ON otherwise (`set_if_neq`). The 30 s combat-lock decay (in
      20260713-082330) flips it ON naturally.
- [ ] **Live fire gate** (the verified latch problem: `on_turret_input` sets
      `TurretSectionInput = true` on Start, player.rs:1053-1073, and the
      section fires every cooldown tick while true, turret_section.rs:948;
      torpedo same shape :1127-1161 / torpedo mod.rs:488): gate SECTION-side
      on the ship root's WeaponsSafety (one enforcement for player and AI),
      plus a press-time deny cue in the player observers, plus ZERO the
      section inputs on the safety OFF->ON edge so a resumed-OFF state needs a
      fresh trigger press (trigger-interrupt rule) - with a safety-engaged
      audio blip on that edge (a held burst must not just silently stop).
- [ ] **AI parity (scoped as components + thin mirror, not a unification)**:
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
- [ ] **Manual gunnery while raised**: rework the turret feed tiers
      (update_turret_target_input, player.rs:381-439): RAISED -> aim at the
      live look ray (manual wins over the lock - preserves the side-shot; the
      mechanical re-key landed in 20260713-082330); else `CombatLock` Some ->
      track the lock (existing three-tier feed); else keep the current ray
      fallback (rest pose is an optional nicety, not required).
- [ ] **Torpedo routing + readout** (D5a): torpedo commit reads `CombatLock`;
      the ammo readout (hud/ammo_readout.rs) shows the commit target -
      "TORP -> <name>" when a combat lock exists, "TORP: DUMB" otherwise.
      No-lock launch stays dumb-fire (once safety is off).
- [ ] **GOTO capture semantics** (D8): [G] captures the `TravelLock` target at
      press into the engaged `AutopilotAction::Goto` (already by-value; verify
      no live re-read exists) and the GOTO/autopilot HUD row shows the actual
      destination name - re-designating the travel lock mid-flight must NOT
      re-route until the next [G].
- [ ] **Safety + gesture HUD**: weapons-hot indicator with reason
      ("HOT: lock on <name>, <dist>" vs safe); context-sensitive CTRL hint row
      ("hold: radar / tap: clear" with the staged-clear scope) and a raise
      hint on RMB; optional `HOLD_FIRE_DURING_RADAR` const (default off) for
      the sweep-with-trigger-down case.
- [ ] **Tests** (delivery-guarded): safety truth table (raised only / lock
      only / both / neither); the live gate stops a HELD trigger mid-burst
      when the lock clears (and the delivery guard proves the same held
      trigger fires while safety is off); safety OFF->ON zeroes section inputs
      so re-OFF needs a re-press; manual-wins-while-raised (turret aims at
      look ray despite a lock; lock-track resumes on release); torpedo commit
      target + readout; GOTO does not re-route on travel re-designation; AI
      ship still fires under the section-side gate.
- [ ] Rewrite the remaining 12_hud_range script stages against the final
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
