# Design the destructible-graph seam for promoting integrity to bevy-common-systems

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: spike, v0.4.0, crates, refactor

Goal: the integrity destruction pipeline (`integrity/plugin.rs` + `components.rs` +
`blast.rs`, and `integrity/explode.rs`) is game-agnostic enough to promote to
bevy-common-systems, but it is currently entangled with two nova-specific seams:
`integrity/glue.rs` builds the graph from the ship section grid, and `explode.rs` fires
`nova_events::OnDestroyedEvent`. Before the code can move (task 20260706-151804), design
the seam that lets the promoted module stay generic while nova plugs its ship logic in:

- a graph-builder API / trait (or data-driven `ConnectedTo` fill) that nova's section grid
  implements, replacing the hardcoded "sections one unit apart" adjacency;
- a generic "entity destroyed" trigger the promoted pipeline emits, that nova maps onto
  `OnDestroyedEvent` - check whether bcs already has a suitable event surface first;
- where the bundle lands (`physics/` vs a new `destructible/` module).

This is a design task (a follow-up spike is fine); it blocks the Tier-B/C portion of the
cross-repo move.

## Notes

Spike: tasks/20260708-110317/SPIKE.md (Tier B / Tier C and the
Open questions). Downstream move: task 20260706-151804. Catalog + markers: task
20260707-095020.
