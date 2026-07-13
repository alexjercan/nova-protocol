# Spike: One hit, one cue - dedup HealthApplyDamage propagation in audio + juice

- DATE: 20260709-091536
- STATUS: RECOMMENDED
- TAGS: spike, audio, juice, bug, v0.4.0

## Question

`HealthApplyDamage` auto-propagates up `ChildOf` (section -> ship root), and the
game depends on that bubbling for ship death (`integrity/glue.rs:118`). But the
audio and juice modules attach *global* observers to that event, and a global
observer fires once per propagation hop - so one logical hit can produce two
impact cues (sound, camera kick, flash ring). How should the combat-cue
observers deduplicate propagation so one hit reads as exactly one cue, and does
the fix live per-module or in a shared seam?

A good answer names a concrete mechanism, says where it applies (audio, juice,
or both at once), and settles what happens to the two modules' duplicated
scaffold (throttle + area-cell + listener) now that there are two copies.

## Context

- Confirmed empirically during the PR #54 review (scratch minimal-App test): a
  damaged child whose parent sits one area cell away yields `flashes = 2,
  trauma = 0.16` for a single `HealthApplyDamage` - double the tuned 0.08
  impulse, plus a phantom ring at the ship root's origin. The per-cell throttle
  only collapses the hops when both positions quantize to the same 6-unit cell;
  cells are absolute-aligned and ships move continuously, so whether a hit
  single- or double-fires depends on where the ship happens to sit.
- The same exposure shipped in `audio.rs` (PR #53 review F3, judged MINOR and
  deferred); the "exact fix" noted there - key the throttle by the entity's
  `IntegrityRoot` - was recorded only inside CLOSED task 20260708-215922 and
  never became a task. PR #54 duplicated the behavior into `juice.rs`, which
  is what promoted this from "noted heuristic" to "file it".
- The bubbling itself is load-bearing and must not be stopped: bcs `on_damage`
  relies on the root hop to drive aggregate ship death. Only the *cue*
  observers should ignore non-original hops.
- Bevy 0.19 exposes exactly the needed API: for a propagating entity event,
  `On<E>` provides `original_event_target()` (`bevy_ecs-0.19.0/src/event/
  trigger.rs:235`), the entity the event was originally triggered on, alongside
  the current-hop `event.entity`.
- Damage is always triggered on the collider entity (the section / asteroid
  node - `bcs/src/integrity/plugin.rs:133,197`), so the original target is also
  the *better* cue position: the actual hit location, not the body origin.

## Options considered

- **A. Original-target guard in each cue observer.** One line at the top of
  `on_damage_play_impact` (audio) and `on_damage_juice` (juice):
  `if damage.entity != damage.original_event_target() { return; }`.
  Pros: fixes the cause (hop re-entry), not the symptom; keeps the per-cell
  throttle doing the job it is actually good at (collapsing spatially co-located
  bursts, e.g. a blast damaging a dozen colliders); cue position becomes
  consistently the hit location; trivially testable with a parented spawn.
  Cons: two call sites to guard today and any future damage-cue observer must
  remember the same guard (mitigated by a doc comment on the seam and by the
  hierarchy regression test).
- **B. Key the throttle by `IntegrityRoot`** (the fix sketched in 215922's
  notes). Resolve the damaged entity's root and throttle per `(kind, root)`.
  Pros: also collapses the hops; additionally collapses multi-point bursts on
  one ship regardless of cell geometry. Cons: over-collapses - two genuinely
  distinct hit locations on a large ship (nose and engine struck by different
  guns within 40ms) become one cue, which is the wrong feel; requires an
  ancestor/root resolution in every observer; and it still fires the observer
  body per hop, just suppressing output - the root hop would win or lose the
  throttle race against the section hop nondeterministically by observer order.
  Treats the symptom.
- **C. Extract a shared combat-cue seam now.** One module owns the two
  observers, dedups propagation once, resolves the position once, and emits a
  unified `CombatCue { kind, pos }` that audio and juice (and future rumble /
  HUD damage direction / hit-stop) consume. Pros: fix lives in exactly one
  place; kills the ~80-line scaffold duplication (throttle map, `area_cell`,
  `listener_position`); a third consumer becomes one observer instead of a
  third copy. Cons: the duplication is at two copies - under the rule of
  three - and the two modules genuinely differ where it matters (attenuation
  curves: geometric for loudness vs smoothstep for shake; throttle intervals;
  audio keys turret fire by entity, not cell), so the shared core is smaller
  than it looks; doing it now is a cross-cutting refactor of two shipped
  modules to save two one-line guards.
- **Do nothing.** The precedent (audio F3 deferred). Cost: the inconsistency
  now affects a *visual* effect where a phantom ring at the ship origin is far
  more noticeable than a doubled quiet impact sound, and the "deliberately
  subtle" shake tuning is silently 2x on cell-straddling hits - the feel work
  from the playtest iteration is being randomly undone.

## Recommendation

**A, applied to both modules in one task.** Guard both damage-cue observers
with `original_event_target()`, in the same change, with an observer-level
regression test that spawns a parented hierarchy (the exact scratch test from
the review: parent one cell away, assert one flash / single trauma) and its
audio equivalent. While in those observers, add the two missing
attenuation-path tests (camera past `far_distance` -> no cue *and* no throttle
stamp; mid-ramp camera -> scaled trauma) and fix the stale "Three effects"
count in the juice module doc.

B is rejected as symptom-treatment with worse feel semantics. C is deferred,
not rejected: the moment a third cue consumer appears (rumble, HUD damage
direction, hit-stop - all named in the roadmap spike's juice dimensions), the
extraction is justified and this spike plus the guards mark exactly where the
seam is; a note to that effect belongs in the promotion catalog
(`tasks/20260708-110317/SPIKE.md` territory) rather
than a speculative refactor now.

Related but separate: the "first `Camera3d`" listener fragility got three new
call sites in juice; that stays with existing OPEN task 20260708-224254, whose
scope was extended to cover `juice.rs` as part of the PR #54 review (F2). It
is one refactor over the same files and needs no further research.

## Open questions

- Contact-point cue sourcing (PR #54 R1.3, left unfiled by the retro on
  purpose): the collision manifold would give the true spark position instead
  of the section transform. Unblocked by A (which standardizes on the original
  target's transform); becomes worthwhile only with bigger sections or visible
  misalignment. Deliberately still unfiled.
- Whether `ensure_camera_shake` should be state-gated (it currently attaches
  `CameraShake` to editor cameras too - harmless at zero trauma, but the
  restore/apply pair then writes those transforms every frame). Fold into
  224254's step about marking the gameplay camera if it turns out to matter.

## Next steps

Direction-level task this spike seeded, for `/plan` to break into steps:

- tatr 20260709-091756: one hit = one cue - dedup HealthApplyDamage
  propagation in audio + juice observers (guard + hierarchy/attenuation
  regression tests + juice doc count fix).
- tatr 20260708-224254 (existing, scope extended): robust listener/camera
  marker now also covers the juice call sites.
