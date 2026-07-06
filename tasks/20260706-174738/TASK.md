# Sections disable but never destroy; ship does not die at zero health

- STATUS: IN_PROGRESS
- PRIORITY: 100
- TAGS: v0.3.1,bug,health


Reported in play: when a spaceship takes damage, some sections (e.g. the controller)
get *disabled* when their health hits zero, but they are never *destroyed* (not removed,
not exploded), and the ship as a whole never "dies" when its health is depleted. So the
destruction pipeline stalls at the disabled stage.

Expected: a section at zero health should be disabled and then destroyed (removed +
exploded via the leaf/chain rules), and when the whole ship is destroyed the ship dies
(player death handling fires, camera reverts, etc.).

Investigate the integrity pipeline (crates/nova_gameplay/src/integrity/): the
disabled -> leaf -> destroy chain (handle_destroy / handle_chain_destroy /
handle_parent_destroy), the IntegrityGraph construction, and the leaf-marker logic. Likely
a Bevy 0.19 behavioral change in an observer/marker or the graph not updating.
