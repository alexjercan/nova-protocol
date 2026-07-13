# Retro: Contextual keybind hints (resolver, cluster, anchored cues)

- TASK: 20260710-174646
- BRANCH: keybind-hints (squash-merged to master as 7c0d241)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 1 MAJOR + 4 MINOR,
  round 2 APPROVE)

Third and last task of the day's HUD arc; same-day spike-to-shipped for
the second time on the same substrate.

## What went well

- **The compute-at-the-truth pattern transferred wholesale.** The
  resolver-in-the-input-layer / render-dumb split (named as a lesson in
  the instruments retro hours earlier) made every HUD system a pure view
  and the whole feature testable with run_system_once worlds - six tests,
  zero harness invention.
- **Plan-time API verification again paid for itself**: the spike's open
  question (can hint labels derive from live bevy_enhanced_input
  bindings?) was answered with a 10-minute registry-source read before
  planning, and the introspection worked first try, including in tests
  with real `bindings!` spawns.
- **The absorption was clean**: the hand-placed [O] ORBIT cue (shipped
  two tasks ago as a deliberate down payment) was deleted and reborn
  resolver-driven with no leftovers - the "hints become one table row"
  bet from the spike held.

## What went wrong

- **R1.1 (MAJOR): availability ignored ship capability.** The resolver
  gated on what the input observers check (lock, well, engagement) but
  not on what the autopilot itself requires (live controller + engines),
  so a crippled ship showed lit hints whose presses were visible no-ops -
  contradicting the field's own doc ("pressing the key right now would do
  something"). Root cause: the availability rules were copied from the
  engage-side observers, but the promise spans the whole verb lifecycle,
  and the disengage-side conditions live in a different module. When a
  field's doc makes a promise, enumerate every system that can break it,
  not just the one next to the cursor.
- Small guard asymmetries (cues unguarded while the cluster was, is_empty
  vs single) - the cost of writing three sibling systems in one pass and
  only polishing the first.

## What to improve next time

- A component/resource doc that promises behavior ("X would do
  something") is a spec: grep for every consumer/producer of the
  underlying state before calling the implementation done - same failure
  shape as the instruments' flip-vs-arrival-rule MAJOR, one task apart.
  If the same lesson appears a third time, it belongs in AGENTS.md.

## Action items

- [ ] Playtest: cluster position (above the status line) vs the velocity
  sphere; DIM_COLOR legibility; whether the GOTO cue yielding during
  maneuvers feels right.
- [ ] Remaining arc task: 20260710-174629 (holo expansion - trajectory
  ribbon, SOI shell, flip gate), parked at priority 40.
