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

- [ ] Reproduce first: a harness test that destroys a multi-section ship (the
      Rust Tally, or a scripted equivalent) and drives `mark_section_meshes`
      with a section entity despawned the same frame - fail-first on the panic.
      Prefer the highest-fidelity rig (an autopilot/scenario walk that kills a
      sectioned ship) per AGENTS harness-first.
- [ ] Fix `mark_section_meshes` (and audit sibling damage_tint / section
      systems) to apply the tint insert defensively: skip despawned entities
      (check existence, or use `commands.entity(e).queue_handled(..)` /
      `queue_silenced`, or `get_entity`), so a section chain-destroyed the same
      frame is a graceful no-op, not a panic.
- [ ] Check the same class repo-wide: any deferred insert/mutate on a section
      or child entity that a same-frame explode/chain-destroy can invalidate
      (mark by class, not just this one call site).
- [ ] Probe the broadside example (kills the Rust Tally) to confirm no panic;
      CHANGELOG Fixes.

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
