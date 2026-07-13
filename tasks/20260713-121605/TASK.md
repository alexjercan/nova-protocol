# Turret routing: the combat lock wins over manual aim while raised

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,combat,input,playtest

## Outcome (CLOSED 2026-07-13)

Shipped: the raised special-case is GONE from update_turret_target_input -
the feed is the pure three-tier component -> lock -> ray again (with no
lock, the ray IS the raised manual aim, so lock-wins needed less code, not
more; the WeaponsRaised query param dropped out). The inverted pin
`the_combat_lock_holds_the_turrets_even_while_raised` asserts the lock
holds under a raised cursor move and that clearing it hands the turrets to
the ray. Spike 082207 carries the playtest verdict banner on its routing
paragraph + knob line; the CHANGELOG free-aim supersession note extended.
D5's torpedo/turret asymmetry is closed (both weapons follow the lock).
Verified: 471 nova_gameplay lib tests, fmt, 12_hud_range autopilot exit 0.

## Goal

Playtest (user, 2026-07-13): "while in combat mode (hold RMB) if you lock
on something and you move the cursor the weapons follow in free mode; they
should stay locked unless you tap CTRL and remove the lock". Flip the
turret routing default chosen in spike 20260713-082207 (manual-wins, an
explicit playtest knob): the COMBAT LOCK now wins over the raised look-aim;
manual gunnery applies only with NO combat lock (tap CTRL in combat mode
clears the lock and hands the turrets back to the cursor).

## Steps

- [x] `update_turret_target_input` (player.rs:364): remove the raised
      special-case entirely - with lock-wins, "raised + no lock" and the
      ray fallback aim identically, so the feed reduces to the pure
      three-tier `component -> lock -> ray`; drop the `WeaponsRaised`
      query param and rewrite the manual-gunnery comment.
- [x] Invert the pinned test `raised_manual_aim_wins_over_the_lock`
      (player.rs:1940) -> the lock wins while raised; clearing the lock
      (the tap-clear effect) hands the turrets to the ray. Fail-first
      against the old routing by construction.
- [x] Docs: spike 20260713-082207 fix record + the knob line (playtest
      verdict recorded); CHANGELOG free-aim supersession note extended
      (manual aim is now the NO-LOCK combat stance).
- [x] fmt + check; player module tests; 12_hud_range autopilot.

## Notes

- Spike: tasks/20260713-082207/SPIKE.md (the
  routing default was "confirm in playtest" - this IS the verdict).
- The side-shot use case the old default preserved is deliberately traded
  away: tap-clear is the explicit road back to manual.
- D5's torpedo/turret asymmetry closes: both weapons now follow the lock.
