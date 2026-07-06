# Fix TODOs across the codebase

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Sweep all TODO comments, resolve them or convert into tatr tickets. Legacy #160.

## Resolution (CLOSED)

Swept all 22 TODO/FIXME comments in crates/, src/ and examples/. None were trivially
resolvable in place (they are design decisions, enhancements, or known limitations), so
each was converted into a tracked ticket and the code comment annotated with the ticket
id: `// TODO(<id>): ...`. A future reader can now jump from any TODO to its ticket.

New v0.4.0 tickets created for the uncovered substantive TODOs:
- 20260706-162908 - Re-enable particle effects on wasm (plugin.rs, torpedo_section.rs,
  turret_section.rs x2)
- 20260706-162909 - Use inertia tensor for projectile muzzle velocity (torpedo/turret)
- 20260706-162911 - Refactor integrity plugin: graph via relations, split glue systems
  (integrity/plugin.rs x3)
- 20260706-162912 - OnDestroyed event fires inconsistently (integrity/plugin.rs FIXME)
- 20260706-162913 - Extract torpedo into its own module/plugin; unhardcode blast params
  (torpedo_section.rs x3, plus player.rs "don't just despawn the torpedo")

TODOs mapped to already-existing tickets (annotated, no new ticket):
- input/player.rs "NEED TO REFACTOR" -> 20260525-132943 (improve input system)
- input/player.rs target selection -> 20260525-133018 (torpedo follows target)
- input/player.rs + hud/torpedo_target.rs targeting HUD -> 20260525-133022 (torpedo HUD)
- torpedo_section.rs explosion visuals -> 20260525-133023 (blast radius visual)
- integrity/plugin.rs generic blast/impact -> 20260706-151804 (promote to bcs)
- nova_assets/lib.rs "refactor this" (sections/scenarios hardcoded in Rust) ->
  20260525-133028 (scenario config resource)

Left un-ticketed (intentional): examples/07b_slicer.rs "move this to bevy_common_systems
as a small game" - an aspiration for the external crate, not this repo's backlog.

Verified: build --features dev clean, fmt clean (comment-only changes).

Self-reflection: "fix all TODOs" almost never means fix - it means make them tracked.
Converting to tickets + back-annotating the code is the honest interpretation; resisting
the urge to actually implement 5 substantive features inline kept this task bounded. One
gotcha: tatr uses second-granularity ids, so creating tickets in a tight loop collides -
had to space them out.
