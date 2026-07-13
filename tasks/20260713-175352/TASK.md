# Investigate Entity-despawned command error on menu to game transition

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.5.0,bug

## Goal

The v0.5.0 web console shows, right when Play is hit (menu -> game
transition, before the fatal render error tracked in 20260713-175415):

    WARN Encountered an error in command `<...>`: Entity despawned: The
    entity with ID 891v0 is invalid; its index now has generation 1.

Some system queues a command against a menu-lifetime entity that is despawned
by the time the command applies. Non-fatal, but it is a real lifecycle bug
(or a missing `queue_handled`). Root-cause it and fix or document.

## Steps

- [ ] Reproduce natively: run the editor app, enter Playing from the main
      menu (and from editor Play), with `bevy_ecs::error` logs visible;
      confirm whether the warn fires and on which transition.
- [ ] If it reproduces, bisect the offending system: build with the `debug`
      feature (the error names the command when track_location/debug is on)
      or audit menu/editor teardown (`DespawnOnExit`, manual despawns in
      nova_menu / nova_editor) against systems that hold `Entity` ids across
      the state change.
- [ ] Fix the root cause (correct ordering / stop holding stale ids), or if
      the command is legitimately racy, switch it to `queue_handled` /
      `queue_silenced` with a comment.
- [ ] Add a regression pin where practical (state-transition test asserting
      no command errors, or the specific system's behavior).
- [ ] If it does not reproduce and the trail goes cold, record the evidence
      rig and close as documented-not-reproducible, per the flow guidance
      that a falsification is a legitimate cycle end.
- [ ] CHANGELOG.md entry if a code fix lands.

## Notes

- Generation 1 at index 891 means the entity was spawned and despawned once;
  the reuse happens quickly during the menu teardown / gameplay spawn burst.
- Observed on wasm (Firefox) in the v0.5.0 build; not yet confirmed native.
- Independent of the two render fixes (20260713-175415, 20260713-175416);
  scheduled after them.
