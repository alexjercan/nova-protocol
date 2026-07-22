# Ledger ch3 depth: clock-paced opening act, breather-gated corridor, a second distinct encounter

- STATUS: CLOSED
- PRIORITY: 54
- TAGS: v0.8.0, content, scenario

## Story

Deepen chapter three (The Quiet Channel) - the thinnest chapter: today one
linear position-gated act (gate 1..4, a 4-beacon corridor) with a single
optional ambush at NAV-2 and zero clock pacing. Add real acts, breathers, and a
second distinct encounter, using only shipped scenario vocabulary. No new
chapter, no new engine features (owner clarification 2026-07-22: deepen existing
chapters, especially ch3).

Umbrella: 20260722-212808. Implements the ch3 findings from the diagnostic
pace-map (dep). File: `webmods/the-ledger/ledger_ch3.content.ron`.

## Steps

- [x] Add a clock-paced opening act: a beat_gate/scenario_elapsed conversation
      cascade (Vesh briefs, Kestrel answers, "go dark") before NAV-1 arms, with
      the first objective lazy-posting on hand-off (Shakedown opening idiom).
- [x] Split the corridor into acts with breathers: stamp beat_gate at each gate
      transition, land the next Vesh line a fixed beat later (announce ->
      arrive -> confirm -> breathe), not instantly on OnEnter.
- [x] Add a SECOND distinct encounter beyond the NAV-2 ambush, shipped
      vocabulary only: e.g. a staggered contact at NAV-3 with a different
      loadout + engage_delay telegraph, or a debris-pinch hazard (invulnerable
      rocks tightening the lane, gauntlet-style). Keep the "fighting is
      optional" contract; give the channel two textured beats, not one.
- [x] Make the scattered debris load-bearing for at least one beat (thread it),
      so the field is content, not pure decoration.
- [x] Clock-pump test path for the new deferred objectives; `content lint
      --target the-ledger` clean; probe the chapter against the real loader
      (probe-content-not-just-code).

## Definition of Done

- ch3 has a clock-paced opening act + breather-gated corridor + a second
  distinct encounter, materially deeper than the current single act. (manual:
  owner replay at Finish confirms it no longer feels thin; cmd: git diff shows
  substantial authored growth in ledger_ch3.content.ron.)
- `content lint --target the-ledger` clean (acks with reasons). (cmd.)
- The chapter still chains cleanly into ch4 and its Defeat/retry holds.
  (test: a walk/probe reaches YARD victory -> ledger_ch4_the_buyer.)

## Notes

Optional-fight contract stays: a careful pilot can still thread the channel.
Encounter variety is about texture and telegraph, not raising the wall.
