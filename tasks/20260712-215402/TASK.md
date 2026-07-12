# Unified cone target list + universal sticky lock (absorbs: cyclable nav bodies)

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, targeting, navigation, spike

## Goal

One available-targets list, one lock, per the user's model (steer 2026-07-12,
spike 20260712-215733): the list holds EVERY lockable body inside a cone
around the aim ray - combat (hostile ships, committed torpedoes) AND nav
(signed asteroids, beacons, wells) - ranked angle-to-aim then distance. With
no lock, the computer auto-picks the best entry: hostiles first from anywhere
in the list, else the best nav entry inside the tight pick cone (the spike's
auto-pick policy; the 550 m hostile signature fallback stays for an empty
list). A held lock of ANY class never changes on
its own: only CTRL+scroll (next entry), death, or the range gate moves it.
Unsigned debris stays out by construction (no LockSignature at range).

Edge indicators must decouple from the cycle list FIRST (their own
all-directions hostile-combat threat set + the lock), so behind-you torpedo
warnings do not regress when the list becomes cone-gated.

Builds on the componentized TargetLock/AvailableTargets from task
20260712-215957 (do that one first).

## Notes

- Spikes: docs/spikes/20260712-215733-unified-target-computer.md (the model,
  ranking, cone width knob, open questions);
  docs/spikes/20260712-215256-combat-travel-lock-separation.md (Part A -
  original motivation: flick to a far body you cannot pixel-aim).
- 2026-07-12 steer: this task originally implemented option A1 from the
  combat/travel spike - nav bodies as NON-sticky, combat-first cycle entries,
  keeping aim re-designation. The user's newer steer supersedes A1: keep it
  simple, one lock for both classes, sticky for both, cone membership is the
  only gate. Aim re-designation of nav targets is deliberately removed;
  re-designating = cycling (nav bodies are in the list now). The combat/travel
  mode toggle stays a future direction recorded in that spike doc (not a
  seeded task; see its Next steps).
- Clutter guard in dense fields (04_asteroids) is now a plan-time/playtest
  question: cone half-angle (start ~45-60 deg), the 5-slot cap, signature
  threshold and/or combat-reserved slots - see the spike's open questions.
- Relevant code: `update_spaceship_target_input`, `rank_combat_targets`,
  `maintain_candidates`, the `held` sticky gate, `TARGET_CANDIDATE_COUNT`,
  `pinned_until` (shrinks to cycle-order freezing only), LockSignature range
  model (unchanged).
- Playtest: behind-you threats vs the strict cone; does cycling-to-redesignate
  feel OK for GOTO; explicit unlock input wanted?
- Plan-time option (spike, "Combat vs travel separation"): gate auto-pick on
  the Turret camera view (RMB held) so cruising in Normal view stays quiet -
  the lock and list persist, only re-picking pauses. Playtest knob: gated vs
  always-on.
