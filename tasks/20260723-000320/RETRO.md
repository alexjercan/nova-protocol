# Retro: ch3 stealth rework (20260723-000320)

## What went well

- The playtest verdict was precise ("go dark says stealth, but they always
  fight") and the engine investigation had already resolved the mechanism, so
  this cycle was pure execution against a known design: Neutral patrol +
  SetAllegiance wake on proximity/paint + a clean-run payoff.
- Building the engine primitive FIRST (SetAllegiance, 20260723-000253) and
  verifying its consumers read allegiance live meant the content task could
  trust the flip - no "advertised is not wired" surprise. The rig asserts the
  live `Allegiance` component after driving the real wake handlers, so a
  regression to hostile-spawn or half-wake fails a test.
- The detection geometry was authored against MEASURED values (bubble centers
  derived from the shipped pinch gap center + perpendicular), and the pin is a
  computed margin, not a literal - the reviewer independently recomputed ~13u of
  clearance and agreed the safe lane is genuinely safe. This is the
  author-against-measured-values discipline that makes a "sneak past" claim real
  rather than hopeful.
- The pinch and the stealth reinforce each other: the debris gap IS the blind
  spot between the pickets, so the earlier ch3-depth beat now pays double.

## What went wrong / was tricky

- Spawn timing was a genuine fork the agent resolved correctly: spawning the
  Magpies at OnStart (not the old NAV-2 trigger) is required for stealth - the
  detection bubbles guard the NAV-1->NAV-2 leg, which the player reaches BEFORE
  the old NAV-2 spawn, so a late spawn would leave the guarded leg empty and the
  threat invisible until too late. Stealth needs the threat visible before the
  choice.
- Comms had to read correctly on BOTH paths: the NAV-2 "contacts" call became
  path-neutral and fight-only lines gate on `spotted==1`, or the sneak path
  would narrate a fight that never happens.

## Lessons / what to do differently

- A "sneak past" mechanic is only real if the safe corridor is pinned OUTSIDE
  the detection volume by COMPUTED geometry (worst-case body radius, bubble
  radius, leg centerline) - the same rigor as a "threadable gap". A hand-placed
  bubble that "looks avoidable" is a false-green waiting to happen.
- When a stealth/aggression state is driven by a new engine flip, the rig must
  assert the live COMPONENT after driving the real handler (production-faithful),
  not the presence of the action in RON - otherwise it greens while the flip
  silently no-ops.

## Follow-ups

- None blocking. Owner Finish checks: replay a sneak run (slip past, payoff
  line) AND a provoked run (bubble/paint wakes them, fight), then publish 1.7.0.
  The vertical-bypass edge (a big y-detour clears both pinch and pickets) is the
  same honor-system limit the shipped pinch already had - accepted, noted.
