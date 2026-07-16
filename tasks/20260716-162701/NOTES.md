# Ghost ship at 0 HP - investigation record (task 20260716-162701)

## Verified mechanism (all in source, cited)

- `HealthZeroMarker` is inserted ONLY by bcs `on_damage`
  (bevy-common-systems src/health/mod.rs): it applies
  `min(amount, current)`, mutates the bubbled amount to what landed, marks
  at `<= 0`, and EARLY-RETURNS with `amount = 0.0` on already-marked or
  already-zero nodes (swallowing the bubble).
- The death chain is marker-driven: HealthZeroMarker ->
  IntegrityDisabledMarker (`on_health_depleted_insert_disabled`) ->
  IntegrityDestroyMarker via TWO hops (bcs integrity/plugin.rs): the
  leaf-gated `handle_destroy`/`handle_chain_destroy` for SECTIONS, and
  `handle_parent_destroy` (plugin.rs:258) for disabled ROOTS - roots carry
  no ConnectedTo, are never leaves, so the root hop is what actually kills
  the ship. CORRECTION (review R1.1): the first draft of this record
  claimed handle_parent_destroy "no longer exists" after grepping only the
  nova crates; it lives in the bcs dependency and is the very observer the
  backstop relies on. The original glue.rs comment was right.
- `aggregate_ship_health` (integrity/glue.rs) rewrites the root's Health to
  the section sum EVERY frame - a zero written this way carries no marker.

## The hole and the reproduction

Root death depended entirely on the LAST fatal bubble reaching the root
with a nonzero amount. Any path that removes the final living section
without a qualifying bubble leaves the recompute writing a marker-less 0:
a permanent, targetable, empty hull. The boundary rig
(`ghost_ship_tests`, integrity/glue.rs) walked five paths:

- killing_every_section_kills_the_ship: PASSED pre-fix
- simultaneous_fatal_hits_kill_the_ship: PASSED pre-fix
- double_hit_on_the_last_section_kills_the_ship: PASSED pre-fix
- many_small_hits_kill_the_ship (fractional 3.7-damage fire): PASSED pre-fix
- last_section_destroyed_without_damage_still_kills_the_ship: FAILED
  pre-fix (root alive after 10 frames, no marker) - the reproduced ghost.

Fail-first evidence: the rig was written before the fix; the red run is
recorded in TASK.md. The four passing cases stay as pins
(null-result-becomes-a-pin).

## The fix

Structural death backstop in `aggregate_ship_health`: a root that HAS had
living sections (previous-frame max > 0) whose section sum is now <= 0 and
which carries no HealthZeroMarker gets one; the ordinary
disable -> leaf -> destroy chain takes it from there. The damage-path
bubbles stay untouched; the backstop only catches what they miss. On<Add>
observer semantics + the already-zero guard make it fire once (no double
OnDestroyed). The mid-spawn guard (max > 0) keeps a root whose sections
have not landed yet from being executed at birth.

## Honesty note on the live sighting

The reported kill was turret fire - the damage path - whose four rig cases
were NOT broken. Two candidate explanations remain: (1) some live path
removed the last section outside the damage chain (the hole is real and now
closed regardless of which path fired that day); (2) the sighting was the
DISPLAY sibling: bcs health_display.rs rounds sub-1% health to "0%", so a
ship with a hidden sliver section reads dead while alive - filed as
20260716-165617 (v0.7.0, p50). With both landed, every known road to
"alive at 0 HP" is shut. The report stays formally unreproduced; if it
recurs after both fixes, reopen with the new evidence.
