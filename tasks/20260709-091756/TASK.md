# One hit = one cue: dedup HealthApplyDamage propagation in audio + juice

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.4.0,audio,juice,bug,spike

Spike: docs/spikes/20260709-091536-combat-cue-propagation-dedup.md
Source: PR #54 review F1/F3/F4 (docs/reviews/2026-07-09-pr54-combat-juice.md);
supersedes the "key by IntegrityRoot" note in CLOSED task 20260708-215922 and
PR #53 review F3.

## Goal

`HealthApplyDamage` auto-propagates section -> ship root (the game depends on
the bubbling for ship death), and the global cue observers in
`crates/nova_gameplay/src/audio.rs` (`on_damage_play_impact`) and
`crates/nova_gameplay/src/juice.rs` (`on_damage_juice`) fire once per
propagation hop. When the section and root straddle a 6-unit area-cell
boundary, one hit produces two cues: doubled impact sound, 2x camera trauma
(0.16 vs the tuned 0.08), and a phantom flash ring at the ship root's origin
(confirmed empirically in the PR #54 review).

Make one logical hit produce exactly one cue in both modules: guard each
damage-cue observer to react only when `damage.entity ==
damage.original_event_target()` (Bevy 0.19, available on propagating entity
events), leaving the propagation itself untouched. Per the spike, this beats
rekeying the throttle by `IntegrityRoot` (over-collapses distinct hit points,
treats the symptom) and a shared-cue-seam extraction (deferred under the rule
of three until a third cue consumer exists).

In the same change, since it touches the same observers:

- Add observer-level regression tests: a parented hierarchy with the parent
  one cell away must yield a single cue (flash count 1 / single trauma /
  single sound).
- Document the propagation caveat on both observers so a future damage-cue
  observer copies the guard along with the shape.

(Originally this task also carried the attenuation-path observer tests and the
"Three effects" doc-count fix from review findings F3/F4; those landed in the
PR #54 branch itself while addressing the Copilot comments - see the review
addendum - along with flash distance attenuation, F7.)

No Steps yet on purpose - this is a spike-seeded, direction-level task;
`/plan` breaks it down when it is picked up.
