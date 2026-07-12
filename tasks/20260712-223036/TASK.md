# Fire gating on combat stance + natural combat-lock clearing (no unlock key)

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.5.0, targeting, input, spike

## Goal

Make firing deliberate per spike 20260712-222610 rounds 2c + 3: the
trigger works only while the weapon is RAISED - fires when combat-locked,
acquires when not - and outside the raise it is an inert deny cue. No
manual unlock key ships; stale locks are harmless by construction and
clear naturally (death, range, allegiance flip, optional decay - the
lifecycle itself lands in 20260712-223035; this task wires the trigger
and the feedback).

Body rewritten after round-3 adversarial review; supersedes the earlier
X/SHIFT+X/MMB unlock plans (retired in round 2c; X = STOP,
player.rs:584).

## Steps

- [ ] Gate the turret trigger in its observers (player.rs:1053-1073;
      bindings stay scenario-authored): RAISED + CombatLock -> fire
      (three-tier feed unchanged); RAISED + no lock -> acquire the best
      enemy from the CONE/ON-SCREEN pool (spike round 3 delta 4; no-op
      deny cue when none) and do NOT fire; NOT RAISED -> deny cue,
      never fire, never acquire - regardless of lock state. Keyboard and
      pad identical.
- [ ] CTRL free-aim bypass: fires at the camera ray in ANY view,
      lock-independent (the only unlocked fire path). WIRING TRAP
      (feasibility m7): free-aim reads raw CTRL keys (player.rs:434) -
      do NOT reuse the cycle-modifier helper, which is SHIFT after
      20260712-223034.
- [ ] Held-trigger interrupt (UX M1): when the CombatLock dies while the
      trigger is held, the stream STOPS; auto-seed (223035) may refill
      the slot but firing resumes only on a fresh press.
- [ ] Torpedo launch (player.rs:1128): commit requires the CombatLock
      stable for ~0.5 s (const knob) - else dumb-fire + deny cue. Keep
      no-lock dumb-fire. Comment cites the spike: the turret/torpedo
      gating asymmetry is chosen, not accidental. (Round 4: guided
      shots at nav bodies WORK - the combat slot can hold any cone
      member via deliberate scroll.)
- [ ] Feedback + hints: deny cue on gated presses (reuse an existing
      HUD affordance; no new audio/VFX scope); trigger hint reflects
      state (FIRE / ACQUIRE / RAISE FIRST); no unlock hints anywhere.
- [ ] Tests (state-per-step): lowered + lock -> LMB denies, no bullets;
      raised + lock -> fires; raised + no lock -> first press acquires
      (no bullets), second fires; raised + no enemies -> deny, no
      acquisition; CTRL+LMB fires in any view without a lock;
      held-trigger stops on lock death and does not resume on auto-seed
      (flag on and off); torpedo within stability window commits, a
      just-switched lock dumb-fires with deny cue; pad trigger follows
      every gate identically.
- [ ] cargo fmt + cargo check + run targeting/input test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md (rounds
  2c, 3 deltas 4/7/8/11).
- Depends on: 20260712-223035 (slots + raised state + pools).
- MMB stays unbound (reserved if playtest finds a "safe the guns while
  raised" need).
- Playtest flags: firing-requires-raise feel (fleeing gunfights mean
  flying raised); torpedo stability window width.
