# Ledger beat-sheet pacing pass: ch1/ch2/ch2b - implementation notes

Data-only (RON) pacing rework of the three hand-authored Ledger chapters,
replicating the generated Shakedown opening idiom by hand: OnStart seeds
sequencer counters + a HOLDING objective + set-dressing spawns only; a
clock-paced open_step cascade posts the opening conversation one line per
handler; the first real objective lazy-posts on the hand-off; breathers
(beat_gate stamps + gated one-shot OnUpdate handlers) sit between later
beats; and per-line `dwell` was introduced (it was unused across the whole
mod). No engine/Rust-builder change, no new engine feature. Spawn geometry in
ch2/ch2b is untouched.

All counts below are read from the FINAL diff, not intent.

## ch1 - Dead Weight (ledger_ch1.content.ron)

Before: OnStart dumped 1 Okono line + `obj_ch1_recap` + `obj_crate_1/2/3` +
markers + all 8 spawns in one handler; three pickups each did
count+complete+INSTANT Okono line; one OnUpdate flipped `act` and dumped the
4th-ping line + objective + marker together.

After (handlers in the file: 17 total):

- OnStart: keeps `act`/`crates` + all spawns (player, two wreck masses, three
  crates, blackbox, magpie, debris). Now ALSO seeds the sequencer: `open_step`,
  `quota_posted`, `beat_gate`, `setup_last`, `ack1_gate`/`ack2_gate`/`ack3_gate`,
  `ack1_said`/`ack2_said`/`ack3_said`, `ping_gate`, `ping_said` (all 0). The
  three crate objectives + markers were REMOVED from OnStart. The intro line
  was removed; `obj_ch1_recap` was retasked to a "stand by for the work order"
  HOLDING objective.
- Opening conversation: 4 new OnUpdate cascade handlers gated
  `open_step == N && scenario_elapsed > T` for T = 2, 11, 20, 29, one
  StoryMessage each (Okono / You / Okono / You), advancing open_step 0->4.
  Reuses the recap text as the Okono quota beat.
- Hand-off: 1 new OnUpdate gated `open_step == 4 && quota_posted == 0`
  completes the holding objective and lazy-posts `obj_crate_1/2/3` + their
  markers. A blind burn cannot start the quota before it is called.
- Pickups: the three OnEnter handlers keep despawn+count+complete but their
  INSTANT Okono line was removed; each now stamps its OWN gate
  (`ackN_gate = scenario_elapsed + 2.5`). Three NEW delayed-ack OnUpdate
  handlers (gated `ackN_gate > 0 && ackN_said == 0 && scenario_elapsed >
  ackN_gate`) carry the one-line acknowledgment a beat later. Per-crate gates
  keep the pickups order-free (a single shared gate would collide / mis-order).
- 4th-ping reveal: the old single handler was split into three beats:
  (1) `crates > 2 && act == 1` sets `act = 2` and stamps `ping_gate = elapsed
  + 4` (ARM only, no line, no objective); (2) `act == 2 && ping_said == 0 &&
  elapsed > ping_gate` fires the reveal StoryMessage (`dwell: 8.0`) and
  re-stamps `beat_gate = elapsed + 3.5`; (3) `ping_said == 1 && setup_last < 1
  && elapsed > beat_gate` posts `obj_blackbox` + its marker. announce ->
  breathe -> objective.
- The blackbox->victory OnEnter and the player-death Defeat handler are
  UNCHANGED.

New variables (ch1): open_step, quota_posted, beat_gate, setup_last,
ack1_gate, ack2_gate, ack3_gate, ack1_said, ack2_said, ack3_said, ping_gate,
ping_said. dwell used on 3 lines.

## ch2 - Claim Jumpers (ledger_ch2.content.ron)

Additive only; NO spawn position/rotation/count/loadout/engage_delay/geometry
change (the fairness rig owns those).

After (11 handlers total):

- OnStart: keeps `act`/`kills`/`teach_sent` + all spawns (engage_delay: 8.0
  telegraph untouched). Now ALSO seeds `open_step`, `obj_posted`, `win_gate`,
  `win_said` (all 0). The three objectives (`obj_ch2_recap`, `obj_escort`,
  `obj_wave`) were REMOVED from OnStart; `obj_ch2_recap` is retasked to a
  "stand by - contacts inbound" HOLDING objective.
- Opener (short, per the pace-map - a returning player has met the cast): 2
  new OnUpdate cascade handlers (T = 2, 11) - Okono's contact call (reused
  text) then a "You" reply - advancing open_step 0->2.
- Hand-off: 1 new OnUpdate gated `open_step == 2 && obj_posted == 0` completes
  the holding line and lazy-posts `obj_escort` + `obj_wave`.
- Teach beat: its gate moved from `scenario_elapsed > 8` to `> 18` so it lands
  after the opener hands off (one line per beat, no stacking). Text unchanged.
- Kill handlers UNCHANGED.
- Win/handoff breather (the pace-map's ask): the `kills > 1` handler keeps
  act=2 + the two ObjectiveCompletes, DROPS the inline Outcome/NextScenario,
  and instead fires a win COMMS line (`dwell: 6.0`) + stamps `win_gate =
  elapsed + 3`. A NEW delayed OnUpdate (gated `act == 2 && win_said == 0 &&
  elapsed > win_gate`) carries the Victory Outcome + lingering NextScenario a
  beat later. This keeps the StoryMessage OFF the Outcome frame (lint arm) and
  gives the checkpoint a breather.
- Defeat handlers UNCHANGED.

New variables (ch2): open_step, obj_posted, win_gate, win_said. dwell on 2
lines.

## ch2b - The Heavies (ledger_ch2b.content.ron)

Near-mirror of ch2, same additive discipline, same geometry untouched.

After (11 handlers total): identical shape to ch2 - OnStart seeds the same 4
new vars + retasks `obj_ch2b_recap` to a holding line and drops the 3 OnStart
objectives; a 2-line opener (T = 2, 11) advancing open_step 0->2; a hand-off
posting `obj_escort` + `obj_wave`; the teach gate moved 8 -> 18; the
lane-clear breather splits the win into a comms line (`dwell: 6.0`) + stamp
then a delayed Outcome + NextScenario(ledger_ch3) handler.

New variables (ch2b): open_step, obj_posted, win_gate, win_said. dwell on 2
lines.

## Test change (crates/nova_assets/tests/ledger_ch2_encounter.rs) - DELIBERATE

The two Victory walk tests (`wave_one_kills_checkpoint_into_the_heavies`,
`heavies_kills_clear_the_lane_to_chapter_three`) legitimately shifted because
the Victory overlay is now DEFERRED a beat behind the win comms line (a
structural pacing change, not a geometry change). The rig drives events
directly and never advances the scenario clock, so a deferred handler would
silently never fire (the time-gated-content-needs-a-clock-pump lesson, task
20260721-211506). Updates, all additive to the rig:

- New `pump_clock(app, secs)` helper: seeds `scenario_elapsed` and ticks.
- `armed_app` now also seeds `win_said = 0` the way OnStart does
  (rig-supplies-precondition: the deferred handler's `win_said == 0` filter
  reads undefined otherwise and fails closed).
- Each Victory walk seeds `scenario_elapsed = 30` before the killing blow (so
  `win_gate = elapsed + 3` stamps a real deadline), asserts the overlay is
  NOT up on the kill frame (the comms line plays first), then `pump_clock(100)`
  and asserts Victory + the queued NextScenario as before.

NO geometry assertion moved. All the range/bearing/cover/loadout/mule-axis
pins, the retry-split behavior walks, and `on_start_seeds_the_act_machine`
(still checks OnStart seeds act+kills and spawns player/mule/wave) are
untouched and green. `deaths_after_the_win_declare_nothing` is unchanged and
still passes (it seeds no win_said/win_gate, so the deferred win handler stays
inert - the post-win death still declares nothing).

## Reachability / clock-pump trace

- ch2/ch2b: covered by the two updated walk tests (clock pumped past win_gate;
  both reach Victory + the correct NextScenario). GREEN.
- ch1 (no rig): manual trace. open_step cascade thresholds ascend strictly
  (2 < 11 < 20 < 29); each handler is gated `open_step == N` and sets N+1, so
  exactly one fires per step; the hand-off `open_step == 4 && quota_posted ==
  0` is reachable once step 4 lands and one-shots via quota_posted. Each
  delayed pickup ack gates `ackN_gate > 0` (stamped only by that crate's
  pickup) + `ackN_said == 0` (one-shot) + `elapsed > ackN_gate`, so it fires
  exactly once, a beat after its own pickup, order-free. The reveal chain:
  crates>2 arms ping_gate; announce fires once (ping_said) past ping_gate and
  re-stamps beat_gate; the objective posts once (setup_last<1) past beat_gate.
  All gates are seeded 0 in OnStart, so no undefined-read fails a beat closed.

## Verification results

- `content lint --target the-ledger`: 0 error(s), 1 warning(s) [pre-existing
  ch4 auditor multi-spawn WARN, both close-spawn findings ACKed by
  20260717-143806 - NOT in my files], 5 scenarios balance-audited, 2 acked.
  ch1/ch2/ch2b are clean (no >1-StoryMessage/handler, no StoryMessage+Outcome,
  no out-of-range dwell).
- `cargo test -p nova_assets --test ledger_ch2_encounter`: 12 passed, 0 failed.

## Decisions a reviewer should scrutinize

1. The deferred-Victory test update: is pumping `scenario_elapsed` in the rig
   the right call vs. leaving the overlay instant? I chose the deferred overlay
   because the pace-map explicitly asks for "a breather between the kills==2
   victory and the next-scenario handoff" and a StoryMessage cannot sit beside
   an Outcome (lint arm), so the only way to voice the win is a line THEN a
   delayed overlay. The test change is the honest consequence.
2. Opener length: ch2/ch2b openers are 2 lines (~11s to hand-off) vs ch1's 4
   lines (~29s), per the pace-map note that a returning player has met the cast
   (owner playtest question 3 is still open on exact dwell).
3. Bundle version left at 1.5.0 (test only requires > 1.0.0; sibling tasks own
   version bumps per the shared-checkout convention).
