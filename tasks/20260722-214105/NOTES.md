# Ledger ch3 depth - implementation notes (task 20260722-214105)

Deepened `webmods/the-ledger/ledger_ch3.content.ron` (The Quiet Channel), the
campaign's thinnest chapter, from one linear position-gated act into a
clock-paced opening + breather-gated corridor with TWO distinct textured
encounters. Data-only RON change + a new production-faithful test. No engine /
Rust-builder changes; only shipped scenario vocabulary (copied from the
ch1/Shakedown idiom already on master).

## What changed (from the final diff)

### OnStart (the sequencer + the cast)
- Removed the frame-0 Vesh line (moved into the cascade below) and the frame-0
  `obj_gates` NAV objective (now lazy-posted). OnStart now posts ONLY a holding
  recap (`obj_ch3_recap` reworded to "stand by for Vesh's nav drops") - the
  objectives panel stays empty through the opening conversation (beat-sheet ban
  on objective-shares-a-frame-with-conversation).
- Seeded the opening/pacing counters: `open_step`, `nav_posted`, `beat_gate`,
  and the per-beat one-shots `arrive1_said`, `arrive3_said`, `pinch_gate`,
  `pinch_warn_said`, `pinch_clear_said` (all 0 - an undefined gate fails closed
  forever).
- Added the debris pinch cast: two invulnerable boulders
  (`pinch_boulder_port`, `pinch_boulder_starboard`) straddling the NAV-1 ->
  NAV-2 leg, and a silent trigger beacon `pinch_clear` (NARROWS) on the far
  side of the gap. The 26-rock `ScatterObjects` field is unchanged (still
  decoration), but the field now HAS load-bearing debris: the two pinch
  boulders are a real piloting gate.

### Opening conversation cascade (new)
- Five `OnUpdate` handlers gated `Equal(open_step, N)` + `GreaterThan(
  scenario_elapsed, T)` with T ascending 2 -> 11 -> 20 -> 30s (~9s apart):
  Vesh briefs ("run dark through the channel to my yard"), the player ("You")
  answers, Vesh gives the nav-drop instruction, player acks. The final step
  (`open_step == 4`, one-shot on `nav_posted == 0`) completes `obj_ch3_recap`
  and lazy-posts `obj_gates`. A blind burn cannot start threading before the
  run is called.
- `dwell: Some(...)` added on the longer lines (clamped [3,30]): 7, 9, 9, 7, 6s.

### Breather-gated corridor (announce -> arrive -> confirm -> breathe)
- NAV-1 OnEnter: sets `gate=2`, stamps `pinch_gate = scenario_elapsed + 4`, and
  posts the arrival line. A separate one-shot `OnUpdate` (guarded
  `pinch_warn_said == 0`, `pinch_gate > 0`, clock past `pinch_gate`) fires the
  pinch WARNING a beat later.
- Pinch CONFIRM: an OnEnter on `pinch_clear` (guarded `pinch_clear_said == 0`
  AND `pinch_warn_said == 1`, so it cannot pre-empt the warning) fires the "clean
  through" line when the player clears the far side of the gap.
- NAV-2 OnEnter (the ambush, unchanged geometry + `engage_delay: 8.0`
  telegraph): sets `gate=3`, stamps `beat_gate = elapsed + 5`, posts the
  contacts warning + spawns the two Magpies. A separate one-shot post-ambush
  BREATHER (`arrive1_said`, gate==3, clock past beat_gate) lands the "don't
  chase them, the box is the job" reassurance a beat later.
- NAV-3 OnEnter: sets `gate=4`, stamps `beat_gate = elapsed + 4`, posts the
  drop-three line. A final-leg BREATHER one-shot (`arrive3_said`, gate==4 AND
  act==1, clock past beat_gate) lands the yard-welcome line. The `act==1` guard
  stops it firing after a Victory (which sets act=2).
- YARD OnEnter and the player-death Defeat handler are UNCHANGED: YARD -> act=2
  -> Victory -> NextScenario `ledger_ch4_the_buyer` (linger); player death
  (act<2) -> Defeat -> retry `ledger_ch3_quiet_channel` (linger).

## New variables
`open_step` (0..4 opening cascade), `nav_posted` (0/1 hand-off one-shot),
`beat_gate` (re-stamped deadline for the NAV-2 and NAV-3 breathers),
`arrive1_said` (post-ambush breather one-shot), `arrive3_said` (final-leg
breather one-shot), `pinch_gate` (pinch-warning deadline stamped on NAV-1),
`pinch_warn_said` (pinch warning one-shot), `pinch_clear_said` (pinch confirm
one-shot). Pre-existing `act`, `gate` unchanged.

## The pinch geometry (the second encounter - a piloting hazard)
- Boulders (nominal radius 3.5, invulnerable, health 1000): port at
  (2.5, 7.5, -153.3), starboard at (57.5, 7.5, -116.7).
- They straddle the NAV-1 (0,0,-90) -> NAV-2 (60,15,-180) leg: the gap centre
  (30, 7.5, -135) sits EXACTLY on the leg (0u off, progress t=0.5), so the pinch
  is on the lane the player already flies.
- Centre-to-centre 66.1u; worst-case bodies at the 6x geometric factor are
  (3.5+3.5)*6 = 42u, leaving a CLEAR gap of ~24.1u. The tug body is ~4-5u, so
  even the fattest possible boulders leave >= 24u of lane - far wider than the
  ship (test pins clear_gap >= ship 5u + 6u margin = 11u). A careful pilot
  threads it; no lock helps (invulnerable), preserving the fighting-is-optional
  contract - this beat is FLOWN, not fought.
- `pinch_clear` (NARROWS) trigger at (45, 11, -160), area_radius 22, sits 29.4u
  from the gap centre - outside the gap, so its confirm fires only AFTER the
  thread.

## Reachability trace (no soft-lock)
Every open_step threshold ASCENDS (2 < 11 < 20 < 30) and each cascade handler
sets `open_step := N+1`, so the chain is strictly monotone and every step is
reachable on the clock. Every one-shot self-disqualifies:
- Opening cascade: each handler's `Equal(open_step, N)` guard is falsified the
  instant it sets `open_step = N+1`; the hand-off is guarded `nav_posted == 0`
  and sets it to 1.
- Pinch warning: `pinch_warn_said == 0` guard, sets 1. Fires only after
  `pinch_gate` is stamped ( > 0 guard) by NAV-1.
- Pinch confirm: `pinch_clear_said == 0` guard, sets 1; also gated
  `pinch_warn_said == 1` so ordering is warning-then-confirm.
- Post-ambush breather: `arrive1_said == 0` guard + gate==3 + beat_gate>0, sets 1.
- Final-leg breather: `arrive3_said == 0` guard + gate==4 + act==1 + beat_gate>0,
  sets 1; act==1 prevents a post-victory fire.
- Gate machine: each OnEnter is guarded `Equal(gate, N)` and sets `gate=N+1`, so
  the corridor is strictly sequential and terminates at YARD (gate==4 ->
  Victory). The player-death Defeat is gated act<2 (the deliberate global
  exception) and cannot overwrite an earned win.
No clock gate is unreachable (thresholds are seconds-scale and the clock always
advances); no objective is orphaned (`obj_ch3_recap` completes at the hand-off,
`obj_gates` posts there and completes at YARD); the pinch has no soft-lock (the
gap is threadable and the confirm is a comms line, not a required trigger - even
if the player somehow misses `pinch_clear`, the corridor still advances on the
NAV beacons).

## Deferred owner question (Finish playtest)
Pace-map ch3 question #2 offered two flavors for the second encounter: a
staggered second COMBAT contact at NAV-3 (a heavier, telegraphed loadout) vs a
debris-PINCH hazard (invulnerable rocks tightening the lane, no new enemies).
**I chose the debris-pinch** - it makes the existing decorative debris field
load-bearing (a task requirement), keeps the "fighting is optional" contract
cleanly (a piloting challenge, distinct texture from the NAV-2 gunfight), and
does not raise the combat wall. The staggered-combat-contact-at-NAV-3
alternative is DEFERRED for the owner's replay verdict: if the channel wants
more teeth rather than more piloting texture, NAV-3 could gain a second
telegraphed contact (different loadout + `engage_delay`) as a follow-up.

## Verification
- `content lint --target the-ledger`: 0 errors, 1 warning + 1 finding, 2 acked.
  The single WARN cites `ledger_ch4.content.ron` (auditor multi-spawn) - NOT
  ch3, exactly as the task predicted. ch3's beat-sheet arms pass clean (no
  >1 StoryMessage/handler, no StoryMessage+Outcome co-fire, no OnStart objective
  during conversation).
- `cargo test -p nova_assets --test ledger_ch3_channel`: 9 passed.
- `cargo test -p nova_assets --test ledger_ch2_encounter`: unchanged, green (no
  regression - ch3 edits are isolated to ch3).
