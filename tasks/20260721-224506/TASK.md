# Bug: crash destroying the Rust Tally - damage_tint inserts PendingSectionTint on a chain-destroyed section entity

- STATUS: OPEN
- PRIORITY: 91
- TAGS: v0.8.0, bug, crash, gameplay

## Story

Playtest crash (owner, 2026-07-21): destroying the Rust Tally (Broadside
chapter two gunship) panics the game. The chain-destroy of the gunship's
sections despawns a section leaf, and `damage_tint::mark_section_meshes` then
applies a DEFERRED `insert(PendingSectionTint)` command against that now-dead
entity:

```
handle_chain_destroy: entity 3796v10 became a disabled leaf, destroying
Encountered an error in command insert<PendingSectionTint>: Entity despawned:
  entity 3877v9 is invalid; its index now has generation 10.
Encountered a panic when applying buffers for system
  nova_gameplay::sections::damage_tint::mark_section_meshes!
```

So a section that is queued for tinting in the same frame it is chain-destroyed
makes the buffered insert panic (unhandled EntityCommand error). The explosion
path (`integrity::explode::on_explode_entity` + bcs `handle_chain_destroy`)
races the damage-tint mark.

## Steps

- [x] Reproduce first: a harness test that destroys a multi-section ship (the
      Rust Tally, or a scripted equivalent) and drives `mark_section_meshes`
      with a section entity despawned the same frame - fail-first on the panic.
      Prefer the highest-fidelity rig (an autopilot/scenario walk that kills a
      sectioned ship) per AGENTS harness-first.
- [x] Fix `mark_section_meshes` (and audit sibling damage_tint / section
      systems) to apply the tint insert defensively: skip despawned entities
      (check existence, or use `commands.entity(e).queue_handled(..)` /
      `queue_silenced`, or `get_entity`), so a section chain-destroyed the same
      frame is a graceful no-op, not a panic.
- [x] Check the same class repo-wide: any deferred insert/mutate on a section
      or child entity that a same-frame explode/chain-destroy can invalidate
      (mark by class, not just this one call site).
- [x] Probe the broadside example (kills the Rust Tally) to confirm no panic;
      CHANGELOG Fixes.

## Fix (2026-07-21)

Reproduced fail-first: `tinting_a_section_mesh_chain_destroyed_the_same_frame_does_not_panic`
(damage_tint tests) mirrors the frame order with `chain_ignore_deferred` - a
despawn queued so it applies BEFORE `mark_section_meshes`'s buffer, plus
`FallbackErrorHandler(panic)` to match the game. Pre-fix it panicked with the
EXACT reported error (`insert<PendingSectionTint>: Entity despawned`, same call
site); post-fix it passes.

Fix: `commands.entity(e).insert(..)` uses the panicking command path when `e` is
despawned at apply time; switched the two damage_tint inserts to `try_insert`
(`= queue_silenced(insert)`, the Bevy-documented remedy for exactly this
despawn race, already the repo idiom in `gravity.rs`/`camera_controller.rs`):
- `mark_section_meshes` (the reported call site).
- `resolve_pending_tints` (same class - a pending mesh can be chain-destroyed
  before it resolves; its `.remove` also switched to `try_remove` for a fully
  silent despawned path). `remove` already used `queue_handled(_, warn)`, so it
  never panicked - only `insert` did.

Class audit (mark-by-class step): grepped every `commands.entity(_).insert(` in
`sections/` + `integrity/`. The section-render builders (hull/thruster/turret/
controller `insert(children![..])`) key on section SPAWN (Added render marker),
not the destruction frame, so they do not race an independent chain-destroy. The
integrity observers (`glue.rs` `SectionInactiveMarker`, `explode.rs`
`ExplodableEntity`) act on the just-changed entity inside the orchestrated
chain-destroy; converting them speculatively (no reproduction) risks masking a
real ordering bug (a silently-dropped integrity marker), so they were left as-is
and noted here. Only damage_tint's `Added<Material>`-keyed system races an
independent same-frame destroy - that is the one fixed.

Verified: 6 damage_tint tests pass; broadside probe (real Rust Tally kill) run
to confirm no panic in the shipped path.

## Definition of Done

- Killing the Rust Tally (and any sectioned ship) no longer panics
  (test: harness kill of a sectioned ship with a same-frame tint mark;
  manual: owner replays Broadside and destroys the Rust Tally).
- CHANGELOG entry under Fixes (cmd: `grep -ni "tint\|rust tally\|section" CHANGELOG.md`).

## Notes

- Root is the deferred-command-vs-despawn race (a known Bevy 0.19 hazard: an
  EntityCommand queued this frame can hit an entity despawned later this frame).
  bcs `handle_chain_destroy` disables+destroys leaves; nova's damage_tint marks
  meshes; the two run in the same frame on a dying ship.
