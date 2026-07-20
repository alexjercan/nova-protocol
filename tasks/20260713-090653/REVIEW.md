# Review - 20260713-090653 shakedown radar-era rework

## Round 1 (2026-07-13)

Walked the player experience through all beats under the new gating, then
re-read the diff and the pinned tests cold.

- **Beats 1-3 remain completable** with LOCK withheld: W/X burn, Alt
  look, fly-into-crate salvage - none needs the radar. Side effect traced
  and judged DELIBERATE: GOTO (granted at beat 2) is unusable until beat
  4 because it needs a nav lock, and the contextual cluster therefore
  hides [G] in beats 2-3 - manual flight IS those beats' lesson, and the
  computer-offline framing covers it diegetically.
- **No beat-4 softlock**: the beat completes via OnOrbit ([O] near the
  planetoid), which needs neither the lock nor GOTO - the radar is
  taught, not force-gated. A player who ignores CTRL can still finish.
- **Action ordering**: the Lock grant precedes the objective + emphasis
  in the handler's list, so the RADAR row is available the frame its
  pulse starts; the walk pins the grant against the REAL controller
  section with a boot-time withheld delivery guard - which also proves
  SetControllerVerb executes through the rig's real pipeline (the GOTO
  grant never had this pin).
- **Restart hygiene**: the withheld state is config-authored (clone-and-
  override), so a death restart re-withholds it with the fresh ship; no
  action-ordering window (same reasoning as the GOTO withhold).
- **Emphasis**: "RADAR" is a ROW_VERBS member (130305), set/cleared as a
  pair on the same handlers as GOTO - the pairing test now pins both.
- **Text**: no test pins the objective strings (verified in 082344); the
  walk asserts ids/presence and stays green.
- **Honesty**: the outcome records the gesture-rows deviation (superseded
  by cluster + emphasis + text), the compositional scavenger-fire close,
  and the deferred autopilot with its reason.

Playtest notes (knobs, not blockers): a pre-beat-4 CTRL press buzzes with
no textual explanation (diegetic, but watch for confusion); beat-4 text is
three sentences - trim if it wraps badly.

- VERDICT: APPROVE (round 1).
