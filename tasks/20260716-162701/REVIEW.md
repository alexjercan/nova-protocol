# Review: ghost ship at 0 HP - structural death backstop

- TASK: 20260716-162701
- BRANCH: fix/ghost-ship-at-zero-hp

## Round 1

- VERDICT: REQUEST_CHANGES
- Reviewer: out-of-context pass (fresh-context agent; ran both suites, fmt,
  workspace check, AND the sabotage A/B - backstop removed: exactly the
  recorded 1-of-5 case fails; verified no-false-positive reasoning, marker
  re-insert semantics from bevy_ecs source, and all lifecycle exclusions).

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/integrity/glue.rs:119 +
  NOTES.md + TASK.md close record - the "stale comment fix" is itself
  wrong and the record's mechanism claim is FALSE: `handle_parent_destroy`
  exists in the pinned bcs rev (integrity/plugin.rs:258, registered :53)
  and is precisely the observer that destroys a disabled ROOT - roots have
  no ConnectedTo, never get IntegrityLeafMarker, so the replacement chain
  written into the comment (`handle_destroy`) cannot fire for them. The
  implementer's grep covered nova crates only, not the bcs checkout. No
  code impact (the fix works BECAUSE handle_parent_destroy exists). Fix:
  restore the original comment name; correct NOTES.md and the close
  record.
  - Response: fixed - glue.rs comment restored to handle_parent_destroy with the root-specific truth (roots are never leaves); NOTES.md carries an explicit CORRECTION block naming the nova-only-grep root cause; the close record now states the two-hop chain.
- [x] R1.2 (MINOR) glue.rs ghost_ship_tests - narrower than the ticked plan
  step: asserts despawn only (the rig root lacks EntityId/EntityTypeName,
  so explode.rs:154 early-returns and OnDestroyed never fires), and the
  planned direct-to-root-damage interleave case was silently replaced by
  the blast co-hit. "Fires OnDestroyed exactly once" rests on reasoning,
  not a test. Fix: give the rig ship the id components, assert exactly one
  OnDestroyedEvent in the backstop case, and add the interleave case.
  - Response: fixed - the rig root carries EntityId + EntityTypeName; a FiredEvents counter (all GameEvents == the root's OnDestroyed in this rig, documented why) asserts exactly-once in the canonical, backstop and new interleave cases; the promised direct-root-damage interleave case is restored (with a delivery guard pinning the recompute overwrite).
- [x] R1.3 (MINOR) tasks/20260716-165617/TASK.md - the sibling HUD task
  misses the NaN half: the aggregate writes Health{0,0} on a section-less
  root, and bcs health_display divides current/max - "NaN%" during the
  death window. Widen the task to also guard max <= 0 (as nova's
  hud/torpedo_target.rs:289 already does).
  - Response: fixed - task 20260716-165617 widened to guard max <= 0 (NaN%) alongside the sub-1% ceil, citing the torpedo_target.rs precedent.
Clean areas verified by the reviewer: no false positives (living ship
cannot sum <= 0; mid-spawn/authored-sectionless roots protected), the
exclusions (torpedo warhead roots, editor preview, asteroids), no
double-fire (bevy Add-observer semantics verified in bevy_ecs source;
marker never removed; broadside gates idempotent), rig honesty (sabotage
A/B: 1 fails / 4 pass, worktree restored clean), deterministic frame
budgets, suites/fmt/check green.

## Round 2

- VERDICT: APPROVE

Verified each response against the new diff:

- R1.1: glue.rs:119 names handle_parent_destroy again with the
  roots-are-never-leaves explanation; NOTES.md's CORRECTION block and the
  close record now match the bcs source (plugin.rs:258). Ticked.
- R1.2: the rig root carries the id components; FiredEvents counts all
  GameEvents with the only-possible-source argument documented in the
  helper's doc comment; exactly-once asserted in three cases; the
  interleave case exists with its recompute-overwrite delivery guard
  (200.0 assert). Rig 6/6 green. Ticked.
- R1.3: the sibling task's Widened section covers max <= 0 -> 0% plus the
  sub-1% ceil. Ticked.

Suites after fixes: ghost rig 6/6, integrity 17/17 (the rig grew one),
fmt clean. The sabotage A/B from round 1 stands (code unchanged since;
only records, rig instrumentation and one new case landed). APPROVED.
